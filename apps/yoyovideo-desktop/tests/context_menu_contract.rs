use slint::Model;
use yoyovideo_desktop::MainWindow;

#[test]
fn main_window_context_menu_daily_actions_compile() {
    let window = MainWindow::new().unwrap();

    window.on_open_file_requested(|| {});
    window.on_open_folder_requested(|| {});
    window.on_screenshot_requested(|| {});
    window.on_settings_requested(|| {});
    window.on_toggle_fullscreen_requested(|| {});
    window.on_show_playlist_tab_requested(|| {});
    window.on_show_history_tab_requested(|| {});
    window.on_recent_open_item_requested(|_| {});
    assert_eq!(window.get_recent_open_rows().row_count(), 0);
}
