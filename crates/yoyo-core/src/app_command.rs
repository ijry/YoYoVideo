use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum AppCommand {
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
    OpenUrl(String),
    TogglePause,
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetSpeed(f32),
    ResetSpeed,
    SetVolume(u8),
    AdjustVolume(i8),
    CycleAudioChannel,
    RotateClockwise,
    ZoomIn,
    ZoomOut,
    SetABLoopPointA,
    SetABLoopPointB,
    ClearABLoop,
    ToggleFullscreen,
    NextItem,
    PreviousItem,
}
