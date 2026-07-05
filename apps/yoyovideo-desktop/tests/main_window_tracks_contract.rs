use slint::ModelRc;
use yoyovideo_desktop::{MainWindow, TrackPopupRowData};

#[test]
fn main_window_exports_track_popup_properties() {
    let constructor: fn() -> Result<MainWindow, slint::PlatformError> = MainWindow::new;
    let set_audio_rows: fn(&MainWindow, ModelRc<TrackPopupRowData>) =
        MainWindow::set_audio_track_rows;
    let set_subtitle_rows: fn(&MainWindow, ModelRc<TrackPopupRowData>) =
        MainWindow::set_subtitle_track_rows;
    let set_visible: fn(&MainWindow, bool) = MainWindow::set_subtitle_visible;
    let set_delay: fn(&MainWindow, f32) = MainWindow::set_subtitle_delay_value;
    let set_scale: fn(&MainWindow, f32) = MainWindow::set_subtitle_scale_value;
    let set_position: fn(&MainWindow, f32) = MainWindow::set_subtitle_position_value;
    let set_status: fn(&MainWindow, slint::SharedString) = MainWindow::set_tracks_status_label;
    let _ = (
        constructor,
        set_audio_rows,
        set_subtitle_rows,
        set_visible,
        set_delay,
        set_scale,
        set_position,
        set_status,
    );
}
