use crate::{MAX_MANIFEST_BYTES, MAX_SIGNATURE_BYTES, Platform, UpdateError, VerifiedManifest};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    manifest: String,
    signature: String,
}
pub(crate) struct PendingCache {
    path: PathBuf,
}
impl PendingCache {
    pub fn new(directory: &Path) -> Self {
        Self { path: directory.join("pending-update.json") }
    }
    pub fn load(
        &self,
        platform: Platform,
        key: &str,
        current: &Version,
        packages: &Path,
    ) -> Result<Option<VerifiedManifest>, UpdateError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let limit = MAX_MANIFEST_BYTES * 2 + MAX_SIGNATURE_BYTES + 4096;
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(UpdateError::Cache);
        }
        let receipt: Receipt = serde_json::from_slice(&bytes).map_err(|_| UpdateError::Cache)?;
        if receipt.schema_version != 1 {
            return Err(UpdateError::Cache);
        }
        let verified = VerifiedManifest::verify(
            receipt.manifest.as_bytes(),
            &receipt.signature,
            key,
            platform,
        )?;
        if !verified.is_newer_than(current) {
            return Ok(None);
        }
        verified.verify_package(&packages.join(&verified.asset().FileName))?;
        Ok(Some(verified))
    }
    pub fn save(&self, snapshot: &VerifiedManifest) -> Result<(), UpdateError> {
        let receipt = Receipt {
            schema_version: 1,
            manifest: std::str::from_utf8(snapshot.raw_bytes())
                .map_err(|_| UpdateError::Cache)?
                .into(),
            signature: snapshot.signature().into(),
        };
        let bytes = serde_json::to_vec(&receipt).map_err(|_| UpdateError::Cache)?;
        let parent = self.path.parent().ok_or(UpdateError::Cache)?;
        std::fs::create_dir_all(parent)?;
        let mut temp = NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|e| UpdateError::Io(e.error))?;
        Ok(())
    }
    pub fn clear(&self) -> Result<(), UpdateError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
