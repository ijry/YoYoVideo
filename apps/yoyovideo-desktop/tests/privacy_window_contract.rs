use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::SoftwareRenderer;
use slint::platform::{
    LayoutConstraints, Platform, Renderer, WindowAdapter, WindowEvent, WindowProperties,
};
use slint::{ComponentHandle, LogicalSize, PhysicalSize, Window, WindowSize};
use yoyovideo_desktop::MainWindow;

// Exercise the compiled Slint layout and pointer routing without a desktop or user settings.
struct HeadlessWindow {
    window: Window,
    renderer: SoftwareRenderer,
    size: Cell<PhysicalSize>,
    constraints: Cell<LayoutConstraints>,
}

impl WindowAdapter for HeadlessWindow {
    fn window(&self) -> &Window {
        &self.window
    }
    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
    fn size(&self) -> PhysicalSize {
        self.size.get()
    }
    fn set_size(&self, size: WindowSize) {
        self.size.set(size.to_physical(self.window.scale_factor()));
        self.window.dispatch_event(WindowEvent::Resized {
            size: size.to_logical(self.window.scale_factor()),
        });
    }
    fn update_window_properties(&self, properties: WindowProperties<'_>) {
        let constraints = properties.layout_constraints();
        self.constraints.set(constraints);
        if self.size.get().width == 0 {
            self.set_size(constraints.preferred.into());
        }
    }
}

struct HeadlessPlatform(Rc<HeadlessWindow>);
impl Platform for HeadlessPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn setup() -> (MainWindow, Rc<HeadlessWindow>) {
    let adapter = Rc::new_cyclic(|weak: &std::rc::Weak<HeadlessWindow>| HeadlessWindow {
        window: Window::new(weak.clone()),
        renderer: SoftwareRenderer::new(),
        size: Cell::new(PhysicalSize::default()),
        constraints: Cell::new(LayoutConstraints::default()),
    });
    slint::platform::set_platform(Box::new(HeadlessPlatform(adapter.clone()))).unwrap();
    let app = MainWindow::new().unwrap();
    app.show().unwrap();
    (app, adapter)
}

fn render(adapter: &HeadlessWindow) -> slint::SharedPixelBuffer<slint::Rgb8Pixel> {
    slint::platform::update_timers_and_animations();
    let size = adapter.size.get();
    let mut pixels = slint::SharedPixelBuffer::new(size.width, size.height);
    adapter.renderer.render(pixels.make_mut_slice(), size.width as usize);
    pixels
}

#[test]
fn pin_form_requires_ascii_digits_confirmation_and_no_cooldown() {
    let (_main, _adapter) = setup();
    let window = yoyovideo_desktop::PrivacyWindow::new().unwrap();
    yoyovideo_desktop::privacy::bind_pin_validation(&window);
    window.set_mode(1);
    window.set_pin("0123".into());
    assert!(window.get_submit_enabled());
    for pin in ["123", "12345", "１２３４", "12a4", " 123"] {
        window.set_pin(pin.into());
        assert!(!window.get_submit_enabled());
    }
    window.set_pin("0123".into());
    window.set_busy(true);
    assert!(!window.get_submit_enabled());
    window.set_busy(false);
    window.set_cooldown_seconds(1);
    assert!(!window.get_submit_enabled());
    window.set_cooldown_seconds(0);
    window.set_mode(0);
    window.set_confirmation("1230".into());
    assert!(!window.get_submit_enabled());
    window.set_confirmation("0123".into());
    assert!(window.get_submit_enabled());
}
#[test]
fn privacy_placeholder_is_opaque_over_an_actual_visible_frame() {
    let (app, adapter) = setup();
    adapter.set_size(LogicalSize::new(800.0, 600.0).into());
    let mut pixels = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(8, 8);
    pixels.make_mut_slice().fill(slint::Rgb8Pixel { r: 240, g: 12, b: 10 });
    app.set_has_media(true);
    app.set_video_frame_active(true);
    app.set_video_frame(slint::Image::from_rgb8(pixels));
    let before = render(&adapter);
    let x = (app.get_video_area_x() + app.get_video_area_width() / 2.0 - 120.0) as usize;
    let y = (app.get_video_area_y() + app.get_video_area_height() / 2.0) as usize;
    let red = before.as_slice()[y * before.width() as usize + x];
    assert!(red.r > 200 && red.g < 40, "fixture must first show a real red frame");
    app.set_privacy_blocked(true);
    let after = render(&adapter);
    let hidden = after.as_slice()[y * after.width() as usize + x];
    assert!(
        hidden.r < 60 && hidden.g < 60 && hidden.b < 70,
        "the opaque placeholder must conceal the frame"
    );
}
#[test]
fn privacy_settings_render_within_a_compact_window() {
    let adapter = Rc::new_cyclic(|weak: &std::rc::Weak<HeadlessWindow>| HeadlessWindow {
        window: Window::new(weak.clone()),
        renderer: SoftwareRenderer::new(),
        size: Cell::new(PhysicalSize::default()),
        constraints: Cell::new(LayoutConstraints::default()),
    });
    slint::platform::set_platform(Box::new(HeadlessPlatform(adapter.clone()))).unwrap();
    let window = yoyovideo_desktop::PrivacyWindow::new().unwrap();
    yoyovideo_desktop::privacy::bind_pin_validation(&window);
    window.set_mode(2);
    window.set_privacy_status("手动开启 · 下一周期 10-10 09:00 接管".into());
    window.set_rules(
        Rc::new(slint::VecModel::from(vec![slint::SharedString::from("每天  09:00 – 18:00")]))
            .into(),
    );
    window.set_protected_items(
        Rc::new(slint::VecModel::from(vec![slint::SharedString::from(
            "C:/Samples/fictional-protected-video.mp4",
        )]))
        .into(),
    );
    window.show().unwrap();
    let pixels = render(&adapter);
    assert!(
        pixels.width() <= 560 && pixels.height() <= 700,
        "settings must fit a small desktop window"
    );
    let mut label_ink = 0;
    for y in 88..108 {
        for x in 50..180 {
            let p = pixels.as_slice()[y * pixels.width() as usize + x];
            if p.r > 120 && p.g > 130 && p.b > 130 {
                label_ink += 1;
            }
        }
    }
    if let Some(path) = std::env::var_os("YOYOVIDEO_PRIVACY_PREVIEW_PPM") {
        let mut ppm = format!("P6\n{} {}\n255\n", pixels.width(), pixels.height()).into_bytes();
        ppm.extend_from_slice(pixels.as_bytes());
        std::fs::write(path, ppm).unwrap();
    }
    assert!(
        label_ink > 20,
        "the schedule checkbox label must remain readable on the dark panel; found {label_ink} bright pixels"
    );
}
