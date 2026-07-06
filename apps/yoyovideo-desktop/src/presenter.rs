use crate::UiLanguage;
use yoyo_core::{AudioChannelMode, PlayerState, Rotation, VideoAdjustmentKind, VideoFilterPreset};

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn format_transport_label(state: &PlayerState) -> String {
    format_transport_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_transport_label_for_language(state: &PlayerState, language: UiLanguage) -> String {
    match (language, state.paused) {
        (UiLanguage::Chinese, true) => "播放".into(),
        (UiLanguage::Chinese, false) => "暂停".into(),
        (UiLanguage::English, true) => "Play".into(),
        (UiLanguage::English, false) => "Pause".into(),
    }
}

pub fn format_speed_label(state: &PlayerState) -> String {
    format!("{:.2}x", state.speed)
}

pub fn format_time_label(state: &PlayerState) -> String {
    match state.duration_seconds {
        Some(duration) => {
            format!("{} / {}", fmt_clock(state.position_seconds), fmt_clock(duration))
        }
        None => format!("{} / --:--", fmt_clock(state.position_seconds)),
    }
}

pub fn progress_ratio(state: &PlayerState) -> f32 {
    match state.duration_seconds {
        Some(duration) if duration > 0.0 => {
            (state.position_seconds / duration).clamp(0.0, 1.0) as f32
        }
        _ => 0.0,
    }
}

pub fn format_volume_label(state: &PlayerState) -> String {
    format_volume_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_volume_label_for_language(state: &PlayerState, language: UiLanguage) -> String {
    match language {
        UiLanguage::Chinese => format!("音量 {}%", state.volume_percent),
        UiLanguage::English => format!("Vol {}%", state.volume_percent),
    }
}

pub fn format_rotation_label(state: &PlayerState) -> String {
    format_rotation_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_rotation_label_for_language(state: &PlayerState, language: UiLanguage) -> String {
    match (language, state.rotation) {
        (UiLanguage::Chinese, Rotation::Deg0) => "0°".into(),
        (UiLanguage::Chinese, Rotation::Deg90) => "90°".into(),
        (UiLanguage::Chinese, Rotation::Deg180) => "180°".into(),
        (UiLanguage::Chinese, Rotation::Deg270) => "270°".into(),
        (UiLanguage::English, Rotation::Deg0) => "0 deg".into(),
        (UiLanguage::English, Rotation::Deg90) => "90 deg".into(),
        (UiLanguage::English, Rotation::Deg180) => "180 deg".into(),
        (UiLanguage::English, Rotation::Deg270) => "270 deg".into(),
    }
}

pub fn format_audio_channel_label(state: &PlayerState) -> String {
    format_audio_channel_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_audio_channel_label_for_language(
    state: &PlayerState,
    language: UiLanguage,
) -> String {
    match (language, state.audio_channel) {
        (UiLanguage::Chinese, AudioChannelMode::Stereo) => "立体声".into(),
        (UiLanguage::Chinese, AudioChannelMode::MonoLeft) => "左声道".into(),
        (UiLanguage::Chinese, AudioChannelMode::MonoRight) => "右声道".into(),
        (UiLanguage::English, AudioChannelMode::Stereo) => "Stereo".into(),
        (UiLanguage::English, AudioChannelMode::MonoLeft) => "Mono L".into(),
        (UiLanguage::English, AudioChannelMode::MonoRight) => "Mono R".into(),
    }
}

pub fn format_zoom_label(state: &PlayerState) -> String {
    format_zoom_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_zoom_label_for_language(state: &PlayerState, language: UiLanguage) -> String {
    match (language, state.zoom_step.cmp(&0)) {
        (UiLanguage::Chinese, std::cmp::Ordering::Greater) => {
            format!("缩放 +{}", state.zoom_step)
        }
        (UiLanguage::Chinese, std::cmp::Ordering::Less) => format!("缩放 {}", state.zoom_step),
        (UiLanguage::Chinese, std::cmp::Ordering::Equal) => "缩放 0".into(),
        (UiLanguage::English, std::cmp::Ordering::Greater) => {
            format!("Zoom +{}", state.zoom_step)
        }
        (UiLanguage::English, std::cmp::Ordering::Less) => format!("Zoom {}", state.zoom_step),
        (UiLanguage::English, std::cmp::Ordering::Equal) => "Zoom 0".into(),
    }
}

pub fn format_loop_label(state: &PlayerState) -> String {
    format_loop_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_loop_label_for_language(state: &PlayerState, _language: UiLanguage) -> String {
    match (state.loop_state.point_a, state.loop_state.point_b) {
        (Some(a), Some(b)) => format!("A {} / B {}", fmt_clock(a), fmt_clock(b)),
        (Some(a), None) => format!("A {} / B --:--", fmt_clock(a)),
        (None, Some(b)) => format!("A --:-- / B {}", fmt_clock(b)),
        (None, None) => "A --:-- / B --:--".into(),
    }
}

pub fn format_video_adjustment_label(kind: VideoAdjustmentKind, value: i16) -> String {
    format_video_adjustment_label_for_language(kind, value, UiLanguage::Chinese)
}

pub fn format_video_adjustment_label_for_language(
    kind: VideoAdjustmentKind,
    value: i16,
    language: UiLanguage,
) -> String {
    let name = match (language, kind) {
        (UiLanguage::Chinese, VideoAdjustmentKind::Brightness) => "亮度",
        (UiLanguage::Chinese, VideoAdjustmentKind::Contrast) => "对比度",
        (UiLanguage::Chinese, VideoAdjustmentKind::Saturation) => "饱和度",
        (UiLanguage::Chinese, VideoAdjustmentKind::Gamma) => "伽马",
        (UiLanguage::Chinese, VideoAdjustmentKind::Hue) => "色调",
        (UiLanguage::English, VideoAdjustmentKind::Brightness) => "Brightness",
        (UiLanguage::English, VideoAdjustmentKind::Contrast) => "Contrast",
        (UiLanguage::English, VideoAdjustmentKind::Saturation) => "Saturation",
        (UiLanguage::English, VideoAdjustmentKind::Gamma) => "Gamma",
        (UiLanguage::English, VideoAdjustmentKind::Hue) => "Hue",
    };
    format!("{name} {value:+}")
}

pub fn format_video_filter_preset_label(preset: VideoFilterPreset) -> &'static str {
    format_video_filter_preset_label_for_language(preset, UiLanguage::Chinese)
}

pub fn format_video_filter_preset_label_for_language(
    preset: VideoFilterPreset,
    language: UiLanguage,
) -> &'static str {
    match (language, preset) {
        (UiLanguage::Chinese, VideoFilterPreset::None) => "滤镜: 无",
        (UiLanguage::Chinese, VideoFilterPreset::Sharpen) => "滤镜: 锐化",
        (UiLanguage::Chinese, VideoFilterPreset::LightDenoise) => "滤镜: 轻降噪",
        (UiLanguage::Chinese, VideoFilterPreset::Grayscale) => "滤镜: 灰度",
        (UiLanguage::Chinese, VideoFilterPreset::Invert) => "滤镜: 反色",
        (UiLanguage::English, VideoFilterPreset::None) => "Filter: None",
        (UiLanguage::English, VideoFilterPreset::Sharpen) => "Filter: Sharpen",
        (UiLanguage::English, VideoFilterPreset::LightDenoise) => "Filter: Light Denoise",
        (UiLanguage::English, VideoFilterPreset::Grayscale) => "Filter: Grayscale",
        (UiLanguage::English, VideoFilterPreset::Invert) => "Filter: Invert",
    }
}
