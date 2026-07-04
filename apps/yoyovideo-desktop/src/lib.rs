mod app;
mod presenter;
mod video_texture;
pub mod platform;

pub use app::{dispatch_shortcut, run, DesktopController};
pub use platform::scan_media_folder;
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
