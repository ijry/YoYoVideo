use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, BackendEvent, MediaLocator, MediaTrack,
    MediaTrackKind, PlayerBackend,
};

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
    pending_events: Vec<BackendEvent>,
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
        std::mem::take(&mut self.pending_events)
    }
}

fn track(id: i64, kind: MediaTrackKind, title: &str, selected: bool) -> MediaTrack {
    MediaTrack {
        id,
        kind,
        title: Some(title.into()),
        language: None,
        codec: None,
        source_path: None,
        external: false,
        selected,
    }
}

#[test]
fn selecting_subtitle_track_enables_subtitles_and_sends_backend_command() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.backend_mut().pending_events.push(BackendEvent::TracksChanged {
        audio: vec![],
        subtitles: vec![
            track(3, MediaTrackKind::Subtitle, "English", false),
            track(4, MediaTrackKind::Subtitle, "Commentary", false),
        ],
        video: vec![],
    });
    session.poll_backend().unwrap();

    session.handle_command(AppCommand::SelectSubtitleTrack(4)).unwrap();

    assert_eq!(session.backend().commands, vec![BackendCommand::SelectSubtitleTrack(4)]);
    assert!(session.state().subtitle.visible);
    assert_eq!(session.state().selected_subtitle_track_id(), Some(4));
}

#[test]
fn tracks_event_updates_selected_track_helpers_and_external_subtitle_path() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut external = track(8, MediaTrackKind::Subtitle, "external.ass", true);
    external.external = true;
    external.source_path = Some(PathBuf::from("D:/subs/external.ass"));

    session.backend_mut().pending_events.push(BackendEvent::TracksChanged {
        audio: vec![track(2, MediaTrackKind::Audio, "Japanese", true)],
        subtitles: vec![external.clone()],
        video: vec![track(1, MediaTrackKind::Video, "Main", true)],
    });
    session.backend_mut().pending_events.push(BackendEvent::SubtitleVisibilityChanged(false));
    session.backend_mut().pending_events.push(BackendEvent::SubtitleDelayChanged(1.25));

    session.poll_backend().unwrap();

    assert_eq!(session.state().selected_audio_track_id(), Some(2));
    assert_eq!(session.state().selected_subtitle_track_id(), Some(8));
    assert_eq!(session.state().selected_video_track_id(), Some(1));
    assert_eq!(session.state().subtitle.external_path, Some(PathBuf::from("D:/subs/external.ass")));
    assert!(!session.state().subtitle.visible);
    assert_eq!(session.state().subtitle.delay_seconds, 1.25);
}

#[test]
fn opening_new_media_clears_track_cache_and_restore_flag() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.backend_mut().pending_events.push(BackendEvent::TracksChanged {
        audio: vec![track(2, MediaTrackKind::Audio, "Japanese", true)],
        subtitles: vec![track(3, MediaTrackKind::Subtitle, "English", true)],
        video: vec![track(1, MediaTrackKind::Video, "Main", true)],
    });
    session.poll_backend().unwrap();
    session.set_subtitle_preferences_restored(true);

    session.handle_command(AppCommand::OpenFile(PathBuf::from("movie-2.mkv"))).unwrap();

    assert!(session.state().audio_tracks.is_empty());
    assert!(session.state().subtitle_tracks.is_empty());
    assert!(session.state().video_tracks.is_empty());
    assert!(!session.state().subtitle_preferences_restored);
    assert_eq!(session.backend().opened, vec![MediaLocator::File(PathBuf::from("movie-2.mkv"))]);
}
