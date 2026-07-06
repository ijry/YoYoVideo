use std::path::PathBuf;

use crate::{
    AudioChannelMode, FrameStepDirection, MediaChapter, MediaLocator, MediaTrack, Rotation,
    VideoAdjustmentKind, VideoFilterPreset,
};

#[derive(Debug, Clone, PartialEq)]
pub enum BackendCommand {
    SetPaused(bool),
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetSpeed(f32),
    SetVolume(u8),
    SetMuted(bool),
    SetAudioChannel(AudioChannelMode),
    SetRotation(Rotation),
    AdjustZoom(i8),
    SetVideoPan { x: f64, y: f64 },
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
    TakeScreenshot(PathBuf),
    StepFrame(FrameStepDirection),
    SetVideoAdjustment(VideoAdjustmentKind, i16),
    ResetVideoAdjustments,
    SetVideoFilterPreset(VideoFilterPreset),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    PauseChanged(bool),
    PositionChanged(f64),
    DurationChanged(Option<f64>),
    SpeedChanged(f32),
    VolumeChanged(u8),
    MutedChanged(bool),
    AudioChannelChanged(AudioChannelMode),
    RotationChanged(Rotation),
    TracksChanged { audio: Vec<MediaTrack>, subtitles: Vec<MediaTrack>, video: Vec<MediaTrack> },
    ChaptersChanged(Vec<MediaChapter>),
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
