use thiserror::Error;

#[derive(Debug, Error)]
pub enum MpvError {
    #[error("mpv runtime feature is disabled")]
    RuntimeDisabled,
    #[error("mpv api error: {0}")]
    Api(String),
}
