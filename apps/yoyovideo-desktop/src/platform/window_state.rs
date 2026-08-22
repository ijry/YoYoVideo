use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::StorageError;

use super::AppPaths;

pub const MIN_WINDOW_WIDTH: u32 = 900;
pub const MIN_WINDOW_HEIGHT: u32 = 560;

/// Windows reports a minimized window at (-32000, -32000). Persisting that would
/// reopen the window off-screen, so treat coordinates beyond this as unusable.
const MIN_ONSCREEN_COORDINATE: i32 = -30000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

fn onscreen_coordinate(value: Option<i32>) -> Option<i32> {
    value.filter(|value| *value > MIN_ONSCREEN_COORDINATE)
}

impl WindowState {
    pub fn clamped(self) -> Self {
        // Drop the position entirely when either axis is off-screen: keeping only one
        // axis would still place the window somewhere the user cannot reach it.
        let position = match (onscreen_coordinate(self.x), onscreen_coordinate(self.y)) {
            (Some(x), Some(y)) => (Some(x), Some(y)),
            _ => (None, None),
        };

        Self {
            width: self.width.max(MIN_WINDOW_WIDTH),
            height: self.height.max(MIN_WINDOW_HEIGHT),
            x: position.0,
            y: position.1,
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
