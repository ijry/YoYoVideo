use std::cell::RefCell;
use std::rc::Rc;

use yoyo_core::{AppCommand, AppConfig, AppSession, PlayerBackend, ShortcutAction, ShortcutMap};
use yoyo_mpv::MpvBackend;

use crate::platform::scan_media_folder;
use crate::video_texture::VideoTexture;

slint::include_modules!();

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
        self.session.handle_command(command)
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }

    pub fn open_folder(&mut self, path: &std::path::Path) -> Result<(), yoyo_core::AppError> {
        let entries = scan_media_folder(path)?;
        self.session.replace_playlist(entries, 0)
    }

    pub fn dispatch_shortcut(&mut self, gesture: &str) -> Result<(), yoyo_core::AppError> {
        if let Some(command) = dispatch_shortcut(&self.shortcuts, gesture) {
            self.dispatch(command)?;
        }
        Ok(())
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
    let session = AppSession::new(AppConfig::default(), MpvBackend::default());
    let controller = Rc::new(RefCell::new(DesktopController::new(session)));

    app.on_toggle_pause_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::TogglePause).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    app.set_transport_label(
                        crate::format_transport_label(controller.session().state()).into(),
                    );
                }
            }
        }
    });

    app.run()?;
    Ok(())
}
