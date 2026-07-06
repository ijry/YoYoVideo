use yoyo_core::MediaChapter;

#[cfg_attr(not(feature = "mpv-runtime"), allow(dead_code))]
pub(crate) fn normalize_chapters(chapters: Vec<MediaChapter>) -> Vec<MediaChapter> {
    let mut chapters = chapters
        .into_iter()
        .filter(|chapter| chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0)
        .collect::<Vec<_>>();
    chapters.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
    for (index, chapter) in chapters.iter_mut().enumerate() {
        if chapter.title.as_deref().unwrap_or("").trim().is_empty() {
            chapter.title = Some(format!("Chapter {}", index + 1));
        }
    }
    chapters
}

#[cfg(feature = "mpv-runtime")]
pub(crate) fn decode_chapter_list_property(
    property: &libmpv_sys::mpv_event_property,
) -> Option<crate::MpvEvent> {
    if property.data.is_null() {
        return None;
    }

    let node = unsafe { &*(property.data as *const libmpv_sys::mpv_node) };
    Some(crate::MpvEvent::Chapters(normalize_chapters(decode_chapters(node)?)))
}

#[cfg(feature = "mpv-runtime")]
fn decode_chapters(node: &libmpv_sys::mpv_node) -> Option<Vec<MediaChapter>> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_NODE_ARRAY {
        return None;
    }

    let list = unsafe { node.u.list.as_ref()? };
    let mut chapters = Vec::new();

    for index in 0..list.num {
        let entry = unsafe { list.values.add(index as usize).as_ref()? };
        if let Some(chapter) = decode_chapter(entry) {
            chapters.push(chapter);
        }
    }

    Some(chapters)
}

#[cfg(feature = "mpv-runtime")]
fn decode_chapter(node: &libmpv_sys::mpv_node) -> Option<MediaChapter> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_NODE_MAP {
        return None;
    }

    let map = unsafe { node.u.list.as_ref()? };
    let mut title = None;
    let mut time_seconds = None;

    for index in 0..map.num {
        let key_ptr = unsafe { map.keys.add(index as usize).as_ref()? };
        let key = unsafe { std::ffi::CStr::from_ptr(*key_ptr) }.to_string_lossy();
        let value = unsafe { map.values.add(index as usize).as_ref()? };

        match key.as_ref() {
            "title" => title = decode_string(value),
            "time" => time_seconds = decode_f64(value),
            _ => {}
        }
    }

    Some(MediaChapter { title, time_seconds: time_seconds? })
}

#[cfg(feature = "mpv-runtime")]
fn decode_string(node: &libmpv_sys::mpv_node) -> Option<String> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_STRING {
        return None;
    }
    if unsafe { node.u.string }.is_null() {
        return None;
    }

    Some(unsafe { std::ffi::CStr::from_ptr(node.u.string) }.to_string_lossy().into_owned())
}

#[cfg(feature = "mpv-runtime")]
fn decode_f64(node: &libmpv_sys::mpv_node) -> Option<f64> {
    match node.format {
        libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE => Some(unsafe { node.u.double_ }),
        libmpv_sys::mpv_format_MPV_FORMAT_INT64 => Some(unsafe { node.u.int64 as f64 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_chapters;
    use yoyo_core::MediaChapter;

    #[test]
    fn normalize_chapters_sorts_skips_negative_and_generates_titles() {
        let chapters = normalize_chapters(vec![
            MediaChapter { title: None, time_seconds: 20.0 },
            MediaChapter { title: Some("Bad".into()), time_seconds: -1.0 },
            MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 },
        ]);

        assert_eq!(
            chapters,
            vec![
                MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 },
                MediaChapter { title: Some("Chapter 2".into()), time_seconds: 20.0 },
            ]
        );
    }
}
