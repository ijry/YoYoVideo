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
    AppConfig, MAX_DEFAULT_SPEED, MIN_DEFAULT_SPEED, PlaybackDefaults, PlaybackEndBehavior,
    UiPreferences,
};
pub use error::{AppError, StorageError, ValidationError};
pub use history::{HistoryEntry, HistoryStore};
pub use media::MediaLocator;
pub use player_state::{
    AudioChannelMode, FrameStepDirection, LoopState, MARKER_DEDUPE_TOLERANCE_SECONDS,
    MAX_VIDEO_ADJUSTMENT, MIN_VIDEO_ADJUSTMENT, MediaChapter, MediaMarker, MediaTrack,
    MediaTrackKind, PlayerState, Rotation, SubtitlePlaybackState, VideoAdjustmentKind,
    VideoAdjustments, VideoFilterPreset,
};
pub use playlist::{Playlist, PlaylistEntry, PlaylistSnapshot};
pub use session::AppSession;
pub use shortcut::{Shortcut, ShortcutAction, ShortcutMap};
