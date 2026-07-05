use std::path::PathBuf;

use yoyo_core::{AudioChannelMode, BackendCommand, MediaLocator, Rotation};
use yoyo_mpv::{MpvAction, translate_command, translate_open};

#[test]
fn open_file_translates_to_loadfile_replace() {
    let actions = translate_open(&MediaLocator::File("movie.mp4".into()));
    assert_eq!(
        actions,
        vec![MpvAction::Command(vec!["loadfile".into(), "movie.mp4".into(), "replace".into(),])]
    );
}

#[test]
fn set_speed_translates_to_speed_property() {
    let actions = translate_command(&BackendCommand::SetSpeed(1.25));
    assert_eq!(actions, vec![MpvAction::SetDouble { name: "speed".into(), value: 1.25 }]);
}

#[test]
fn mono_left_uses_front_left_layout() {
    let actions = translate_command(&BackendCommand::SetAudioChannel(AudioChannelMode::MonoLeft));
    assert_eq!(
        actions,
        vec![MpvAction::SetString { name: "audio-channels".into(), value: "fl".into() }]
    );
}

#[test]
fn rotation_translates_to_video_rotate() {
    let actions = translate_command(&BackendCommand::SetRotation(Rotation::Deg90));
    assert_eq!(actions, vec![MpvAction::SetInt { name: "video-rotate".into(), value: 90 }]);
}

#[test]
fn track_selection_and_subtitle_controls_translate_to_expected_properties() {
    assert_eq!(
        translate_command(&BackendCommand::SelectAudioTrack(2)),
        vec![MpvAction::SetInt { name: "aid".into(), value: 2 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetSubtitleVisible(false)),
        vec![MpvAction::SetFlag { name: "sub-visibility".into(), value: false }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetSubtitleVerticalPosition(88)),
        vec![MpvAction::SetInt { name: "sub-pos".into(), value: 88 }]
    );
}

#[test]
fn external_subtitle_loading_uses_sub_add_select() {
    assert_eq!(
        translate_command(&BackendCommand::LoadExternalSubtitle(PathBuf::from("movie.ass"))),
        vec![MpvAction::Command(vec!["sub-add".into(), "movie.ass".into(), "select".into(),])]
    );
}
