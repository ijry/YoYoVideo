use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use directories::UserDirs;

use super::AppPaths;

const SCREENSHOT_DIR_NAME: &str = "YoYoVideo Screenshots";

pub fn default_screenshot_dir(paths: Option<&AppPaths>) -> PathBuf {
    if let Some(pictures) =
        UserDirs::new().and_then(|dirs| dirs.picture_dir().map(Path::to_path_buf))
    {
        return pictures.join(SCREENSHOT_DIR_NAME);
    }

    paths
        .map(|paths| paths.data_dir.join("screenshots"))
        .unwrap_or_else(|| PathBuf::from("screenshots"))
}

pub fn screenshot_timestamp_now() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn next_screenshot_path(dir: &Path, timestamp: &str) -> PathBuf {
    let first = dir.join(format!("yoyovideo-{timestamp}.png"));
    if !first.exists() {
        return first;
    }

    for suffix in 1..=9999 {
        let candidate = dir.join(format!("yoyovideo-{timestamp}-{suffix}.png"));
        if !candidate.exists() {
            return candidate;
        }
    }

    dir.join(format!("yoyovideo-{timestamp}-overflow.png"))
}

pub fn prepare_screenshot_path_in_dir(
    dir: &Path,
    timestamp: &str,
) -> Result<PathBuf, std::io::Error> {
    fs::create_dir_all(dir)?;
    Ok(next_screenshot_path(dir, timestamp))
}

pub fn prepare_screenshot_path(paths: Option<&AppPaths>) -> Result<PathBuf, std::io::Error> {
    let dir = default_screenshot_dir(paths);
    prepare_screenshot_path_in_dir(&dir, &screenshot_timestamp_now())
}
