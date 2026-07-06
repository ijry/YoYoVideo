mod app;
mod history_runtime;
mod i18n;
mod keyboard;
mod osd;
pub mod platform;
mod presenter;
mod progress;
mod settings_controller;
mod sidebar;
mod subtitle_prefs;
mod track_popup;
mod video_host;
#[cfg(feature = "mpv-runtime")]
mod video_host_winit;
mod video_texture;

pub use app::{
    DesktopController, MainWindow, NavigationRowData, ProgressTickRowData, SettingsWindow,
    ShortcutDispatch, TrackPopupRowData, build_desktop_backend,
    build_desktop_backend_with_video_window, dispatch_shortcut, dropped_media_status,
    format_runtime_startup_error, recent_item_status, refresh_window, resolve_shortcut, run,
};
pub use history_runtime::{
    FlushReason, HistoryActivation, HistoryActivationError, HistoryRuntime, PendingResumeSeek,
};
pub use i18n::UiLanguage;
pub use keyboard::{
    DesktopKey, KeyboardInput, NamedDesktopKey, shortcut_allowed, shortcut_gesture,
};
pub use osd::{OsdKind, OsdState, format_osd_message, format_osd_message_for_language};
pub use platform::scan_media_folder;
pub use presenter::{
    format_audio_channel_label, format_audio_channel_label_for_language, format_loop_label,
    format_loop_label_for_language, format_rotation_label, format_rotation_label_for_language,
    format_speed_label, format_time_label, format_transport_label,
    format_transport_label_for_language, format_video_adjustment_label,
    format_video_adjustment_label_for_language, format_video_filter_preset_label,
    format_video_filter_preset_label_for_language, format_volume_label,
    format_volume_label_for_language, format_zoom_label, format_zoom_label_for_language,
    progress_ratio,
};
pub use progress::{
    NavigationRow, ProgressTick, ProgressTickKind, build_navigation_rows, build_progress_ticks,
    format_preview_label, parse_jump_time,
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
