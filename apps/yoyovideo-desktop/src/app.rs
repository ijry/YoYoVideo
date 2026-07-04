use std::cell::RefCell;
use std::rc::Rc;

use yoyo_core::{AppCommand, AppConfig, AppSession, PlayerBackend};
use yoyo_mpv::MpvBackend;

use crate::video_texture::VideoTexture;

slint::include_modules!();

pub struct DesktopController<B: PlayerBackend> {
    session: AppSession<B>,
    #[allow(dead_code)]
    video_texture: VideoTexture,
}

impl<B: PlayerBackend> DesktopController<B> {
    pub fn new(session: AppSession<B>) -> Self {
        Self {
            session,
            video_texture: VideoTexture::default(),
        }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), yoyo_core::AppError> {
        self.session.handle_command(command)
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;

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
