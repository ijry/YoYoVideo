mod app;
mod history_runtime;
mod keyboard;
pub mod platform;
mod presenter;
mod settings_controller;
mod sidebar;
mod subtitle_prefs;
mod track_popup;
mod video_host;
#[cfg(feature = "mpv-runtime")]
mod video_host_winit;
mod video_texture;

pub use app::{
    DesktopController, MainWindow, SettingsWindow, ShortcutDispatch, TrackPopupRowData,
    build_desktop_backend, build_desktop_backend_with_video_window, dispatch_shortcut,
    dropped_media_status, refresh_window, resolve_shortcut, run,
};
pub use history_runtime::{
    FlushReason, HistoryActivation, HistoryActivationError, HistoryRuntime, PendingResumeSeek,
};
pub use keyboard::{
    DesktopKey, KeyboardInput, NamedDesktopKey, shortcut_allowed, shortcut_gesture,
};
pub use platform::scan_media_folder;
pub use presenter::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_speed_label,
    format_time_label, format_transport_label, format_video_adjustment_label,
    format_video_filter_preset_label, format_volume_label, format_zoom_label, progress_ratio,
};
pub use settings_controller::{SettingsController, SettingsShortcutRow, SettingsSnapshot};
pub use sidebar::{
    HistorySidebarRow, PlaylistSidebarRow, SidebarState, SidebarTab, build_history_rows,
    build_playlist_rows, expanded_sidebar_width, initial_sidebar_state,
};
pub use subtitle_prefs::{
    SubtitlePreferenceEntry, SubtitlePrefsFlushReason, SubtitlePrefsRuntime, SubtitleRestoreError,
    SubtitleRestorePlan,
};
pub use track_popup::{
    TrackPopupRow, build_audio_track_rows, build_subtitle_track_rows, build_video_track_rows,
    format_subtitle_delay_label, format_subtitle_scale_label, format_track_label,
};
pub use video_host::{
    LogicalVideoRect, NativeVideoWindowId, UnsupportedVideoHost, VideoHost, VideoHostBounds,
    VideoHostError,
};
