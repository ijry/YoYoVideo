use chrono::{TimeZone, Utc};
use yoyo_core::{
    MediaLocator,
    privacy::{ManualPrivacy, MediaKey, PrivacySchedule, PrivacyTimeRule},
};

fn at(day: u32, hour: u32, minute: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, hour, minute, 0).unwrap()
}
fn daily(start: u16, end: u16) -> PrivacySchedule {
    PrivacySchedule {
        enabled: true,
        rules: vec![PrivacyTimeRule { weekdays: 127, start_minute: start, end_minute: end }],
    }
}

#[test]
fn restricted_period_is_half_open() {
    let s = daily(540, 1080);
    for (now, expected) in
        [(at(8, 8, 59), false), (at(8, 9, 0), true), (at(8, 17, 59), true), (at(8, 18, 0), false)]
    {
        assert_eq!(s.evaluate(now, &Utc).enabled, expected, "{now}");
    }
}
#[test]
fn manual_off_lasts_through_end_until_next_start() {
    let s = daily(540, 1080);
    assert_eq!(s.evaluate(at(8, 10, 0), &Utc).next_start, Some(at(9, 9, 0)));
    let manual = ManualPrivacy { enabled: false, until: Some(at(9, 9, 0)) };
    for now in [at(8, 10, 1), at(8, 18, 0), at(9, 8, 59)] {
        let d = s.with_override(Some(&manual), now, &Utc);
        assert!(!d.enabled);
        assert!(d.manual);
    }
    let d = s.with_override(Some(&manual), at(9, 9, 0), &Utc);
    assert!(d.enabled);
    assert!(!d.manual);
}
#[test]
fn manual_on_is_not_cleared_by_current_period_end_or_restart() {
    let s = daily(540, 1080);
    let manual = ManualPrivacy { enabled: true, until: Some(at(9, 9, 0)) };
    let serialized = toml::to_string(&manual).unwrap();
    let restored: ManualPrivacy = toml::from_str(&serialized).unwrap();
    assert!(s.with_override(Some(&restored), at(8, 20, 0), &Utc).enabled);
    assert!(s.with_override(Some(&restored), at(9, 9, 0), &Utc).enabled);
    assert!(!s.with_override(Some(&restored), at(9, 18, 0), &Utc).enabled);
}
#[test]
fn missed_start_expires_override_even_after_that_period_ends() {
    let s = daily(540, 1080);
    let manual = ManualPrivacy { enabled: true, until: Some(at(9, 9, 0)) };
    let d = s.with_override(Some(&manual), at(10, 20, 0), &Utc);
    assert!(!d.enabled);
    assert!(!d.manual);
}
#[test]
fn no_schedule_keeps_manual_state_indefinitely() {
    let s = PrivacySchedule::default();
    assert!(!s.evaluate(at(8, 10, 0), &Utc).enabled);
    assert_eq!(s.evaluate(at(8, 10, 0), &Utc).next_start, None);
    for enabled in [false, true] {
        let manual = ManualPrivacy { enabled, until: None };
        let d = s.with_override(Some(&manual), at(31, 10, 0), &Utc);
        assert_eq!(d.enabled, enabled);
        assert!(d.manual);
    }
}
#[test]
fn overnight_belongs_to_start_weekday() {
    // 2026-10-09 is Friday: Monday = bit 0.
    let s = PrivacySchedule {
        enabled: true,
        rules: vec![PrivacyTimeRule {
            weekdays: 1 << 4,
            start_minute: 22 * 60,
            end_minute: 2 * 60,
        }],
    };
    for (now, expected) in [
        (at(8, 23, 0), false),
        (at(9, 22, 0), true),
        (at(10, 1, 59), true),
        (at(10, 2, 0), false),
        (at(10, 22, 0), false),
    ] {
        assert_eq!(s.evaluate(now, &Utc).enabled, expected, "{now}");
    }
    assert_eq!(s.evaluate(at(10, 3, 0), &Utc).next_start, Some(at(16, 22, 0)));
}
#[test]
fn overlapping_and_touching_rules_are_one_cycle() {
    let s = PrivacySchedule {
        enabled: true,
        rules: vec![
            PrivacyTimeRule { weekdays: 127, start_minute: 540, end_minute: 720 },
            PrivacyTimeRule { weekdays: 127, start_minute: 660, end_minute: 780 },
            PrivacyTimeRule { weekdays: 127, start_minute: 780, end_minute: 1080 },
        ],
    };
    assert_eq!(s.evaluate(at(8, 10, 0), &Utc).next_start, Some(at(9, 9, 0)));
    assert!(s.evaluate(at(8, 13, 0), &Utc).enabled);
}
#[test]
fn permanently_contiguous_week_has_no_fake_future_start() {
    let s = PrivacySchedule {
        enabled: true,
        rules: vec![
            PrivacyTimeRule { weekdays: 127, start_minute: 0, end_minute: 720 },
            PrivacyTimeRule { weekdays: 127, start_minute: 720, end_minute: 0 },
        ],
    };
    assert!(s.evaluate(at(8, 10, 0), &Utc).enabled);
    assert_eq!(s.evaluate(at(8, 10, 0), &Utc).next_start, None);
}
#[test]
fn invalid_time_or_weekday_cannot_be_saved() {
    for rule in [
        PrivacyTimeRule { weekdays: 0, start_minute: 1, end_minute: 2 },
        PrivacyTimeRule { weekdays: 128, start_minute: 1, end_minute: 2 },
        PrivacyTimeRule { weekdays: 1, start_minute: 0, end_minute: 1440 },
        PrivacyTimeRule { weekdays: 1, start_minute: 60, end_minute: 60 },
    ] {
        assert!(PrivacySchedule { enabled: true, rules: vec![rule] }.validate().is_err());
    }
    assert!(daily(540, 1080).validate().is_ok());
}
#[test]
fn spring_gap_advances_to_first_real_minute() {
    let tz = chrono_tz::America::New_York;
    let s = daily(2 * 60 + 30, 4 * 60);
    let before = Utc.with_ymd_and_hms(2026, 3, 8, 6, 59, 0).unwrap();
    let start = Utc.with_ymd_and_hms(2026, 3, 8, 7, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 3, 8, 8, 0, 0).unwrap();
    assert!(!s.evaluate(before, &tz).enabled);
    assert_eq!(s.evaluate(before, &tz).next_start, Some(start));
    assert!(s.evaluate(start, &tz).enabled);
    assert!(!s.evaluate(end, &tz).enabled);
}
#[test]
fn repeated_dst_time_uses_earliest_start_and_latest_end() {
    let tz = chrono_tz::America::New_York;
    let s = daily(60 + 15, 60 + 45);
    for (h, m, expected) in
        [(5, 14, false), (5, 15, true), (5, 59, true), (6, 30, true), (6, 45, false)]
    {
        let now = Utc.with_ymd_and_hms(2026, 11, 1, h, m, 0).unwrap();
        assert_eq!(s.evaluate(now, &tz).enabled, expected, "{now}");
    }
}
#[test]
fn url_identity_normalizes_host_scheme_port_but_keeps_path_and_query() {
    let key = |s: &str| MediaKey::from_locator(&MediaLocator::Url(s.into())).unwrap();
    assert_eq!(key("HTTPS://EXAMPLE.COM:443/A.mp4?q=One"), key("https://example.com/A.mp4?q=One"));
    assert_ne!(key("https://example.com/A.mp4?q=One"), key("https://example.com/a.mp4?q=One"));
    assert_ne!(key("https://example.com/A.mp4?q=One"), key("https://example.com/A.mp4?q=Two"));
    assert_eq!(key("rtsp://EXAMPLE.COM:554/live"), key("rtsp://example.com/live"));
}
#[test]
fn file_identity_merges_existing_and_missing_lexical_aliases_without_rewriting_history() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("child")).unwrap();
    for exists in [false, true] {
        let name = if exists { "existing.mp4" } else { "missing.mp4" };
        let actual = dir.path().join(name);
        if exists {
            std::fs::write(&actual, b"test").unwrap();
        }
        let alias = MediaLocator::File(dir.path().join("child").join("..").join(name));
        let old_label = alias.as_label();
        assert_eq!(
            MediaKey::from_locator(&alias).unwrap(),
            MediaKey::from_locator(&MediaLocator::File(actual)).unwrap()
        );
        assert_eq!(alias.as_label(), old_label);
    }
}
#[cfg(windows)]
#[test]
fn windows_file_identity_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("MiXeD.mp4");
    std::fs::write(&p, b"test").unwrap();
    let a = MediaKey::from_locator(&MediaLocator::File(p.clone())).unwrap();
    let b = MediaKey::from_locator(&MediaLocator::File(p.to_string_lossy().to_uppercase().into()))
        .unwrap();
    assert_eq!(a, b);
}
#[cfg(unix)]
#[test]
fn existing_file_symlink_matches_real_identity() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.mp4");
    let link = dir.path().join("alias.mp4");
    std::fs::write(&real, b"test").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(
        MediaKey::from_locator(&MediaLocator::File(real)).unwrap(),
        MediaKey::from_locator(&MediaLocator::File(link)).unwrap()
    );
}
