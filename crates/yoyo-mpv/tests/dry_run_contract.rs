use yoyo_core::{BackendCommand, MediaLocator, PlayerBackend};
use yoyo_mpv::DryRunMpvBackend;

#[test]
fn dry_run_backend_records_open_and_pause_actions() {
    let mut backend = DryRunMpvBackend::default();

    backend.open(&MediaLocator::File("clip.mp4".into())).unwrap();
    backend.send(BackendCommand::SetPaused(true)).unwrap();

    assert_eq!(
        backend.recorded_actions(),
        &[
            "Command([\"loadfile\", \"clip.mp4\", \"replace\"])",
            "SetFlag { name: \"pause\", value: true }",
        ]
    );
}
