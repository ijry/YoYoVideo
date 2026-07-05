use yoyo_core::{AudioChannelMode, BackendCommand, MediaLocator, Rotation};

#[derive(Debug, Clone, PartialEq)]
pub enum MpvAction {
    Command(Vec<String>),
    SetString { name: String, value: String },
    SetInt { name: String, value: i64 },
    SetDouble { name: String, value: f64 },
    SetFlag { name: String, value: bool },
}

pub fn translate_open(locator: &MediaLocator) -> Vec<MpvAction> {
    vec![MpvAction::Command(vec!["loadfile".into(), locator.as_label(), "replace".into()])]
}

pub fn translate_command(command: &BackendCommand) -> Vec<MpvAction> {
    match command {
        BackendCommand::SetPaused(paused) => {
            vec![MpvAction::SetFlag { name: "pause".into(), value: *paused }]
        }
        BackendCommand::SeekRelative(seconds) => {
            vec![MpvAction::Command(vec!["seek".into(), seconds.to_string(), "relative".into()])]
        }
        BackendCommand::SeekAbsolute(seconds) => vec![MpvAction::Command(vec![
            "seek".into(),
            seconds.to_string(),
            "absolute+exact".into(),
        ])],
        BackendCommand::SetSpeed(speed) => {
            vec![MpvAction::SetDouble { name: "speed".into(), value: *speed as f64 }]
        }
        BackendCommand::SetVolume(volume) => {
            vec![MpvAction::SetDouble { name: "volume".into(), value: *volume as f64 }]
        }
        BackendCommand::SetAudioChannel(mode) => {
            let value = match mode {
                AudioChannelMode::Stereo => "stereo",
                AudioChannelMode::MonoLeft => "fl",
                AudioChannelMode::MonoRight => "fr",
            };
            vec![MpvAction::SetString { name: "audio-channels".into(), value: value.into() }]
        }
        BackendCommand::SetRotation(rotation) => {
            let degrees = match rotation {
                Rotation::Deg0 => 0,
                Rotation::Deg90 => 90,
                Rotation::Deg180 => 180,
                Rotation::Deg270 => 270,
            };
            vec![MpvAction::SetInt { name: "video-rotate".into(), value: degrees }]
        }
        BackendCommand::AdjustZoom(delta) => vec![MpvAction::Command(vec![
            "add".into(),
            "video-zoom".into(),
            (*delta as f64 * 0.25).to_string(),
        ])],
        BackendCommand::SetABLoopPointA(seconds) => {
            vec![MpvAction::SetDouble { name: "ab-loop-a".into(), value: *seconds }]
        }
        BackendCommand::SetABLoopPointB(seconds) => {
            vec![MpvAction::SetDouble { name: "ab-loop-b".into(), value: *seconds }]
        }
        BackendCommand::ClearABLoop => vec![
            MpvAction::SetString { name: "ab-loop-a".into(), value: "no".into() },
            MpvAction::SetString { name: "ab-loop-b".into(), value: "no".into() },
        ],
        BackendCommand::SelectAudioTrack(id) => {
            vec![MpvAction::SetInt { name: "aid".into(), value: *id }]
        }
        BackendCommand::SelectSubtitleTrack(id) => {
            vec![MpvAction::SetInt { name: "sid".into(), value: *id }]
        }
        BackendCommand::SelectVideoTrack(id) => {
            vec![MpvAction::SetInt { name: "vid".into(), value: *id }]
        }
        BackendCommand::SetSubtitleVisible(visible) => {
            vec![MpvAction::SetFlag { name: "sub-visibility".into(), value: *visible }]
        }
        BackendCommand::LoadExternalSubtitle(path) => vec![MpvAction::Command(vec![
            "sub-add".into(),
            path.display().to_string(),
            "select".into(),
        ])],
        BackendCommand::SetSubtitleDelay(delay) => {
            vec![MpvAction::SetDouble { name: "sub-delay".into(), value: *delay }]
        }
        BackendCommand::SetSubtitleScale(scale) => {
            vec![MpvAction::SetDouble { name: "sub-scale".into(), value: f64::from(*scale) }]
        }
        BackendCommand::SetSubtitleVerticalPosition(position) => {
            vec![MpvAction::SetInt { name: "sub-pos".into(), value: i64::from(*position) }]
        }
    }
}
