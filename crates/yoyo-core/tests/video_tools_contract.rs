use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, FrameStepDirection, MediaLocator,
    PlayerBackend, PlayerState, VideoAdjustmentKind, VideoAdjustments, VideoFilterPreset,
};

#[derive(Default)]
struct MockBackend {
    commands: Vec<BackendCommand>,
    fail_next_send: bool,
}

impl PlayerBackend for MockBackend {
    fn open(&mut self, _locator: &MediaLocator) -> Result<(), String> {
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        if self.fail_next_send {
            self.fail_next_send = false;
            return Err("backend rejected command".into());
        }
        self.commands.push(command);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<yoyo_core::BackendEvent> {
        Vec::new()
    }
}

#[test]
fn default_video_tool_state_is_neutral() {
    let state = PlayerState::default();

    assert_eq!(state.video_adjustments, VideoAdjustments::default());
    assert_eq!(state.video_filter_preset, VideoFilterPreset::None);
    assert_eq!(state.video_adjustments.get(VideoAdjustmentKind::Brightness), 0);
    assert_eq!(state.video_adjustments.get(VideoAdjustmentKind::Contrast), 0);
    assert_eq!(state.video_adjustments.get(VideoAdjustmentKind::Saturation), 0);
    assert_eq!(state.video_adjustments.get(VideoAdjustmentKind::Gamma), 0);
    assert_eq!(state.video_adjustments.get(VideoAdjustmentKind::Hue), 0);
}

#[test]
fn screenshot_and_frame_step_forward_to_backend() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    let path = PathBuf::from("shot.png");

    session.handle_command(AppCommand::TakeScreenshot(path.clone())).unwrap();
    session.handle_command(AppCommand::StepFrame(FrameStepDirection::Next)).unwrap();
    session.handle_command(AppCommand::StepFrame(FrameStepDirection::Previous)).unwrap();

    assert_eq!(
        session.backend().commands,
        vec![
            BackendCommand::TakeScreenshot(path.clone()),
            BackendCommand::StepFrame(FrameStepDirection::Next),
            BackendCommand::StepFrame(FrameStepDirection::Previous),
        ]
    );
    assert_eq!(session.state().status_message.as_deref(), Some("Screenshot saved: shot.png"));
}

#[test]
fn video_adjustment_values_are_clamped_before_state_and_backend_update() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());

    session
        .handle_command(AppCommand::SetVideoAdjustment(VideoAdjustmentKind::Brightness, 140))
        .unwrap();
    session.handle_command(AppCommand::SetVideoAdjustment(VideoAdjustmentKind::Hue, -140)).unwrap();

    assert_eq!(session.state().video_adjustments.brightness, 100);
    assert_eq!(session.state().video_adjustments.hue, -100);
    assert_eq!(
        session.backend().commands,
        vec![
            BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Brightness, 100),
            BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Hue, -100),
        ]
    );
}

#[test]
fn reset_video_adjustments_restores_neutral_state() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());

    session
        .handle_command(AppCommand::SetVideoAdjustment(VideoAdjustmentKind::Contrast, 44))
        .unwrap();
    session.handle_command(AppCommand::ResetVideoAdjustments).unwrap();

    assert_eq!(session.state().video_adjustments, VideoAdjustments::default());
    assert_eq!(
        session.backend().commands,
        vec![
            BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Contrast, 44),
            BackendCommand::ResetVideoAdjustments,
        ]
    );
}

#[test]
fn video_pan_updates_state_and_backend_properties() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());

    session
        .handle_command(AppCommand::AdjustVideoPan { delta_x: 0.25, delta_y: -0.5 })
        .unwrap();
    session
        .handle_command(AppCommand::AdjustVideoPan { delta_x: 4.0, delta_y: -4.0 })
        .unwrap();

    assert_eq!(session.state().video_pan_x, 3.0);
    assert_eq!(session.state().video_pan_y, -3.0);
    assert_eq!(
        session.backend().commands,
        vec![
            BackendCommand::SetVideoPan { x: 0.25, y: -0.5 },
            BackendCommand::SetVideoPan { x: 3.0, y: -3.0 },
        ]
    );
}

#[test]
fn reset_video_pan_restores_center_position() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());

    session
        .handle_command(AppCommand::AdjustVideoPan { delta_x: 0.5, delta_y: 0.5 })
        .unwrap();
    session.handle_command(AppCommand::ResetVideoPan).unwrap();

    assert_eq!(session.state().video_pan_x, 0.0);
    assert_eq!(session.state().video_pan_y, 0.0);
    assert_eq!(
        session.backend().commands,
        vec![
            BackendCommand::SetVideoPan { x: 0.5, y: 0.5 },
            BackendCommand::SetVideoPan { x: 0.0, y: 0.0 },
        ]
    );
}

#[test]
fn filter_state_is_not_changed_when_backend_rejects_command() {
    let mut backend = MockBackend::default();
    backend.fail_next_send = true;
    let mut session = AppSession::new(AppConfig::default(), backend);

    let error = session
        .handle_command(AppCommand::SetVideoFilterPreset(VideoFilterPreset::Sharpen))
        .unwrap_err();

    assert!(error.to_string().contains("backend rejected command"));
    assert_eq!(session.state().video_filter_preset, VideoFilterPreset::None);
    assert!(session.backend().commands.is_empty());
}
