use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::MediaLocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioChannelMode {
    Stereo,
    MonoLeft,
    MonoRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct LoopState {
    pub point_a: Option<f64>,
    pub point_b: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaTrackKind {
    Audio,
    Subtitle,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaTrack {
    pub id: i64,
    pub kind: MediaTrackKind,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub source_path: Option<PathBuf>,
    pub external: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePlaybackState {
    pub visible: bool,
    pub delay_seconds: f64,
    pub scale: f32,
    pub vertical_position_percent: u8,
    pub external_path: Option<PathBuf>,
}

impl Default for SubtitlePlaybackState {
    fn default() -> Self {
        Self {
            visible: true,
            delay_seconds: 0.0,
            scale: 1.0,
            vertical_position_percent: 100,
            external_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub current: Option<MediaLocator>,
    pub paused: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub volume_percent: u8,
    pub speed: f32,
    pub audio_channel: AudioChannelMode,
    pub rotation: Rotation,
    pub zoom_step: i8,
    pub loop_state: LoopState,
    pub fullscreen: bool,
    pub status_message: Option<String>,
    pub last_error: Option<String>,
    pub audio_tracks: Vec<MediaTrack>,
    pub subtitle_tracks: Vec<MediaTrack>,
    pub video_tracks: Vec<MediaTrack>,
    pub subtitle: SubtitlePlaybackState,
    pub subtitle_preferences_restored: bool,
}

impl PlayerState {
    pub fn selected_audio_track_id(&self) -> Option<i64> {
        self.audio_tracks.iter().find(|track| track.selected).map(|track| track.id)
    }

    pub fn selected_subtitle_track_id(&self) -> Option<i64> {
        self.subtitle_tracks.iter().find(|track| track.selected).map(|track| track.id)
    }

    pub fn selected_video_track_id(&self) -> Option<i64> {
        self.video_tracks.iter().find(|track| track.selected).map(|track| track.id)
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current: None,
            paused: true,
            position_seconds: 0.0,
            duration_seconds: None,
            volume_percent: 100,
            speed: 1.0,
            audio_channel: AudioChannelMode::Stereo,
            rotation: Rotation::Deg0,
            zoom_step: 0,
            loop_state: LoopState::default(),
            fullscreen: false,
            status_message: None,
            last_error: None,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            video_tracks: Vec::new(),
            subtitle: SubtitlePlaybackState::default(),
            subtitle_preferences_restored: false,
        }
    }
}
