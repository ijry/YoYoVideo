use crate::{ServiceConfig, UpdateError, UpdatePhase, UpdateService, UpdateSnapshot};
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug)]
pub enum UpdateCommand {
    Check,
    Download,
    VerifyForInstall,
    LaunchInstaller,
    AbortInstall(String),
    Stop,
}
#[derive(Debug)]
pub struct UpdateRequest {
    pub id: u64,
    pub command: UpdateCommand,
}
#[derive(Debug)]
pub enum UpdateEvent {
    Snapshot(UpdateSnapshot),
    Progress(i16),
    VerifiedForInstall,
    InstallerStarted,
}
#[derive(Debug)]
pub struct UpdateMessage {
    pub id: u64,
    pub event: UpdateEvent,
}
pub struct UpdateWorker {
    pub requests: Sender<UpdateRequest>,
    pub events: Receiver<UpdateMessage>,
    pub(crate) thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for UpdateWorker {
    fn drop(&mut self) {
        let _ = self.requests.send(UpdateRequest { id: 0, command: UpdateCommand::Stop });
        // Never block the UI on an outstanding network request during shutdown.
        self.thread.take();
    }
}
pub fn spawn_worker(config: ServiceConfig) -> std::io::Result<UpdateWorker> {
    spawn_with_service(move || UpdateService::new(config))
}
pub(crate) fn spawn_with_service(
    make: impl FnOnce() -> Result<UpdateService, UpdateError> + Send + 'static,
) -> std::io::Result<UpdateWorker> {
    let (requests, rx) = mpsc::channel::<UpdateRequest>();
    let (tx, events) = mpsc::channel::<UpdateMessage>();
    let thread = std::thread::Builder::new()
        .name("yoyo-updater".into())
        .spawn(move || run_worker(make(), rx, tx))?;
    Ok(UpdateWorker { requests, events, thread: Some(thread) })
}
fn send(events: &Sender<UpdateMessage>, id: u64, event: UpdateEvent) {
    let _ = events.send(UpdateMessage { id, event });
}
fn run_worker(
    mut service: Result<UpdateService, UpdateError>,
    requests: Receiver<UpdateRequest>,
    events: Sender<UpdateMessage>,
) {
    let unavailable = service.as_ref().err().map(|error| {
        let mut snapshot = UpdateSnapshot::empty(if matches!(error, UpdateError::Unsupported) {
            UpdatePhase::Unsupported
        } else {
            UpdatePhase::Error
        });
        snapshot.error = error.to_string();
        snapshot
    });
    let initial = service
        .as_ref()
        .map(|s| s.snapshot())
        .unwrap_or_else(|_| unavailable.clone().expect("error snapshot"));
    send(&events, 0, UpdateEvent::Snapshot(initial));
    let mut last_id = 0;
    for request in requests {
        if matches!(request.command, UpdateCommand::Stop) {
            break;
        }
        if request.id < last_id {
            continue;
        }
        last_id = request.id;
        let Ok(service) = service.as_mut() else {
            send(
                &events,
                request.id,
                UpdateEvent::Snapshot(unavailable.clone().expect("error snapshot")),
            );
            continue;
        };
        let mut special = None;
        match request.command {
            UpdateCommand::Check => {
                send(
                    &events,
                    request.id,
                    UpdateEvent::Snapshot(UpdateSnapshot::empty(UpdatePhase::Checking)),
                );
                let _ = service.check();
            }
            UpdateCommand::Download => {
                let mut progress_state = service.snapshot();
                progress_state.phase = UpdatePhase::Downloading;
                progress_state.progress = 0;
                send(&events, request.id, UpdateEvent::Snapshot(progress_state.clone()));
                let (progress, progress_events) = mpsc::channel::<i16>();
                std::thread::scope(|scope| {
                    let events = &events;
                    scope.spawn(move || {
                        let mut last = -1;
                        for value in progress_events {
                            let value = value.clamp(0, 100);
                            if value <= last {
                                continue;
                            }
                            last = value;
                            send(events, request.id, UpdateEvent::Progress(value));
                        }
                    });
                    let _ = service.download(progress);
                }); // Join before ReadyToInstall, so late progress cannot regress the UI.
            }
            UpdateCommand::VerifyForInstall => {
                let mut state = service.snapshot();
                state.phase = UpdatePhase::PreparingInstall;
                send(&events, request.id, UpdateEvent::Snapshot(state));
                if service.verify_for_install().is_ok() {
                    special = Some(UpdateEvent::VerifiedForInstall);
                }
            }
            UpdateCommand::LaunchInstaller => {
                if service.launch_installer().is_ok() {
                    special = Some(UpdateEvent::InstallerStarted);
                }
            }
            UpdateCommand::AbortInstall(reason) => service.abort_install(reason),
            UpdateCommand::Stop => break,
        }
        send(&events, request.id, UpdateEvent::Snapshot(service.snapshot()));
        if let Some(event) = special {
            send(&events, request.id, event);
        }
    }
}
