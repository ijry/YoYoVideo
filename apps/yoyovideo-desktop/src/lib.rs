mod app;
pub mod platform;
mod presenter;
mod settings_controller;
mod video_texture;

pub use app::{DesktopController, build_desktop_backend, dispatch_shortcut, refresh_window, run};
pub use platform::scan_media_folder;
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
pub use settings_controller::SettingsController;
