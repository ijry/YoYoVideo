use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::SoftwareRenderer;
use slint::platform::{
    LayoutConstraints, Platform, PointerEventButton, Renderer, WindowAdapter, WindowEvent,
    WindowProperties,
};
use slint::{ComponentHandle, LogicalPosition, PhysicalSize, Window, WindowSize};
use yoyovideo_desktop::UpdateWindow;

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

fn setup() -> (UpdateWindow, Rc<HeadlessWindow>) {
    let adapter = Rc::new_cyclic(|weak: &std::rc::Weak<HeadlessWindow>| HeadlessWindow {
        window: Window::new(weak.clone()),
        renderer: SoftwareRenderer::new(),
        size: Cell::new(PhysicalSize::default()),
        constraints: Cell::new(LayoutConstraints::default()),
    });
    slint::platform::set_platform(Box::new(HeadlessPlatform(adapter.clone()))).unwrap();
    let app = UpdateWindow::new().unwrap();
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

fn click(app: &UpdateWindow, x: f32, y: f32) {
    let position = LogicalPosition::new(x, y);
    app.window()
        .dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
    app.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}
#[test]
fn update_actions_are_gated_by_state_and_do_not_install_on_later() {
    let (app, adapter) = setup();
    let downloads = Rc::new(Cell::new(0));
    let installs = Rc::new(Cell::new(0));
    let checks = Rc::new(Cell::new(0));
    let later = Rc::new(Cell::new(0));
    let releases = Rc::new(Cell::new(0));
    app.on_download_requested({
        let count = downloads.clone();
        move || count.set(count.get() + 1)
    });
    app.on_install_requested({
        let count = installs.clone();
        move || count.set(count.get() + 1)
    });
    app.on_check_requested({
        let count = checks.clone();
        move || count.set(count.get() + 1)
    });
    app.on_later_requested({
        let count = later.clone();
        move || count.set(count.get() + 1)
    });
    app.on_open_releases_requested({
        let count = releases.clone();
        move || count.set(count.get() + 1)
    });
    for language in ["zh", "en"] {
        app.set_ui_language_code(language.into());
        let d = downloads.get();
        let i = installs.get();
        let c = checks.get();
        let l = later.get();
        let r = releases.get();
        app.set_phase_index(4);
        render(&adapter);
        click(&app, 320.0, 383.0);
        assert_eq!(downloads.get(), d + 1);
        app.set_phase_index(6);
        render(&adapter);
        click(&app, 454.0, 383.0);
        assert_eq!(later.get(), l + 1);
        assert_eq!(installs.get(), i);
        click(&app, 320.0, 383.0);
        assert_eq!(installs.get(), i + 1);
        for phase in [0, 1, 2, 3, 5, 7, 8] {
            app.set_phase_index(phase);
            render(&adapter);
            click(&app, 320.0, 383.0);
        }
        assert_eq!(installs.get(), i + 1);
        assert_eq!(downloads.get(), d + 1);
        app.set_phase_index(1);
        render(&adapter);
        click(&app, 92.0, 383.0);
        assert_eq!(checks.get(), c + 1);
        app.set_phase_index(2);
        render(&adapter);
        click(&app, 92.0, 383.0);
        assert_eq!(checks.get(), c + 1);
        app.set_phase_index(0);
        render(&adapter);
        click(&app, 92.0, 383.0);
        assert_eq!(releases.get(), r + 1);
    }
}
#[test]
fn status_is_localized_and_preferences_are_user_controlled() {
    let (app, adapter) = setup();
    app.set_phase_index(6);
    app.set_ui_language_code("zh".into());
    render(&adapter);
    assert!(app.get_rendered_status().contains("更新"));
    app.set_ui_language_code("en".into());
    render(&adapter);
    assert!(app.get_rendered_status().contains("Ready"));
    assert!(app.get_can_install());
    let changed = Rc::new(Cell::new(None));
    app.on_automatic_check_changed({
        let changed = changed.clone();
        move |v| changed.set(Some(v))
    });
    app.set_phase_index(1);
    render(&adapter);
    click(&app, 32.0, 308.0);
    assert_eq!(changed.get(), Some(false));
    click(&app, 92.0, 383.0); // Focus the check button before clicking the checkbox label.
    click(&app, 100.0, 308.0);
    assert_eq!(changed.get(), Some(true));
    app.window().dispatch_event(WindowEvent::KeyPressed { text: " ".into() });
    app.window().dispatch_event(WindowEvent::KeyReleased { text: " ".into() });
    assert_eq!(changed.get(), Some(false), "label click must move keyboard focus to the checkbox");
    if let Some(path) = std::env::var_os("YOYOVIDEO_UPDATE_UI_PPM") {
        app.set_phase_index(6);
        app.set_available_version("0.0.2".into());
        app.set_current_version("0.0.1".into());
        app.set_progress_value(100);
        app.set_release_notes("Improved playback and stability.".into());
        let pixels = render(&adapter);
        let mut ppm = format!("P6\n{} {}\n255\n", pixels.width(), pixels.height()).into_bytes();
        ppm.extend_from_slice(pixels.as_bytes());
        std::fs::write(path, ppm).unwrap();
    }
}

#[test]
fn progress_fills_from_the_left_and_keeps_the_percentage_clear() {
    let (app, adapter) = setup();
    app.set_phase_index(5);
    app.set_progress_value(50);
    let pixels = render(&adapter);
    let bytes = pixels.as_bytes();
    let left = (278 * pixels.width() as usize + 25) * 3;
    let right = (278 * pixels.width() as usize + 350) * 3;
    assert!(bytes[left] > 180 && bytes[left + 1] < 80, "left half must contain the progress fill");
    assert!(bytes[right] < 100, "right half must remain unfilled at 50 percent");
}
