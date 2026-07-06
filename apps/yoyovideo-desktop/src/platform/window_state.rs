use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::StorageError;

use super::AppPaths;

pub const MIN_WINDOW_WIDTH: u32 = 900;
pub const MIN_WINDOW_HEIGHT: u32 = 560;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

impl WindowState {
    pub fn clamped(self) -> Self {
        Self {
            width: self.width.max(MIN_WINDOW_WIDTH),
            height: self.height.max(MIN_WINDOW_HEIGHT),
            x: self.x,
            y: self.y,
            maximized: self.maximized,
        }
    }
}

pub fn window_state_path(paths: Option<&AppPaths>) -> Option<PathBuf> {
    paths.map(|paths| paths.config_dir.join("window-state.toml"))
}

pub fn load_window_state(path: Option<PathBuf>) -> Result<Option<WindowState>, StorageError> {
    let Some(path) = path else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    Ok(toml::from_str::<WindowState>(&raw).ok().map(WindowState::clamped))
}

pub fn save_window_state(path: Option<PathBuf>, state: &WindowState) -> Result<(), StorageError> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(state)?;
    fs::write(path, raw)?;
    Ok(())
}
