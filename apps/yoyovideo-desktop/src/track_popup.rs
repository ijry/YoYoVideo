use yoyo_core::{MediaTrack, PlayerState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPopupRow {
    pub track_id: Option<i64>,
    pub label: String,
    pub is_selected: bool,
}

pub fn format_track_label(track: &MediaTrack) -> String {
    match (&track.title, &track.language) {
        (Some(title), Some(language)) => format!("{title} ({language})"),
        (Some(title), None) => title.clone(),
        (None, Some(language)) => format!("{language} [#{}]", track.id),
        (None, None) => format!("Track #{}", track.id),
    }
}

pub fn build_audio_track_rows(state: &PlayerState) -> Vec<TrackPopupRow> {
    state
        .audio_tracks
        .iter()
        .map(|track| TrackPopupRow {
            track_id: Some(track.id),
            label: format_track_label(track),
            is_selected: track.selected,
        })
        .collect()
}

pub fn build_subtitle_track_rows(state: &PlayerState) -> Vec<TrackPopupRow> {
    let mut rows = vec![TrackPopupRow {
        track_id: None,
        label: "Off".into(),
        is_selected: !state.subtitle.visible,
    }];
    rows.extend(state.subtitle_tracks.iter().map(|track| TrackPopupRow {
        track_id: Some(track.id),
        label: format_track_label(track),
        is_selected: state.subtitle.visible && track.selected,
    }));
    rows
}

pub fn build_video_track_rows(state: &PlayerState) -> Vec<TrackPopupRow> {
    state
        .video_tracks
        .iter()
        .map(|track| TrackPopupRow {
            track_id: Some(track.id),
            label: format_track_label(track),
            is_selected: track.selected,
        })
        .collect()
}

pub fn format_subtitle_delay_label(delay_seconds: f64) -> String {
    format!("{delay_seconds:+.2}s")
}

pub fn format_subtitle_scale_label(scale: f32) -> String {
    format!("{:.0}%", scale * 100.0)
}
