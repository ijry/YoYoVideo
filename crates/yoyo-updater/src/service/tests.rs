use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tempfile::{TempDir, tempdir};

#[derive(Default)]
struct Calls {
    installs: AtomicUsize,
    checks: AtomicUsize,
    corrupt: AtomicBool,
    fail_install: AtomicBool,
}
struct FakeBackend {
    packages: PathBuf,
    version: Version,
    offer: VerifiedManifest,
    calls: Arc<Calls>,
}
impl Backend for FakeBackend {
    fn current_version(&self) -> &Version {
        &self.version
    }
    fn packages_dir(&self) -> &Path {
        &self.packages
    }
    fn check(&mut self) -> Result<Option<VerifiedManifest>, UpdateError> {
        self.calls.checks.fetch_add(1, Ordering::SeqCst);
        Ok(Some(self.offer.clone()))
    }
    fn download(
        &mut self,
        candidate: &VerifiedManifest,
        progress: Sender<i16>,
    ) -> Result<(), UpdateError> {
        std::fs::create_dir_all(&self.packages)?;
        let bytes = if self.calls.corrupt.load(Ordering::SeqCst) { b"abd" } else { b"abc" };
        std::fs::write(self.packages.join(&candidate.asset().FileName), bytes)?;
        let _ = progress.send(100);
        Ok(())
    }
    fn launch(&mut self, _: &VerifiedManifest) -> Result<(), UpdateError> {
        self.calls.installs.fetch_add(1, Ordering::SeqCst);
        if self.calls.fail_install.load(Ordering::SeqCst) {
            Err(UpdateError::Install("test helper failed".into()))
        } else {
            Ok(())
        }
    }
}
struct Fixture {
    dir: TempDir,
    key: String,
    offer: VerifiedManifest,
    calls: Arc<Calls>,
}
impl Fixture {
    fn new() -> Self {
        let raw = serde_json::to_vec(&json!({"schema_version":1,"app_id":"YoYoVideo","platform":"windows-x64","channel":"stable-windows-x64","version":"0.0.2","release_tag":"v0.0.2","feed":{"Assets":[{"PackageId":"YoYoVideo","Version":"0.0.2","Type":"Full","FileName":"YoYoVideo-0.0.2-full.nupkg","Size":3,"SHA256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","SHA1":"a9993e364706816aba3e25717850c26c9cd0d89d","NotesMarkdown":"New release","NotesHtml":""}]}})).unwrap();
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let key = STANDARD.encode(pair.pk.to_box().unwrap().into_string());
        let sig = STANDARD.encode(
            minisign::sign(Some(&pair.pk), &pair.sk, raw.as_slice(), Some("service test"), None)
                .unwrap()
                .to_string(),
        );
        let offer = VerifiedManifest::verify(&raw, &sig, &key, Platform::WindowsX64).unwrap();
        Self { dir: tempdir().unwrap(), key, offer, calls: Arc::new(Calls::default()) }
    }
    fn config(&self) -> ServiceConfig {
        ServiceConfig {
            platform: Platform::WindowsX64,
            public_key: self.key.clone(),
            cache_dir: self.dir.path().join("receipts"),
        }
    }
    fn service(&self) -> UpdateService {
        self.service_at(Version::new(0, 0, 1))
    }
    fn service_at(&self, version: Version) -> UpdateService {
        UpdateService::with_backend(
            self.config(),
            Box::new(FakeBackend {
                packages: self.dir.path().join("packages"),
                version,
                offer: self.offer.clone(),
                calls: self.calls.clone(),
            }),
        )
    }
    fn package(&self) -> PathBuf {
        self.dir.path().join("packages").join(&self.offer.asset().FileName)
    }
    fn ready(&self) -> UpdateService {
        let mut service = self.service();
        assert_eq!(service.check().unwrap().phase, UpdatePhase::Available);
        let (tx, _) = std::sync::mpsc::channel();
        service.download(tx).unwrap();
        assert_eq!(service.snapshot().phase, UpdatePhase::ReadyToInstall);
        service
    }
}
#[test]
fn confirmation_is_two_phase_and_installer_is_launched_once() {
    let f = Fixture::new();
    let mut service = f.ready();
    service.verify_for_install().unwrap();
    assert_eq!(service.snapshot().phase, UpdatePhase::PreparingInstall);
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
    service.launch_installer().unwrap();
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 1);
    assert!(service.launch_installer().is_err());
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 1);
}
#[test]
fn download_and_launch_require_their_previous_steps() {
    let f = Fixture::new();
    let mut service = f.service();
    let (tx, _) = std::sync::mpsc::channel();
    assert!(service.download(tx).is_err());
    assert!(service.verify_for_install().is_err());
    assert!(service.launch_installer().is_err());
    let mut service = f.ready();
    assert!(service.launch_installer().is_err());
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
}
#[test]
fn restart_restores_authenticated_pending_update_without_installing() {
    let f = Fixture::new();
    drop(f.ready());
    let mut restored = f.service();
    assert_eq!(restored.snapshot().phase, UpdatePhase::ReadyToInstall);
    assert_eq!(restored.snapshot().version, "0.0.2");
    assert_eq!(f.calls.checks.load(Ordering::SeqCst), 1);
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
    restored.verify_for_install().unwrap();
}
#[test]
fn same_or_older_cached_version_is_not_installed() {
    let f = Fixture::new();
    drop(f.ready());
    for current in [Version::new(0, 0, 2), Version::new(0, 0, 3)] {
        let mut service = f.service_at(current);
        assert_eq!(service.snapshot().phase, UpdatePhase::Idle);
        assert_eq!(service.check().unwrap().phase, UpdatePhase::UpToDate);
        assert!(service.verify_for_install().is_err());
    }
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
}
#[test]
fn cache_and_package_tampering_are_rejected_on_restore_and_before_launch() {
    let f = Fixture::new();
    let mut service = f.ready();
    service.verify_for_install().unwrap();
    std::fs::write(f.package(), b"abd").unwrap();
    assert!(service.launch_installer().is_err());
    assert_eq!(f.service().snapshot().phase, UpdatePhase::Error);
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
    std::fs::write(f.package(), b"abc").unwrap();
    let receipt = f.config().cache_dir.join("pending-update.json");
    let mut saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).unwrap()).unwrap();
    saved["signature"] = json!("invalid");
    std::fs::write(&receipt, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert_eq!(f.service().snapshot().phase, UpdatePhase::Error);
    let mut recovered = f.service();
    assert_eq!(recovered.check().unwrap().phase, UpdatePhase::Available);
}
#[test]
fn bad_download_or_receipt_failure_never_becomes_installable() {
    let f = Fixture::new();
    f.calls.corrupt.store(true, Ordering::SeqCst);
    let mut service = f.service();
    service.check().unwrap();
    let (tx, _) = std::sync::mpsc::channel();
    assert!(service.download(tx).is_err());
    assert!(!f.config().cache_dir.join("pending-update.json").exists());
    assert!(service.launch_installer().is_err());
    f.calls.corrupt.store(false, Ordering::SeqCst);
    let mut service = f.service();
    service.check().unwrap();
    std::fs::write(&f.config().cache_dir, b"blocks directory creation").unwrap();
    let (tx, _) = std::sync::mpsc::channel();
    assert!(service.download(tx).is_err());
    assert!(service.verify_for_install().is_err());
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
}
#[test]
fn abort_after_failed_player_save_revokes_install_authorization() {
    let f = Fixture::new();
    let mut service = f.ready();
    service.verify_for_install().unwrap();
    service.abort_install("state save failed".into());
    assert_eq!(service.snapshot().phase, UpdatePhase::ReadyToInstall);
    assert!(service.launch_installer().is_err());
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
}
#[test]
fn helper_failure_is_reported_without_success_state() {
    let f = Fixture::new();
    let mut service = f.ready();
    f.calls.fail_install.store(true, Ordering::SeqCst);
    service.verify_for_install().unwrap();
    assert!(service.launch_installer().is_err());
    assert_eq!(service.snapshot().phase, UpdatePhase::Error);
    assert!(service.snapshot().error.contains("test helper failed"));
}
#[test]
fn unsigned_or_cross_platform_backend_candidate_is_not_accepted() {
    let f = Fixture::new();
    let mut service = f.service();
    service.config.platform = Platform::MacosX64;
    assert!(service.check().is_err());
    assert!(service.verify_for_install().is_err());
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
}

#[test]
fn worker_orders_download_progress_and_requires_current_install_authorization() {
    use crate::worker::{UpdateCommand, UpdateEvent, UpdateRequest, spawn_with_service};
    let f = Fixture::new();
    let service = f.service();
    let mut worker = spawn_with_service(move || Ok(service)).unwrap();
    for (id, command) in [
        (2, UpdateCommand::Check),
        (1, UpdateCommand::Download),
        (3, UpdateCommand::Download),
        (4, UpdateCommand::VerifyForInstall),
        (4, UpdateCommand::LaunchInstaller),
        (4, UpdateCommand::LaunchInstaller),
        (0, UpdateCommand::Stop),
    ] {
        worker.requests.send(UpdateRequest { id, command }).unwrap();
    }
    worker.thread.take().unwrap().join().unwrap();
    let events: Vec<_> = worker.events.try_iter().collect();
    assert_eq!(events.iter().filter(|m| matches!(&m.event, UpdateEvent::Snapshot(s) if s.phase == UpdatePhase::Downloading)).count(), 1, "progress must not repeatedly copy release notes");
    assert!(!events.iter().any(|m| m.id == 1));
    assert_eq!(
        events.iter().filter(|m| matches!(m.event, UpdateEvent::InstallerStarted)).count(),
        1
    );
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 1);
    let ready = events.iter().position(|m| matches!(&m.event, UpdateEvent::Snapshot(s) if s.phase == UpdatePhase::ReadyToInstall)).unwrap();
    assert!(!events[ready + 1..].iter().any(
        |m| matches!(&m.event, UpdateEvent::Snapshot(s) if s.phase == UpdatePhase::Downloading)
    ));
}
#[test]
fn worker_discards_stale_launch_after_a_new_check() {
    use crate::worker::{UpdateCommand, UpdateEvent, UpdateRequest, spawn_with_service};
    let f = Fixture::new();
    let service = f.service();
    let mut worker = spawn_with_service(move || Ok(service)).unwrap();
    for (id, command) in [
        (1, UpdateCommand::Check),
        (2, UpdateCommand::Download),
        (3, UpdateCommand::VerifyForInstall),
        (4, UpdateCommand::Check),
        (3, UpdateCommand::LaunchInstaller),
        (0, UpdateCommand::Stop),
    ] {
        worker.requests.send(UpdateRequest { id, command }).unwrap();
    }
    worker.thread.take().unwrap().join().unwrap();
    assert!(!worker.events.try_iter().any(|m| matches!(m.event, UpdateEvent::InstallerStarted)));
    assert_eq!(f.calls.installs.load(Ordering::SeqCst), 0);
    assert_eq!(f.calls.checks.load(Ordering::SeqCst), 2);
}
#[test]
fn an_unmanaged_installation_is_a_nonfatal_worker_state() {
    use crate::worker::{UpdateCommand, UpdateEvent, UpdateRequest, spawn_with_service};
    let mut worker = spawn_with_service(|| Err(UpdateError::Unsupported)).unwrap();
    worker.requests.send(UpdateRequest { id: 1, command: UpdateCommand::Check }).unwrap();
    worker.requests.send(UpdateRequest { id: 0, command: UpdateCommand::Stop }).unwrap();
    worker.thread.take().unwrap().join().unwrap();
    let messages: Vec<_> = worker.events.try_iter().collect();
    assert!(messages.iter().any(
        |m| matches!(&m.event, UpdateEvent::Snapshot(s) if s.phase == UpdatePhase::Unsupported)
    ));
    assert!(!messages.iter().any(|m| matches!(m.event, UpdateEvent::InstallerStarted)));
}
