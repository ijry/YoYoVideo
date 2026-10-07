#![cfg(target_os = "windows")]

use slint::ComponentHandle;
use slint::winit_030::WinitWindowAccessor;
use yoyovideo_desktop::MainWindow;
use yoyovideo_desktop::platform::{PendingWindowRestore, WindowState};

#[test]
#[ignore = "requires a Windows desktop; briefly shows an independent window without reading user settings"]
fn real_main_window_recovers_from_an_offscreen_saved_position() {
    // The geometry test does not need an OpenGL driver or a playback runtime.
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()
        .unwrap();

    let app = MainWindow::new().unwrap();
    // A fresh component, not app::run(): no user history/settings are read or written.
    app.show().unwrap();
    app.window().set_size(slint::PhysicalSize::new(2910, 1650));
    app.window().set_position(slint::PhysicalPosition::new(11830, 1001));
    let mut restore = PendingWindowRestore::new(Some(WindowState {
        width: 2910,
        height: 1650,
        x: Some(11830),
        y: Some(1001),
        maximized: false,
    }));
    // This may be too early. Keep the pending state until a real native event,
    // using the same one-shot adapter as the application entry point.
    restore.apply(app.window());
    app.window().on_winit_window_event(move |window, _event| {
        restore.apply(window);
        slint::winit_030::EventResult::Propagate
    });

    let handle = app.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(250), move || {
        let app = handle.upgrade().unwrap();
        app.window()
            .with_winit_window(|window| {
                assert_eq!(window.is_visible(), Some(true));
                assert_ne!(window.is_minimized(), Some(true));
                let position = window.outer_position().unwrap();
                let size = window.inner_size();
                // The frameless title/drag area must actually be on a live display,
                // not merely represented by an alive process or taskbar button.
                let reachable = window.available_monitors().any(|monitor| {
                    let origin = monitor.position();
                    let bounds = monitor.size();
                    let visible_width = (i64::from(position.x) + i64::from(size.width))
                        .min(i64::from(origin.x) + i64::from(bounds.width))
                        - i64::from(position.x).max(i64::from(origin.x));
                    position.y >= origin.y
                        && i64::from(position.y) + 32
                            <= i64::from(origin.y) + i64::from(bounds.height)
                        && visible_width >= 64
                });
                assert!(reachable, "title bar is offscreen: position={position:?}, size={size:?}");
            })
            .expect("the real native window must exist");
        let pixels = app.window().take_snapshot().expect("main window must render");
        assert!(pixels.width() > 0 && pixels.height() > 0);
        let bytes = pixels.as_bytes();
        assert!(
            bytes.chunks_exact(4).any(|pixel| pixel[..3] != bytes[..3]),
            "main window rendered a blank, single-color surface"
        );
        // Optional test-only image output for visual QA; never user window data.
        if let Some(path) = std::env::var_os("YOYOVIDEO_WINDOW_SMOKE_PPM") {
            let mut ppm = format!("P6\n{} {}\n255\n", pixels.width(), pixels.height()).into_bytes();
            for pixel in bytes.chunks_exact(4) {
                ppm.extend_from_slice(&pixel[..3]);
            }
            std::fs::write(path, ppm).unwrap();
        }
        // A later user move must not be undone by the next native Moved event.
        let position = app.window().position();
        let moved = slint::PhysicalPosition::new(position.x + 12, position.y + 12);
        app.window().set_position(moved);
        let handle = app.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
            let app = handle.upgrade().unwrap();
            assert_eq!(app.window().position(), moved, "restoration must run only once");
            slint::quit_event_loop().unwrap();
        });
    });
    app.run().unwrap();
}
