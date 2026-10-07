#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("update operation failed: {0}")]
    Backend(String),
    #[error("application is not installed with the updater")]
    Unsupported,
    #[error("invalid update operation: {0}")]
    State(&'static str),
    #[error("update installation failed: {0}")]
    Install(String),
    #[error("invalid pending update cache")]
    Cache,
    #[error("update network request failed: {0}")]
    Network(&'static str),
    #[error("invalid update preferences")]
    Preferences,
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
