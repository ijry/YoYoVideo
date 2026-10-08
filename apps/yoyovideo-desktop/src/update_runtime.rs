use crate::{MainWindow, UpdateWindow, platform::AppPaths};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use yoyo_updater::{
    ControlEffect, Platform, ServiceConfig, UpdateCommand, UpdateControl, UpdateEvent,
    UpdateMessage, UpdatePhase, UpdatePreferences, UpdateRequest, UpdateSnapshot, UpdateWorker,
    spawn_worker,
};

#[cfg(not(feature = "updater-qa"))]
const PUBLIC_KEY: &str = include_str!("../assets/updater.pub");
const RELEASES: &str = "https://github.com/ijry/YoYoVideo/releases/latest";
fn configured_public_key() -> std::io::Result<String> {
    #[cfg(feature = "updater-qa")]
    {
        yoyo_updater::QaFixture::from_env()?.public_key()
    }
    #[cfg(not(feature = "updater-qa"))]
    {
        Ok(PUBLIC_KEY.into())
    }
}

type SaveState = Box<dyn FnMut() -> Result<(), String>>;
type ExitPlayer = Box<dyn FnMut()>;
trait Bridge {
    fn send(&self, request: UpdateRequest) -> bool;
    fn receive(&self) -> Result<UpdateMessage, TryRecvError>;
}
impl Bridge for UpdateWorker {
    fn send(&self, request: UpdateRequest) -> bool {
        self.requests.send(request).is_ok()
    }
    fn receive(&self) -> Result<UpdateMessage, TryRecvError> {
        self.events.try_recv()
    }
}
struct UnavailableBridge {
    state: UpdateSnapshot,
    queue: RefCell<VecDeque<UpdateMessage>>,
}
impl UnavailableBridge {
    fn new(phase: UpdatePhase, error: String) -> Self {
        let mut state = UpdateSnapshot::empty(phase);
        state.error = error;
        let queue = RefCell::new(VecDeque::from([UpdateMessage {
            id: 0,
            event: UpdateEvent::Snapshot(state.clone()),
        }]));
        Self { state, queue }
    }
}
impl Bridge for UnavailableBridge {
    fn send(&self, request: UpdateRequest) -> bool {
        if !matches!(request.command, UpdateCommand::Stop) {
            self.queue.borrow_mut().push_back(UpdateMessage {
                id: request.id,
                event: UpdateEvent::Snapshot(self.state.clone()),
            });
        }
        true
    }
    fn receive(&self) -> Result<UpdateMessage, TryRecvError> {
        self.queue.borrow_mut().pop_front().ok_or(TryRecvError::Empty)
    }
}
struct RuntimeState {
    main: slint::Weak<MainWindow>,
    window: UpdateWindow,
    bridge: Box<dyn Bridge>,
    control: UpdateControl,
    paths: Option<AppPaths>,
    started_at: Instant,
    next_auto_check: Instant,
    preferences: UpdatePreferences,
    preferences_path: Option<PathBuf>,
    local_error: Option<String>,
    save: SaveState,
    exit: ExitPlayer,
    exiting: bool,
    disconnected: bool,
}
pub(crate) struct UpdateRuntime {
    state: Rc<RefCell<RuntimeState>>,
    timer: slint::Timer,
}
impl UpdateRuntime {
    pub(crate) fn attach(
        main: &MainWindow,
        paths: Option<AppPaths>,
        save: impl FnMut() -> Result<(), String> + 'static,
        exit: impl FnMut() + 'static,
    ) -> Result<Self, slint::PlatformError> {
        let bridge: Box<dyn Bridge> =
            match (configured_public_key(), paths.as_ref(), Platform::current()) {
                (Ok(public_key), Some(paths), Some(platform)) => match spawn_worker(ServiceConfig {
                    platform,
                    public_key,
                    cache_dir: paths.cache_dir.join("updates"),
                }) {
                    Ok(worker) => Box::new(worker),
                    Err(error) => {
                        Box::new(UnavailableBridge::new(UpdatePhase::Error, error.to_string()))
                    }
                },
                _ => Box::new(UnavailableBridge::new(
                    UpdatePhase::Unsupported,
                    "Update directories or platform unavailable".into(),
                )),
            };
        Self::with_bridge(main, paths, bridge, Box::new(save), Box::new(exit))
    }
    fn with_bridge(
        main: &MainWindow,
        paths: Option<AppPaths>,
        bridge: Box<dyn Bridge>,
        save: SaveState,
        exit: ExitPlayer,
    ) -> Result<Self, slint::PlatformError> {
        let preferences_path = paths.as_ref().map(|p| p.config_dir.join("updater.toml"));
        let (preferences, local_error) =
            match preferences_path.as_ref().map(|p| UpdatePreferences::load(p)).transpose() {
                Ok(preferences) => (preferences.unwrap_or_default(), None),
                Err(error) => (
                    UpdatePreferences { automatic_check: false, last_checked_at: None },
                    Some(error.to_string()),
                ),
            };
        let window = UpdateWindow::new()?;
        window.set_current_version(env!("CARGO_PKG_VERSION").into());
        let now = Instant::now();
        let state = Rc::new(RefCell::new(RuntimeState {
            main: main.as_weak(),
            window: window.clone_strong(),
            bridge,
            control: UpdateControl::default(),
            paths,
            started_at: now,
            next_auto_check: now + Duration::from_secs(10),
            preferences,
            preferences_path,
            local_error,
            save,
            exit,
            exiting: false,
            disconnected: false,
        }));
        main.on_check_updates_requested({
            let state = Rc::downgrade(&state);
            move || {
                if let Some(state) = state.upgrade() {
                    state.borrow_mut().open();
                }
            }
        });
        window.on_check_requested({
            let state = Rc::downgrade(&state);
            move || {
                if let Some(state) = state.upgrade() {
                    state.borrow_mut().check(true);
                }
            }
        });
        window.on_download_requested({
            let state = Rc::downgrade(&state);
            move || {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    if let Some(request) = state.control.download() {
                        state.local_error = None;
                        state.send(request);
                    }
                    state.render();
                }
            }
        });
        window.on_install_requested({
            let state = Rc::downgrade(&state);
            move || {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    if let Some(request) = state.control.install() {
                        state.local_error = None;
                        state.send(request);
                    }
                    state.render();
                }
            }
        });
        window.on_later_requested({
            let state = Rc::downgrade(&state);
            move || {
                if let Some(state) = state.upgrade() {
                    let _ = state.borrow().window.hide();
                }
            }
        });
        window.window().on_close_requested(|| slint::CloseRequestResponse::HideWindow);
        window.on_automatic_check_changed({
            let state = Rc::downgrade(&state);
            move |enabled| {
                if let Some(state) = state.upgrade() {
                    state.borrow_mut().set_automatic(enabled);
                }
            }
        });
        window.on_open_releases_requested({
            let state = Rc::downgrade(&state);
            move || {
                if let Err(error) = open::that_detached(RELEASES) {
                    if let Some(state) = state.upgrade() {
                        let mut state = state.borrow_mut();
                        state.warn(&error.to_string());
                        state.local_error = Some(error.to_string());
                        state.render();
                    }
                }
            }
        });
        state.borrow().render();
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(200), {
            let state = Rc::downgrade(&state);
            move || {
                if let Some(state) = state.upgrade() {
                    state.borrow_mut().poll();
                }
            }
        });
        Ok(Self { state, timer })
    }
    #[cfg(feature = "updater-qa")]
    pub(crate) fn qa_window(&self) -> UpdateWindow {
        self.state.borrow().window.clone_strong()
    }
    #[cfg(test)]
    fn poll(&self) {
        self.state.borrow_mut().poll();
    }
}
impl Drop for UpdateRuntime {
    fn drop(&mut self) {
        self.timer.stop();
        if let Ok(state) = self.state.try_borrow() {
            let _ = state.bridge.send(UpdateRequest { id: 0, command: UpdateCommand::Stop });
        }
    }
}
impl RuntimeState {
    fn warn(&self, message: &str) {
        let _ = crate::platform::append_diagnostic(self.paths.as_ref(), "WARN", message);
    }
    fn send(&mut self, request: UpdateRequest) {
        if !self.bridge.send(request) {
            self.control.disconnected();
            self.disconnected = true;
            self.warn("Update worker stopped");
        }
    }
    fn open(&mut self) {
        self.render();
        if let Err(error) = self.window.show() {
            self.warn(&error.to_string());
        }
        if matches!(
            self.control.snapshot().phase,
            UpdatePhase::Idle | UpdatePhase::UpToDate | UpdatePhase::Error
        ) {
            self.check(true);
        }
    }
    fn check(&mut self, manual: bool) {
        let Some(request) = self.control.check() else {
            return;
        };
        self.local_error = None;
        self.preferences.last_checked_at = Some(unix_now());
        if let Some(path) = &self.preferences_path {
            if let Err(error) = self.preferences.save(path) {
                self.warn(&error.to_string());
                self.preferences.automatic_check = false;
                self.local_error = Some(error.to_string());
                if !manual {
                    let mut error_state = UpdateSnapshot::empty(UpdatePhase::Error);
                    error_state.error = error.to_string();
                    self.control.receive(UpdateMessage {
                        id: request.id,
                        event: UpdateEvent::Snapshot(error_state),
                    });
                    self.render();
                    return;
                }
            }
        }
        self.send(request);
        self.render();
    }
    fn set_automatic(&mut self, enabled: bool) {
        let previous = self.preferences.clone();
        self.preferences.automatic_check = enabled;
        if let Some(path) = &self.preferences_path {
            if let Err(error) = self.preferences.save(path) {
                self.preferences = previous;
                self.local_error = Some(error.to_string());
                self.warn(&error.to_string());
                self.render();
                return;
            }
        }
        self.local_error = None;
        self.next_auto_check = Instant::now();
        self.render();
    }
    fn poll(&mut self) {
        if self.exiting {
            return;
        }
        let mut changed = false;
        if !self.disconnected {
            loop {
                match self.bridge.receive() {
                    Ok(message) => {
                        if let UpdateEvent::Snapshot(snapshot) = &message.event {
                            if snapshot.phase == UpdatePhase::Error {
                                self.warn(&snapshot.error);
                            }
                        }
                        let effect = self.control.receive(message);
                        changed = true;
                        match effect {
                            ControlEffect::None => {}
                            ControlEffect::SavePlayback(id) => {
                                let result = (self.save)();
                                if let Err(error) = &result {
                                    self.warn(error);
                                }
                                if let Some(request) = self.control.saved(id, result) {
                                    self.send(request);
                                }
                            }
                            ControlEffect::Exit => {
                                self.exiting = true;
                                let _ = self.window.hide();
                                (self.exit)();
                                return;
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.disconnected = true;
                        self.control.disconnected();
                        self.warn("Update worker disconnected");
                        changed = true;
                        break;
                    }
                }
            }
        }
        let now = Instant::now();
        if !self.disconnected
            && now.duration_since(self.started_at) >= Duration::from_secs(10)
            && now >= self.next_auto_check
        {
            self.next_auto_check = now + Duration::from_secs(60);
            if matches!(
                self.control.snapshot().phase,
                UpdatePhase::Idle | UpdatePhase::UpToDate | UpdatePhase::Error
            ) && self.preferences.should_check(unix_now(), false)
            {
                self.check(false);
                changed = true;
            }
        }
        if let Some(main) = self.main.upgrade() {
            if self.window.get_ui_language_code() != main.get_ui_language_code() {
                changed = true;
            }
        }
        if changed {
            self.render();
        }
    }
    fn render(&self) {
        let state = self.control.snapshot();
        if let Some(main) = self.main.upgrade() {
            self.window.set_ui_language_code(main.get_ui_language_code());
            main.set_update_available(matches!(
                state.phase,
                UpdatePhase::Available
                    | UpdatePhase::Downloading
                    | UpdatePhase::ReadyToInstall
                    | UpdatePhase::PreparingInstall
            ));
        }
        self.window.set_phase_index(state.phase as i32);
        self.window.set_available_version(state.version.clone().into());
        if self.window.get_release_notes().as_str() != state.notes {
            self.window.set_release_notes(state.notes.clone().into());
        }
        self.window.set_progress_value(state.progress as i32);
        self.window.set_status_message(self.local_error.as_deref().unwrap_or(&state.error).into());
        self.window.set_automatic_check(self.preferences.automatic_check);
    }
}
fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().min(i64::MAX as u64)
        as i64
}
#[cfg(test)]
mod tests;
