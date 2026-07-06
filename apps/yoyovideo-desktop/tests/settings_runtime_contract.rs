use std::time::Duration;

use tempfile::tempdir;
use yoyo_core::{
    AppConfig, AppSession, BackendCommand, BackendEvent, MediaLocator, PlaybackEndBehavior,
    PlayerBackend, PlaylistEntry, Shortcut, ShortcutAction, ShortcutMap,
};
use yoyovideo_desktop::{DesktopController, FlushReason, HistoryRuntime};

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
    events: Vec<BackendEvent>,
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

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.events)
    }
}

#[test]
fn controller_uses_replaced_shortcut_maps_immediately() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);
    let mut shortcuts = ShortcutMap::default();
    shortcuts
        .set_binding(ShortcutAction::TogglePause, Some(Shortcut::parse("Ctrl+P").unwrap()))
        .unwrap();

    controller.set_shortcuts(shortcuts);
    controller.dispatch_shortcut("Ctrl+P").unwrap();

    assert_eq!(controller.session().backend().commands, vec![BackendCommand::SetPaused(false)]);
}

#[test]
fn disabling_history_runtime_stops_future_history_writes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), Default::default(), true);

    runtime.set_enabled(false);
    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/new.mp4".into()),
        "New",
        Some(42.0),
    );

    assert!(!runtime.flush_if_needed(Duration::from_secs(5), FlushReason::Pause).unwrap());
    assert!(!path.exists());
}

#[test]
fn saved_playback_end_behavior_updates_active_session() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);
    controller
        .open_playlist_entries(vec![
            PlaylistEntry::new(MediaLocator::File("one.mp4".into())),
            PlaylistEntry::new(MediaLocator::File("two.mp4".into())),
        ])
        .unwrap();
    let mut config = AppConfig::default();
    config.playback.end_behavior = PlaybackEndBehavior::Stop;

    controller.set_config(config);
    controller.session_mut().backend_mut().events.push(BackendEvent::EndOfFile);
    controller.poll_backend().unwrap();

    assert_eq!(controller.session().backend().opened, vec![MediaLocator::File("one.mp4".into())]);
}
