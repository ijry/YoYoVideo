use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, BackendEvent, FrameStepDirection,
    MediaLocator, PlaybackAccess, PlaybackEndBehavior, PlayerBackend, PlaylistEntry,
    privacy::MediaKey,
};

struct Gate {
    enabled: AtomicBool,
    key: MediaKey,
}
impl PlaybackAccess for Gate {
    fn restricted(&self, key: &MediaKey) -> bool {
        self.enabled.load(Ordering::SeqCst) && *key == self.key
    }
}
#[derive(Default)]
struct RecordingBackend {
    boundaries: Vec<&'static str>,
    opens: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
    events: Vec<BackendEvent>,
    fail_pause: bool,
    fail_stop: bool,
    fail_open: bool,
    lock_during_open: Option<Arc<Gate>>,
}
impl PlayerBackend for RecordingBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.boundaries.push("open");
        if self.fail_open {
            if let Some(gate) = &self.lock_during_open {
                gate.enabled.store(true, Ordering::SeqCst);
            }
            return Err("open failure".into());
        }
        self.opens.push(locator.clone());
        if let Some(gate) = &self.lock_during_open {
            gate.enabled.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        if command == BackendCommand::Stop {
            self.boundaries.push("stop");
        }
        self.commands.push(command.clone());
        if (self.fail_pause && command == BackendCommand::SetPaused(true))
            || (self.fail_stop && command == BackendCommand::Stop)
        {
            return Err("backend failure".into());
        }
        Ok(())
    }
    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.events)
    }
}
fn private() -> MediaLocator {
    MediaLocator::File(PathBuf::from("protected.mp4"))
}
fn ordinary() -> MediaLocator {
    MediaLocator::File(PathBuf::from("ordinary.mp4"))
}
fn setup() -> (AppSession<RecordingBackend>, Arc<Gate>) {
    let guard = Arc::new(Gate {
        enabled: AtomicBool::new(false),
        key: MediaKey::from_locator(&private()).unwrap(),
    });
    let mut session = AppSession::new(AppConfig::default(), RecordingBackend::default());
    session.set_playback_access(guard.clone());
    (session, guard)
}
fn open(session: &mut AppSession<RecordingBackend>, locator: MediaLocator) {
    session.replace_playlist(vec![PlaylistEntry::new(locator)], 0).unwrap();
}

#[test]
fn protecting_playing_or_paused_media_mutes_then_pauses_idempotently() {
    for paused in [false, true] {
        let (mut session, guard) = setup();
        open(&mut session, private());
        if paused {
            session.handle_command(AppCommand::SetPaused(true)).unwrap();
        }
        session.backend_mut().commands.clear();
        guard.enabled.store(true, Ordering::SeqCst);
        session.enforce_privacy().unwrap();
        assert!(session.privacy_blocked());
        assert!(session.state().paused);
        assert_eq!(
            session.backend().commands,
            vec![BackendCommand::SetMuted(true), BackendCommand::SetPaused(true)]
        );
        session.enforce_privacy().unwrap();
        assert_eq!(session.backend().commands.len(), 2);
    }
}
#[test]
fn set_paused_is_idempotent_not_a_toggle() {
    let (mut session, _) = setup();
    open(&mut session, ordinary());
    session.handle_command(AppCommand::SetPaused(true)).unwrap();
    session.handle_command(AppCommand::SetPaused(true)).unwrap();
    assert!(session.state().paused);
    assert!(!session.backend().commands.contains(&BackendCommand::SetPaused(false)));
}
#[test]
fn blocked_open_cannot_mutate_current_playlist_or_track_state() {
    let (mut session, guard) = setup();
    open(&mut session, ordinary());
    session
        .handle_command(AppCommand::AddMarkerAtCurrentPosition { created_at: "2026-10-08".into() })
        .unwrap();
    let before = session.state().clone();
    let queue = session.playlist_snapshot();
    guard.enabled.store(true, Ordering::SeqCst);
    assert!(session.handle_command(AppCommand::OpenFile(PathBuf::from("protected.mp4"))).is_err());
    assert!(session.replace_playlist(vec![PlaylistEntry::new(private())], 0).is_err());
    assert_eq!(session.state(), &before);
    assert_eq!(session.playlist_snapshot(), queue);
    assert_eq!(session.backend().opens.len(), 1);
}
#[test]
fn backend_open_failure_also_keeps_previous_queue() {
    let (mut session, _) = setup();
    open(&mut session, ordinary());
    let before = session.state().clone();
    let queue = session.playlist_snapshot();
    session.backend_mut().fail_open = true;
    assert!(session.handle_command(AppCommand::OpenFile(PathBuf::from("second.mp4"))).is_err());
    assert_eq!(session.state(), &before);
    assert_eq!(session.playlist_snapshot(), queue);
}
#[test]
fn playlist_selection_and_eof_cannot_open_a_protected_target() {
    for mode in [PlaybackEndBehavior::PlayNext, PlaybackEndBehavior::LoopPlaylist] {
        let (mut session, guard) = setup();
        let mut config = AppConfig::default();
        config.playback.end_behavior = mode;
        session.set_config(config);
        session
            .replace_playlist(
                vec![PlaylistEntry::new(ordinary()), PlaylistEntry::new(private())],
                0,
            )
            .unwrap();
        guard.enabled.store(true, Ordering::SeqCst);
        assert!(session.open_playlist_index(1).is_err());
        assert_eq!(session.playlist_snapshot().current_index, Some(0));
        session.backend_mut().events.push(BackendEvent::EndOfFile);
        assert!(session.poll_backend().is_err());
        assert_eq!(session.backend().opens.len(), 1);
        assert_eq!(session.state().current, Some(ordinary()));
        assert!(session.state().paused);
    }
}
#[test]
fn restoring_or_exporting_frames_is_denied_while_restricted() {
    let commands = vec![
        AppCommand::TogglePause,
        AppCommand::SetPaused(false),
        AppCommand::SeekRelative(1.0),
        AppCommand::SeekAbsolute(8.0),
        AppCommand::JumpToTime(3.0),
        AppCommand::StepFrame(FrameStepDirection::Next),
        AppCommand::TakeScreenshot("shot.png".into()),
        AppCommand::SeekToChapter(0),
        AppCommand::SeekToNextChapterOrMarker,
    ];
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    session.backend_mut().commands.clear();
    for command in commands {
        assert!(session.handle_command(command).is_err());
    }
    assert!(session.backend().commands.is_empty());
    assert!(session.state().paused);
}
#[test]
fn privacy_mute_does_not_become_user_preference_and_unlock_never_resumes() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    session.backend_mut().events.push(BackendEvent::MutedChanged(true));
    session.poll_backend().unwrap();
    assert!(!session.state().muted);
    session.handle_command(AppCommand::SetMuted(true)).unwrap();
    session.handle_command(AppCommand::SetMuted(false)).unwrap();
    session.handle_command(AppCommand::SetVolume(37)).unwrap();
    assert!(!session.backend().commands.contains(&BackendCommand::SetMuted(false)));
    guard.enabled.store(false, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    assert!(session.state().paused);
    assert!(!session.state().muted);
    assert_eq!(session.state().volume_percent, 37);
    assert_eq!(session.backend().commands.last(), Some(&BackendCommand::SetMuted(false)));
    session
        .backend_mut()
        .events
        .extend([BackendEvent::MutedChanged(true), BackendEvent::MutedChanged(false)]);
    session.poll_backend().unwrap();
    assert!(!session.state().muted);
    assert!(!session.backend().commands.contains(&BackendCommand::SetPaused(false)));
}
#[test]
fn stale_unpause_events_and_eof_do_not_undo_protection() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    session.backend_mut().events.extend([
        BackendEvent::PauseChanged(false),
        BackendEvent::MutedChanged(false),
        BackendEvent::EndOfFile,
    ]);
    session.poll_backend().unwrap();
    assert!(session.state().paused);
    assert_eq!(session.backend().opens.len(), 1);
    assert_eq!(session.backend().commands.last(), Some(&BackendCommand::SetPaused(true)));
}
#[test]
fn pause_failure_unloads_keeps_progress_and_only_explicit_resume_reopens() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    session.backend_mut().events.push(BackendEvent::PositionChanged(42.0));
    session.poll_backend().unwrap();
    session.backend_mut().fail_pause = true;
    guard.enabled.store(true, Ordering::SeqCst);
    assert!(session.enforce_privacy().is_err());
    assert_eq!(session.backend().commands.last(), Some(&BackendCommand::Stop));
    assert_eq!(session.state().current, Some(private()));
    assert_eq!(session.state().position_seconds, 42.0);
    assert!(session.state().paused);
    guard.enabled.store(false, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    assert_eq!(session.backend().opens.len(), 1);
    assert!(session.state().paused);
    session.backend_mut().fail_pause = false;
    session.handle_command(AppCommand::SetPaused(false)).unwrap();
    assert_eq!(session.backend().opens.len(), 2);
    session.backend_mut().events.push(BackendEvent::DurationChanged(Some(90.0)));
    session.poll_backend().unwrap();
    assert!(session.backend().commands.contains(&BackendCommand::SeekAbsolute(42.0)));
}
#[test]
fn ordinary_media_can_replace_a_locked_item_without_inheriting_forced_mute() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    open(&mut session, ordinary());
    assert!(!session.privacy_blocked());
    assert!(!session.state().paused);
    assert!(session.backend().commands.contains(&BackendCommand::SetMuted(false)));
    assert_eq!(session.current_media_key(), Some(&MediaKey::from_locator(&ordinary()).unwrap()));
}

#[test]
fn a_new_load_gets_its_own_pause_even_if_previous_media_was_already_concealed() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    guard.enabled.store(false, Ordering::SeqCst);
    session.backend_mut().lock_during_open = Some(guard.clone());
    session.backend_mut().commands.clear();
    open(&mut session, private());
    assert!(session.state().paused);
    assert!(
        session
            .backend()
            .commands
            .ends_with(&[BackendCommand::SetMuted(true), BackendCommand::SetPaused(true)])
    );
}

#[test]
fn a_backend_error_crossing_a_restriction_boundary_cannot_disclose_the_target() {
    let (mut session, guard) = setup();
    open(&mut session, ordinary());
    session.backend_mut().fail_open = true;
    session.backend_mut().lock_during_open = Some(guard);
    let error = session.handle_command(AppCommand::OpenFile("protected.mp4".into())).unwrap_err();
    assert_eq!(error.to_string(), "Protected content");
    assert_eq!(session.state().current, Some(ordinary()));
}
#[test]
fn explicit_ordinary_open_after_a_privacy_pause_resumes_the_new_backend_load() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    session.backend_mut().commands.clear();
    open(&mut session, ordinary());
    assert!(session.backend().commands.contains(&BackendCommand::SetPaused(false)));
    assert!(session.state().status_message.is_none());
}
#[test]
fn unlocking_removes_the_privacy_status_without_sending_a_resume() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    guard.enabled.store(false, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    assert!(session.state().status_message.is_none());
    assert!(session.state().paused);
    assert!(!session.backend().commands.contains(&BackendCommand::SetPaused(false)));
}

#[test]
fn opening_ordinary_media_discards_protected_output_before_starting_the_new_load() {
    let (mut session, guard) = setup();
    open(&mut session, private());
    guard.enabled.store(true, Ordering::SeqCst);
    session.enforce_privacy().unwrap();
    session.backend_mut().boundaries.clear();
    open(&mut session, ordinary());
    assert_eq!(session.backend().boundaries, vec!["stop", "open"]);
}
