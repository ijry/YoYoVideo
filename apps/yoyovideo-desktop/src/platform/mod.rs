mod dialogs;
mod drop;
mod logging;
mod markers;
mod media_scan;
mod paths;
mod recent;
mod screenshot;
mod window_state;

pub use dialogs::{DialogService, RfdDialogService};
pub use drop::{DroppedMediaAction, classify_dropped_paths};
pub use logging::{
    append_diagnostic, append_diagnostic_line, default_log_file, diagnostic_timestamp_now,
};
pub use markers::{
    MAX_MARKER_SETS, MAX_MARKERS_PER_MEDIA, MarkerStore, MediaMarkerSet, marker_store_path,
};
pub use media_scan::scan_media_folder;
pub use paths::AppPaths;
pub use recent::{
    MAX_RECENT_OPEN_ITEMS, RecentOpenItem, RecentOpenKind, RecentOpenStore, recent_open_path,
};
pub use screenshot::{
    default_screenshot_dir, next_screenshot_path, prepare_screenshot_path,
    prepare_screenshot_path_in_dir, screenshot_timestamp_now,
};
pub use window_state::{
    MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, WindowState, load_window_state, save_window_state,
    window_state_path,
};
