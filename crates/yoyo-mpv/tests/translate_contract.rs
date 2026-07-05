use std::path::PathBuf;

use yoyo_core::{
    AudioChannelMode, BackendCommand, FrameStepDirection, MediaLocator, Rotation,
    VideoAdjustmentKind, VideoFilterPreset,
};
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

#[test]
fn screenshot_translates_to_screenshot_to_file_command() {
    assert_eq!(
        translate_command(&BackendCommand::TakeScreenshot(PathBuf::from("shot.png"))),
        vec![MpvAction::Command(vec![
            "screenshot-to-file".into(),
            "shot.png".into(),
            "subtitles".into(),
        ])]
    );
}

#[test]
fn frame_step_translates_to_mpv_frame_commands() {
    assert_eq!(
        translate_command(&BackendCommand::StepFrame(FrameStepDirection::Next)),
        vec![MpvAction::Command(vec!["frame-step".into()])]
    );
    assert_eq!(
        translate_command(&BackendCommand::StepFrame(FrameStepDirection::Previous)),
        vec![MpvAction::Command(vec!["frame-back-step".into()])]
    );
}

#[test]
fn video_adjustments_translate_to_matching_mpv_properties() {
    assert_eq!(
        translate_command(
            &BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Brightness, 12,)
        ),
        vec![MpvAction::SetDouble { name: "brightness".into(), value: 12.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Contrast, -7,)),
        vec![MpvAction::SetDouble { name: "contrast".into(), value: -7.0 }]
    );
    assert_eq!(
        translate_command(
            &BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Saturation, 22,)
        ),
        vec![MpvAction::SetDouble { name: "saturation".into(), value: 22.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Gamma, 5)),
        vec![MpvAction::SetDouble { name: "gamma".into(), value: 5.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Hue, -9)),
        vec![MpvAction::SetDouble { name: "hue".into(), value: -9.0 }]
    );
}

#[test]
fn reset_video_adjustments_translates_to_neutral_properties() {
    assert_eq!(
        translate_command(&BackendCommand::ResetVideoAdjustments),
        vec![
            MpvAction::SetDouble { name: "brightness".into(), value: 0.0 },
            MpvAction::SetDouble { name: "contrast".into(), value: 0.0 },
            MpvAction::SetDouble { name: "saturation".into(), value: 0.0 },
            MpvAction::SetDouble { name: "gamma".into(), value: 0.0 },
            MpvAction::SetDouble { name: "hue".into(), value: 0.0 },
        ]
    );
}

#[test]
fn filter_presets_translate_to_yoyovideo_owned_vf_slot() {
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::None)),
        vec![MpvAction::Command(vec!["vf".into(), "remove".into(), "@yoyovideo-preset".into(),])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Sharpen)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[unsharp=5:5:0.6:3:3:0.3]".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::LightDenoise)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[hqdn3d=1.5:1.5:6:6]".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Grayscale)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[format=gray]".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Invert)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[negate]".into(),
        ])]
    );
}
