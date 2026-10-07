use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::SoftwareRenderer;
use slint::platform::{
    LayoutConstraints, Platform, PointerEventButton, Renderer, WindowAdapter, WindowEvent,
    WindowProperties,
};
use slint::{ComponentHandle, LogicalPosition, LogicalSize, PhysicalSize, Window, WindowSize};
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

fn click_video_center(app: &MainWindow) {
    let position = LogicalPosition::new(
        app.get_video_area_x() + app.get_video_area_width() / 2.0,
        app.get_video_area_y() + app.get_video_area_height() / 2.0,
    );
    app.window()
        .dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
    app.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

#[test]
fn fresh_main_window_prefers_800_by_600_and_allows_that_size() {
    let (_app, adapter) = setup();
    render(&adapter);
    let constraints = adapter.constraints.get();
    assert_eq!(constraints.preferred, LogicalSize::new(800.0, 600.0));
    let min = constraints.min.unwrap_or_default();
    assert!(min.width <= 800.0 && min.height <= 600.0, "minimum is {min:?}");
}

#[test]
fn empty_video_center_opens_file_but_loaded_media_and_grid_do_not() {
    let (app, adapter) = setup();
    adapter.set_size(LogicalSize::new(800.0, 600.0).into());
    let opens = Rc::new(Cell::new(0));
    app.on_open_file_requested({
        let opens = opens.clone();
        move || opens.set(opens.get() + 1)
    });

    let pixels = render(&adapter);
    // Optional image of this test-only window for visual QA (never user data).
    if let Some(path) = std::env::var_os("YOYOVIDEO_EMPTY_STATE_PPM") {
        let mut ppm = format!("P6\n{} {}\n255\n", pixels.width(), pixels.height()).into_bytes();
        ppm.extend_from_slice(pixels.as_bytes());
        std::fs::write(path, ppm).unwrap();
    }
    click_video_center(&app);
    assert_eq!(opens.get(), 1, "the empty-state button must receive the click");

    app.set_has_media(true);
    render(&adapter);
    click_video_center(&app);
    assert_eq!(opens.get(), 1, "loaded media must hide the open button");

    app.set_has_media(false);
    render(&adapter);
    click_video_center(&app);
    assert_eq!(opens.get(), 2, "stopping playback must restore the button");

    app.set_grid_mode(true);
    render(&adapter);
    click_video_center(&app);
    assert_eq!(opens.get(), 2, "batch playback must not show the single-video button");
}
