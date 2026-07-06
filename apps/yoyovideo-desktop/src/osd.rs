#[derive(Debug, Clone, PartialEq)]
pub enum OsdKind {
    Muted(bool),
    JumpedTo(f64),
    SeekedTo(f64),
    Volume(u8),
    Speed(f32),
    MarkerAdded,
    MarkerRemoved,
    Chapter(String),
    Screenshot(String),
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OsdState {
    pub visible: bool,
    pub message: String,
    pub generation: u64,
}

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

pub fn format_osd_message(kind: OsdKind) -> String {
    match kind {
        OsdKind::Muted(true) => "Muted".into(),
        OsdKind::Muted(false) => "Sound On".into(),
        OsdKind::JumpedTo(seconds) => format!("Jumped to {}", fmt_clock(seconds)),
        OsdKind::SeekedTo(seconds) => format!("Seek {}", fmt_clock(seconds)),
        OsdKind::Volume(volume) => format!("Volume {volume}%"),
        OsdKind::Speed(speed) => format!("{speed:.2}x"),
        OsdKind::MarkerAdded => "Marker added".into(),
        OsdKind::MarkerRemoved => "Marker removed".into(),
        OsdKind::Chapter(title) => title,
        OsdKind::Screenshot(path) => format!("Screenshot saved: {path}"),
        OsdKind::Message(message) => message,
    }
}
