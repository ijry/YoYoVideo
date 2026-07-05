use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{MediaLocator, StorageError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub locator: MediaLocator,
    pub title: String,
    pub last_position_seconds: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoryStore {
    pub items: Vec<HistoryEntry>,
}

impl HistoryStore {
    pub fn items(&self) -> &[HistoryEntry] {
        &self.items
    }

    pub fn entry(&self, index: usize) -> Option<&HistoryEntry> {
        self.items.get(index)
    }

    pub fn remember(
        &mut self,
        locator: MediaLocator,
        title: String,
        last_position_seconds: Option<f64>,
    ) {
        let last_position_seconds =
            last_position_seconds.filter(|seconds| seconds.is_finite() && *seconds > 0.0);

        self.items.retain(|item| item.locator != locator);
        self.items.insert(0, HistoryEntry { locator, title, last_position_seconds });
    }

    pub fn load(path: &Path) -> Result<Self, StorageError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}
