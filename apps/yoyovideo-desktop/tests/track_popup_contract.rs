use yoyo_core::{MediaTrack, MediaTrackKind, PlayerState};
use yoyovideo_desktop::{
    build_subtitle_track_rows, format_subtitle_delay_label, format_track_label,
};

fn track(
    id: i64,
    kind: MediaTrackKind,
    title: Option<&str>,
    language: Option<&str>,
    selected: bool,
) -> MediaTrack {
    MediaTrack {
        id,
        kind,
        title: title.map(str::to_string),
        language: language.map(str::to_string),
        codec: None,
        source_path: None,
        external: false,
        selected,
    }
}

#[test]
fn subtitle_rows_include_off_and_current_track_selection() {
    let mut state = PlayerState::default();
    state.subtitle.visible = true;
    state.subtitle_tracks = vec![
        track(3, MediaTrackKind::Subtitle, Some("English"), Some("eng"), true),
        track(4, MediaTrackKind::Subtitle, Some("Commentary"), None, false),
    ];

    let rows = build_subtitle_track_rows(&state);

    assert_eq!(rows[0].track_id, None);
    assert!(!rows[0].is_selected);
    assert_eq!(rows[1].track_id, Some(3));
    assert!(rows[1].is_selected);
}

#[test]
fn track_label_prefers_title_then_language_then_numeric_id() {
    assert_eq!(
        format_track_label(&track(8, MediaTrackKind::Audio, Some("Japanese"), Some("jpn"), true)),
        "Japanese (jpn)"
    );
    assert_eq!(
        format_track_label(&track(5, MediaTrackKind::Subtitle, None, Some("eng"), false)),
        "eng [#5]"
    );
    assert_eq!(format_subtitle_delay_label(-1.25), "-1.25s");
}
