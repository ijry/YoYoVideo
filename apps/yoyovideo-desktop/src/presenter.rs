use yoyo_core::PlayerState;

pub fn format_transport_label(state: &PlayerState) -> String {
    if state.paused { "Play".into() } else { "Pause".into() }
}

pub fn format_speed_label(state: &PlayerState) -> String {
    format!("{:.2}x", state.speed)
}

pub fn format_time_label(state: &PlayerState) -> String {
    fn fmt(seconds: f64) -> String {
        let total = seconds.max(0.0) as u64;
        format!("{:02}:{:02}", total / 60, total % 60)
    }

    match state.duration_seconds {
        Some(duration) => format!("{} / {}", fmt(state.position_seconds), fmt(duration)),
        None => format!("{} / --:--", fmt(state.position_seconds)),
    }
}
