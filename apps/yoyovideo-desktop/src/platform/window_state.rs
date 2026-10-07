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

/// A connected monitor's physical-pixel rectangle. Callers put the preferred
/// (normally primary) display first. Do not use the union: monitor gaps are not
/// usable desktop space, and valid secondary monitors can have negative origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

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
    /// Restore into one of the currently connected displays. Persistence alone
    /// cannot validate a position: a once-valid monitor may have been unplugged.
    pub fn restored_on(self, displays: &[DisplayBounds]) -> Self {
        let mut state = self.clamped();
        let usable = || displays.iter().filter(|d| d.width > 0 && d.height > 0);
        let Some(fallback) = usable().next() else {
            // Wayland/headless backends may not expose global monitor geometry.
            // Leave positioning to the compositor rather than reuse stale coords.
            state.x = None;
            state.y = None;
            return state;
        };
        let overlap = |display: &&DisplayBounds| -> i64 {
            let (Some(x), Some(y)) = (state.x, state.y) else {
                return 0;
            };
            let right = (i64::from(x) + i64::from(state.width))
                .min(i64::from(display.x) + i64::from(display.width));
            let bottom = (i64::from(y) + i64::from(state.height))
                .min(i64::from(display.y) + i64::from(display.height));
            let width = (right - i64::from(x).max(i64::from(display.x))).max(0);
            let height = (bottom - i64::from(y).max(i64::from(display.y))).max(0);
            width.saturating_mul(height)
        };
        let current = usable().filter(|d| overlap(d) > 0).max_by_key(overlap);
        let display = current.unwrap_or(fallback);
        state.width = state.width.min(display.width);
        state.height = state.height.min(display.height);
        let min_x = i64::from(display.x);
        let min_y = i64::from(display.y);
        let max_x = min_x + i64::from(display.width - state.width);
        let max_y = min_y + i64::from(display.height - state.height);
        let (x, y) = if current.is_some() {
            (
                i64::from(state.x.unwrap()).clamp(min_x, max_x),
                i64::from(state.y.unwrap()).clamp(min_y, max_y),
            )
        } else {
            // An unreachable/disconnected position gets a predictable placement.
            (min_x + (max_x - min_x) / 2, min_y + (max_y - min_y) / 2)
        };
        state.x = Some(i32::try_from(x).unwrap_or(display.x));
        state.y = Some(i32::try_from(y).unwrap_or(display.y));
        state
    }

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
