use yoyo_core::{BackendEvent, MediaChapter, MediaTrack};

#[derive(Debug, Clone, PartialEq)]
pub enum MpvEvent {
    Pause(bool),
    Position(f64),
    Duration(Option<f64>),
    /// Raw `dwidth` / `dheight` from mpv. Non-positive means "not decoded yet".
    VideoWidth(i64),
    VideoHeight(i64),
    Speed(f32),
    Volume(u8),
    Muted(bool),
    Rotation(i64),
    Tracks { audio: Vec<MediaTrack>, subtitles: Vec<MediaTrack>, video: Vec<MediaTrack> },
    Chapters(Vec<MediaChapter>),
    SubtitleVisible(bool),
    SubtitleDelay(f64),
    SubtitleScale(f32),
    SubtitlePosition(u8),
    Warning(String),
    Error(String),
    EndFile,
}

/// mpv reports 0 for `dwidth`/`dheight` before anything is decoded, which means
/// "unknown" rather than a zero-sized picture.
fn positive_dimension(value: i64) -> Option<u32> {
    (value > 0).then(|| value.min(u32::MAX as i64) as u32)
}

pub fn map_event(event: MpvEvent) -> Option<BackendEvent> {
    match event {
        MpvEvent::Pause(value) => Some(BackendEvent::PauseChanged(value)),
        MpvEvent::Position(value) => Some(BackendEvent::PositionChanged(value)),
        MpvEvent::Duration(value) => Some(BackendEvent::DurationChanged(value)),
        MpvEvent::VideoWidth(value) => {
            Some(BackendEvent::VideoWidthChanged(positive_dimension(value)))
        }
        MpvEvent::VideoHeight(value) => {
            Some(BackendEvent::VideoHeightChanged(positive_dimension(value)))
        }
        MpvEvent::Speed(value) => Some(BackendEvent::SpeedChanged(value)),
        MpvEvent::Volume(value) => Some(BackendEvent::VolumeChanged(value)),
        MpvEvent::Muted(value) => Some(BackendEvent::MutedChanged(value)),
        MpvEvent::Rotation(0) => Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg0)),
        MpvEvent::Rotation(90) => Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg90)),
        MpvEvent::Rotation(180) => Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg180)),
        MpvEvent::Rotation(270) => Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg270)),
        MpvEvent::Tracks { audio, subtitles, video } => {
            Some(BackendEvent::TracksChanged { audio, subtitles, video })
        }
        MpvEvent::Chapters(chapters) => Some(BackendEvent::ChaptersChanged(chapters)),
        MpvEvent::SubtitleVisible(value) => Some(BackendEvent::SubtitleVisibilityChanged(value)),
        MpvEvent::SubtitleDelay(value) => Some(BackendEvent::SubtitleDelayChanged(value)),
        MpvEvent::SubtitleScale(value) => Some(BackendEvent::SubtitleScaleChanged(value)),
        MpvEvent::SubtitlePosition(value) => {
            Some(BackendEvent::SubtitleVerticalPositionChanged(value))
        }
        MpvEvent::Rotation(other) => {
            Some(BackendEvent::Warning(format!("unsupported rotation reported by mpv: {other}")))
        }
        MpvEvent::Warning(message) => Some(BackendEvent::Warning(message)),
        MpvEvent::Error(message) => Some(BackendEvent::Error(message)),
        MpvEvent::EndFile => Some(BackendEvent::EndOfFile),
    }
}
