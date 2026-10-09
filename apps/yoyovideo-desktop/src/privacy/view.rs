use super::{PrivacyError, PrivacySnapshot};
use crate::{HistorySidebarRow, PlaylistSidebarRow, UiLanguage};
use yoyo_core::{
    HistoryStore, MediaLocator, PlaybackAccess, PlayerState, PlaylistSnapshot, privacy::MediaKey,
};

pub fn protected_label(language: UiLanguage) -> &'static str {
    match language {
        UiLanguage::Chinese => "受保护内容",
        UiLanguage::English => "Protected content",
    }
}
pub fn locator_restricted(access: &dyn PlaybackAccess, locator: &MediaLocator) -> bool {
    MediaKey::from_locator(locator).map_or(true, |key| access.restricted(&key))
}
/// Presentation only: callers must keep the original state for persistence.
pub fn redact_state(state: &PlayerState, blocked: bool, language: UiLanguage) -> PlayerState {
    let mut view = state.clone();
    if !blocked {
        return view;
    }
    let label = protected_label(language);
    view.current = state.current.as_ref().map(|_| MediaLocator::File(label.into()));
    view.paused = true;
    view.position_seconds = 0.0;
    view.duration_seconds = None;
    view.video_width = None;
    view.video_height = None;
    view.chapters.clear();
    view.markers.clear();
    view.audio_tracks.clear();
    view.subtitle_tracks.clear();
    view.video_tracks.clear();
    view.subtitle = Default::default();
    view.subtitle.visible = false;
    view.loop_state = Default::default();
    view.status_message = Some(label.into());
    view.last_error = None;
    view
}
pub fn history_rows(
    store: &HistoryStore,
    access: &dyn PlaybackAccess,
    language: UiLanguage,
) -> Vec<HistorySidebarRow> {
    crate::build_history_rows(store)
        .into_iter()
        .zip(store.items())
        .map(|(mut row, entry)| {
            if locator_restricted(access, &entry.locator) {
                row.title = protected_label(language).into();
                row.subtitle.clear();
            }
            row
        })
        .collect()
}
pub fn playlist_rows(
    playlist: &PlaylistSnapshot,
    access: &dyn PlaybackAccess,
    language: UiLanguage,
) -> Vec<PlaylistSidebarRow> {
    crate::build_playlist_rows(playlist)
        .into_iter()
        .zip(&playlist.entries)
        .map(|(mut row, entry)| {
            if locator_restricted(access, &entry.locator) {
                row.title = protected_label(language).into();
            }
            row
        })
        .collect()
}
pub fn parse_hhmm(value: &str) -> Result<u16, PrivacyError> {
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || ![bytes[0], bytes[1], bytes[3], bytes[4]].iter().all(u8::is_ascii_digit)
    {
        return Err(PrivacyError::InvalidSchedule);
    }
    let hours = u16::from(bytes[0] - b'0') * 10 + u16::from(bytes[1] - b'0');
    let minutes = u16::from(bytes[3] - b'0') * 10 + u16::from(bytes[4] - b'0');
    if hours >= 24 || minutes >= 60 {
        return Err(PrivacyError::InvalidSchedule);
    }
    Ok(hours * 60 + minutes)
}
pub fn status_text(snapshot: &PrivacySnapshot, language: UiLanguage) -> String {
    let zh = language == UiLanguage::Chinese;
    if snapshot.fail_closed {
        return if zh {
            "配置不可用，保持保护"
        } else {
            "Configuration unavailable; protected"
        }
        .into();
    }
    if !snapshot.configured {
        return if zh { "请先设置 4 位 PIN" } else { "Set a 4-digit PIN first" }.into();
    }
    let mode = match (snapshot.manual, snapshot.enabled, zh) {
        (true, true, true) => "手动开启",
        (true, false, true) => "手动关闭",
        (false, true, true) => "自动受限时段",
        (false, false, true) => "自动：未受限",
        (true, true, false) => "Manually on",
        (true, false, false) => "Manually off",
        (false, true, false) => "Scheduled restriction",
        (false, false, false) => "Automatic: unrestricted",
    };
    if snapshot.manual {
        if let Some(next) = snapshot.next_start {
            let next = next.with_timezone(&chrono::Local).format("%m-%d %H:%M");
            return if zh {
                format!("{mode} · 下一周期 {next} 接管")
            } else {
                format!("{mode} · Next cycle {next}")
            };
        }
    }
    mode.into()
}
pub fn error_text(error: &PrivacyError, language: UiLanguage) -> String {
    if language == UiLanguage::English {
        return error.to_string();
    }
    match error {
        PrivacyError::InvalidPin => "PIN 必须是 4 位数字（0–9）",
        PrivacyError::ConfirmationMismatch => "两次输入的 PIN 不一致",
        PrivacyError::IncorrectPin => "PIN 不正确，请重试",
        PrivacyError::Unconfigured => "请先设置 4 位 PIN",
        PrivacyError::Locked => "此操作需要重新验证 PIN",
        PrivacyError::Busy => "验证尚未结束，请稍候",
        PrivacyError::Stale => "授权已失效，请重新输入 PIN",
        PrivacyError::Cooldown(_) => "尝试次数过多，请等待冷却结束",
        PrivacyError::CorruptConfig => "隐私配置损坏或不可读，保持保护；请修复配置后重启",
        PrivacyError::Persistence => "隐私配置未能保存，已有保护不会解除",
        PrivacyError::InvalidSchedule => "请选择星期并输入有效 HH:mm，起止时间不能相同",
        PrivacyError::InvalidMedia => "无法识别此媒体",
        PrivacyError::WindowUnavailable => "无法打开隐私窗口",
    }
    .into()
}
pub fn rule_label(rule: &yoyo_core::privacy::PrivacyTimeRule, language: UiLanguage) -> String {
    let names = if language == UiLanguage::Chinese {
        ["一", "二", "三", "四", "五", "六", "日"]
    } else {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    };
    let days = if rule.weekdays == 127 {
        if language == UiLanguage::Chinese { "每天" } else { "Every day" }.into()
    } else {
        names
            .iter()
            .enumerate()
            .filter(|(i, _)| rule.weekdays & (1 << i) != 0)
            .map(|(_, day)| *day)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let overnight = if rule.end_minute < rule.start_minute {
        if language == UiLanguage::Chinese { "（次日）" } else { " (+1 day)" }
    } else {
        ""
    };
    format!(
        "{days}  {:02}:{:02} – {:02}:{:02}{overnight}",
        rule.start_minute / 60,
        rule.start_minute % 60,
        rule.end_minute / 60,
        rule.end_minute % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoyo_core::{MediaChapter, PlaylistEntry};
    struct Restricted;
    impl PlaybackAccess for Restricted {
        fn restricted(&self, key: &MediaKey) -> bool {
            key.as_str().contains("protected")
        }
    }
    #[test]
    fn redaction_clears_sensitive_metadata_but_never_mutates_persisted_state() {
        let mut state = PlayerState {
            current: Some(MediaLocator::File("protected.mp4".into())),
            position_seconds: 73.0,
            duration_seconds: Some(200.0),
            status_message: Some("protected.mp4 decoder".into()),
            last_error: Some("private path".into()),
            chapters: vec![MediaChapter {
                title: Some("private chapter".into()),
                time_seconds: 12.0,
            }],
            ..PlayerState::default()
        };
        state.subtitle.external_path = Some("private.srt".into());
        let before = state.clone();
        let public = redact_state(&state, true, UiLanguage::Chinese);
        assert_eq!(public.current.unwrap().as_label(), "受保护内容");
        assert_eq!(public.position_seconds, 0.0);
        assert_eq!(public.duration_seconds, None);
        assert!(public.chapters.is_empty());
        assert!(public.subtitle.external_path.is_none());
        assert_eq!(public.status_message.as_deref(), Some("受保护内容"));
        assert_eq!(state, before);
        assert_eq!(redact_state(&state, false, UiLanguage::English), state);
    }
    #[test]
    fn history_and_playlist_hide_only_restricted_labels() {
        let mut history = HistoryStore::default();
        history.remember(MediaLocator::File("ordinary.mp4".into()), "ordinary".into(), Some(3.0));
        history.remember(
            MediaLocator::File("protected.mp4".into()),
            "secret title".into(),
            Some(70.0),
        );
        let rows = history_rows(&history, &Restricted, UiLanguage::English);
        assert_eq!(rows[0].title, "Protected content");
        assert!(rows[0].subtitle.is_empty());
        assert_eq!(rows[1].title, "ordinary");
        assert_eq!(history.items()[0].title, "secret title");
        let playlist = PlaylistSnapshot {
            entries: vec![
                PlaylistEntry::new(MediaLocator::File("protected.mp4".into())),
                PlaylistEntry::new(MediaLocator::File("ordinary.mp4".into())),
            ],
            current_index: Some(0),
        };
        let rows = playlist_rows(&playlist, &Restricted, UiLanguage::Chinese);
        assert_eq!(rows[0].title, "受保护内容");
        assert!(rows[0].is_current);
        assert!(rows[1].title.contains("ordinary"));
    }
    #[test]
    fn schedule_inputs_require_exact_ascii_hh_mm() {
        for (value, minutes) in [("00:00", 0), ("09:30", 570), ("23:59", 1439)] {
            assert_eq!(parse_hhmm(value).unwrap(), minutes);
        }
        for value in ["9:00", "24:00", "12:60", "１２:００", "12.00", " 9:00", "09:00 "] {
            assert!(parse_hhmm(value).is_err());
        }
    }
}
