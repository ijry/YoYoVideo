use tempfile::tempdir;
use yoyo_updater::UpdatePreferences;
#[test]
fn manual_check_bypasses_automatic_interval_and_disabled_setting() {
    let p = UpdatePreferences { automatic_check: true, last_checked_at: Some(1000) };
    assert!(!p.should_check(1001, false));
    assert!(p.should_check(1001, true));
    assert!(p.should_check(87400, false));
    let disabled = UpdatePreferences { automatic_check: false, last_checked_at: None };
    assert!(!disabled.should_check(100000, false));
    assert!(disabled.should_check(100000, true));
}
#[test]
fn first_check_and_clock_rollback_are_not_blocked_forever() {
    assert!(UpdatePreferences::default().should_check(1000, false));
    let p = UpdatePreferences { automatic_check: true, last_checked_at: Some(2000) };
    assert!(p.should_check(1000, false));
}
#[test]
fn settings_round_trip_without_using_the_real_user_profile() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings/updater.toml");
    assert!(UpdatePreferences::load(&path).unwrap().automatic_check);
    let p = UpdatePreferences { automatic_check: false, last_checked_at: Some(123456) };
    p.save(&path).unwrap();
    assert_eq!(UpdatePreferences::load(&path).unwrap(), p);
    let p = UpdatePreferences { automatic_check: true, last_checked_at: None };
    p.save(&path).unwrap();
    assert_eq!(UpdatePreferences::load(&path).unwrap(), p);
}
#[test]
fn malformed_or_oversized_preferences_report_an_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("updater.toml");
    std::fs::write(&path, "automatic_check = 'not a boolean'").unwrap();
    assert!(UpdatePreferences::load(&path).is_err());
    std::fs::write(&path, vec![b' '; 64 * 1024 + 1]).unwrap();
    assert!(UpdatePreferences::load(&path).is_err());
    assert!(UpdatePreferences::default().save(dir.path()).is_err());
}
