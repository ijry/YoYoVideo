use super::{PinCredential, PrivacyError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};
use yoyo_core::{
    MediaLocator,
    privacy::{ManualPrivacy, MediaKey, PrivacySchedule},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyDocument {
    pub schema_version: u32,
    pub revision: u64,
    pub credential: Option<PinCredential>,
    pub failed_attempts: u8,
    pub blocked_until: Option<DateTime<Utc>>,
    pub schedule: PrivacySchedule,
    pub manual: Option<ManualPrivacy>,
    pub protected: BTreeMap<MediaKey, MediaLocator>,
}
impl Default for PrivacyDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            credential: None,
            failed_attempts: 0,
            blocked_until: None,
            schedule: PrivacySchedule::default(),
            manual: None,
            protected: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct PrivacyStore {
    path: PathBuf,
}
const MAX_DOCUMENT_BYTES: usize = 1_048_576;
impl PrivacyDocument {
    pub(crate) fn validate(&self) -> Result<(), PrivacyError> {
        if self.schema_version != 1
            || self.failed_attempts > 5
            || self.protected.len() > 10_000
            || self.schedule.validate().is_err()
        {
            return Err(PrivacyError::CorruptConfig);
        }
        match &self.credential {
            Some(credential) => credential.validate()?,
            None if !self.protected.is_empty()
                || self.schedule.enabled
                || !self.schedule.rules.is_empty()
                || self.manual.is_some()
                || self.failed_attempts != 0
                || self.blocked_until.is_some() =>
            {
                return Err(PrivacyError::CorruptConfig);
            }
            None => {}
        }
        if self.protected.iter().any(|(key, locator)| {
            key.as_str().is_empty() || key.as_str().len() > 8192 || locator.as_label().len() > 8192
        }) {
            return Err(PrivacyError::CorruptConfig);
        }
        Ok(())
    }
}
impl PrivacyStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<PrivacyDocument, PrivacyError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(PrivacyDocument::default());
            }
            Err(_) => return Err(PrivacyError::CorruptConfig),
        };
        let mut raw = String::new();
        file.take((MAX_DOCUMENT_BYTES + 1) as u64)
            .read_to_string(&mut raw)
            .map_err(|_| PrivacyError::CorruptConfig)?;
        if raw.len() > MAX_DOCUMENT_BYTES {
            return Err(PrivacyError::CorruptConfig);
        }
        let document: PrivacyDocument =
            toml::from_str(&raw).map_err(|_| PrivacyError::CorruptConfig)?;
        document.validate()?;
        Ok(document)
    }

    pub fn save(&self, value: &PrivacyDocument) -> Result<(), PrivacyError> {
        value.validate()?;
        let raw = toml::to_string(value).map_err(|_| PrivacyError::Persistence)?;
        if raw.len() > MAX_DOCUMENT_BYTES {
            return Err(PrivacyError::Persistence);
        }
        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or(PrivacyError::Persistence)?;
        fs::create_dir_all(parent).map_err(|_| PrivacyError::Persistence)?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| PrivacyError::Persistence)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|_| PrivacyError::Persistence)?;
        }
        file.write_all(raw.as_bytes()).map_err(|_| PrivacyError::Persistence)?;
        file.as_file().sync_all().map_err(|_| PrivacyError::Persistence)?;
        // tempfile uses atomic replace on Windows as well as Unix; it never
        // removes the destination first. Its Drop only cleans our temporary file.
        file.persist(&self.path).map_err(|_| PrivacyError::Persistence)?;
        #[cfg(unix)]
        File::open(parent).and_then(|dir| dir.sync_all()).map_err(|_| PrivacyError::Persistence)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn configured() -> PrivacyDocument {
        PrivacyDocument {
            credential: Some(PinCredential::create("0123").unwrap()),
            ..PrivacyDocument::default()
        }
    }
    #[test]
    fn missing_is_unconfigured_but_existing_corruption_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("privacy.toml");
        let store = PrivacyStore::new(path.clone());
        assert!(store.load().unwrap().credential.is_none());
        for content in ["", "not toml[", "schema_version = 99"] {
            std::fs::write(&path, content).unwrap();
            assert!(store.load().is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        }
    }
    #[test]
    fn atomic_roundtrip_preserves_cooldown_and_never_contains_plaintext_pin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings").join("privacy.toml");
        let store = PrivacyStore::new(path.clone());
        let mut doc = configured();
        doc.failed_attempts = 5;
        doc.blocked_until = Some("2026-10-08T12:00:30Z".parse().unwrap());
        doc.manual = Some(ManualPrivacy { enabled: true, until: None });
        let locator = MediaLocator::File(dir.path().join("fictional.mp4"));
        doc.protected.insert(MediaKey::from_locator(&locator).unwrap(), locator);
        store.save(&doc).unwrap();
        let restored = store.load().unwrap();
        assert_eq!(restored.failed_attempts, 5);
        assert_eq!(restored.blocked_until, doc.blocked_until);
        assert_eq!(restored.manual, doc.manual);
        assert_eq!(restored.protected, doc.protected);
        assert!(restored.credential.unwrap().verify("0123").unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains(r#""0123""#));
        doc.failed_attempts = 0;
        doc.blocked_until = None;
        store.save(&doc).unwrap();
        assert_eq!(store.load().unwrap().failed_attempts, 0);
        assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }
    #[test]
    fn invalid_save_keeps_last_good_document_and_protection_requires_pin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("privacy.toml");
        let store = PrivacyStore::new(path.clone());
        let mut doc = configured();
        store.save(&doc).unwrap();
        let before = std::fs::read(&path).unwrap();
        doc.schema_version = 99;
        assert!(store.save(&doc).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let invalid = PrivacyDocument {
            manual: Some(ManualPrivacy { enabled: true, until: None }),
            ..PrivacyDocument::default()
        };
        assert!(store.save(&invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    #[test]
    fn oversized_file_and_unreadable_destination_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("privacy.toml");
        std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
        assert!(PrivacyStore::new(path).load().is_err());
        assert!(PrivacyStore::new(dir.path().to_path_buf()).save(&configured()).is_err());
    }
}
