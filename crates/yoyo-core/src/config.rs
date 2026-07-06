use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{ShortcutMap, StorageError, ValidationError};

pub const MIN_DEFAULT_SPEED: f32 = 0.25;
pub const MAX_DEFAULT_SPEED: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaybackEndBehavior {
    PlayNext,
    Stop,
    LoopCurrent,
    LoopPlaylist,
}

impl Default for PlaybackEndBehavior {
    fn default() -> Self {
        Self::PlayNext
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackDefaults {
    pub default_speed: f32,
    pub default_volume_percent: u8,
    pub prefer_hardware_decode: bool,
    #[serde(default)]
    pub end_behavior: PlaybackEndBehavior,
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
                end_behavior: PlaybackEndBehavior::PlayNext,
            },
            ui: UiPreferences { remember_history: true, show_playlist_on_startup: true },
            shortcuts: ShortcutMap::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !self.playback.default_speed.is_finite()
            || self.playback.default_speed < MIN_DEFAULT_SPEED
            || self.playback.default_speed > MAX_DEFAULT_SPEED
        {
            return Err(ValidationError::InvalidConfig(format!(
                "default speed must be within {MIN_DEFAULT_SPEED:.2}x..={MAX_DEFAULT_SPEED:.2}x"
            )));
        }

        if self.playback.default_volume_percent > 100 {
            return Err(ValidationError::InvalidConfig(
                "default volume must be within 0..=100".to_string(),
            ));
        }

        Ok(())
    }

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
