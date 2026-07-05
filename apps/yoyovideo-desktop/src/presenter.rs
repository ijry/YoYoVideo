use yoyo_core::{AudioChannelMode, PlayerState, Rotation};

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn format_transport_label(state: &PlayerState) -> String {
    if state.paused { "Play".into() } else { "Pause".into() }
}

pub fn format_speed_label(state: &PlayerState) -> String {
    format!("{:.2}x", state.speed)
}

pub fn format_time_label(state: &PlayerState) -> String {
    match state.duration_seconds {
        Some(duration) => format!("{} / {}", fmt_clock(state.position_seconds), fmt_clock(duration)),
        None => format!("{} / --:--", fmt_clock(state.position_seconds)),
    }
}

pub fn progress_ratio(state: &PlayerState) -> f32 {
    match state.duration_seconds {
        Some(duration) if duration > 0.0 => (state.position_seconds / duration).clamp(0.0, 1.0) as f32,
        _ => 0.0,
    }
}

pub fn format_volume_label(state: &PlayerState) -> String {
    format!("Vol {}%", state.volume_percent)
}

pub fn format_rotation_label(state: &PlayerState) -> String {
    match state.rotation {
        Rotation::Deg0 => "0 deg".into(),
        Rotation::Deg90 => "90 deg".into(),
        Rotation::Deg180 => "180 deg".into(),
        Rotation::Deg270 => "270 deg".into(),
    }
}

pub fn format_audio_channel_label(state: &PlayerState) -> String {
    match state.audio_channel {
        AudioChannelMode::Stereo => "Stereo".into(),
        AudioChannelMode::MonoLeft => "Mono L".into(),
        AudioChannelMode::MonoRight => "Mono R".into(),
    }
}

pub fn format_zoom_label(state: &PlayerState) -> String {
    match state.zoom_step.cmp(&0) {
        std::cmp::Ordering::Greater => format!("Zoom +{}", state.zoom_step),
        std::cmp::Ordering::Less => format!("Zoom {}", state.zoom_step),
        std::cmp::Ordering::Equal => "Zoom 0".into(),
    }
}

pub fn format_loop_label(state: &PlayerState) -> String {
    match (state.loop_state.point_a, state.loop_state.point_b) {
        (Some(a), Some(b)) => format!("A {} / B {}", fmt_clock(a), fmt_clock(b)),
        (Some(a), None) => format!("A {} / B --:--", fmt_clock(a)),
        (None, Some(b)) => format!("A --:-- / B {}", fmt_clock(b)),
        (None, None) => "A --:-- / B --:--".into(),
    }
}
