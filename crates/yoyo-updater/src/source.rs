use crate::{MAX_MANIFEST_BYTES, MAX_SIGNATURE_BYTES, Platform, UpdateError, VerifiedManifest};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::time::Duration;
use velopack::sources::UpdateSource;

const REPOSITORY: &str = "https://github.com/ijry/YoYoVideo";
trait Transport: Send + Sync {
    fn open(&self, url: &str, timeout: Duration) -> Result<Box<dyn Read + Send>, UpdateError>;
}
struct HttpsTransport;
impl Transport for HttpsTransport {
    fn open(&self, url: &str, timeout: Duration) -> Result<Box<dyn Read + Send>, UpdateError> {
        if !url.starts_with("https://") {
            return Err(UpdateError::Network("HTTPS is required"));
        }
        let config = ureq::Agent::config_builder()
            .https_only(true)
            .max_redirects(5)
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(timeout))
            .build();
        let response = ureq::Agent::new_with_config(config)
            .get(url)
            .header("User-Agent", concat!("YoYoVideo/", env!("CARGO_PKG_VERSION")))
            .call()
            .map_err(|_| UpdateError::Network("request failed or timed out"))?;
        Ok(Box::new(response.into_body().into_reader()))
    }
}

/// Only the network boundary is replaceable in unit tests. Production URLs are
/// derived from this repository and authenticated release metadata, not an env var.
#[derive(Clone)]
pub struct SignedSource {
    platform: Platform,
    public_key: Arc<str>,
    transport: Arc<dyn Transport>,
    verified: Arc<Mutex<Option<VerifiedManifest>>>,
}
impl SignedSource {
    pub fn new(platform: Platform, key: String) -> Self {
        Self::from_transport(platform, key, Arc::new(HttpsTransport))
    }
    fn from_transport(platform: Platform, key: String, transport: Arc<dyn Transport>) -> Self {
        Self { platform, public_key: key.into(), transport, verified: Arc::new(Mutex::new(None)) }
    }
    pub fn snapshot(&self) -> Option<VerifiedManifest> {
        self.verified.lock().ok().and_then(|v| v.clone())
    }
    fn metadata(&self, url: &str, limit: usize) -> Result<Vec<u8>, UpdateError> {
        let mut bytes = Vec::new();
        self.transport
            .open(url, Duration::from_secs(30))?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(UpdateError::Manifest("response exceeds size limit"));
        }
        Ok(bytes)
    }
    fn check(
        &self,
        channel: &str,
        app: &velopack::bundle::Manifest,
    ) -> Result<velopack::VelopackAssetFeed, UpdateError> {
        // A failed recheck cannot leave a previous candidate implicitly trusted.
        *self.verified.lock().map_err(|_| UpdateError::Network("update state unavailable"))? = None;
        if app.id != "YoYoVideo" || channel != self.platform.channel() || app.channel != channel {
            return Err(UpdateError::Manifest("installed application or channel mismatch"));
        }
        let url = format!(
            "{REPOSITORY}/releases/latest/download/yoyovideo-update.{}.json",
            self.platform.as_str()
        );
        let raw = self.metadata(&url, MAX_MANIFEST_BYTES)?;
        let signature = self.metadata(&format!("{url}.sig"), MAX_SIGNATURE_BYTES)?;
        let signature = std::str::from_utf8(&signature).map_err(|_| UpdateError::Signature)?;
        let snapshot = VerifiedManifest::verify(&raw, signature, &self.public_key, self.platform)?;
        let feed = snapshot.manifest().feed.clone();
        *self.verified.lock().map_err(|_| UpdateError::Network("update state unavailable"))? =
            Some(snapshot);
        Ok(feed)
    }
    fn download(
        &self,
        asset: &velopack::VelopackAsset,
        file: &Path,
        progress: Option<Sender<i16>>,
    ) -> Result<(), UpdateError> {
        let snapshot =
            self.snapshot().ok_or(UpdateError::Manifest("no authenticated candidate"))?;
        if serde_json::to_vec(asset).map_err(|_| UpdateError::Manifest("invalid asset"))?
            != serde_json::to_vec(snapshot.asset())
                .map_err(|_| UpdateError::Manifest("invalid asset"))?
        {
            return Err(UpdateError::Manifest("asset differs from the authenticated candidate"));
        }
        let url = format!(
            "{REPOSITORY}/releases/download/{}/{}",
            snapshot.manifest().release_tag,
            asset.FileName
        );
        let mut reader =
            self.transport.open(&url, Duration::from_secs(30 * 60))?.take(asset.Size + 1);
        // The caller (Velopack) supplies its cache's partial path. No file is
        // touched until the candidate and the network response are accepted.
        let mut output = File::create(file)?;
        let result = (|| -> Result<(), UpdateError> {
            let mut buffer = [0_u8; 64 * 1024];
            let mut total = 0_u64;
            if let Some(tx) = &progress {
                let _ = tx.send(0);
            }
            loop {
                let count = reader.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                total += count as u64;
                if total > asset.Size {
                    return Err(UpdateError::PackageSize);
                }
                output.write_all(&buffer[..count])?;
                if let Some(tx) = &progress {
                    let _ = tx.send(((total * 100 / asset.Size).min(99)) as i16);
                }
            }
            if total != asset.Size {
                return Err(UpdateError::PackageSize);
            }
            output.sync_all()?;
            Ok(())
        })();
        drop(output);
        let result = result.and_then(|_| snapshot.verify_package(file));
        if result.is_err() {
            let _ = std::fs::remove_file(file);
        }
        if result.is_ok() {
            if let Some(tx) = progress {
                let _ = tx.send(100);
            }
        }
        result
    }
}
impl UpdateSource for SignedSource {
    fn get_release_feed(
        &self,
        channel: &str,
        app: &velopack::bundle::Manifest,
        _: &str,
    ) -> Result<velopack::VelopackAssetFeed, velopack::Error> {
        self.check(channel, app).map_err(|e| velopack::Error::Other(e.to_string()))
    }
    fn download_release_entry(
        &self,
        asset: &velopack::VelopackAsset,
        file: &Path,
        progress: Option<Sender<i16>>,
    ) -> Result<(), velopack::Error> {
        self.download(asset, file, progress).map_err(|e| velopack::Error::Other(e.to_string()))
    }
}
#[cfg(test)]
mod tests;
