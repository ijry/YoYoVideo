use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[cfg(feature = "mpv-runtime")]
use i_slint_backend_winit::{Backend as WinitBackend, CustomApplicationHandler, EventResult};
use slint::winit_030::WinitWindowAccessor;
#[cfg(feature = "mpv-runtime")]
use yoyo_core::AppConfig;
use yoyo_core::{AppCommand, AppSession, PlayerBackend, PlayerState, ShortcutAction, ShortcutMap};
use yoyo_mpv::{MpvBackend, MpvError};

use crate::NativeVideoWindowId;
#[cfg(feature = "mpv-runtime")]
use crate::VideoHost;
use crate::platform::{DialogService, RfdDialogService, scan_media_folder};
#[cfg(feature = "mpv-runtime")]
use crate::video_host_winit::WinitVideoHost;
use crate::video_texture::VideoTexture;

slint::include_modules!();

pub fn build_desktop_backend() -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime()
}

pub fn build_desktop_backend_with_video_window(
    window_id: NativeVideoWindowId,
) -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime_with_options(yoyo_mpv::MpvClientOptions {
        video_window: Some(yoyo_mpv::MpvVideoWindow::new(window_id.0)),
        force_window: true,
        profile: None,
    })
}

pub fn refresh_window(window: &MainWindow, state: &PlayerState) {
    window.set_transport_label(crate::format_transport_label(state).into());
    window.set_speed_label(crate::format_speed_label(state).into());
    window.set_time_label(crate::format_time_label(state).into());
    window.set_volume_label(crate::format_volume_label(state).into());
    window.set_rotation_label(crate::format_rotation_label(state).into());
    window.set_audio_channel_label(crate::format_audio_channel_label(state).into());
    window.set_zoom_label(crate::format_zoom_label(state).into());
    window.set_loop_label(crate::format_loop_label(state).into());
    window.set_progress_value(crate::progress_ratio(state));
    window.set_volume_value(i32::from(state.volume_percent));
    window.set_status_label(
        state
            .last_error
            .clone()
            .or_else(|| state.status_message.clone())
            .unwrap_or_default()
            .into(),
    );
}

pub struct DesktopController<B: PlayerBackend> {
    session: AppSession<B>,
    shortcuts: ShortcutMap,
    #[allow(dead_code)]
    video_texture: VideoTexture,
}

impl<B: PlayerBackend> DesktopController<B> {
    pub fn new(session: AppSession<B>) -> Self {
        Self { session, shortcuts: ShortcutMap::default(), video_texture: VideoTexture::default() }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), yoyo_core::AppError> {
        self.session.handle_command(command)?;
        self.session.poll_backend()?;
        Ok(())
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }

    pub fn open_folder(&mut self, path: &std::path::Path) -> Result<(), yoyo_core::AppError> {
        let entries = scan_media_folder(path)?;
        self.session.replace_playlist(entries, 0)?;
        self.session.poll_backend()
    }

    pub fn dispatch_shortcut(&mut self, gesture: &str) -> Result<(), yoyo_core::AppError> {
        if let Some(command) = dispatch_shortcut(&self.shortcuts, gesture) {
            self.dispatch(command)?;
        }
        Ok(())
    }

    pub fn poll_backend(&mut self) -> Result<(), yoyo_core::AppError> {
        self.session.poll_backend()
    }
}

pub fn dispatch_shortcut(map: &ShortcutMap, gesture: &str) -> Option<AppCommand> {
    let shortcut = yoyo_core::Shortcut::parse(gesture).ok()?;
    match map.action_for(&shortcut)? {
        ShortcutAction::TogglePause => Some(AppCommand::TogglePause),
        ShortcutAction::SeekBackwardSmall => Some(AppCommand::SeekRelative(-5.0)),
        ShortcutAction::SeekForwardSmall => Some(AppCommand::SeekRelative(5.0)),
        ShortcutAction::VolumeUp => Some(AppCommand::AdjustVolume(5)),
        ShortcutAction::VolumeDown => Some(AppCommand::AdjustVolume(-5)),
        ShortcutAction::SpeedDown => Some(AppCommand::SetSpeed(0.75)),
        ShortcutAction::SpeedUp => Some(AppCommand::SetSpeed(1.25)),
        ShortcutAction::ResetSpeed => Some(AppCommand::ResetSpeed),
        ShortcutAction::SetABLoopPointA => Some(AppCommand::SetABLoopPointA),
        ShortcutAction::SetABLoopPointB => Some(AppCommand::SetABLoopPointB),
        ShortcutAction::ClearABLoop => Some(AppCommand::ClearABLoop),
        ShortcutAction::RotateClockwise => Some(AppCommand::RotateClockwise),
        ShortcutAction::ZoomOut => Some(AppCommand::ZoomOut),
        ShortcutAction::ZoomIn => Some(AppCommand::ZoomIn),
        ShortcutAction::CycleAudioChannel => Some(AppCommand::CycleAudioChannel),
        ShortcutAction::ToggleFullscreen => Some(AppCommand::ToggleFullscreen),
        ShortcutAction::OpenFile | ShortcutAction::OpenUrl => None,
    }
}

struct DesktopRuntime {
    controller: Option<DesktopController<MpvBackend>>,
    video_host_error: Option<String>,
    app_handle: Option<slint::Weak<MainWindow>>,
    #[cfg(feature = "mpv-runtime")]
    video_host: Option<WinitVideoHost>,
}

impl DesktopRuntime {
    fn new() -> Self {
        Self {
            controller: None,
            video_host_error: initial_runtime_error(),
            app_handle: None,
            #[cfg(feature = "mpv-runtime")]
            video_host: None,
        }
    }

    fn controller(&self) -> Option<&DesktopController<MpvBackend>> {
        self.controller.as_ref()
    }

    fn controller_mut(&mut self) -> Option<&mut DesktopController<MpvBackend>> {
        self.controller.as_mut()
    }

    fn status_message(&self) -> String {
        self.video_host_error
            .clone()
            .unwrap_or_else(|| "Playback runtime is still initializing".to_string())
    }

    #[cfg(feature = "mpv-runtime")]
    fn mark_error(&mut self, message: impl Into<String>) {
        self.video_host_error = Some(message.into());
    }

    #[cfg(feature = "mpv-runtime")]
    fn set_runtime(
        &mut self,
        controller: DesktopController<MpvBackend>,
        video_host: WinitVideoHost,
    ) {
        self.controller = Some(controller);
        self.video_host = Some(video_host);
        self.video_host_error = None;
    }
}

#[cfg(feature = "mpv-runtime")]
fn initial_runtime_error() -> Option<String> {
    None
}

#[cfg(not(feature = "mpv-runtime"))]
fn initial_runtime_error() -> Option<String> {
    Some("Playback runtime is disabled in this build".to_string())
}

fn refresh_runtime_window(window: &MainWindow, runtime: &DesktopRuntime) {
    if let Some(controller) = runtime.controller() {
        refresh_window(window, controller.session().state());
    } else {
        window.set_status_label(runtime.status_message().into());
    }
}

fn ensure_runtime_ready(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
) -> bool {
    let runtime = runtime.borrow();
    if runtime.controller().is_some() {
        return true;
    }

    if let Some(app) = app_handle.upgrade() {
        app.set_status_label(runtime.status_message().into());
    }
    false
}

fn with_runtime_controller<F>(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    action: F,
) where
    F: FnOnce(&mut DesktopController<MpvBackend>) -> Result<(), yoyo_core::AppError>,
{
    let mut runtime = runtime.borrow_mut();
    let Some(controller) = runtime.controller_mut() else {
        if let Some(app) = app_handle.upgrade() {
            app.set_status_label(runtime.status_message().into());
        }
        return;
    };

    match action(controller) {
        Ok(()) => {
            if let Some(app) = app_handle.upgrade() {
                refresh_window(&app, controller.session().state());
                apply_fullscreen_state(&app, controller.session().state());
            }
        }
        Err(error) => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(error.to_string().into());
            }
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt().with_target(false).try_init();

    let runtime = Rc::new(RefCell::new(DesktopRuntime::new()));
    configure_backend(Rc::clone(&runtime))?;

    let app = MainWindow::new()?;
    runtime.borrow_mut().app_handle = Some(app.as_weak());
    let dialogs = Rc::new(RfdDialogService);

    refresh_runtime_window(&app, &runtime.borrow());

    app.on_open_file_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if !ensure_runtime_ready(&app_handle, &runtime) {
                return;
            }
            if let Some(path) = dialogs.pick_file() {
                with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.dispatch(AppCommand::OpenFile(path))
                });
            }
        }
    });

    app.on_open_folder_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if !ensure_runtime_ready(&app_handle, &runtime) {
                return;
            }
            if let Some(path) = dialogs.pick_folder() {
                with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.open_folder(&path)
                });
            }
        }
    });

    app.on_open_url_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |url| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::OpenUrl(url.to_string()))
            });
        }
    });

    app.on_toggle_pause_requested(command_callback(&app, &runtime, AppCommand::TogglePause));
    app.on_speed_down_requested(command_callback(&app, &runtime, AppCommand::SetSpeed(0.75)));
    app.on_speed_up_requested(command_callback(&app, &runtime, AppCommand::SetSpeed(1.25)));
    app.on_reset_speed_requested(command_callback(&app, &runtime, AppCommand::ResetSpeed));
    app.on_seek_percent_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |percent| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                let Some(duration) = controller.session().state().duration_seconds else {
                    return Ok(());
                };
                let position = duration * f64::from(percent.clamp(0.0, 1.0));
                controller.dispatch(AppCommand::SeekAbsolute(position))
            });
        }
    });
    app.on_volume_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |volume| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::SetVolume(volume.clamp(0, 100) as u8))
            });
        }
    });
    app.on_rotate_requested(command_callback(&app, &runtime, AppCommand::RotateClockwise));
    app.on_cycle_audio_requested(command_callback(&app, &runtime, AppCommand::CycleAudioChannel));
    app.on_zoom_in_requested(command_callback(&app, &runtime, AppCommand::ZoomIn));
    app.on_zoom_out_requested(command_callback(&app, &runtime, AppCommand::ZoomOut));
    app.on_set_ab_point_a_requested(command_callback(&app, &runtime, AppCommand::SetABLoopPointA));
    app.on_set_ab_point_b_requested(command_callback(&app, &runtime, AppCommand::SetABLoopPointB));
    app.on_clear_ab_loop_requested(command_callback(&app, &runtime, AppCommand::ClearABLoop));
    app.on_toggle_fullscreen_requested(command_callback(
        &app,
        &runtime,
        AppCommand::ToggleFullscreen,
    ));

    app.on_settings_requested({
        let app_handle = app.as_weak();
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label("Settings persistence is enabled".into());
            }
        }
    });

    let keyboard_state =
        Rc::new(RefCell::new(crate::keyboard::winit_adapter::WinitKeyboardState::default()));
    app.window().on_winit_window_event({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let keyboard_state = Rc::clone(&keyboard_state);
        move |_window, event| {
            let Some(app) = app_handle.upgrade() else {
                return slint::winit_030::EventResult::Propagate;
            };
            if !crate::shortcut_allowed(app.get_url_focused()) {
                return slint::winit_030::EventResult::Propagate;
            }
            let Some(input) = keyboard_state.borrow_mut().update(event) else {
                return slint::winit_030::EventResult::Propagate;
            };
            let Some(gesture) = crate::shortcut_gesture(input) else {
                return slint::winit_030::EventResult::Propagate;
            };

            let mut runtime = runtime.borrow_mut();
            let Some(controller) = runtime.controller_mut() else {
                app.set_status_label(runtime.status_message().into());
                return slint::winit_030::EventResult::PreventDefault;
            };

            if controller.dispatch_shortcut(gesture).is_ok() {
                refresh_window(&app, controller.session().state());
                apply_fullscreen_state(&app, controller.session().state());
            }
            slint::winit_030::EventResult::PreventDefault
        }
    });

    let poll_timer = slint::Timer::default();
    poll_timer.start(slint::TimerMode::Repeated, Duration::from_millis(250), {
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            let Some(app) = app_handle.upgrade() else {
                return;
            };

            let mut runtime = runtime.borrow_mut();
            if let Some(controller) = runtime.controller_mut() {
                match controller.poll_backend() {
                    Ok(()) => {
                        refresh_window(&app, controller.session().state());
                        #[cfg(feature = "mpv-runtime")]
                        sync_runtime_video_host(&app, &mut runtime);
                    }
                    Err(error) => app.set_status_label(error.to_string().into()),
                }
            } else {
                refresh_runtime_window(&app, &runtime);
            }
        }
    });

    app.run()?;
    Ok(())
}

fn command_callback(
    app: &MainWindow,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    command: AppCommand,
) -> impl Fn() + 'static {
    let app_handle = app.as_weak();
    let runtime = Rc::clone(runtime);
    move || {
        with_runtime_controller(&app_handle, &runtime, |controller| {
            controller.dispatch(command.clone())
        });
    }
}

fn apply_fullscreen_state(app: &MainWindow, state: &PlayerState) {
    app.window().with_winit_window(|winit_window| {
        if state.fullscreen {
            winit_window.set_fullscreen(Some(
                slint::winit_030::winit::window::Fullscreen::Borderless(None),
            ));
        } else {
            winit_window.set_fullscreen(None);
        }
    });
}

#[cfg(feature = "mpv-runtime")]
fn current_video_rect(window: &MainWindow) -> crate::LogicalVideoRect {
    crate::LogicalVideoRect {
        x: window.get_video_area_x() as f32,
        y: window.get_video_area_y() as f32,
        width: window.get_video_area_width() as f32,
        height: window.get_video_area_height() as f32,
    }
}

#[cfg(feature = "mpv-runtime")]
fn sync_video_host_bounds<H: crate::VideoHost>(
    window: &MainWindow,
    host: &mut H,
    scale_factor: f64,
) -> Result<(), crate::VideoHostError> {
    let bounds = current_video_rect(window).to_physical(scale_factor);
    host.set_bounds(bounds)?;
    host.show()
}

#[cfg(feature = "mpv-runtime")]
fn sync_runtime_video_host(window: &MainWindow, runtime: &mut DesktopRuntime) {
    if runtime.video_host.is_none() {
        return;
    }
    let Some(scale_factor) =
        window.window().with_winit_window(|winit_window| winit_window.scale_factor())
    else {
        return;
    };

    let result = {
        let host = runtime.video_host.as_mut().expect("video host checked above");
        sync_video_host_bounds(window, host, scale_factor).map_err(|error| error.to_string())
    };

    if let Err(error) = result {
        if let Some(host) = runtime.video_host.as_mut() {
            let _ = host.hide();
        }
        runtime.mark_error(error.clone());
        window.set_status_label(error.into());
    }
}

#[cfg(feature = "mpv-runtime")]
fn configure_backend(
    runtime: Rc<RefCell<DesktopRuntime>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = WinitBackend::builder()
        .with_custom_application_handler(Box::new(DesktopWinitHandler::new(runtime)))
        .build()?;
    slint::platform::set_platform(Box::new(backend))?;
    Ok(())
}

#[cfg(not(feature = "mpv-runtime"))]
fn configure_backend(
    _runtime: Rc<RefCell<DesktopRuntime>>,
) -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new().backend_name("winit".into()).select()?;
    Ok(())
}

#[cfg(feature = "mpv-runtime")]
struct DesktopWinitHandler {
    runtime: Rc<RefCell<DesktopRuntime>>,
}

#[cfg(feature = "mpv-runtime")]
impl DesktopWinitHandler {
    fn new(runtime: Rc<RefCell<DesktopRuntime>>) -> Self {
        Self { runtime }
    }

    fn initialize_runtime(
        &mut self,
        event_loop: &slint::winit_030::winit::event_loop::ActiveEventLoop,
        winit_window: Option<&slint::winit_030::winit::window::Window>,
    ) {
        let Some(parent_window) = winit_window else {
            return;
        };

        let mut runtime = self.runtime.borrow_mut();
        if runtime.controller.is_some() || runtime.video_host_error.is_some() {
            return;
        }

        let result = (|| -> Result<(DesktopController<MpvBackend>, WinitVideoHost), String> {
            let video_host = WinitVideoHost::new_child(event_loop, parent_window)
                .map_err(|error| error.to_string())?;
            let window_id = video_host.mpv_window_id().map_err(|error| error.to_string())?;
            let backend = build_desktop_backend_with_video_window(window_id)
                .map_err(|error| error.to_string())?;
            let session = AppSession::new(AppConfig::default(), backend);
            Ok((DesktopController::new(session), video_host))
        })();

        match result {
            Ok((controller, video_host)) => {
                runtime.set_runtime(controller, video_host);
                let app_handle = runtime.app_handle.clone();
                if let Some(app_handle) = app_handle {
                    if let Some(app) = app_handle.upgrade() {
                        if let Some(controller) = runtime.controller() {
                            refresh_window(&app, controller.session().state());
                        }
                        sync_runtime_video_host(&app, &mut runtime);
                    }
                }
            }
            Err(error) => runtime.mark_error(error),
        }
    }
}

#[cfg(feature = "mpv-runtime")]
impl CustomApplicationHandler for DesktopWinitHandler {
    fn window_event(
        &mut self,
        event_loop: &slint::winit_030::winit::event_loop::ActiveEventLoop,
        _window_id: slint::winit_030::winit::window::WindowId,
        winit_window: Option<&slint::winit_030::winit::window::Window>,
        _slint_window: Option<&slint::Window>,
        _event: &slint::winit_030::winit::event::WindowEvent,
    ) -> EventResult {
        self.initialize_runtime(event_loop, winit_window);
        EventResult::Propagate
    }
}
