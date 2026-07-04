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
