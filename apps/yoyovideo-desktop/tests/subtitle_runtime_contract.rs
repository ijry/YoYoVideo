use yoyo_core::{AppConfig, AppSession, PlayerBackend};
use yoyovideo_desktop::DesktopController;

#[derive(Default)]
struct QuietBackend;

impl PlayerBackend for QuietBackend {
    fn open(&mut self, _locator: &yoyo_core::MediaLocator) -> Result<(), String> {
        Ok(())
    }

    fn send(&mut self, _command: yoyo_core::BackendCommand) -> Result<(), String> {
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<yoyo_core::BackendEvent> {
        Vec::new()
    }
}

#[test]
fn controller_can_mark_subtitle_preferences_restored_without_backend_traffic() {
    let session = AppSession::new(AppConfig::default(), QuietBackend);
    let mut controller = DesktopController::new(session);

    controller.set_subtitle_preferences_restored(true);

    assert!(controller.session().state().subtitle_preferences_restored);
}
