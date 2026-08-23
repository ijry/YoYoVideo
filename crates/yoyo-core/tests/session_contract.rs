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
    assert_eq!(session.backend().commands, vec![BackendCommand::SetRotation(Rotation::Deg90)]);
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

#[test]
fn eof_stop_behavior_does_not_advance_playlist() {
    let backend = MockBackend::default();
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::Stop;
    let mut session = AppSession::new(config, backend);
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

    assert_eq!(session.backend().opened, vec![MediaLocator::File(PathBuf::from("one.mp4"))]);
    assert!(session.state().paused);
    assert_eq!(session.state().status_message.as_deref(), Some("Playback ended"));
}

#[test]
fn eof_loop_current_reopens_current_playlist_item() {
    let backend = MockBackend::default();
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::LoopCurrent;
    let mut session = AppSession::new(config, backend);
    session
        .replace_playlist(vec![PlaylistEntry::new(MediaLocator::File(PathBuf::from("one.mp4")))], 0)
        .unwrap();

    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(
        session.backend().opened,
        vec![
            MediaLocator::File(PathBuf::from("one.mp4")),
            MediaLocator::File(PathBuf::from("one.mp4")),
        ]
    );
    assert!(!session.state().paused);
}

#[test]
fn eof_loop_playlist_wraps_from_last_item_to_first() {
    let backend = MockBackend::default();
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::LoopPlaylist;
    let mut session = AppSession::new(config, backend);
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("one.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("two.mp4"))),
            ],
            1,
        )
        .unwrap();

    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(
        session.backend().opened,
        vec![
            MediaLocator::File(PathBuf::from("two.mp4")),
            MediaLocator::File(PathBuf::from("one.mp4")),
        ]
    );
    assert_eq!(session.playlist_snapshot().current_index, Some(0));
}

#[test]
fn replacing_session_config_changes_future_eof_behavior() {
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
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::Stop;

    session.set_config(config);
    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(session.backend().opened, vec![MediaLocator::File(PathBuf::from("one.mp4"))]);
}

#[test]
fn toggle_mute_updates_state_and_backend() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::ToggleMute).unwrap();

    assert!(session.state().muted);
    assert_eq!(session.backend().commands, vec![BackendCommand::SetMuted(true)]);

    session.backend_mut().events.push(BackendEvent::MutedChanged(false));
    session.poll_backend().unwrap();

    assert!(!session.state().muted);
}

#[test]
fn jump_to_time_clamps_when_duration_is_known() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session.backend_mut().events.push(BackendEvent::DurationChanged(Some(90.0)));
    session.poll_backend().unwrap();

    session.handle_command(AppCommand::JumpToTime(120.0)).unwrap();

    assert_eq!(session.backend().commands, vec![BackendCommand::SeekAbsolute(90.0)]);
}

#[test]
fn chapters_event_replaces_chapter_state_and_opening_media_clears_it() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    let chapters = vec![
        yoyo_core::MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 },
        yoyo_core::MediaChapter { title: Some("Scene".into()), time_seconds: 42.0 },
    ];

    session.backend_mut().events.push(BackendEvent::ChaptersChanged(chapters.clone()));
    session.poll_backend().unwrap();
    assert_eq!(session.state().chapters, chapters);

    session.handle_command(AppCommand::OpenFile(PathBuf::from("movie.mp4"))).unwrap();
    assert!(session.state().chapters.is_empty());
}

#[test]
fn markers_add_dedupe_remove_and_seek() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session.backend_mut().events.push(BackendEvent::PositionChanged(12.25));
    session.poll_backend().unwrap();

    session
        .handle_command(AppCommand::AddMarkerAtCurrentPosition {
            created_at: "2026-07-06T10:00:00+08:00".into(),
        })
        .unwrap();
    session.backend_mut().events.push(BackendEvent::PositionChanged(12.80));
    session.poll_backend().unwrap();
    session
        .handle_command(AppCommand::AddMarkerAtCurrentPosition {
            created_at: "2026-07-06T10:00:01+08:00".into(),
        })
        .unwrap();

    assert_eq!(session.state().markers.len(), 1);
    let id = session.state().markers[0].id.clone();
    session.handle_command(AppCommand::SeekToMarker(id.clone())).unwrap();
    assert_eq!(session.backend().commands.last(), Some(&BackendCommand::SeekAbsolute(12.25)));

    session.handle_command(AppCommand::RemoveMarker(id)).unwrap();
    assert!(session.state().markers.is_empty());
}

#[test]
fn seek_to_next_and_previous_chapter_or_marker_uses_sorted_points() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session.backend_mut().events.push(BackendEvent::PositionChanged(20.0));
    session.backend_mut().events.push(BackendEvent::ChaptersChanged(vec![
        yoyo_core::MediaChapter { title: Some("A".into()), time_seconds: 10.0 },
        yoyo_core::MediaChapter { title: Some("B".into()), time_seconds: 50.0 },
    ]));
    session.poll_backend().unwrap();
    session.set_markers(vec![yoyo_core::MediaMarker {
        id: "marker-30000".into(),
        title: "Marker 00:30".into(),
        time_seconds: 30.0,
        created_at: "2026-07-06T10:00:00+08:00".into(),
    }]);

    session.handle_command(AppCommand::SeekToNextChapterOrMarker).unwrap();
    session.handle_command(AppCommand::SeekToPreviousChapterOrMarker).unwrap();

    assert_eq!(
        session.backend().commands,
        vec![BackendCommand::SeekAbsolute(30.0), BackendCommand::SeekAbsolute(10.0)]
    );
}

#[test]
fn stop_unloads_the_file_and_clears_per_media_state() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();
    session.handle_command(AppCommand::SetVolume(70)).unwrap();
    session.handle_command(AppCommand::SetABLoopPointA).unwrap();
    assert!(session.state().current.is_some());

    session.handle_command(AppCommand::Stop).unwrap();

    let state = session.state();
    assert_eq!(state.current, None, "the file is closed");
    assert!(state.paused, "playback stops");
    assert_eq!(state.position_seconds, 0.0);
    assert_eq!(state.duration_seconds, None);
    assert_eq!(state.loop_state, yoyo_core::LoopState::default(), "AB loop is cleared");
    assert!(state.audio_tracks.is_empty());
    assert!(state.subtitle_tracks.is_empty());
    assert!(state.chapters.is_empty());

    // Preferences that outlive a file must survive.
    assert_eq!(state.volume_percent, 70, "volume is a user preference, not media state");
}

#[test]
fn stop_tells_the_backend_to_stop() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();

    session.handle_command(AppCommand::Stop).unwrap();

    assert!(
        session.backend().commands.contains(&BackendCommand::Stop),
        "the backend has to unload the file, not just pause"
    );
}

#[test]
fn stop_empties_the_playlist_selection() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();

    session.handle_command(AppCommand::Stop).unwrap();

    // Nothing is loaded, so there is no current playlist position to resume from.
    assert_eq!(session.playlist_snapshot().current_index, None);
}

#[test]
fn events_queued_before_stop_do_not_resurrect_the_old_time() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();
    session.backend_mut().events.push(BackendEvent::DurationChanged(Some(6.0)));
    session.backend_mut().events.push(BackendEvent::PositionChanged(5.0));
    session.poll_backend().unwrap();
    assert_eq!(session.state().position_seconds, 5.0);

    session.handle_command(AppCommand::Stop).unwrap();

    // mpv keeps delivering whatever was in flight when it went idle. Applying it would
    // put a stale time and a full progress bar back in the deck.
    session.backend_mut().events.push(BackendEvent::PositionChanged(5.5));
    session.backend_mut().events.push(BackendEvent::DurationChanged(Some(6.0)));
    session.poll_backend().unwrap();

    assert_eq!(session.state().position_seconds, 0.0, "position stays cleared after stop");
    assert_eq!(session.state().duration_seconds, None, "duration stays cleared after stop");
}

#[test]
fn position_events_still_apply_while_a_file_is_loaded() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();

    session.backend_mut().events.push(BackendEvent::PositionChanged(3.5));
    session.poll_backend().unwrap();

    assert_eq!(session.state().position_seconds, 3.5, "the guard must not block normal playback");
}

#[test]
fn adjust_volume_saturates_at_the_ends() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());

    session.handle_command(AppCommand::SetVolume(98)).unwrap();
    session.handle_command(AppCommand::AdjustVolume(5)).unwrap();
    assert_eq!(session.state().volume_percent, 100, "wheel past the top clamps to 100");

    session.handle_command(AppCommand::SetVolume(3)).unwrap();
    session.handle_command(AppCommand::AdjustVolume(-5)).unwrap();
    assert_eq!(session.state().volume_percent, 0, "wheel past the bottom clamps to 0");
}

#[test]
fn video_dimension_events_update_player_state() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();

    session.backend_mut().events.push(BackendEvent::VideoWidthChanged(Some(1920)));
    session.backend_mut().events.push(BackendEvent::VideoHeightChanged(Some(1080)));
    session.poll_backend().unwrap();

    assert_eq!(session.state().video_width, Some(1920));
    assert_eq!(session.state().video_height, Some(1080));
}

#[test]
fn stop_clears_the_video_dimensions() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.handle_command(AppCommand::OpenFile(PathBuf::from("/tmp/movie.mkv"))).unwrap();
    session.backend_mut().events.push(BackendEvent::VideoWidthChanged(Some(1920)));
    session.backend_mut().events.push(BackendEvent::VideoHeightChanged(Some(1080)));
    session.poll_backend().unwrap();

    session.handle_command(AppCommand::Stop).unwrap();
    assert_eq!(session.state().video_width, None);
    assert_eq!(session.state().video_height, None);

    // In-flight events from before mpv went idle must not resurrect the old size,
    // otherwise a closed tile would keep its aspect ratio.
    session.backend_mut().events.push(BackendEvent::VideoWidthChanged(Some(1920)));
    session.poll_backend().unwrap();
    assert_eq!(session.state().video_width, None);
}
