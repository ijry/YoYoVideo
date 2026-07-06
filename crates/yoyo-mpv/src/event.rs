use yoyo_core::{BackendEvent, MediaChapter, MediaTrack};

#[derive(Debug, Clone, PartialEq)]
pub enum MpvEvent {
    Pause(bool),
    Position(f64),
    Duration(Option<f64>),
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

pub fn map_event(event: MpvEvent) -> Option<BackendEvent> {
    match event {
        MpvEvent::Pause(value) => Some(BackendEvent::PauseChanged(value)),
        MpvEvent::Position(value) => Some(BackendEvent::PositionChanged(value)),
        MpvEvent::Duration(value) => Some(BackendEvent::DurationChanged(value)),
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
