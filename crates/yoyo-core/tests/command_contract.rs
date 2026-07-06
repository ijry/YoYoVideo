use std::path::PathBuf;

use yoyo_core::{AppCommand, AudioChannelMode, LoopState, MediaLocator, PlayerState, Rotation};

#[test]
fn default_player_state_is_safe_for_empty_launch() {
    let state = PlayerState::default();

    assert!(state.current.is_none());
    assert!(state.paused);
    assert_eq!(state.speed, 1.0);
    assert_eq!(state.volume_percent, 100);
    assert_eq!(state.audio_channel, AudioChannelMode::Stereo);
    assert_eq!(state.rotation, Rotation::Deg0);
    assert_eq!(state.video_pan_x, 0.0);
    assert_eq!(state.video_pan_y, 0.0);
    assert_eq!(state.loop_state, LoopState::default());
}

#[test]
fn open_file_command_carries_target_path() {
    let path = PathBuf::from("demo.mp4");
    let command = AppCommand::OpenFile(path.clone());

    match command {
        AppCommand::OpenFile(actual) => assert_eq!(actual, path),
        other => panic!("expected OpenFile, got {other:?}"),
    }
}

#[test]
fn media_locator_label_round_trip() {
    let locator = MediaLocator::Url("https://example.com/live.m3u8".to_string());
    assert_eq!(locator.as_label(), "https://example.com/live.m3u8");
}
