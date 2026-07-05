use thiserror::Error;

#[derive(Debug, Error)]
pub enum MpvError {
    #[error("mpv runtime feature is disabled")]
    RuntimeDisabled,
    #[error("mpv handle creation failed")]
    CreateHandle,
    #[error("mpv initialization failed: {0}")]
    Initialize(String),
    #[error("mpv command failed: {0}")]
    Command(String),
    #[error("mpv property failed: {0}")]
    Property(String),
    #[error("mpv video output failed: {0}")]
    VideoOutput(String),
    #[error("mpv string contained an interior null byte: {0}")]
    InvalidString(String),
    #[error("mpv api error: {0}")]
    Api(String),
}
