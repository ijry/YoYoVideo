use std::path::PathBuf;

use crate::{AudioChannelMode, MediaLocator, MediaTrack, Rotation};

#[derive(Debug, Clone, PartialEq)]
pub enum BackendCommand {
    SetPaused(bool),
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetSpeed(f32),
    SetVolume(u8),
    SetAudioChannel(AudioChannelMode),
    SetRotation(Rotation),
    AdjustZoom(i8),
    SetABLoopPointA(f64),
    SetABLoopPointB(f64),
    ClearABLoop,
    SelectAudioTrack(i64),
    SelectSubtitleTrack(i64),
    SelectVideoTrack(i64),
    SetSubtitleVisible(bool),
    LoadExternalSubtitle(PathBuf),
    SetSubtitleDelay(f64),
    SetSubtitleScale(f32),
    SetSubtitleVerticalPosition(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    PauseChanged(bool),
    PositionChanged(f64),
    DurationChanged(Option<f64>),
    SpeedChanged(f32),
    VolumeChanged(u8),
    AudioChannelChanged(AudioChannelMode),
    RotationChanged(Rotation),
    TracksChanged {
        audio: Vec<MediaTrack>,
        subtitles: Vec<MediaTrack>,
        video: Vec<MediaTrack>,
    },
    SubtitleVisibilityChanged(bool),
    SubtitleDelayChanged(f64),
    SubtitleScaleChanged(f32),
    SubtitleVerticalPositionChanged(u8),
    Warning(String),
    Error(String),
    EndOfFile,
}

pub trait PlayerBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String>;
    fn send(&mut self, command: BackendCommand) -> Result<(), String>;
    fn drain_events(&mut self) -> Vec<BackendEvent>;
}
