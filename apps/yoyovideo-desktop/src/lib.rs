mod app;
mod history_runtime;
mod keyboard;
pub mod platform;
mod presenter;
mod settings_controller;
mod sidebar;
mod video_host;
#[cfg(feature = "mpv-runtime")]
mod video_host_winit;
mod video_texture;

pub use app::{
    DesktopController, build_desktop_backend, build_desktop_backend_with_video_window,
    dispatch_shortcut, refresh_window, run,
};
pub use history_runtime::{
    FlushReason, HistoryActivation, HistoryActivationError, HistoryRuntime, PendingResumeSeek,
};
pub use keyboard::{DesktopKey, KeyboardInput, shortcut_allowed, shortcut_gesture};
pub use platform::scan_media_folder;
pub use presenter::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_speed_label,
    format_time_label, format_transport_label, format_volume_label, format_zoom_label,
    progress_ratio,
};
pub use settings_controller::SettingsController;
pub use sidebar::{
    HistorySidebarRow, PlaylistSidebarRow, SidebarState, SidebarTab, build_history_rows,
    build_playlist_rows, expanded_sidebar_width, initial_sidebar_state,
};
pub use video_host::{
    LogicalVideoRect, NativeVideoWindowId, UnsupportedVideoHost, VideoHost, VideoHostBounds,
    VideoHostError,
};
