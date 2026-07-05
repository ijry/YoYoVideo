use yoyovideo_desktop::MainWindow;

#[test]
fn main_window_with_video_tools_surface_compiles() {
    let constructor: fn() -> Result<MainWindow, slint::PlatformError> = MainWindow::new;
    let _ = constructor;
}
