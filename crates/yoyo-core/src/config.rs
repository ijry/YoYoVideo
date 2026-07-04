use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{ShortcutMap, StorageError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackDefaults {
    pub default_speed: f32,
    pub default_volume_percent: u8,
    pub prefer_hardware_decode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiPreferences {
    pub remember_history: bool,
    pub show_playlist_on_startup: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub playback: PlaybackDefaults,
    pub ui: UiPreferences,
    pub shortcuts: ShortcutMap,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            playback: PlaybackDefaults {
                default_speed: 1.0,
                default_volume_percent: 100,
                prefer_hardware_decode: true,
            },
            ui: UiPreferences { remember_history: true, show_playlist_on_startup: true },
            shortcuts: ShortcutMap::default(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, StorageError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}
