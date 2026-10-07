use yoyo_updater::{
    ControlEffect, UpdateCommand, UpdateControl, UpdateEvent, UpdateMessage, UpdatePhase,
    UpdateSnapshot,
};
fn message(id: u64, phase: UpdatePhase) -> UpdateMessage {
    UpdateMessage { id, event: UpdateEvent::Snapshot(UpdateSnapshot::empty(phase)) }
}
#[test]
fn stale_messages_and_duplicate_actions_cannot_regress_current_work() {
    let mut c = UpdateControl::default();
    let check = c.check().unwrap();
    assert!(c.check().is_none());
    c.receive(message(check.id, UpdatePhase::Available));
    let download = c.download().unwrap();
    assert!(c.download().is_none());
    c.receive(message(check.id, UpdatePhase::UpToDate));
    assert_eq!(c.snapshot().phase, UpdatePhase::Downloading);
    c.receive(message(download.id, UpdatePhase::ReadyToInstall));
    assert_eq!(c.snapshot().phase, UpdatePhase::ReadyToInstall);
}
#[test]
fn saving_state_is_required_before_an_installer_start_can_close_the_player() {
    let mut c = UpdateControl::default();
    c.receive(message(0, UpdatePhase::ReadyToInstall));
    assert_eq!(
        c.receive(UpdateMessage { id: 0, event: UpdateEvent::VerifiedForInstall }),
        ControlEffect::None
    );
    let request = c.install().unwrap();
    assert_eq!(
        c.receive(UpdateMessage { id: request.id, event: UpdateEvent::InstallerStarted }),
        ControlEffect::None
    );
    assert_eq!(
        c.receive(UpdateMessage { id: request.id, event: UpdateEvent::VerifiedForInstall }),
        ControlEffect::SavePlayback(request.id)
    );
    assert_eq!(
        c.receive(UpdateMessage { id: request.id, event: UpdateEvent::VerifiedForInstall }),
        ControlEffect::None
    );
    let launch = c.saved(request.id, Ok(())).unwrap();
    assert!(matches!(launch.command, UpdateCommand::LaunchInstaller));
    assert!(c.saved(request.id, Ok(())).is_none());
    assert_eq!(
        c.receive(UpdateMessage { id: request.id, event: UpdateEvent::InstallerStarted }),
        ControlEffect::Exit
    );
    assert!(c.check().is_none());
}
#[test]
fn save_failure_aborts_the_handshake_instead_of_closing() {
    let mut c = UpdateControl::default();
    c.receive(message(0, UpdatePhase::ReadyToInstall));
    let request = c.install().unwrap();
    c.receive(UpdateMessage { id: request.id, event: UpdateEvent::VerifiedForInstall });
    let abort = c.saved(request.id, Err("storage unavailable".into())).unwrap();
    assert!(matches!(abort.command, UpdateCommand::AbortInstall(_)));
    assert_eq!(c.snapshot().phase, UpdatePhase::ReadyToInstall);
    assert_eq!(
        c.receive(UpdateMessage { id: request.id, event: UpdateEvent::InstallerStarted }),
        ControlEffect::None
    );
}
#[test]
fn unsupported_and_disconnected_workers_do_not_trigger_installation() {
    let mut c = UpdateControl::default();
    c.receive(message(0, UpdatePhase::Unsupported));
    assert!(c.check().is_none());
    assert!(c.download().is_none());
    assert!(c.install().is_none());
    c.disconnected();
    assert_eq!(c.snapshot().phase, UpdatePhase::Error);
    assert_eq!(
        c.receive(UpdateMessage { id: 0, event: UpdateEvent::InstallerStarted }),
        ControlEffect::None
    );
}
