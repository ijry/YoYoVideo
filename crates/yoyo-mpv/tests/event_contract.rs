use yoyo_core::{BackendEvent, MediaTrack, MediaTrackKind};
use yoyo_mpv::{MpvEvent, map_event};

#[test]
fn pause_event_maps_to_backend_pause_changed() {
    assert_eq!(map_event(MpvEvent::Pause(true)), Some(BackendEvent::PauseChanged(true)));
}

#[test]
fn duration_event_maps_to_backend_duration_changed() {
    assert_eq!(
        map_event(MpvEvent::Duration(Some(120.0))),
        Some(BackendEvent::DurationChanged(Some(120.0)))
    );
}

#[test]
fn end_file_maps_to_backend_eof() {
    assert_eq!(map_event(MpvEvent::EndFile), Some(BackendEvent::EndOfFile));
}

#[test]
fn warning_is_preserved() {
    assert_eq!(
        map_event(MpvEvent::Warning("rotation fallback".into())),
        Some(BackendEvent::Warning("rotation fallback".into()))
    );
}

#[test]
fn track_list_event_maps_to_backend_tracks_changed() {
    let audio = vec![MediaTrack {
        id: 2,
        kind: MediaTrackKind::Audio,
        title: Some("Japanese".into()),
        language: Some("jpn".into()),
        codec: Some("aac".into()),
        source_path: None,
        external: false,
        selected: true,
    }];

    assert_eq!(
        map_event(MpvEvent::Tracks { audio: audio.clone(), subtitles: vec![], video: vec![] }),
        Some(BackendEvent::TracksChanged { audio, subtitles: vec![], video: vec![] })
    );
}

#[test]
fn subtitle_scale_and_position_events_are_preserved() {
    assert_eq!(
        map_event(MpvEvent::SubtitleScale(1.25)),
        Some(BackendEvent::SubtitleScaleChanged(1.25))
    );
    assert_eq!(
        map_event(MpvEvent::SubtitlePosition(90)),
        Some(BackendEvent::SubtitleVerticalPositionChanged(90))
    );
}

#[test]
fn mute_event_maps_to_backend_muted_changed() {
    assert_eq!(map_event(MpvEvent::Muted(true)), Some(BackendEvent::MutedChanged(true)));
}

#[test]
fn chapter_event_maps_to_backend_chapters_changed() {
    let chapters = vec![yoyo_core::MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 }];

    assert_eq!(
        map_event(MpvEvent::Chapters(chapters.clone())),
        Some(BackendEvent::ChaptersChanged(chapters))
    );
}
