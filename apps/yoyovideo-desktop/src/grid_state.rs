use std::path::PathBuf;

use yoyo_core::MediaLocator;

use crate::grid_layout::{DEFAULT_ASPECT, MAX_GRID_TILES};

/// What startup arguments should open.
#[derive(Debug, Clone, PartialEq)]
pub enum StartupOpen {
    Nothing,
    /// One file plays in the normal single-video surface.
    Single(MediaLocator),
    /// Several files open as a grid, which is what batch playback is for.
    Grid(Vec<MediaLocator>),
}

/// Decides what `yoyovideo-desktop <paths...>` should do.
///
/// Existing files only, so a stray flag or a deleted path cannot open a broken tile.
/// More than [`MAX_GRID_TILES`] is truncated.
pub fn plan_startup_open(paths: Vec<PathBuf>) -> StartupOpen {
    let mut usable: Vec<MediaLocator> =
        paths.into_iter().filter(|path| path.is_file()).map(MediaLocator::File).collect();

    match usable.len() {
        0 => StartupOpen::Nothing,
        1 => StartupOpen::Single(usable.remove(0)),
        _ => {
            usable.truncate(MAX_GRID_TILES);
            StartupOpen::Grid(usable)
        }
    }
}

/// Splits a batch of requested tiles into what fits and what has to be dropped.
///
/// Returns `(accepted, dropped)`. The dropped count is reported rather than swallowed so
/// the caller can tell the user some files were left out.
pub fn accepted_tile_count(existing: usize, requested: usize) -> (usize, usize) {
    let capacity = MAX_GRID_TILES.saturating_sub(existing);
    let accepted = requested.min(capacity);
    (accepted, requested - accepted)
}

/// Where the selection lands after the tile at `removed` is closed.
///
/// `len_before` is the tile count before removal. Keeps the same tile selected when one
/// ahead of it disappears, and never points past the end.
pub fn active_after_removal(
    active: Option<usize>,
    removed: usize,
    len_before: usize,
) -> Option<usize> {
    let active = active?;
    let len_after = len_before.saturating_sub(1);
    if len_after == 0 {
        return None;
    }

    let shifted = if active > removed { active - 1 } else { active };
    // Removing the last tile while it was selected leaves the index out of range.
    Some(shifted.min(len_after - 1))
}

/// Aspect ratio for a tile, falling back to [`DEFAULT_ASPECT`] until mpv reports a size.
pub fn aspect_from_size(width: Option<u32>, height: Option<u32>) -> f32 {
    match (width, height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => width as f32 / height as f32,
        _ => DEFAULT_ASPECT,
    }
}
