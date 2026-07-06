use yoyovideo_desktop::SettingsWindow;

#[test]
fn settings_window_type_is_exported() {
    let constructor: fn() -> Result<SettingsWindow, slint::PlatformError> = SettingsWindow::new;
    let _ = constructor;
}

#[test]
fn settings_window_playback_end_behavior_surface_compiles() {
    let window = SettingsWindow::new().unwrap();

    window.set_playback_end_behavior_index(2);
    assert_eq!(window.get_playback_end_behavior_index(), 2);
    window.on_playback_end_behavior_changed(|_| {});
}
