use yoyo_core::{PlayerState, Rotation};
use yoyovideo_desktop::{format_speed_label, format_time_label, format_transport_label};

#[test]
fn transport_label_shows_pause_when_playing() {
    let state = PlayerState {
        paused: false,
        ..PlayerState::default()
    };

    assert_eq!(format_transport_label(&state), "Pause");
}

#[test]
fn speed_label_renders_two_decimals() {
    let state = PlayerState {
        speed: 1.25,
        ..PlayerState::default()
    };

    assert_eq!(format_speed_label(&state), "1.25x");
}

#[test]
fn time_label_formats_minutes_and_seconds() {
    let state = PlayerState {
        position_seconds: 65.0,
        duration_seconds: Some(130.0),
        rotation: Rotation::Deg90,
        ..PlayerState::default()
    };

    assert_eq!(format_time_label(&state), "01:05 / 02:10");
}
