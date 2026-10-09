use super::*;
use crate::privacy::{AuthPurpose, PrivacyDocument, PrivacyService, PrivacyStore};
use std::sync::Arc;

fn runtime_for(dir: &std::path::Path) -> DesktopRuntime {
    DesktopRuntime::new(
        AppConfig::default(),
        crate::HistoryRuntime::new(Some(dir.join("history.json")), HistoryStore::default(), true),
        crate::platform::RecentOpenStore::load(None).unwrap(),
        crate::SubtitlePrefsRuntime::load(None).unwrap(),
        crate::platform::MarkerStore::with_path(Some(dir.join("markers.toml"))),
        crate::initial_sidebar_state(false, 800.0),
        dir.join("unused.log"),
        None,
    )
}
fn configured(path: PathBuf) -> Arc<PrivacyService> {
    let service = Arc::new(PrivacyService::load(PrivacyStore::new(path)));
    let ticket = service.begin_auth(AuthPurpose::Setup).unwrap();
    service.setup(ticket, "0123", "0123").unwrap();
    service
}
#[test]
fn shutdown_flushes_a_newly_enabled_privacy_override() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("privacy.toml");
    let service = configured(path.clone());
    service.enable().unwrap();
    let mut runtime = runtime_for(dir.path());
    runtime.configure_privacy(service.clone());
    persist_playback_for_shutdown(&mut runtime).unwrap();
    let doc: PrivacyDocument = PrivacyStore::new(path).load().unwrap();
    assert!(doc.manual.is_some_and(|manual| manual.enabled));
    assert!(!service.snapshot().dirty);
}
#[test]
fn shutdown_rejects_a_failed_privacy_flush_and_keeps_protection_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("privacy.toml");
    let service = configured(path.clone());
    service.enable().unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut runtime = runtime_for(dir.path());
    runtime.configure_privacy(service.clone());
    assert!(persist_playback_for_shutdown(&mut runtime).is_err());
    assert!(service.snapshot().enabled);
    assert!(service.snapshot().dirty);
}
#[cfg(not(feature = "mpv-runtime"))]
#[test]
fn runtime_injects_privacy_before_a_controller_can_open_media() {
    let dir = tempfile::tempdir().unwrap();
    let service = configured(dir.path().join("privacy.toml"));
    let grant = service
        .authenticate(service.begin_auth(AuthPurpose::Settings).unwrap(), "0123")
        .unwrap()
        .unwrap();
    service
        .set_protection(
            &grant,
            MediaLocator::Url("https://example.test/protected.mp4".into()),
            true,
        )
        .unwrap();
    service.enable().unwrap();
    let mut runtime = runtime_for(dir.path());
    runtime.controller =
        Some(DesktopController::new(AppSession::new(AppConfig::default(), MpvBackend::default())));
    runtime.configure_privacy(service);
    let error = runtime
        .controller_mut()
        .unwrap()
        .dispatch(AppCommand::OpenUrl("https://example.test/protected.mp4".into()))
        .unwrap_err();
    assert_eq!(error.to_string(), "Protected content");
}

#[derive(Default)]
struct EventBackend {
    events: Vec<yoyo_core::BackendEvent>,
}
impl PlayerBackend for EventBackend {
    fn open(&mut self, _: &MediaLocator) -> Result<(), String> {
        Ok(())
    }
    fn send(&mut self, _: yoyo_core::BackendCommand) -> Result<(), String> {
        Ok(())
    }
    fn drain_events(&mut self) -> Vec<yoyo_core::BackendEvent> {
        std::mem::take(&mut self.events)
    }
}
struct Restricted;
impl yoyo_core::PlaybackAccess for Restricted {
    fn restricted(&self, _: &yoyo_core::privacy::MediaKey) -> bool {
        true
    }
}
fn protected_controller() -> DesktopController<EventBackend> {
    let mut session = AppSession::new(AppConfig::default(), EventBackend::default());
    session.handle_command(AppCommand::OpenFile("protected.mp4".into())).unwrap();
    session.backend_mut().events.extend([
        yoyo_core::BackendEvent::DurationChanged(Some(90.0)),
        yoyo_core::BackendEvent::TracksChanged {
            audio: vec![],
            subtitles: vec![],
            video: vec![yoyo_core::MediaTrack {
                id: 1,
                kind: yoyo_core::MediaTrackKind::Video,
                title: None,
                language: None,
                codec: None,
                source_path: None,
                external: false,
                selected: true,
            }],
        },
    ]);
    session.poll_backend().unwrap();
    session.set_playback_access(Arc::new(Restricted));
    session.enforce_privacy().unwrap();
    DesktopController::new(session)
}
#[test]
fn pending_history_seek_waits_while_protected_instead_of_failing_every_safe_ui_command() {
    let mut controller = protected_controller();
    let pending = crate::PendingResumeSeek::new(35.0);
    assert_eq!(apply_pending_resume(&mut controller, pending).unwrap(), pending);
    assert!(controller.session().state().paused);
}
#[test]
fn subtitle_restore_waits_while_protected_and_does_not_mark_unapplied_preferences_restored() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = runtime_for(dir.path());
    let mut controller = protected_controller();
    let snapshot = controller.session().state().clone();
    runtime.subtitle_prefs.remember_from_state(&snapshot);
    apply_subtitle_restore_if_needed(&mut runtime, &mut controller, None).unwrap();
    assert!(!controller.session().state().subtitle_preferences_restored);
}
#[test]
fn privacy_activation_clears_a_protected_url_left_in_the_open_menu_without_current_media() {
    struct Software;
    impl slint::platform::Platform for Software {
        fn create_window_adapter(
            &self,
        ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
            Ok(slint::platform::software_renderer::MinimalSoftwareWindow::new(
                slint::platform::software_renderer::RepaintBufferType::NewBuffer,
            ))
        }
    }
    slint::platform::set_platform(Box::new(Software)).unwrap();
    let app = MainWindow::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let service = configured(dir.path().join("privacy.toml"));
    let grant = service
        .authenticate(service.begin_auth(AuthPurpose::Settings).unwrap(), "0123")
        .unwrap()
        .unwrap();
    let url = "https://example.test/protected.mp4";
    service.set_protection(&grant, MediaLocator::Url(url.into()), true).unwrap();
    let mut runtime = runtime_for(dir.path());
    runtime.configure_privacy(service.clone());
    app.set_url_input_text(url.into());
    service.enable().unwrap();
    enforce_runtime_privacy(&app, &mut runtime);
    assert!(app.get_url_input_text().is_empty());
    app.set_url_input_text("https://example.test/ordinary.mp4".into());
    enforce_runtime_privacy(&app, &mut runtime);
    assert_eq!(app.get_url_input_text().as_str(), "https://example.test/ordinary.mp4");
}
