use std::path::PathBuf;

use yoyo_core::{MediaTrack, MediaTrackKind};

#[cfg_attr(not(feature = "mpv-runtime"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTrackEntry {
    pub id: i64,
    pub kind: MediaTrackKind,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub source_path: Option<PathBuf>,
    pub external: bool,
    pub selected: bool,
}

#[cfg_attr(not(feature = "mpv-runtime"), allow(dead_code))]
pub(crate) fn split_tracks(
    entries: &[RawTrackEntry],
) -> (Vec<MediaTrack>, Vec<MediaTrack>, Vec<MediaTrack>) {
    let mut audio = Vec::new();
    let mut subtitles = Vec::new();
    let mut video = Vec::new();

    for entry in entries {
        let track = MediaTrack {
            id: entry.id,
            kind: entry.kind,
            title: entry.title.clone(),
            language: entry.language.clone(),
            codec: entry.codec.clone(),
            source_path: entry.source_path.clone(),
            external: entry.external,
            selected: entry.selected,
        };

        match entry.kind {
            MediaTrackKind::Audio => audio.push(track),
            MediaTrackKind::Subtitle => subtitles.push(track),
            MediaTrackKind::Video => video.push(track),
        }
    }

    (audio, subtitles, video)
}

#[cfg(feature = "mpv-runtime")]
pub(crate) fn decode_track_list_property(
    property: &libmpv_sys::mpv_event_property,
) -> Option<crate::MpvEvent> {
    if property.data.is_null() {
        return None;
    }

    let node = unsafe { &*(property.data as *const libmpv_sys::mpv_node) };
    let entries = decode_raw_track_entries(node)?;
    let (audio, subtitles, video) = split_tracks(&entries);
    Some(crate::MpvEvent::Tracks { audio, subtitles, video })
}

#[cfg(feature = "mpv-runtime")]
fn decode_raw_track_entries(node: &libmpv_sys::mpv_node) -> Option<Vec<RawTrackEntry>> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_NODE_ARRAY {
        return None;
    }

    let list = unsafe { node.u.list.as_ref()? };
    let mut tracks = Vec::new();

    for index in 0..list.num {
        let entry = unsafe { list.values.add(index as usize).as_ref()? };
        if let Some(track) = decode_raw_track_entry(entry) {
            tracks.push(track);
        }
    }

    Some(tracks)
}

#[cfg(feature = "mpv-runtime")]
fn decode_raw_track_entry(node: &libmpv_sys::mpv_node) -> Option<RawTrackEntry> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_NODE_MAP {
        return None;
    }

    let map = unsafe { node.u.list.as_ref()? };
    let mut id = None;
    let mut kind = None;
    let mut title = None;
    let mut language = None;
    let mut codec = None;
    let mut source_path = None;
    let mut external = false;
    let mut selected = false;

    for index in 0..map.num {
        let key_ptr = unsafe { map.keys.add(index as usize).as_ref()? };
        let key = unsafe { std::ffi::CStr::from_ptr(*key_ptr) }.to_string_lossy();
        let value = unsafe { map.values.add(index as usize).as_ref()? };

        match key.as_ref() {
            "id" => id = decode_i64(value),
            "type" => kind = decode_kind(value),
            "title" => title = decode_string(value),
            "lang" => language = decode_string(value),
            "codec" => codec = decode_string(value),
            "external-filename" => source_path = decode_string(value).map(PathBuf::from),
            "external" => external = decode_bool(value).unwrap_or(false),
            "selected" => selected = decode_bool(value).unwrap_or(false),
            _ => {}
        }
    }

    Some(RawTrackEntry {
        id: id?,
        kind: kind?,
        title,
        language,
        codec,
        source_path: source_path.clone(),
        external: external || source_path.is_some(),
        selected,
    })
}

#[cfg(feature = "mpv-runtime")]
fn decode_kind(node: &libmpv_sys::mpv_node) -> Option<MediaTrackKind> {
    match decode_string(node)?.as_str() {
        "audio" => Some(MediaTrackKind::Audio),
        "sub" => Some(MediaTrackKind::Subtitle),
        "video" => Some(MediaTrackKind::Video),
        _ => None,
    }
}

#[cfg(feature = "mpv-runtime")]
fn decode_string(node: &libmpv_sys::mpv_node) -> Option<String> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_STRING {
        return None;
    }
    let value = unsafe { std::ffi::CStr::from_ptr(node.u.string) };
    Some(value.to_string_lossy().into_owned())
}

#[cfg(feature = "mpv-runtime")]
fn decode_i64(node: &libmpv_sys::mpv_node) -> Option<i64> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_INT64 {
        return None;
    }
    Some(unsafe { node.u.int64 })
}

#[cfg(feature = "mpv-runtime")]
fn decode_bool(node: &libmpv_sys::mpv_node) -> Option<bool> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_FLAG {
        return None;
    }
    Some(unsafe { node.u.flag != 0 })
}

#[cfg(test)]
mod tests {
    use super::{RawTrackEntry, split_tracks};
    use yoyo_core::MediaTrackKind;

    #[test]
    fn split_tracks_groups_kinds_and_preserves_external_path() {
        let (audio, subtitles, video) = split_tracks(&[
            RawTrackEntry {
                id: 1,
                kind: MediaTrackKind::Video,
                title: Some("Main".into()),
                language: None,
                codec: Some("h264".into()),
                source_path: None,
                external: false,
                selected: true,
            },
            RawTrackEntry {
                id: 2,
                kind: MediaTrackKind::Audio,
                title: Some("Japanese".into()),
                language: Some("jpn".into()),
                codec: Some("aac".into()),
                source_path: None,
                external: false,
                selected: true,
            },
            RawTrackEntry {
                id: 3,
                kind: MediaTrackKind::Subtitle,
                title: Some("external.ass".into()),
                language: Some("eng".into()),
                codec: Some("ass".into()),
                source_path: Some("D:/subs/external.ass".into()),
                external: true,
                selected: true,
            },
        ]);

        assert_eq!(audio.len(), 1);
        assert_eq!(subtitles.len(), 1);
        assert_eq!(video.len(), 1);
        assert_eq!(subtitles[0].source_path.as_deref(), Some(std::path::Path::new("D:/subs/external.ass")));
        assert!(subtitles[0].external);
        assert!(subtitles[0].selected);
    }
}
