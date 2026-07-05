mod app;
pub mod platform;
mod presenter;
mod settings_controller;
mod video_texture;

pub use app::{DesktopController, build_desktop_backend, dispatch_shortcut, refresh_window, run};
pub use platform::scan_media_folder;
pub use presenter::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_speed_label,
    format_time_label, format_transport_label, format_volume_label, format_zoom_label,
    progress_ratio,
};
pub use settings_controller::SettingsController;
