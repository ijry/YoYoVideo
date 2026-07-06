use slint::Model;
use yoyovideo_desktop::MainWindow;

fn exercise_cinema_deck_surface(window: &MainWindow) {
    window.set_muted(true);
    assert!(window.get_muted());
    window.set_mute_label("Muted".into());
    window.set_osd_visible(true);
    window.set_osd_message("Muted".into());
    window.set_progress_preview_visible(true);
    window.set_progress_preview_label("01:15".into());
    window.set_progress_preview_value(0.5);
    window.set_action_panel_visible(true);
    window.set_jump_panel_visible(true);
    window.set_jump_input_text("01:15".into());
    window.set_progress_tick_rows(
        [
            yoyovideo_desktop::ProgressTickRowData {
                percent: 0.25,
                label: "Chapter 1".into(),
                is_marker: false,
            },
            yoyovideo_desktop::ProgressTickRowData {
                percent: 0.5,
                label: "Marker 00:30".into(),
                is_marker: true,
            },
        ]
        .into(),
    );
    window.set_navigation_rows(
        [yoyovideo_desktop::NavigationRowData {
            title: "Marker 00:30".into(),
            subtitle: "00:30".into(),
            id: "marker-30000".into(),
            is_marker: true,
        }]
        .into(),
    );
    assert_eq!(window.get_progress_tick_rows().row_count(), 2);
    assert_eq!(window.get_navigation_rows().row_count(), 1);

    window.on_toggle_mute_requested(|| {});
    window.on_progress_preview_requested(|_| {});
    window.on_progress_commit_requested(|_| {});
    window.on_progress_preview_cleared(|| {});
    window.on_jump_panel_requested(|| {});
    window.on_jump_input_changed(|_| {});
    window.on_jump_commit_requested(|_| {});
    window.on_action_panel_requested(|| {});
    window.on_action_panel_close_requested(|| {});
    window.on_add_marker_requested(|| {});
    window.on_remove_marker_requested(|_| {});
    window.on_navigation_row_requested(|_| {});
    window.on_previous_chapter_marker_requested(|| {});
    window.on_next_chapter_marker_requested(|| {});
    window.set_ui_language_code("zh".into());
    assert_eq!(window.get_ui_language_code().as_str(), "zh");
    window.on_window_drag_requested(|| {});
    window.on_window_minimize_requested(|| {});
    window.on_window_maximize_restore_requested(|| {});
    window.on_window_close_requested(|| {});
    window.on_language_changed(|_| {});
    window.on_video_double_clicked(|| {});
    window.on_video_dragged(|_, _| {});
    window.on_reset_video_pan_requested(|| {});
}

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
    exercise_cinema_deck_surface(&window);
}

#[test]
fn main_window_cinema_deck_surface_compiles() {
    let surface_check: fn(&MainWindow) = exercise_cinema_deck_surface;
    let _ = surface_check;
}
