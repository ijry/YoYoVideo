use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, FrameStepDirection, MediaLocator,
    PlayerBackend, VideoAdjustmentKind, VideoFilterPreset,
};
use yoyovideo_desktop::DesktopController;

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
}

impl PlayerBackend for MockBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.opened.push(locator.clone());
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.commands.push(command);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<yoyo_core::BackendEvent> {
        Vec::new()
    }
}

#[test]
fn controller_forward_toggle_pause_to_session() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::TogglePause).unwrap();

    assert_eq!(controller.session().backend().commands, vec![BackendCommand::SetPaused(false)]);
}

#[test]
fn controller_open_url_updates_current_media() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::OpenUrl("https://example.com/live.m3u8".into())).unwrap();

    assert_eq!(
        controller.session().backend().opened,
        vec![MediaLocator::Url("https://example.com/live.m3u8".into())]
    );
    assert_eq!(
        controller.session().state().current,
        Some(MediaLocator::Url("https://example.com/live.m3u8".into()))
    );
}

#[test]
fn controller_can_open_a_specific_playlist_index() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session
        .replace_playlist(
            vec![
                yoyo_core::PlaylistEntry::new(MediaLocator::File("a.mp4".into())),
                yoyo_core::PlaylistEntry::new(MediaLocator::File("b.mp4".into())),
            ],
            0,
        )
        .unwrap();

    let mut controller = DesktopController::new(session);
    controller.open_playlist_index(1).unwrap();

    assert_eq!(
        controller.session().backend().opened.last(),
        Some(&MediaLocator::File("b.mp4".into()))
    );
    assert_eq!(controller.session().state().current, Some(MediaLocator::File("b.mp4".into())));
}

#[test]
fn controller_can_start_with_non_default_shortcuts() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut shortcuts = yoyo_core::ShortcutMap::default();
    shortcuts
        .set_binding(
            yoyo_core::ShortcutAction::TogglePause,
            Some(yoyo_core::Shortcut::parse("Ctrl+P").unwrap()),
        )
        .unwrap();

    let mut controller = DesktopController::with_shortcuts(session, shortcuts);
    controller.dispatch_shortcut("Ctrl+P").unwrap();

    assert_eq!(controller.session().backend().commands, vec![BackendCommand::SetPaused(false)]);
}

#[test]
fn controller_forwards_external_subtitle_and_visibility_commands() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller
        .dispatch(AppCommand::LoadExternalSubtitle(std::path::PathBuf::from("movie.ass")))
        .unwrap();
    controller.dispatch(AppCommand::SetSubtitleVisible(false)).unwrap();

    assert_eq!(
        controller.session().backend().commands,
        vec![
            BackendCommand::LoadExternalSubtitle(std::path::PathBuf::from("movie.ass")),
            BackendCommand::SetSubtitleVisible(false),
        ]
    );
}

#[test]
fn controller_forwards_video_tool_commands() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::TakeScreenshot(PathBuf::from("shot.png"))).unwrap();
    controller.dispatch(AppCommand::StepFrame(FrameStepDirection::Next)).unwrap();
    controller.dispatch(AppCommand::SetVideoAdjustment(VideoAdjustmentKind::Gamma, 20)).unwrap();
    controller.dispatch(AppCommand::ResetVideoAdjustments).unwrap();
    controller.dispatch(AppCommand::SetVideoFilterPreset(VideoFilterPreset::Invert)).unwrap();

    assert_eq!(
        controller.session().backend().commands,
        vec![
            BackendCommand::TakeScreenshot(PathBuf::from("shot.png")),
            BackendCommand::StepFrame(FrameStepDirection::Next),
            BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Gamma, 20),
            BackendCommand::ResetVideoAdjustments,
            BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Invert),
        ]
    );
}

#[test]
fn controller_can_open_multiple_playlist_entries_from_drop() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);
    let first = yoyo_core::PlaylistEntry::new(MediaLocator::File("first.mp4".into()));
    let second = yoyo_core::PlaylistEntry::new(MediaLocator::File("second.mp4".into()));

    controller.open_playlist_entries(vec![first.clone(), second.clone()]).unwrap();

    assert_eq!(
        controller.session().backend().opened,
        vec![MediaLocator::File("first.mp4".into())]
    );
    let snapshot = controller.session().playlist_snapshot();
    assert_eq!(snapshot.entries, vec![first, second]);
    assert_eq!(snapshot.current_index, Some(0));
}
