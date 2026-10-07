use super::*;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform as SlintPlatform, WindowAdapter};
use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, Sender};
use tempfile::{TempDir, tempdir};
struct Headless;
impl SlintPlatform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
    }
}
struct FakeBridge {
    requests: Sender<UpdateRequest>,
    events: Receiver<UpdateMessage>,
}
impl Bridge for FakeBridge {
    fn send(&self, request: UpdateRequest) -> bool {
        self.requests.send(request).is_ok()
    }
    fn receive(&self) -> Result<UpdateMessage, TryRecvError> {
        self.events.try_recv()
    }
}
struct Harness {
    _dir: TempDir,
    main: MainWindow,
    runtime: UpdateRuntime,
    events: Sender<UpdateMessage>,
    requests: Receiver<UpdateRequest>,
    save_fails: Rc<Cell<bool>>,
    saves: Rc<Cell<usize>>,
    exits: Rc<Cell<usize>>,
}
impl Harness {
    fn new() -> Self {
        slint::platform::set_platform(Box::new(Headless)).unwrap();
        let main = MainWindow::new().unwrap();
        let dir = tempdir().unwrap();
        let paths = AppPaths {
            config_dir: dir.path().join("config"),
            data_dir: dir.path().join("data"),
            cache_dir: dir.path().join("cache"),
        };
        let (request_tx, requests) = mpsc::channel();
        let (events, event_rx) = mpsc::channel();
        let save_fails = Rc::new(Cell::new(false));
        let saves = Rc::new(Cell::new(0));
        let exits = Rc::new(Cell::new(0));
        let save: SaveState = {
            let fail = save_fails.clone();
            let count = saves.clone();
            Box::new(move || {
                count.set(count.get() + 1);
                if fail.get() { Err("test save failed".into()) } else { Ok(()) }
            })
        };
        let exit: ExitPlayer = {
            let count = exits.clone();
            Box::new(move || count.set(count.get() + 1))
        };
        let runtime = UpdateRuntime::with_bridge(
            &main,
            Some(paths),
            Box::new(FakeBridge { requests: request_tx, events: event_rx }),
            save,
            exit,
        )
        .unwrap();
        Self { _dir: dir, main, runtime, events, requests, save_fails, saves, exits }
    }
    fn window(&self) -> UpdateWindow {
        self.runtime.state.borrow().window.clone_strong()
    }
    fn snapshot(&self, id: u64, phase: UpdatePhase) {
        let mut s = yoyo_updater::UpdateSnapshot::empty(phase);
        s.version = "0.0.2".into();
        self.events
            .send(UpdateMessage { id, event: yoyo_updater::UpdateEvent::Snapshot(s) })
            .unwrap();
        self.runtime.poll();
    }
    fn event(&self, id: u64, event: yoyo_updater::UpdateEvent) {
        self.events.send(UpdateMessage { id, event }).unwrap();
        self.runtime.poll();
    }
}
#[test]
fn save_failure_blocks_exit_and_success_waits_for_installer_started() {
    let h = Harness::new();
    h.snapshot(0, UpdatePhase::ReadyToInstall);
    assert_eq!(h.runtime.state.borrow().window.get_phase_index(), 6);
    h.window().invoke_install_requested();
    let verify = h.requests.try_recv().unwrap();
    assert!(matches!(verify.command, UpdateCommand::VerifyForInstall));
    h.save_fails.set(true);
    h.event(verify.id, yoyo_updater::UpdateEvent::VerifiedForInstall);
    let abort = h.requests.try_recv().unwrap();
    assert!(matches!(abort.command, UpdateCommand::AbortInstall(_)));
    assert_eq!(h.exits.get(), 0);
    assert_eq!(h.saves.get(), 1);
    h.save_fails.set(false);
    h.window().invoke_install_requested();
    let verify = h.requests.try_recv().unwrap();
    h.event(verify.id, yoyo_updater::UpdateEvent::VerifiedForInstall);
    assert!(matches!(h.requests.try_recv().unwrap().command, UpdateCommand::LaunchInstaller));
    assert_eq!(h.exits.get(), 0);
    h.event(verify.id, yoyo_updater::UpdateEvent::InstallerStarted);
    assert_eq!(h.exits.get(), 1);
}
#[test]
fn closing_update_window_does_not_cancel_download_or_authorize_install() {
    let h = Harness::new();
    h.snapshot(0, UpdatePhase::Available);
    h.main.invoke_check_updates_requested();
    assert!(h.window().window().is_visible());
    h.window().invoke_download_requested();
    let request = h.requests.try_recv().unwrap();
    assert!(matches!(request.command, UpdateCommand::Download));
    h.window().invoke_later_requested();
    assert!(h.requests.try_recv().is_err());
    h.snapshot(request.id, UpdatePhase::ReadyToInstall);
    assert!(h.main.get_update_available());
    assert!(!h.runtime.state.borrow().window.window().is_visible());
    assert_eq!(h.saves.get(), 0);
    assert_eq!(h.exits.get(), 0);
}
#[test]
fn automatic_checks_are_delayed_throttled_and_can_be_disabled() {
    let h = Harness::new();
    h.snapshot(0, UpdatePhase::Idle);
    assert!(h.requests.try_recv().is_err());
    {
        let mut state = h.runtime.state.borrow_mut();
        state.started_at = Instant::now() - Duration::from_secs(11);
        state.next_auto_check = Instant::now();
    }
    h.runtime.poll();
    let check = h.requests.try_recv().unwrap();
    assert!(matches!(check.command, UpdateCommand::Check));
    h.snapshot(check.id, UpdatePhase::UpToDate);
    h.runtime.state.borrow_mut().next_auto_check = Instant::now();
    h.runtime.poll();
    assert!(h.requests.try_recv().is_err());
    h.window().invoke_automatic_check_changed(false);
    assert!(!h.runtime.state.borrow().preferences.automatic_check);
    let path = h.runtime.state.borrow().preferences_path.clone().unwrap();
    assert!(!UpdatePreferences::load(&path).unwrap().automatic_check);
}
