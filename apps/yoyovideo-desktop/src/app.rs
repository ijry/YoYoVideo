use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, PlayerBackend, PlayerState, ShortcutAction, ShortcutMap,
};
use yoyo_mpv::{MpvBackend, MpvError};

use crate::platform::{DialogService, RfdDialogService, scan_media_folder};
use crate::video_texture::VideoTexture;

slint::include_modules!();

pub fn build_desktop_backend() -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime()
}

pub fn refresh_window(window: &MainWindow, state: &PlayerState) {
    window.set_transport_label(crate::format_transport_label(state).into());
    window.set_speed_label(crate::format_speed_label(state).into());
    window.set_time_label(crate::format_time_label(state).into());
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

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();
    slint::BackendSelector::new().backend_name("winit".into()).select()?;

    let app = MainWindow::new()?;
    let session = AppSession::new(AppConfig::default(), build_desktop_backend()?);
    let controller = Rc::new(RefCell::new(DesktopController::new(session)));
    let dialogs = Rc::new(RfdDialogService);

    refresh_window(&app, controller.borrow().session().state());

    app.on_open_file_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if let Some(path) = dialogs.pick_file() {
                let mut controller = controller.borrow_mut();
                if controller.dispatch(AppCommand::OpenFile(path)).is_ok() {
                    if let Some(app) = app_handle.upgrade() {
                        refresh_window(&app, controller.session().state());
                    }
                }
            }
        }
    });

    app.on_open_folder_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if let Some(path) = dialogs.pick_folder() {
                let mut controller = controller.borrow_mut();
                if controller.open_folder(&path).is_ok() {
                    if let Some(app) = app_handle.upgrade() {
                        refresh_window(&app, controller.session().state());
                    }
                }
            }
        }
    });

    app.on_open_url_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move |url| {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::OpenUrl(url.to_string())).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    app.on_toggle_pause_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::TogglePause).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    app.on_speed_down_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::SetSpeed(0.75)).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    app.on_speed_up_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::SetSpeed(1.25)).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    app.on_rotate_requested(command_callback(&app, &controller, AppCommand::RotateClockwise));
    app.on_cycle_audio_requested(command_callback(
        &app,
        &controller,
        AppCommand::CycleAudioChannel,
    ));
    app.on_set_ab_point_a_requested(command_callback(
        &app,
        &controller,
        AppCommand::SetABLoopPointA,
    ));
    app.on_set_ab_point_b_requested(command_callback(
        &app,
        &controller,
        AppCommand::SetABLoopPointB,
    ));
    app.on_clear_ab_loop_requested(command_callback(&app, &controller, AppCommand::ClearABLoop));
    app.on_toggle_fullscreen_requested(command_callback(
        &app,
        &controller,
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

    let poll_timer = slint::Timer::default();
    poll_timer.start(slint::TimerMode::Repeated, Duration::from_millis(250), {
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.poll_backend().is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    app.run()?;
    Ok(())
}

fn command_callback<B: PlayerBackend + 'static>(
    app: &MainWindow,
    controller: &Rc<RefCell<DesktopController<B>>>,
    command: AppCommand,
) -> impl Fn() + 'static {
    let app_handle = app.as_weak();
    let controller = Rc::clone(controller);
    move || {
        let mut controller = controller.borrow_mut();
        if controller.dispatch(command.clone()).is_ok() {
            if let Some(app) = app_handle.upgrade() {
                refresh_window(&app, controller.session().state());
            }
        }
    }
}
