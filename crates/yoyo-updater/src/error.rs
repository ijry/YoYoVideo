#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("invalid update signature or public key")]
    Signature,
    #[error("invalid update manifest: {0}")]
    Manifest(&'static str),
    #[error("unsupported update platform")]
    Platform,
    #[error("update package size does not match the signed manifest")]
    PackageSize,
    #[error("update package SHA-256 does not match the signed manifest")]
    PackageHash,
    #[error("update file I/O failed: {0}")]
    Io(#[from] std::io::Error),
}
