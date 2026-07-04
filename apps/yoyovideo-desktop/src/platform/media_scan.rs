use std::fs;
use std::path::Path;

use yoyo_core::{AppError, MediaLocator, PlaylistEntry};

pub fn scan_media_folder(path: &Path) -> Result<Vec<PlaylistEntry>, AppError> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(path).map_err(|error| AppError::Message(error.to_string()))? {
        let entry = entry.map_err(|error| AppError::Message(error.to_string()))?;
        let candidate = entry.path();
        if candidate.is_file() && MediaLocator::is_supported_local_path(&candidate) {
            let title = candidate
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| candidate.display().to_string());
            entries.push(PlaylistEntry {
                locator: MediaLocator::File(candidate),
                title,
            });
        }
    }

    entries.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(entries)
}
