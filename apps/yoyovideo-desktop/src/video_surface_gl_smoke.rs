//! Explicit opt-in smoke test for the real GL/Slint/mpv boundary on Windows.
//! It briefly creates a 64px test window; not part of the ordinary test suite.
use super::*;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use yoyo_core::{MediaLocator, PlayerBackend};

slint::slint! {
    export component GlSmokeWindow inherits Window {
        width: 64px;
        height: 64px;
        title: "YoYoVideo GL smoke";
        in property <image> frame;
        Image { source: root.frame; }
    }
}

#[test]
#[ignore = "requires a Windows desktop, OpenGL driver and staged libmpv; opens a small test window"]
fn native_gl_decodes_resizes_and_tears_down() {
    use slint::winit_030::winit::platform::windows::EventLoopBuilderExtWindows;
    let mut event_loop = slint::winit_030::winit::event_loop::EventLoop::with_user_event();
    event_loop.with_any_thread(true);
    let backend = i_slint_backend_winit::Backend::builder()
        .with_event_loop_builder(event_loop)
        .with_renderer_name("femtovg")
        .build()
        .unwrap();
    slint::platform::set_platform(Box::new(backend)).unwrap();

    struct State {
        surface: CompositedVideo,
        backend: MpvBackend,
        width: i32,
        red_frames: usize,
        torn_down: bool,
    }
    let state = Rc::new(RefCell::new(State {
        surface: CompositedVideo::default(),
        backend: MpvBackend::new_runtime_with_options(yoyo_mpv::MpvClientOptions {
            audio_output: Some("null".into()),
            render_api: true,
            ..Default::default()
        })
        .unwrap(),
        width: 64,
        red_frames: 0,
        torn_down: false,
    }));
    let app = GlSmokeWindow::new().unwrap();
    let handle = app.as_weak();
    let state_cb = Rc::clone(&state);
    app.window()
        .set_rendering_notifier(move |event, api| {
            let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api else {
                panic!("smoke test requires native GL");
            };
            let mut state = state_cb.borrow_mut();
            match event {
                slint::RenderingState::BeforeRendering => {
                    if !state.surface.is_ready() {
                        let wake = handle.clone();
                        let State { surface, backend, .. } = &mut *state;
                        unsafe {
                            surface
                                .setup(backend, *get_proc_address, move || {
                                    let _ = wake
                                        .upgrade_in_event_loop(|app| app.window().request_redraw());
                                })
                                .unwrap()
                        };
                        let state = Rc::clone(&state_cb);
                        slint::Timer::single_shot(Duration::ZERO, move || {
                            state
                                .borrow_mut()
                                .backend
                                .open(&MediaLocator::Url(
                                    "av://lavfi:color=c=red:s=64x64:r=30:d=3".into(),
                                ))
                                .unwrap();
                        });
                    }
                    let width = state.width;
                    if let Some(image) = unsafe { state.surface.render(width, 64) }.unwrap() {
                        if let Some(app) = handle.upgrade() {
                            app.set_frame(slint::Image::default());
                            app.set_frame(image);
                        }
                        // Read a real rendered pixel, not merely a mock's call count.
                        let read_pixels: unsafe extern "C" fn(
                            i32,
                            i32,
                            i32,
                            i32,
                            u32,
                            u32,
                            *mut c_void,
                        ) = unsafe { std::mem::transmute(get_proc_address(c"glReadPixels")) };
                        let mut pixel = [0u8; 4];
                        let gl = state.surface.functions.unwrap();
                        unsafe {
                            gl.with_saved_bindings(|| {
                                state.surface.target.as_ref().unwrap().bind(&gl);
                                read_pixels(
                                    width / 2,
                                    32,
                                    1,
                                    1,
                                    0x1908,
                                    0x1401,
                                    pixel.as_mut_ptr().cast(),
                                );
                            })
                        };
                        if pixel[0] > 180 && pixel[1] < 60 && pixel[2] < 60 {
                            state.red_frames += 1;
                            if state.width == 64 {
                                state.width = 128;
                                let _ = handle
                                    .upgrade_in_event_loop(|app| app.window().request_redraw());
                            } else {
                                assert_eq!(
                                    state.surface.target.as_ref().unwrap().size(),
                                    (128, 64)
                                );
                                // A real red GL frame exists first. Privacy must
                                // withhold even a forced resize render, not merely
                                // paint a translucent UI element over that frame.
                                if let Some(app) = handle.upgrade() {
                                    app.set_frame(slint::Image::default());
                                }
                                state.surface.set_privacy_blocked(true);
                                assert!(!state.surface.output_allowed());
                                assert!(
                                    unsafe { state.surface.render(256, 64) }.unwrap().is_none()
                                );
                                assert_eq!(
                                    state.surface.target.as_ref().unwrap().size(),
                                    (128, 64)
                                );
                                state.surface.set_privacy_blocked(false);
                                slint::Timer::single_shot(Duration::ZERO, || {
                                    slint::quit_event_loop().unwrap();
                                });
                            }
                        }
                    }
                }
                slint::RenderingState::AfterRendering => state.surface.report_swap(),
                slint::RenderingState::RenderingTeardown => {
                    if let Some(app) = handle.upgrade() {
                        app.set_frame(slint::Image::default());
                    }
                    unsafe { state.surface.teardown() };
                    state.torn_down = true;
                }
                _ => {}
            }
        })
        .unwrap();
    slint::Timer::single_shot(Duration::from_secs(8), || {
        slint::quit_event_loop().unwrap();
    });
    let poll = slint::Timer::default();
    let poll_state = Rc::clone(&state);
    poll.start(slint::TimerMode::Repeated, Duration::from_millis(100), move || {
        for event in poll_state.borrow_mut().backend.drain_events() {
            if let yoyo_core::BackendEvent::Error(message) = event {
                panic!("mpv smoke playback failed: {message}");
            }
        }
    });
    app.run().unwrap();
    drop(app);
    let state = state.borrow();
    assert!(state.red_frames >= 2, "expected red decoded pixels before and after resize");
    assert!(state.torn_down, "renderer must release GL resources before the playback core");
    assert!(!state.surface.has_context());
}
