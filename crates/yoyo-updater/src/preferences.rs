use crate::UpdateError;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatePreferences {
    pub automatic_check: bool,
    pub last_checked_at: Option<i64>,
}
impl Default for UpdatePreferences {
    fn default() -> Self {
        Self { automatic_check: true, last_checked_at: None }
    }
}
impl UpdatePreferences {
    pub fn should_check(&self, now: i64, manual: bool) -> bool {
        if manual {
            return true;
        }
        if !self.automatic_check {
            return false;
        }
        self.last_checked_at.is_none_or(|last| now < last || now.saturating_sub(last) >= 86400)
    }
    pub fn load(path: &Path) -> Result<Self, UpdateError> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let mut text = String::new();
        file.take(64 * 1024 + 1).read_to_string(&mut text)?;
        if text.len() > 64 * 1024 {
            return Err(UpdateError::Preferences);
        }
        toml::from_str(&text).map_err(|_| UpdateError::Preferences)
    }
    pub fn save(&self, path: &Path) -> Result<(), UpdateError> {
        let parent =
            path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let raw = toml::to_string(self).map_err(|_| UpdateError::Preferences)?;
        let mut file = NamedTempFile::new_in(parent)?;
        file.write_all(raw.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| UpdateError::Io(e.error))?;
        Ok(())
    }
}
