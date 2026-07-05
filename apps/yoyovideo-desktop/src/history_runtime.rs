use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use yoyo_core::{AppCommand, HistoryStore, MediaLocator, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushReason {
    PeriodicTick,
    Pause,
    MediaSwitch,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingResumeSeek {
    target_seconds: f64,
}

impl PendingResumeSeek {
    pub fn new(target_seconds: f64) -> Option<Self> {
        (target_seconds.is_finite() && target_seconds > 0.0).then_some(Self { target_seconds })
    }

    pub fn try_resolve(&self, duration_seconds: Option<f64>) -> Option<f64> {
        duration_seconds
            .filter(|duration| *duration > 0.0)
            .map(|duration| self.target_seconds.clamp(0.0, duration))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryActivation {
    pub command: AppCommand,
    pub pending_seek: Option<PendingResumeSeek>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryActivationError {
    MissingLocalFile(PathBuf),
}

pub struct HistoryRuntime {
    path: Option<PathBuf>,
    store: HistoryStore,
    enabled: bool,
    dirty: bool,
    last_flush_at: Option<Duration>,
}

impl HistoryRuntime {
    pub fn new(path: Option<PathBuf>, store: HistoryStore, enabled: bool) -> Self {
        Self { path, store, enabled, dirty: false, last_flush_at: None }
    }

    pub fn load(path: Option<PathBuf>, enabled: bool) -> Result<Self, StorageError> {
        let store = if enabled {
            match path.as_ref() {
                Some(path) => HistoryStore::load(path)?,
                None => HistoryStore::default(),
            }
        } else {
            HistoryStore::default()
        };

        Ok(Self::new(path, store, enabled))
    }

    pub fn store(&self) -> &HistoryStore {
        &self.store
    }

    pub fn remember_playback(
        &mut self,
        locator: &MediaLocator,
        title: &str,
        position_seconds: Option<f64>,
    ) {
        if !self.enabled {
            return;
        }

        let normalized_position =
            position_seconds.filter(|seconds| seconds.is_finite() && *seconds > 0.0);
        let unchanged = self.store.entry(0).is_some_and(|entry| {
            entry.locator == *locator
                && entry.title == title
                && entry.last_position_seconds == normalized_position
        });

        self.store
            .remember(locator.clone(), title.to_string(), normalized_position);
        self.dirty |= !unchanged;
    }

    pub fn activation_for(
        &self,
        index: usize,
    ) -> Result<Option<HistoryActivation>, HistoryActivationError> {
        let Some(entry) = self.store.entry(index) else {
            return Ok(None);
        };

        let command = match &entry.locator {
            MediaLocator::File(path) => {
                if !path.exists() {
                    return Err(HistoryActivationError::MissingLocalFile(path.clone()));
                }
                AppCommand::OpenFile(path.clone())
            }
            MediaLocator::Url(url) => AppCommand::OpenUrl(url.clone()),
        };

        Ok(Some(HistoryActivation {
            command,
            pending_seek: entry.last_position_seconds.and_then(PendingResumeSeek::new),
        }))
    }

    pub fn flush_if_needed(
        &mut self,
        now: Duration,
        reason: FlushReason,
    ) -> Result<bool, StorageError> {
        if !self.enabled || !self.dirty {
            return Ok(false);
        }

        if matches!(reason, FlushReason::PeriodicTick)
            && self
                .last_flush_at
                .is_some_and(|last| now < last + Duration::from_secs(2))
        {
            return Ok(false);
        }

        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            self.store.save(path)?;
        }

        self.dirty = false;
        self.last_flush_at = Some(now);
        Ok(true)
    }
}
