use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::MediaLocator;

/// Privacy-only identity. Never substitute this for existing history/marker keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MediaKey(String);

impl MediaKey {
    pub fn from_locator(locator: &MediaLocator) -> Result<Self, String> {
        match locator {
            MediaLocator::File(path) => {
                let path = normalized_file_path(path)?;
                let url = Url::from_file_path(path).map_err(|_| "Invalid media identity")?;
                Ok(Self(url.to_string()))
            }
            MediaLocator::Url(value) => {
                let mut url = Url::parse(value).map_err(|_| "Invalid media identity")?;
                if !matches!(url.scheme(), "http" | "https" | "rtsp" | "rtmp")
                    || url.host_str().is_none()
                {
                    return Err("Invalid media identity".into());
                }
                let host = url.host_str().unwrap().to_ascii_lowercase();
                url.set_host(Some(&host)).map_err(|_| "Invalid media identity")?;
                let default_port = match url.scheme() {
                    "http" => 80,
                    "https" => 443,
                    "rtsp" => 554,
                    _ => 1935,
                };
                if url.port() == Some(default_port) {
                    url.set_port(None).map_err(|_| "Invalid media identity")?;
                }
                Ok(Self(url.to_string()))
            }
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalized_file_path(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("Invalid media identity".into());
    }
    // Resolve real aliases first: lexical '..' must not change symlink semantics.
    let absolute = std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .map_err(|_| "Invalid media identity")?;
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    #[cfg(windows)]
    {
        // Canonicalize uses the extended-length prefix. The fallback for a missing
        // file does not; give both spellings the same key (including UNC shares).
        let value = normalized.to_str().ok_or("Invalid media identity")?;
        let value = if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            value.strip_prefix(r"\\?\").unwrap_or(value).to_owned()
        };
        normalized = PathBuf::from(value.to_lowercase());
    }
    Ok(normalized)
}
