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
        }
    }
}
