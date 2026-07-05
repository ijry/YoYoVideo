use yoyo_core::{AudioChannelMode, LoopState, PlayerState, Rotation};
use yoyovideo_desktop::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_speed_label,
    format_time_label, format_transport_label, format_volume_label, format_zoom_label,
    progress_ratio,
};

#[test]
fn transport_label_shows_pause_when_playing() {
    let state = PlayerState { paused: false, ..PlayerState::default() };

    assert_eq!(format_transport_label(&state), "Pause");
}

#[test]
fn speed_label_renders_two_decimals() {
    let state = PlayerState { speed: 1.25, ..PlayerState::default() };

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

#[test]
fn presenter_formats_volume_rotation_audio_zoom_and_loop() {
    let mut state = PlayerState::default();
    state.volume_percent = 73;
    state.rotation = Rotation::Deg90;
    state.audio_channel = AudioChannelMode::MonoLeft;
    state.zoom_step = 2;
    state.loop_state = LoopState { point_a: Some(12.4), point_b: Some(45.9) };

    assert_eq!(format_volume_label(&state), "Vol 73%");
    assert_eq!(format_rotation_label(&state), "90 deg");
    assert_eq!(format_audio_channel_label(&state), "Mono L");
    assert_eq!(format_zoom_label(&state), "Zoom +2");
    assert_eq!(format_loop_label(&state), "A 00:12 / B 00:45");
}

#[test]
fn progress_ratio_is_zero_without_duration_and_clamped_with_duration() {
    let mut state = PlayerState::default();
    assert_eq!(progress_ratio(&state), 0.0);

    state.position_seconds = 25.0;
    state.duration_seconds = Some(100.0);
    assert_eq!(progress_ratio(&state), 0.25);

    state.position_seconds = 150.0;
    assert_eq!(progress_ratio(&state), 1.0);
}
