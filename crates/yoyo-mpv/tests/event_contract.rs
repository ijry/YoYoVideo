use yoyo_core::BackendEvent;
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
