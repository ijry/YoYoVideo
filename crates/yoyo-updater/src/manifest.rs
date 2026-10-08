use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use velopack::{VelopackAsset, VelopackAssetFeed};

use crate::{Platform, UpdateError};

pub const MAX_MANIFEST_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SIGNATURE_BYTES: usize = 16 * 1024;
pub const MAX_PACKAGE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_PUBLIC_KEY_BYTES: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateManifest {
    pub schema_version: u32,
    pub app_id: String,
    pub platform: String,
    pub channel: String,
    pub version: String,
    pub release_tag: String,
    pub feed: VelopackAssetFeed,
}

impl UpdateManifest {
    /// Validate the signed context. This does not itself authenticate the input.
    pub fn validate(&self, platform: Platform) -> Result<(), UpdateError> {
        if self.schema_version != 1 || self.app_id != "YoYoVideo" {
            return Err(UpdateError::Manifest("unknown schema or application"));
        }
        if self.platform != platform.as_str() || self.channel != platform.channel() {
            return Err(UpdateError::Manifest("platform or channel mismatch"));
        }
        let version =
            Version::parse(&self.version).map_err(|_| UpdateError::Manifest("invalid version"))?;
        if !version.pre.is_empty() || version.to_string() != self.version {
            return Err(UpdateError::Manifest("only canonical stable versions are supported"));
        }
        if self.release_tag != format!("v{}", self.version) {
            return Err(UpdateError::Manifest("release tag mismatch"));
        }
        let [asset] = self.feed.Assets.as_slice() else {
            return Err(UpdateError::Manifest("exactly one full package is required"));
        };
        if asset.PackageId != "YoYoVideo" || asset.Version != self.version || asset.Type != "Full" {
            return Err(UpdateError::Manifest("package identity, version or type mismatch"));
        }
        if !safe_package_name(&asset.FileName) {
            return Err(UpdateError::Manifest("unsafe package filename"));
        }
        if asset.Size == 0 || asset.Size > MAX_PACKAGE_BYTES {
            return Err(UpdateError::Manifest("package size outside limits"));
        }
        if !is_hex(&asset.SHA256, 64) || !is_hex(&asset.SHA1, 40) {
            return Err(UpdateError::Manifest("SHA-256 and SHA-1 are required"));
        }
        Ok(())
    }
}

/// An authenticated immutable snapshot. No constructor accepts an unsigned DTO.
#[derive(Debug, Clone)]
pub struct VerifiedManifest {
    manifest: UpdateManifest,
    version: Version,
    raw: Vec<u8>,
    signature: String,
}

impl VerifiedManifest {
    pub fn verify(
        raw: &[u8],
        signature: &str,
        public_key: &str,
        platform: Platform,
    ) -> Result<Self, UpdateError> {
        if raw.len() > MAX_MANIFEST_BYTES {
            return Err(UpdateError::Manifest("metadata exceeds limit"));
        }
        let key_text = decode_box(public_key, MAX_PUBLIC_KEY_BYTES)?;
        let sig_text = decode_box(signature, MAX_SIGNATURE_BYTES)?;
        let key = PublicKey::decode(&key_text).map_err(|_| UpdateError::Signature)?;
        let sig = Signature::decode(&sig_text).map_err(|_| UpdateError::Signature)?;
        // This also checks the trusted-comment signature. Reject legacy non-prehashed signatures.
        key.verify(raw, &sig, false).map_err(|_| UpdateError::Signature)?;
        let manifest: UpdateManifest =
            serde_json::from_slice(raw).map_err(|_| UpdateError::Manifest("invalid JSON"))?;
        manifest.validate(platform)?;
        let version = Version::parse(&manifest.version)
            .map_err(|_| UpdateError::Manifest("invalid version"))?;
        Ok(Self { manifest, version, raw: raw.to_vec(), signature: signature.trim().to_owned() })
    }
    pub fn manifest(&self) -> &UpdateManifest {
        &self.manifest
    }
    pub fn asset(&self) -> &VelopackAsset {
        &self.manifest.feed.Assets[0]
    }
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw
    }
    pub fn signature(&self) -> &str {
        &self.signature
    }
    pub fn is_newer_than(&self, current: &Version) -> bool {
        self.version.cmp_precedence(current).is_gt()
    }

    /// Always recompute the hash, even if the SDK elected to reuse a cached file.
    pub fn verify_package(&self, path: &Path) -> Result<(), UpdateError> {
        verify_package_file(self.asset(), path)
    }
}

/// Checks bytes against supplied metadata, without authenticating that metadata.
/// Callers must validate/authenticate the manifest separately.
pub fn verify_package_file(asset: &VelopackAsset, path: &Path) -> Result<(), UpdateError> {
    let file = File::open(path)?;
    if file.metadata()?.len() != asset.Size {
        return Err(UpdateError::PackageSize);
    }
    let mut reader = BufReader::new(file);
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > asset.Size {
            return Err(UpdateError::PackageSize);
        }
        hash.update(&buffer[..count]);
    }
    if total != asset.Size {
        return Err(UpdateError::PackageSize);
    }
    if !format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&asset.SHA256) {
        return Err(UpdateError::PackageHash);
    }
    Ok(())
}

fn decode_box(encoded: &str, limit: usize) -> Result<String, UpdateError> {
    if encoded.len() > limit {
        return Err(UpdateError::Signature);
    }
    let bytes = STANDARD.decode(encoded.trim()).map_err(|_| UpdateError::Signature)?;
    String::from_utf8(bytes).map_err(|_| UpdateError::Signature)
}
fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn safe_package_name(name: &str) -> bool {
    if name.len() > 240
        || !name.ends_with(".nupkg")
        || name.contains("..")
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return false;
    }
    let stem = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    !stem.is_empty()
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}
