use super::*;
use crate::privacy::{PinCredential, PrivacyDocument};
use chrono::{Duration, Local, TimeZone};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use yoyo_core::privacy::PrivacyTimeRule;

struct Clock(Mutex<DateTime<Utc>>);
impl PrivacyClock for Clock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}
impl Clock {
    fn set(&self, now: DateTime<Utc>) {
        *self.0.lock().unwrap() = now;
    }
}
fn at(day: u32, h: u32, m: u32) -> DateTime<Utc> {
    Local.with_ymd_and_hms(2026, 10, day, h, m, 0).unwrap().with_timezone(&Utc)
}
fn schedule() -> PrivacySchedule {
    PrivacySchedule {
        enabled: true,
        rules: vec![PrivacyTimeRule { weekdays: 127, start_minute: 540, end_minute: 1080 }],
    }
}
fn locator() -> MediaLocator {
    MediaLocator::Url("https://example.test/protected.mp4".into())
}
fn key() -> MediaKey {
    MediaKey::from_locator(&locator()).unwrap()
}
fn fixture() -> (tempfile::TempDir, PathBuf, Arc<Clock>, PrivacyService) {
    static CREDENTIAL: OnceLock<PinCredential> = OnceLock::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("privacy.toml");
    let mut doc = PrivacyDocument {
        credential: Some(CREDENTIAL.get_or_init(|| PinCredential::create("0123").unwrap()).clone()),
        schedule: schedule(),
        ..PrivacyDocument::default()
    };
    doc.protected.insert(key(), locator());
    let store = PrivacyStore::new(path.clone());
    store.save(&doc).unwrap();
    let clock = Arc::new(Clock(Mutex::new(at(8, 10, 0))));
    let service = PrivacyService::with_clock(store, clock.clone());
    (dir, path, clock, service)
}
fn authorize(service: &PrivacyService) -> AuthGrant {
    let ticket = service.begin_auth(AuthPurpose::Settings).unwrap();
    service.authenticate(ticket, "0123").unwrap().unwrap()
}

#[test]
fn first_setup_has_no_default_pin_and_needs_matching_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let service = PrivacyService::load(PrivacyStore::new(dir.path().join("privacy.toml")));
    assert!(!service.snapshot().configured);
    assert!(service.enable().is_err());
    let ticket = service.begin_auth(AuthPurpose::Setup).unwrap();
    assert!(service.setup(ticket, "0123", "3210").is_err());
    assert!(!service.snapshot().configured);
    let ticket = service.begin_auth(AuthPurpose::Setup).unwrap();
    let grant = service.setup(ticket, "0123", "0123").unwrap();
    assert!(service.snapshot().configured);
    assert!(!service.snapshot().enabled);
    assert!(service.settings(&grant).is_ok());
    service.enable().unwrap();
    assert!(service.snapshot().enabled);
    assert!(service.settings(&grant).is_err());
}
#[test]
fn manual_off_survives_poll_end_restart_and_expires_at_next_start() {
    let (_dir, path, clock, service) = fixture();
    assert!(service.restricted(&key()));
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    assert!(service.authenticate(ticket, "0123").unwrap().is_none());
    for now in [at(8, 10, 1), at(8, 18, 0), at(9, 8, 59)] {
        clock.set(now);
        service.tick();
        assert!(!service.restricted(&key()));
        assert!(service.snapshot().manual);
    }
    let restarted = PrivacyService::with_clock(PrivacyStore::new(path), clock.clone());
    assert!(!restarted.restricted(&key()));
    assert!(restarted.snapshot().manual);
    clock.set(at(9, 9, 0));
    // No timer needed: the access check itself sees the new cycle.
    assert!(restarted.restricted(&key()));
    assert!(!restarted.snapshot().manual);
}
#[test]
fn manual_on_outside_hours_persists_until_next_start_then_schedule_can_end() {
    let (_dir, path, clock, service) = fixture();
    clock.set(at(8, 20, 0));
    service.enable().unwrap();
    assert!(service.restricted(&key()));
    service.flush().unwrap();
    let service = PrivacyService::with_clock(PrivacyStore::new(path), clock.clone());
    clock.set(at(9, 8, 0));
    assert!(service.restricted(&key()));
    clock.set(at(9, 9, 0));
    service.tick();
    assert!(service.restricted(&key()));
    assert!(!service.snapshot().manual);
    clock.set(at(9, 18, 0));
    assert!(!service.restricted(&key()));
}
#[test]
fn five_wrong_attempts_survive_restart_until_cooldown_ends() {
    let (_dir, path, clock, service) = fixture();
    for _ in 0..5 {
        let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
        assert!(service.authenticate(ticket, "1234").is_err());
    }
    assert_eq!(service.snapshot().cooldown_seconds, 30);
    let service = PrivacyService::with_clock(PrivacyStore::new(path), clock.clone());
    assert!(matches!(service.begin_auth(AuthPurpose::Unlock), Err(PrivacyError::Cooldown(30))));
    clock.set(at(8, 10, 0) + Duration::seconds(31));
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    service.authenticate(ticket, "0123").unwrap();
    assert!(!service.restricted(&key()));
    assert_eq!(service.snapshot().cooldown_seconds, 0);
}
#[test]
fn cancelling_or_reenabling_invalidates_unlock_tickets_and_settings_grants() {
    for reenabling in [false, true] {
        let (_dir, _path, _clock, service) = fixture();
        let grant = authorize(&service);
        let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
        if reenabling {
            service.enable().unwrap();
        } else {
            service.cancel_authorization();
        }
        assert!(service.authenticate(ticket, "0123").is_err());
        assert!(service.restricted(&key()));
        assert!(service.settings(&grant).is_err());
        // The completed/cancelled job must not permanently leave the service busy.
        assert!(service.begin_auth(AuthPurpose::Settings).is_ok());
    }
}
#[test]
fn a_new_cycle_invalidates_authorization_even_without_a_timer_tick() {
    let (_dir, _path, clock, service) = fixture();
    let grant = authorize(&service);
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    clock.set(at(9, 9, 0));
    assert!(service.authenticate(ticket, "0123").is_err());
    assert!(service.restricted(&key()));
    assert!(service.set_protection(&grant, locator(), false).is_err());
}
#[test]
fn list_and_schedule_edits_are_pin_scoped_and_recompute_override_expiry() {
    let (_dir, _path, clock, service) = fixture();
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    service.authenticate(ticket, "0123").unwrap();
    let grant = authorize(&service);
    let edited = PrivacySchedule {
        enabled: true,
        rules: vec![PrivacyTimeRule { weekdays: 127, start_minute: 12 * 60, end_minute: 13 * 60 }],
    };
    service.set_schedule(&grant, edited).unwrap();
    assert!(!service.restricted(&key()));
    assert_eq!(service.snapshot().next_start, Some(at(8, 12, 0)));
    clock.set(at(8, 12, 0));
    assert!(service.restricted(&key()));
    assert!(service.settings(&grant).is_err());
    let grant = authorize(&service);
    service.remove_protection(&grant, &key()).unwrap();
    assert!(!service.restricted(&key()));
    service.set_protection(&grant, locator(), true).unwrap();
    assert!(service.restricted(&key()));
}
#[test]
fn write_failure_cannot_lower_protection_and_enable_stays_in_memory() {
    let (_dir, path, clock, service) = fixture();
    let grant = authorize(&service);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(service.remove_protection(&grant, &key()).is_err());
    assert!(service.restricted(&key()));
    clock.set(at(8, 20, 0));
    service.enable().unwrap();
    assert!(service.flush().is_err());
    assert!(service.snapshot().dirty);
    assert!(service.restricted(&key()));
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    assert!(service.authenticate(ticket, "0123").is_err());
    assert!(service.restricted(&key()));
}
#[test]
fn corrupt_configuration_fails_closed_without_overwriting_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("privacy.toml");
    std::fs::write(&path, "broken [ schema").unwrap();
    let service = PrivacyService::load(PrivacyStore::new(path.clone()));
    assert!(service.snapshot().fail_closed);
    assert!(service.restricted(&key()));
    assert!(
        service.restricted(
            &MediaKey::from_locator(&MediaLocator::Url("https://example.test/ordinary.mp4".into()))
                .unwrap()
        )
    );
    assert!(service.begin_auth(AuthPurpose::Setup).is_err());
    let _ = service.flush();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "broken [ schema");
}
#[test]
fn change_pin_needs_fresh_old_pin_authorization_and_confirmation() {
    let (_dir, path, _clock, service) = fixture();
    let grant = authorize(&service);
    assert!(service.change_pin(&grant, "4321", "4321").is_err());
    let ticket = service.begin_auth(AuthPurpose::ChangePin).unwrap();
    let grant = service.authenticate(ticket, "0123").unwrap().unwrap();
    assert!(service.change_pin(&grant, "4321", "4320").is_err());
    service.change_pin(&grant, "4321", "4321").unwrap();
    let credential = PrivacyStore::new(path).load().unwrap().credential.unwrap();
    assert!(credential.verify("4321").unwrap());
    assert!(!credential.verify("0123").unwrap());
    assert!(service.settings(&grant).is_err());
}

#[test]
fn background_verification_does_not_unlock_before_the_ui_accepts_it() {
    let (_dir, _path, _clock, service) = fixture();
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    let reply = service.verify_pin(ticket, "0123").unwrap();
    assert!(service.restricted(&key()));
    assert!(service.apply_verification(reply).unwrap().is_none());
    assert!(!service.restricted(&key()));
}
#[test]
fn queued_correct_replies_cannot_unlock_after_cancel_reenable_or_new_cycle() {
    for cause in 0..3 {
        let (_dir, _path, clock, service) = fixture();
        let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
        let reply = service.verify_pin(ticket, "0123").unwrap();
        match cause {
            0 => service.cancel_authorization(),
            1 => service.enable().unwrap(),
            _ => clock.set(at(9, 9, 0)),
        }
        assert!(service.apply_verification(reply).is_err());
        assert!(service.restricted(&key()));
    }
}
#[test]
fn cancelled_guesses_remain_durable_and_the_pending_slot_is_bounded() {
    let (_dir, path, _clock, service) = fixture();
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    let reply = service.verify_pin(ticket, "1234").unwrap();
    service.cancel_authorization();
    assert!(matches!(service.begin_auth(AuthPurpose::Settings), Err(PrivacyError::Busy)));
    assert_eq!(PrivacyStore::new(path).load().unwrap().failed_attempts, 1);
    assert!(service.apply_verification(reply).is_err());
    assert!(service.begin_auth(AuthPurpose::Settings).is_ok());
}
#[test]
fn prepared_pin_changes_and_first_setup_expire_when_their_window_closes() {
    let dir = tempfile::tempdir().unwrap();
    let service = PrivacyService::load(PrivacyStore::new(dir.path().join("privacy.toml")));
    let ticket = service.begin_auth(AuthPurpose::Setup).unwrap();
    let prepared = service.prepare_setup(ticket, "0123", "0123").unwrap();
    service.cancel_authorization();
    assert!(service.apply_setup(prepared).is_err());
    assert!(!service.snapshot().configured);
    let (_dir, path, _clock, service) = fixture();
    let ticket = service.begin_auth(AuthPurpose::ChangePin).unwrap();
    let grant = service.authenticate(ticket, "0123").unwrap().unwrap();
    let prepared = service.prepare_pin_change(&grant, "4321", "4321").unwrap();
    service.cancel_authorization();
    assert!(service.apply_pin_change(prepared).is_err());
    assert!(PrivacyStore::new(path).load().unwrap().credential.unwrap().verify("0123").unwrap());
}
