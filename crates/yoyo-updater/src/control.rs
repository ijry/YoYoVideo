use crate::{
    UpdateCommand, UpdateEvent, UpdateMessage, UpdatePhase, UpdateRequest, UpdateSnapshot,
};
#[derive(Debug, PartialEq, Eq)]
pub enum ControlEffect {
    None,
    SavePlayback(u64),
    Exit,
}
/// UI-thread state only: no file, network, process or Slint operations.
pub struct UpdateControl {
    state: UpdateSnapshot,
    id: u64,
    requested_install: bool,
    pending_save: Option<u64>,
    launch_sent: bool,
    exiting: bool,
}
impl Default for UpdateControl {
    fn default() -> Self {
        Self {
            state: UpdateSnapshot::empty(UpdatePhase::Idle),
            id: 0,
            requested_install: false,
            pending_save: None,
            launch_sent: false,
            exiting: false,
        }
    }
}
impl UpdateControl {
    pub fn snapshot(&self) -> &UpdateSnapshot {
        &self.state
    }
    pub fn check(&mut self) -> Option<UpdateRequest> {
        if self.exiting
            || matches!(
                self.state.phase,
                UpdatePhase::Unsupported
                    | UpdatePhase::Checking
                    | UpdatePhase::Downloading
                    | UpdatePhase::PreparingInstall
            )
        {
            return None;
        }
        let request = self.begin(UpdateCommand::Check, UpdatePhase::Checking)?;
        self.state.version.clear();
        self.state.notes.clear();
        Some(request)
    }
    pub fn download(&mut self) -> Option<UpdateRequest> {
        if self.exiting || self.state.phase != UpdatePhase::Available {
            return None;
        }
        self.begin(UpdateCommand::Download, UpdatePhase::Downloading)
    }
    pub fn install(&mut self) -> Option<UpdateRequest> {
        if self.exiting || self.state.phase != UpdatePhase::ReadyToInstall {
            return None;
        }
        let request = self.begin(UpdateCommand::VerifyForInstall, UpdatePhase::PreparingInstall)?;
        self.requested_install = true;
        Some(request)
    }
    pub fn receive(&mut self, message: UpdateMessage) -> ControlEffect {
        if self.exiting || message.id != self.id {
            return ControlEffect::None;
        }
        match message.event {
            UpdateEvent::Snapshot(snapshot) => {
                if snapshot.phase != UpdatePhase::PreparingInstall {
                    self.clear_authorization();
                }
                self.state = snapshot;
            }
            UpdateEvent::Progress(value) if self.state.phase == UpdatePhase::Downloading => {
                self.state.progress = value.clamp(0, 100);
            }
            UpdateEvent::VerifiedForInstall
                if self.requested_install
                    && self.pending_save.is_none()
                    && self.state.phase == UpdatePhase::PreparingInstall =>
            {
                self.pending_save = Some(message.id);
                return ControlEffect::SavePlayback(message.id);
            }
            UpdateEvent::InstallerStarted
                if self.launch_sent && self.state.phase == UpdatePhase::PreparingInstall =>
            {
                self.exiting = true;
                self.clear_authorization();
                return ControlEffect::Exit;
            }
            _ => {}
        }
        ControlEffect::None
    }
    pub fn saved(&mut self, id: u64, result: Result<(), String>) -> Option<UpdateRequest> {
        if self.exiting
            || id != self.id
            || self.pending_save != Some(id)
            || self.state.phase != UpdatePhase::PreparingInstall
        {
            return None;
        }
        self.pending_save = None;
        self.requested_install = false;
        let command = match result {
            Ok(()) => {
                self.launch_sent = true;
                UpdateCommand::LaunchInstaller
            }
            Err(reason) => {
                self.launch_sent = false;
                self.state.phase = UpdatePhase::ReadyToInstall;
                self.state.error = reason.clone();
                UpdateCommand::AbortInstall(reason)
            }
        };
        Some(UpdateRequest { id, command })
    }
    pub fn disconnected(&mut self) {
        self.clear_authorization();
        self.state.phase = UpdatePhase::Error;
        self.state.error = "Update worker stopped. Restart the application to retry.".into();
    }
    fn begin(&mut self, command: UpdateCommand, phase: UpdatePhase) -> Option<UpdateRequest> {
        self.id = self.id.checked_add(1)?;
        self.clear_authorization();
        self.state.phase = phase;
        self.state.progress = 0;
        self.state.error.clear();
        Some(UpdateRequest { id: self.id, command })
    }
    fn clear_authorization(&mut self) {
        self.requested_install = false;
        self.pending_save = None;
        self.launch_sent = false;
    }
}
