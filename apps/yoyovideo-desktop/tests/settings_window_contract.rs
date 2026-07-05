use yoyovideo_desktop::SettingsWindow;

#[test]
fn settings_window_type_is_exported() {
    let constructor: fn() -> Result<SettingsWindow, slint::PlatformError> = SettingsWindow::new;
    let _ = constructor;
}
