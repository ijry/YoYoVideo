use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, AudioChannelMode, BackendCommand, BackendEvent,
    MediaLocator, PlayerBackend, PlaylistEntry, Rotation,
};

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
fn toggle_pause_emits_backend_pause_command() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::TogglePause).unwrap();

    assert_eq!(session.backend().commands, vec![BackendCommand::SetPaused(false)]);
    assert!(!session.state().paused);
}

#[test]
fn rotate_clockwise_cycles_to_ninety_degrees() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::RotateClockwise).unwrap();

    assert_eq!(session.state().rotation, Rotation::Deg90);
    assert_eq!(
        session.backend().commands,
        vec![BackendCommand::SetRotation(Rotation::Deg90)]
    );
}

#[test]
fn eof_event_opens_next_playlist_item() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("one.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("two.mp4"))),
            ],
            0,
        )
        .unwrap();

    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(
        session.backend().opened,
        vec![
            MediaLocator::File(PathBuf::from("one.mp4")),
            MediaLocator::File(PathBuf::from("two.mp4")),
        ]
    );
}

#[test]
fn cycle_audio_channel_visits_left_then_right() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::CycleAudioChannel).unwrap();
    session.handle_command(AppCommand::CycleAudioChannel).unwrap();

    assert_eq!(session.state().audio_channel, AudioChannelMode::MonoRight);
}
