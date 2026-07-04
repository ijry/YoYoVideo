use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::ValidationError;

const SUPPORTED_EXTENSIONS: &[&str] =
    &["mp4", "mkv", "avi", "mov", "webm", "mp3", "flac", "wav", "m4a", "ts", "m2ts"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaLocator {
    File(PathBuf),
    Url(String),
}

impl MediaLocator {
    pub fn from_url(input: &str) -> Result<Self, ValidationError> {
        let parsed =
            Url::parse(input).map_err(|_| ValidationError::InvalidUrl(input.to_string()))?;
        match parsed.scheme() {
            "http" | "https" | "rtsp" | "rtmp" => Ok(Self::Url(input.to_string())),
            other => Err(ValidationError::UnsupportedUrlScheme(other.to_string())),
        }
    }

    pub fn is_supported_local_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
    }

    pub fn as_label(&self) -> String {
        match self {
            Self::File(path) => path.display().to_string(),
            Self::Url(url) => url.clone(),
        }
    }
}
