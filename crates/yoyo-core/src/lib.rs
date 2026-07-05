mod app_command;
mod backend;
mod config;
mod error;
mod history;
mod media;
mod player_state;
mod playlist;
mod session;
mod shortcut;

pub use app_command::AppCommand;
pub use backend::{BackendCommand, BackendEvent, PlayerBackend};
pub use config::{
    AppConfig, MAX_DEFAULT_SPEED, MIN_DEFAULT_SPEED, PlaybackDefaults, UiPreferences,
};
pub use error::{AppError, StorageError, ValidationError};
pub use history::{HistoryEntry, HistoryStore};
pub use media::MediaLocator;
pub use player_state::{
    AudioChannelMode, LoopState, MediaTrack, MediaTrackKind, PlayerState, Rotation,
    SubtitlePlaybackState,
};
pub use playlist::{Playlist, PlaylistEntry, PlaylistSnapshot};
pub use session::AppSession;
pub use shortcut::{Shortcut, ShortcutAction, ShortcutMap};
