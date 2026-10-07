//! Release-time signing. Credentials are never part of a diagnostic or CLI argument.
use base64::{Engine, engine::general_purpose::STANDARD};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use yoyo_updater::{
    MAX_MANIFEST_BYTES, Platform, UpdateManifest, VerifiedManifest, verify_package_file,
};

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("invalid signing input: {0}")]
    Input(&'static str),
    #[error("invalid signing credentials or public key mismatch")]
    Key,
    #[error("release verification failed: {0}")]
    Validation(#[from] yoyo_updater::UpdateError),
    #[error("release file operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[allow(clippy::too_many_arguments)]
pub fn sign_release(
    feed_path: &Path,
    assets_dir: &Path,
    platform: Platform,
    version: &str,
    public_key: &str,
    private_key: &str,
    password: &str,
) -> Result<(Vec<u8>, String), SignError> {
    let expected_feed = format!("releases.{}.json", platform.channel());
    if feed_path.file_name().and_then(|v| v.to_str()) != Some(expected_feed.as_str()) {
        return Err(SignError::Input("feed filename does not match the selected platform"));
    }
    let feed: velopack::VelopackAssetFeed =
        serde_json::from_slice(&read_bounded(feed_path, MAX_MANIFEST_BYTES)?)
            .map_err(|_| SignError::Input("invalid feed JSON"))?;
    let manifest = UpdateManifest {
        schema_version: 1,
        app_id: "YoYoVideo".into(),
        platform: platform.as_str().into(),
        channel: platform.channel(),
        version: version.into(),
        release_tag: format!("v{version}"),
        feed,
    };
    manifest.validate(platform)?;
    let asset = &manifest.feed.Assets[0];
    verify_package_file(asset, &assets_dir.join(&asset.FileName))?;

    let public_text = decode_key(public_key)?;
    let private_text = decode_key(private_key)?;
    let public = minisign::PublicKeyBox::from_string(&public_text)
        .and_then(|v| v.into_public_key())
        .map_err(|_| SignError::Key)?;
    let private = minisign::SecretKeyBox::from_string(&private_text)
        .and_then(|v| v.into_secret_key(Some(password.to_owned())))
        .map_err(|_| SignError::Key)?;
    let raw = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| SignError::Input("serialization failed"))?;
    let signature = minisign::sign(
        Some(&public),
        &private,
        raw.as_slice(),
        Some("YoYoVideo authenticated update manifest"),
        None,
    )
    .map_err(|_| SignError::Key)?;
    let encoded_signature = STANDARD.encode(signature.to_string());
    // Independent verifier protects against key mismatches and incompatible encodings.
    verify_release(&raw, &encoded_signature, public_key, platform, assets_dir)?;
    Ok((raw, encoded_signature))
}

pub fn verify_release(
    raw: &[u8],
    signature: &str,
    public_key: &str,
    platform: Platform,
    assets_dir: &Path,
) -> Result<(), SignError> {
    let verified = VerifiedManifest::verify(raw, signature, public_key, platform)?;
    verified.verify_package(&assets_dir.join(&verified.asset().FileName))?;
    Ok(())
}

pub fn signature_path(manifest: &Path) -> PathBuf {
    let mut name = manifest.as_os_str().to_os_string();
    name.push(".sig");
    PathBuf::from(name)
}

/// Publish the already-verified pair without overwriting an existing release.
/// The manifest is made visible last; a crash may leave an orphan signature, never
/// a new manifest whose signature was not written first.
pub fn write_release(output: &Path, raw: &[u8], signature: &str) -> Result<(), SignError> {
    let sig_path = signature_path(output);
    if output.try_exists()? || sig_path.try_exists()? {
        return Err(SignError::Input("output already exists"));
    }
    let parent =
        output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut json_temp = NamedTempFile::new_in(parent)?;
    let mut sig_temp = NamedTempFile::new_in(parent)?;
    json_temp.write_all(raw)?;
    json_temp.as_file().sync_all()?;
    sig_temp.write_all(signature.as_bytes())?;
    sig_temp.as_file().sync_all()?;
    sig_temp.persist_noclobber(&sig_path).map_err(|e| SignError::Io(e.error))?;
    if let Err(error) = json_temp.persist_noclobber(output) {
        // This invocation created the signature. No recursive cleanup or unrelated files.
        let _ = std::fs::remove_file(&sig_path);
        return Err(SignError::Io(error.error));
    }
    Ok(())
}

pub fn read_bounded(path: &Path, max_bytes: usize) -> Result<Vec<u8>, SignError> {
    let file = File::open(path)?;
    if file.metadata()?.len() > max_bytes as u64 {
        return Err(SignError::Input("file exceeds size limit"));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(SignError::Input("file exceeds size limit"));
    }
    Ok(bytes)
}
fn decode_key(encoded: &str) -> Result<String, SignError> {
    if encoded.len() > 16 * 1024 {
        return Err(SignError::Key);
    }
    let raw = STANDARD.decode(encoded.trim()).map_err(|_| SignError::Key)?;
    String::from_utf8(raw).map_err(|_| SignError::Key)
}
