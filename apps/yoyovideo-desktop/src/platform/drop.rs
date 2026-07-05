use std::path::PathBuf;

use yoyo_core::{AppError, MediaLocator, PlaylistEntry};

use super::scan_media_folder;

#[derive(Debug, Clone, PartialEq)]
pub enum DroppedMediaAction {
    NoPlayableMedia { ignored_count: usize },
    OpenFile(PathBuf),
    ReplacePlaylist(Vec<PlaylistEntry>),
}

pub fn classify_dropped_paths(paths: &[PathBuf]) -> Result<DroppedMediaAction, AppError> {
    let mut entries = Vec::new();
    let mut ignored_count = 0usize;
    let mut saw_folder = false;

    for path in paths {
        if path.is_dir() {
            saw_folder = true;
            let scanned = scan_media_folder(path)?;
            if scanned.is_empty() {
                ignored_count += 1;
            } else {
                entries.extend(scanned);
            }
        } else if path.is_file() && MediaLocator::is_supported_local_path(path) {
            entries.push(PlaylistEntry::new(MediaLocator::File(path.clone())));
        } else {
            ignored_count += 1;
        }
    }

    match entries.len() {
        0 => Ok(DroppedMediaAction::NoPlayableMedia { ignored_count }),
        1 if !saw_folder => match &entries[0].locator {
            MediaLocator::File(path) => Ok(DroppedMediaAction::OpenFile(path.clone())),
            MediaLocator::Url(_) => Ok(DroppedMediaAction::ReplacePlaylist(entries)),
        },
        _ => Ok(DroppedMediaAction::ReplacePlaylist(entries)),
    }
}
