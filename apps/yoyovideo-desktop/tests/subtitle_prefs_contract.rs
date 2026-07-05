use std::path::PathBuf;
use std::time::Duration;

use tempfile::tempdir;
use yoyo_core::{
    AppCommand, MediaLocator, MediaTrack, MediaTrackKind, PlayerState, SubtitlePlaybackState,
};
use yoyovideo_desktop::{SubtitlePrefsFlushReason, SubtitlePrefsRuntime, SubtitleRestoreError};

fn selected_track(id: i64, kind: MediaTrackKind, title: &str) -> MediaTrack {
    MediaTrack {
        id,
        kind,
        title: Some(title.into()),
        language: None,
        codec: None,
        source_path: None,
        external: false,
        selected: true,
    }
}

#[test]
fn subtitle_prefs_runtime_persists_restore_plan_for_a_media_item() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("subtitle_prefs.json");
    let mut runtime = SubtitlePrefsRuntime::load(Some(path.clone())).unwrap();

    let mut state = PlayerState::default();
    state.current = Some(MediaLocator::File(PathBuf::from("movie.mkv")));
    state.audio_tracks = vec![selected_track(2, MediaTrackKind::Audio, "Japanese")];
    state.subtitle_tracks = vec![selected_track(8, MediaTrackKind::Subtitle, "English")];
    state.video_tracks = vec![selected_track(1, MediaTrackKind::Video, "Main")];
    state.subtitle = SubtitlePlaybackState {
        visible: true,
        delay_seconds: 1.5,
        scale: 1.25,
        vertical_position_percent: 88,
        external_path: None,
    };

    runtime.remember_from_state(&state);
    assert!(
        runtime
            .flush_if_needed(Duration::from_secs(0), SubtitlePrefsFlushReason::MediaSwitch)
            .unwrap()
    );

    let reloaded = SubtitlePrefsRuntime::load(Some(path)).unwrap();
    let plan = reloaded
        .restore_plan_for(&MediaLocator::File(PathBuf::from("movie.mkv")))
        .unwrap()
        .unwrap();

    assert_eq!(
        plan.commands,
        vec![
            AppCommand::SelectAudioTrack(2),
            AppCommand::SelectVideoTrack(1),
            AppCommand::SelectSubtitleTrack(8),
            AppCommand::SetSubtitleVisible(true),
            AppCommand::SetSubtitleDelay(1.5),
            AppCommand::SetSubtitleScale(1.25),
            AppCommand::SetSubtitleVerticalPosition(88),
        ]
    );
}

#[test]
fn missing_external_subtitle_file_returns_a_restore_error() {
    let mut runtime = SubtitlePrefsRuntime::load(None).unwrap();
    let mut state = PlayerState::default();
    state.current = Some(MediaLocator::File(PathBuf::from("movie.mkv")));
    state.subtitle.visible = true;
    state.subtitle.external_path = Some(PathBuf::from("Z:/missing/external.ass"));

    runtime.remember_from_state(&state);

    let error =
        runtime.restore_plan_for(&MediaLocator::File(PathBuf::from("movie.mkv"))).unwrap_err();

    assert_eq!(
        error,
        SubtitleRestoreError::MissingExternalSubtitle(PathBuf::from("Z:/missing/external.ass"))
    );
}
