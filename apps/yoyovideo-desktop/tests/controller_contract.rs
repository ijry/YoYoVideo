use yoyo_core::{AppCommand, AppConfig, AppSession, BackendCommand, MediaLocator, PlayerBackend};
use yoyovideo_desktop::DesktopController;

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
}

impl PlayerBackend for MockBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.opened.push(locator.clone());
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.commands.push(command);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<yoyo_core::BackendEvent> {
        Vec::new()
    }
}

#[test]
fn controller_forward_toggle_pause_to_session() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::TogglePause).unwrap();

    assert_eq!(controller.session().backend().commands, vec![BackendCommand::SetPaused(false)]);
}

#[test]
fn controller_open_url_updates_current_media() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::OpenUrl("https://example.com/live.m3u8".into())).unwrap();

    assert_eq!(
        controller.session().backend().opened,
        vec![MediaLocator::Url("https://example.com/live.m3u8".into())]
    );
    assert_eq!(
        controller.session().state().current,
        Some(MediaLocator::Url("https://example.com/live.m3u8".into()))
    );
}
