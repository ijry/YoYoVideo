use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use yoyo_core::{AppCommand, MediaLocator, PlayerState, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitlePrefsFlushReason {
    PeriodicTick,
    MediaSwitch,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleRestoreError {
    MissingExternalSubtitle(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePreferenceEntry {
    pub locator: MediaLocator,
    pub selected_audio_track_id: Option<i64>,
    pub selected_subtitle_track_id: Option<i64>,
    pub selected_video_track_id: Option<i64>,
    pub subtitle_visible: bool,
    pub external_subtitle_path: Option<PathBuf>,
    pub subtitle_delay_seconds: f64,
    pub subtitle_scale: f32,
    pub subtitle_vertical_position: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePreferenceStore {
    pub items: Vec<SubtitlePreferenceEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleRestorePlan {
    pub commands: Vec<AppCommand>,
}

pub struct SubtitlePrefsRuntime {
    path: Option<PathBuf>,
    store: SubtitlePreferenceStore,
    dirty: bool,
    last_flush_at: Option<Duration>,
}

impl SubtitlePrefsRuntime {
    pub fn new(path: Option<PathBuf>, store: SubtitlePreferenceStore) -> Self {
        Self { path, store, dirty: false, last_flush_at: None }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let store = match path.as_ref() {
            Some(path) if path.exists() => serde_json::from_str(&fs::read_to_string(path)?)?,
            _ => SubtitlePreferenceStore::default(),
        };
        Ok(Self::new(path, store))
    }

    pub fn remember_from_state(&mut self, state: &PlayerState) {
        let Some(locator) = state.current.clone() else {
            return;
        };

        let entry = SubtitlePreferenceEntry {
            locator: locator.clone(),
            selected_audio_track_id: state.selected_audio_track_id(),
            selected_subtitle_track_id: state.selected_subtitle_track_id(),
            selected_video_track_id: state.selected_video_track_id(),
            subtitle_visible: state.subtitle.visible,
            external_subtitle_path: state.subtitle.external_path.clone(),
            subtitle_delay_seconds: state.subtitle.delay_seconds,
            subtitle_scale: state.subtitle.scale,
            subtitle_vertical_position: state.subtitle.vertical_position_percent,
        };

        self.store.items.retain(|item| item.locator != locator);
        self.store.items.insert(0, entry);
        self.dirty = true;
    }

    pub fn restore_plan_for(
        &self,
        locator: &MediaLocator,
    ) -> Result<Option<SubtitleRestorePlan>, SubtitleRestoreError> {
        let Some(entry) = self.store.items.iter().find(|item| &item.locator == locator) else {
            return Ok(None);
        };

        if let Some(path) = &entry.external_subtitle_path {
            if !path.exists() {
                return Err(SubtitleRestoreError::MissingExternalSubtitle(path.clone()));
            }
        }

        let mut commands = Vec::new();
        if let Some(path) = &entry.external_subtitle_path {
            commands.push(AppCommand::LoadExternalSubtitle(path.clone()));
        }
        if let Some(id) = entry.selected_audio_track_id {
            commands.push(AppCommand::SelectAudioTrack(id));
        }
        if let Some(id) = entry.selected_video_track_id {
            commands.push(AppCommand::SelectVideoTrack(id));
        }
        if entry.external_subtitle_path.is_none() {
            if let Some(id) = entry.selected_subtitle_track_id {
                commands.push(AppCommand::SelectSubtitleTrack(id));
            }
        }
        commands.push(AppCommand::SetSubtitleVisible(entry.subtitle_visible));
        commands.push(AppCommand::SetSubtitleDelay(entry.subtitle_delay_seconds));
        commands.push(AppCommand::SetSubtitleScale(entry.subtitle_scale));
        commands.push(AppCommand::SetSubtitleVerticalPosition(entry.subtitle_vertical_position));

        Ok(Some(SubtitleRestorePlan { commands }))
    }

    pub fn flush_if_needed(
        &mut self,
        now: Duration,
        reason: SubtitlePrefsFlushReason,
    ) -> Result<bool, StorageError> {
        if !self.dirty {
            return Ok(false);
        }

        if matches!(reason, SubtitlePrefsFlushReason::PeriodicTick)
            && self.last_flush_at.is_some_and(|last| now < last + Duration::from_secs(2))
        {
            return Ok(false);
        }

        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, serde_json::to_string_pretty(&self.store)?)?;
        }

        self.dirty = false;
        self.last_flush_at = Some(now);
        Ok(true)
    }
}
