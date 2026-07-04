use crate::{AudioChannelMode, MediaLocator, Rotation};

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
    Warning(String),
    Error(String),
    EndOfFile,
}

pub trait PlayerBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String>;
    fn send(&mut self, command: BackendCommand) -> Result<(), String>;
    fn drain_events(&mut self) -> Vec<BackendEvent>;
}
