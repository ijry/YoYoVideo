use crate::UiLanguage;

#[derive(Debug, Clone, PartialEq)]
pub enum OsdKind {
    Muted(bool),
    JumpedTo(f64),
    SeekedTo(f64),
    Volume(u8),
    Speed(f32),
    Stopped,
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
    format_osd_message_for_language(kind, UiLanguage::Chinese)
}

pub fn format_osd_message_for_language(kind: OsdKind, language: UiLanguage) -> String {
    match (language, kind) {
        (UiLanguage::Chinese, OsdKind::Muted(true)) => "已静音".into(),
        (UiLanguage::Chinese, OsdKind::Muted(false)) => "声音开启".into(),
        (UiLanguage::Chinese, OsdKind::JumpedTo(seconds)) => {
            format!("跳转到 {}", fmt_clock(seconds))
        }
        (UiLanguage::Chinese, OsdKind::SeekedTo(seconds)) => format!("定位 {}", fmt_clock(seconds)),
        (UiLanguage::Chinese, OsdKind::Volume(volume)) => format!("音量 {volume}%"),
        (UiLanguage::Chinese, OsdKind::Speed(speed)) => format!("{speed:.2}x"),
        (UiLanguage::Chinese, OsdKind::Stopped) => "已停止".into(),
        (UiLanguage::Chinese, OsdKind::MarkerAdded) => "已添加标记".into(),
        (UiLanguage::Chinese, OsdKind::MarkerRemoved) => "已移除标记".into(),
        (UiLanguage::Chinese, OsdKind::Chapter(title)) => title,
        (UiLanguage::Chinese, OsdKind::Screenshot(path)) => format!("截图已保存: {path}"),
        (UiLanguage::Chinese, OsdKind::Message(message)) => message,
        (UiLanguage::English, OsdKind::Muted(true)) => "Muted".into(),
        (UiLanguage::English, OsdKind::Muted(false)) => "Sound On".into(),
        (UiLanguage::English, OsdKind::JumpedTo(seconds)) => {
            format!("Jumped to {}", fmt_clock(seconds))
        }
        (UiLanguage::English, OsdKind::SeekedTo(seconds)) => {
            format!("Seek {}", fmt_clock(seconds))
        }
        (UiLanguage::English, OsdKind::Volume(volume)) => format!("Volume {volume}%"),
        (UiLanguage::English, OsdKind::Speed(speed)) => format!("{speed:.2}x"),
        (UiLanguage::English, OsdKind::Stopped) => "Stopped".into(),
        (UiLanguage::English, OsdKind::MarkerAdded) => "Marker added".into(),
        (UiLanguage::English, OsdKind::MarkerRemoved) => "Marker removed".into(),
        (UiLanguage::English, OsdKind::Chapter(title)) => title,
        (UiLanguage::English, OsdKind::Screenshot(path)) => format!("Screenshot saved: {path}"),
        (UiLanguage::English, OsdKind::Message(message)) => message,
    }
}
