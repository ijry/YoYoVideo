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

pub const MIN_VIDEO_ADJUSTMENT: i16 = -100;
pub const MAX_VIDEO_ADJUSTMENT: i16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoAdjustmentKind {
    Brightness,
    Contrast,
    Saturation,
    Gamma,
    Hue,
}

impl VideoAdjustmentKind {
    pub const ALL: [Self; 5] =
        [Self::Brightness, Self::Contrast, Self::Saturation, Self::Gamma, Self::Hue];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VideoAdjustments {
    pub brightness: i16,
    pub contrast: i16,
    pub saturation: i16,
    pub gamma: i16,
    pub hue: i16,
}

impl VideoAdjustments {
    pub fn get(&self, kind: VideoAdjustmentKind) -> i16 {
        match kind {
            VideoAdjustmentKind::Brightness => self.brightness,
            VideoAdjustmentKind::Contrast => self.contrast,
            VideoAdjustmentKind::Saturation => self.saturation,
            VideoAdjustmentKind::Gamma => self.gamma,
            VideoAdjustmentKind::Hue => self.hue,
        }
    }

    pub fn set_clamped(&mut self, kind: VideoAdjustmentKind, value: i16) -> i16 {
        let value = value.clamp(MIN_VIDEO_ADJUSTMENT, MAX_VIDEO_ADJUSTMENT);
        match kind {
            VideoAdjustmentKind::Brightness => self.brightness = value,
            VideoAdjustmentKind::Contrast => self.contrast = value,
            VideoAdjustmentKind::Saturation => self.saturation = value,
            VideoAdjustmentKind::Gamma => self.gamma = value,
            VideoAdjustmentKind::Hue => self.hue = value,
        }
        value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFilterPreset {
    None,
    Sharpen,
    LightDenoise,
    Grayscale,
    Invert,
}

impl VideoFilterPreset {
    pub const ALL: [Self; 5] =
        [Self::None, Self::Sharpen, Self::LightDenoise, Self::Grayscale, Self::Invert];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameStepDirection {
    Previous,
    Next,
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
pub struct MediaChapter {
    pub title: Option<String>,
    pub time_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaMarker {
    pub id: String,
    pub title: String,
    pub time_seconds: f64,
    pub created_at: String,
}

pub const MARKER_DEDUPE_TOLERANCE_SECONDS: f64 = 0.75;

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
    pub muted: bool,
    pub speed: f32,
    pub audio_channel: AudioChannelMode,
    pub rotation: Rotation,
    pub zoom_step: i8,
    #[serde(default)]
    pub video_pan_x: f64,
    #[serde(default)]
    pub video_pan_y: f64,
    pub loop_state: LoopState,
    pub fullscreen: bool,
    pub status_message: Option<String>,
    pub last_error: Option<String>,
    pub audio_tracks: Vec<MediaTrack>,
    pub subtitle_tracks: Vec<MediaTrack>,
    pub video_tracks: Vec<MediaTrack>,
    pub subtitle: SubtitlePlaybackState,
    pub subtitle_preferences_restored: bool,
    pub video_adjustments: VideoAdjustments,
    pub video_filter_preset: VideoFilterPreset,
    pub chapters: Vec<MediaChapter>,
    pub markers: Vec<MediaMarker>,
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
            muted: false,
            speed: 1.0,
            audio_channel: AudioChannelMode::Stereo,
            rotation: Rotation::Deg0,
            zoom_step: 0,
            video_pan_x: 0.0,
            video_pan_y: 0.0,
            loop_state: LoopState::default(),
            fullscreen: false,
            status_message: None,
            last_error: None,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            video_tracks: Vec::new(),
            subtitle: SubtitlePlaybackState::default(),
            subtitle_preferences_restored: false,
            video_adjustments: VideoAdjustments::default(),
            video_filter_preset: VideoFilterPreset::None,
            chapters: Vec::new(),
            markers: Vec::new(),
        }
    }
}
