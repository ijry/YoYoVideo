use super::*;
use crate::privacy::{AuthPurpose, PinCredential, PrivacyDocument, PrivacyStore};
use slint::{
    Model,
    platform::{
        Platform, WindowAdapter,
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
    },
};
use yoyo_core::{
    MediaLocator,
    privacy::{ManualPrivacy, MediaKey},
};
struct SoftwarePlatform;
impl Platform for SoftwarePlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
    }
}
fn fixture() -> (tempfile::TempDir, Rc<RefCell<PrivacyUi>>, AuthGrant) {
    slint::platform::set_platform(Box::new(SoftwarePlatform)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = PrivacyStore::new(dir.path().join("privacy.toml"));
    let locator = MediaLocator::Url("https://example.test/fictional-protected.mp4".into());
    let mut document = PrivacyDocument {
        credential: Some(PinCredential::create("0123").unwrap()),
        manual: Some(ManualPrivacy { enabled: true, until: None }),
        ..PrivacyDocument::default()
    };
    document.protected.insert(MediaKey::from_locator(&locator).unwrap(), locator);
    store.save(&document).unwrap();
    let service = Arc::new(PrivacyService::load(store));
    let grant = service
        .authenticate(service.begin_auth(AuthPurpose::Settings).unwrap(), "0123")
        .unwrap()
        .unwrap();
    let ui = PrivacyUi::new(service, Rc::new(|_| {})).unwrap();
    (dir, ui, grant)
}
#[test]
fn closing_settings_clears_pin_and_sensitive_rows_and_revokes_the_grant() {
    let (_dir, ui, grant) = fixture();
    let mut ui = ui.borrow_mut();
    ui.grant = Some(grant.clone());
    ui.show_settings().unwrap();
    assert_eq!(ui.window.get_protected_items().row_count(), 1);
    ui.window.set_pin("0123".into());
    ui.window.set_confirmation("0123".into());
    ui.revoke(true);
    assert!(ui.window.get_pin().is_empty());
    assert!(ui.window.get_confirmation().is_empty());
    assert_eq!(ui.window.get_protected_items().row_count(), 0);
    assert_eq!(ui.window.get_rules().row_count(), 0);
    assert!(ui.service.settings(&grant).is_err());
}
#[test]
fn losing_focus_clears_authorized_text_without_unlocking_the_video() {
    let (_dir, ui, grant) = fixture();
    let mut ui = ui.borrow_mut();
    ui.grant = Some(grant.clone());
    ui.show_settings().unwrap();
    ui.revoke(false);
    assert!(ui.service.settings(&grant).is_err());
    assert!(ui.service.snapshot().enabled);
    assert_eq!(ui.window.get_mode(), 1);
    assert_eq!(ui.window.get_protected_items().row_count(), 0);
}
#[test]
fn polling_an_inactive_privileged_window_clears_sensitive_models_even_if_focus_event_was_missed() {
    let (_dir, ui, grant) = fixture();
    let mut ui = ui.borrow_mut();
    ui.grant = Some(grant.clone());
    ui.show_settings().unwrap();
    assert_eq!(ui.window.get_protected_items().row_count(), 1);
    // This software test window has no active native focus; the poll is the fallback.
    ui.poll();
    assert_eq!(ui.window.get_protected_items().row_count(), 0);
    assert!(ui.service.settings(&grant).is_err());
}
