use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;

use super::AppPaths;

pub fn default_log_file(paths: Option<&AppPaths>) -> PathBuf {
    paths
        .map(|paths| paths.data_dir.join("logs").join("yoyovideo.log"))
        .unwrap_or_else(|| PathBuf::from("logs").join("yoyovideo.log"))
}

pub fn diagnostic_timestamp_now() -> String {
    Local::now().to_rfc3339()
}

pub fn append_diagnostic_line(
    path: &Path,
    timestamp: &str,
    level: &str,
    message: &str,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let sanitized = message.replace(['\r', '\n'], " ");
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{timestamp} {level} {sanitized}")?;
    Ok(())
}

pub fn append_diagnostic(
    paths: Option<&AppPaths>,
    level: &str,
    message: &str,
) -> Result<PathBuf, std::io::Error> {
    let path = default_log_file(paths);
    append_diagnostic_line(&path, &diagnostic_timestamp_now(), level, message)?;
    Ok(path)
}
