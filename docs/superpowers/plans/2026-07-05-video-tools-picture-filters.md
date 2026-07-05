# Video Tools Picture Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add screenshot capture, frame stepping, preset filters, and picture-parameter controls to YoYoVideo.

**Architecture:** Keep `yoyo-core` as the typed command/state boundary, keep mpv command/property/filter translation in `yoyo-mpv`, and keep screenshot path policy plus Slint wiring in `yoyovideo-desktop`. Screenshot shortcuts resolve to a desktop-level shortcut action first because only the desktop layer can create the target file path.

**Tech Stack:** Rust 2024, Slint 1.17.0, libmpv through `libmpv-sys`, `directories` 6.0, `chrono` 0.4.45 for screenshot timestamps, `tempfile` for path tests, PowerShell verification commands.

## Global Constraints

- Filters are preset-based for the first version.
- Screenshots are saved automatically; there is no per-shot save dialog in this phase.
- The default screenshot directory is `Pictures/YoYoVideo Screenshots` when the OS exposes a pictures directory.
- If the system pictures directory cannot be resolved, the desktop app falls back to an app-owned screenshots directory and reports the actual path in status text.
- Picture parameter changes are session-level playback controls, not saved settings.
- The UI is a popup, not a new settings page or sidebar tab.
- The popup sends typed app commands; Slint does not call mpv or mutate core state directly.
- Arbitrary user-authored mpv `vf` filter-chain editing is out of scope.
- Filter profile persistence and per-media picture-parameter persistence are out of scope.
- Default `cargo test` must pass without requiring libmpv runtime files.
- Runtime feature checks are `cargo check -p yoyo-mpv --features mpv-runtime` and `cargo check -p yoyovideo-desktop --features mpv-runtime`.

---

## File Structure

- `crates/yoyo-core/src/player_state.rs`: add video adjustment, filter preset, and frame-step types plus neutral defaults.
- `crates/yoyo-core/src/app_command.rs`: add user-facing screenshot, frame-step, adjustment, reset, and filter commands.
- `crates/yoyo-core/src/backend.rs`: add backend commands for mpv-bound screenshot, frame-step, adjustment, reset, and filter work.
- `crates/yoyo-core/src/session.rs`: clamp adjustment values, update state only after successful backend sends, and report screenshot success through `status_message`.
- `crates/yoyo-core/src/lib.rs`: export new video-tool types and adjustment constants.
- `crates/yoyo-core/tests/video_tools_contract.rs`: cover defaults, command forwarding, clamping, reset, and immediate backend failure behavior.
- `crates/yoyo-mpv/src/translate.rs`: map backend commands to mpv commands/properties/filter actions.
- `crates/yoyo-mpv/tests/translate_contract.rs`: cover screenshot, frame-step, picture properties, reset, and filter preset translations.
- `apps/yoyovideo-desktop/Cargo.toml`: add explicit `chrono = "0.4.45"` dependency.
- `apps/yoyovideo-desktop/src/platform/screenshot.rs`: resolve screenshot directories, create directories, generate timestamped filenames, and handle collisions.
- `apps/yoyovideo-desktop/src/platform/mod.rs`: export screenshot helpers.
- `apps/yoyovideo-desktop/tests/screenshot_paths_contract.rs`: cover timestamp naming, collision suffixes, and directory creation.
- `crates/yoyo-core/src/shortcut.rs`: add new shortcut actions and defaults.
- `apps/yoyovideo-desktop/src/app.rs`: add `ShortcutDispatch`, screenshot shortcut resolution, video-tool callbacks, and UI refresh.
- `apps/yoyovideo-desktop/src/presenter.rs`: add labels for video adjustments and filter presets.
- `apps/yoyovideo-desktop/src/lib.rs`: export new helper types and presenter functions.
- `apps/yoyovideo-desktop/ui/main-window.slint`: add `Video Tools` popup, properties, and callbacks.
- `apps/yoyovideo-desktop/tests/shortcut_contract.rs`: cover new shortcut dispatch behavior.
- `apps/yoyovideo-desktop/tests/keyboard_contract.rs`: cover comma, period, and `S` gesture normalization.
- `apps/yoyovideo-desktop/tests/presenter_contract.rs`: cover adjustment and preset labels.
- `apps/yoyovideo-desktop/tests/controller_contract.rs`: cover forwarding of new video-tool commands.
- `apps/yoyovideo-desktop/tests/video_tools_window_contract.rs`: compile-level Slint contract for the new popup surface.
- `docs/testing/manual-smoke-checklist.md`: add screenshot, frame-step, picture-parameter, and filter smoke checks.

Reference mpv command/property semantics against the official mpv manual: `https://mpv.io/manual/master/`.

---

### Task 1: Core Video Tool State And Commands

**Files:**
- Create: `crates/yoyo-core/tests/video_tools_contract.rs`
- Modify: `crates/yoyo-core/src/player_state.rs`
- Modify: `crates/yoyo-core/src/app_command.rs`
- Modify: `crates/yoyo-core/src/backend.rs`
- Modify: `crates/yoyo-core/src/session.rs`
- Modify: `crates/yoyo-core/src/lib.rs`

**Interfaces:**
- Produces: `pub const MIN_VIDEO_ADJUSTMENT: i16 = -100`
- Produces: `pub const MAX_VIDEO_ADJUSTMENT: i16 = 100`
- Produces: `pub enum VideoAdjustmentKind { Brightness, Contrast, Saturation, Gamma, Hue }`
- Produces: `pub struct VideoAdjustments { pub brightness: i16, pub contrast: i16, pub saturation: i16, pub gamma: i16, pub hue: i16 }`
- Produces: `VideoAdjustments::get(&self, kind: VideoAdjustmentKind) -> i16`
- Produces: `VideoAdjustments::set_clamped(&mut self, kind: VideoAdjustmentKind, value: i16) -> i16`
- Produces: `pub enum VideoFilterPreset { None, Sharpen, LightDenoise, Grayscale, Invert }`
- Produces: `pub enum FrameStepDirection { Previous, Next }`
- Produces: `AppCommand::{TakeScreenshot(PathBuf), StepFrame(FrameStepDirection), SetVideoAdjustment(VideoAdjustmentKind, i16), ResetVideoAdjustments, SetVideoFilterPreset(VideoFilterPreset)}`
- Produces: `BackendCommand::{TakeScreenshot(PathBuf), StepFrame(FrameStepDirection), SetVideoAdjustment(VideoAdjustmentKind, i16), ResetVideoAdjustments, SetVideoFilterPreset(VideoFilterPreset)}`
- Produces: `PlayerState.video_adjustments: VideoAdjustments`
- Produces: `PlayerState.video_filter_preset: VideoFilterPreset`

- [ ] **Step 1: Write the failing core tests**

Create `crates/yoyo-core/tests/video_tools_contract.rs`:

```rust
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
    session
        .handle_command(AppCommand::StepFrame(FrameStepDirection::Next))
        .unwrap();
    session
        .handle_command(AppCommand::StepFrame(FrameStepDirection::Previous))
        .unwrap();

    assert_eq!(
        session.backend().commands,
        vec![
            BackendCommand::TakeScreenshot(path.clone()),
            BackendCommand::StepFrame(FrameStepDirection::Next),
            BackendCommand::StepFrame(FrameStepDirection::Previous),
        ]
    );
    assert_eq!(
        session.state().status_message.as_deref(),
        Some("Screenshot saved: shot.png")
    );
}

#[test]
fn video_adjustment_values_are_clamped_before_state_and_backend_update() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());

    session
        .handle_command(AppCommand::SetVideoAdjustment(
            VideoAdjustmentKind::Brightness,
            140,
        ))
        .unwrap();
    session
        .handle_command(AppCommand::SetVideoAdjustment(
            VideoAdjustmentKind::Hue,
            -140,
        ))
        .unwrap();

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
        .handle_command(AppCommand::SetVideoAdjustment(
            VideoAdjustmentKind::Contrast,
            44,
        ))
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
```

- [ ] **Step 2: Run the failing core tests**

Run:

```powershell
cargo test -p yoyo-core --test video_tools_contract
```

Expected: FAIL with unresolved imports for the new video-tool types and commands.

- [ ] **Step 3: Add video-tool types to player state**

Modify `crates/yoyo-core/src/player_state.rs` by adding these definitions near the existing playback enums:

```rust
pub const MIN_VIDEO_ADJUSTMENT: i16 = -100;
pub const MAX_VIDEO_ADJUSTMENT: i16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoAdjustmentKind {
    Brightness,
    Contrast,
    Saturation,
    Gamma,
    Hue,
}

impl VideoAdjustmentKind {
    pub const ALL: [Self; 5] = [
        Self::Brightness,
        Self::Contrast,
        Self::Saturation,
        Self::Gamma,
        Self::Hue,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoAdjustments {
    pub brightness: i16,
    pub contrast: i16,
    pub saturation: i16,
    pub gamma: i16,
    pub hue: i16,
}

impl VideoAdjustments {
    pub fn get(&self, kind: VideoAdjustmentKind) -> i16 {
        match kind {
            VideoAdjustmentKind::Brightness => self.brightness,
            VideoAdjustmentKind::Contrast => self.contrast,
            VideoAdjustmentKind::Saturation => self.saturation,
            VideoAdjustmentKind::Gamma => self.gamma,
            VideoAdjustmentKind::Hue => self.hue,
        }
    }

    pub fn set_clamped(&mut self, kind: VideoAdjustmentKind, value: i16) -> i16 {
        let value = value.clamp(MIN_VIDEO_ADJUSTMENT, MAX_VIDEO_ADJUSTMENT);
        match kind {
            VideoAdjustmentKind::Brightness => self.brightness = value,
            VideoAdjustmentKind::Contrast => self.contrast = value,
            VideoAdjustmentKind::Saturation => self.saturation = value,
            VideoAdjustmentKind::Gamma => self.gamma = value,
            VideoAdjustmentKind::Hue => self.hue = value,
        }
        value
    }
}

impl Default for VideoAdjustments {
    fn default() -> Self {
        Self {
            brightness: 0,
            contrast: 0,
            saturation: 0,
            gamma: 0,
            hue: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFilterPreset {
    None,
    Sharpen,
    LightDenoise,
    Grayscale,
    Invert,
}

impl VideoFilterPreset {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Sharpen,
        Self::LightDenoise,
        Self::Grayscale,
        Self::Invert,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameStepDirection {
    Previous,
    Next,
}
```

Add fields to `PlayerState`:

```rust
pub video_adjustments: VideoAdjustments,
pub video_filter_preset: VideoFilterPreset,
```

Initialize the fields in `PlayerState::default()`:

```rust
video_adjustments: VideoAdjustments::default(),
video_filter_preset: VideoFilterPreset::None,
```

- [ ] **Step 4: Add app/backend commands and exports**

Modify `crates/yoyo-core/src/app_command.rs`:

```rust
use std::path::PathBuf;

use crate::{FrameStepDirection, VideoAdjustmentKind, VideoFilterPreset};

#[derive(Debug, Clone, PartialEq)]
pub enum AppCommand {
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
    OpenUrl(String),
    TogglePause,
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetSpeed(f32),
    ResetSpeed,
    SetVolume(u8),
    AdjustVolume(i8),
    CycleAudioChannel,
    RotateClockwise,
    ZoomIn,
    ZoomOut,
    SetABLoopPointA,
    SetABLoopPointB,
    ClearABLoop,
    ToggleFullscreen,
    NextItem,
    PreviousItem,
    SelectAudioTrack(i64),
    SelectSubtitleTrack(i64),
    SelectVideoTrack(i64),
    SetSubtitleVisible(bool),
    LoadExternalSubtitle(PathBuf),
    SetSubtitleDelay(f64),
    SetSubtitleScale(f32),
    SetSubtitleVerticalPosition(u8),
    TakeScreenshot(PathBuf),
    StepFrame(FrameStepDirection),
    SetVideoAdjustment(VideoAdjustmentKind, i16),
    ResetVideoAdjustments,
    SetVideoFilterPreset(VideoFilterPreset),
}
```

Modify `crates/yoyo-core/src/backend.rs`:

```rust
use std::path::PathBuf;

use crate::{
    AudioChannelMode, FrameStepDirection, MediaLocator, MediaTrack, Rotation,
    VideoAdjustmentKind, VideoFilterPreset,
};

#[derive(Debug, Clone, PartialEq)]
pub enum BackendCommand {
    SetPaused(bool),
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetSpeed(f32),
    SetVolume(u8),
    SetAudioChannel(AudioChannelMode),
    SetRotation(Rotation),
    AdjustZoom(i8),
    SetABLoopPointA(f64),
    SetABLoopPointB(f64),
    ClearABLoop,
    SelectAudioTrack(i64),
    SelectSubtitleTrack(i64),
    SelectVideoTrack(i64),
    SetSubtitleVisible(bool),
    LoadExternalSubtitle(PathBuf),
    SetSubtitleDelay(f64),
    SetSubtitleScale(f32),
    SetSubtitleVerticalPosition(u8),
    TakeScreenshot(PathBuf),
    StepFrame(FrameStepDirection),
    SetVideoAdjustment(VideoAdjustmentKind, i16),
    ResetVideoAdjustments,
    SetVideoFilterPreset(VideoFilterPreset),
}
```

Modify `crates/yoyo-core/src/lib.rs` exports:

```rust
pub use player_state::{
    AudioChannelMode, FrameStepDirection, LoopState, MediaTrack, MediaTrackKind, PlayerState,
    Rotation, SubtitlePlaybackState, VideoAdjustmentKind, VideoAdjustments, VideoFilterPreset,
    MAX_VIDEO_ADJUSTMENT, MIN_VIDEO_ADJUSTMENT,
};
```

- [ ] **Step 5: Handle the new commands in the session**

Modify `crates/yoyo-core/src/session.rs` by adding these match arms inside `handle_command`:

```rust
AppCommand::TakeScreenshot(path) => {
    self.backend
        .send(BackendCommand::TakeScreenshot(path.clone()))
        .map_err(AppError::Message)?;
    self.state.last_error = None;
    self.state.status_message = Some(format!("Screenshot saved: {}", path.display()));
}
AppCommand::StepFrame(direction) => {
    self.backend
        .send(BackendCommand::StepFrame(direction))
        .map_err(AppError::Message)?;
}
AppCommand::SetVideoAdjustment(kind, value) => {
    let clamped = value.clamp(crate::MIN_VIDEO_ADJUSTMENT, crate::MAX_VIDEO_ADJUSTMENT);
    self.backend
        .send(BackendCommand::SetVideoAdjustment(kind, clamped))
        .map_err(AppError::Message)?;
    self.state.video_adjustments.set_clamped(kind, clamped);
}
AppCommand::ResetVideoAdjustments => {
    self.backend
        .send(BackendCommand::ResetVideoAdjustments)
        .map_err(AppError::Message)?;
    self.state.video_adjustments = Default::default();
}
AppCommand::SetVideoFilterPreset(preset) => {
    self.backend
        .send(BackendCommand::SetVideoFilterPreset(preset))
        .map_err(AppError::Message)?;
    self.state.video_filter_preset = preset;
}
```

- [ ] **Step 6: Run core tests**

Run:

```powershell
cargo test -p yoyo-core --test video_tools_contract
cargo test -p yoyo-core --test command_contract
cargo test -p yoyo-core --test session_contract
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```powershell
git add crates/yoyo-core/src/player_state.rs crates/yoyo-core/src/app_command.rs crates/yoyo-core/src/backend.rs crates/yoyo-core/src/session.rs crates/yoyo-core/src/lib.rs crates/yoyo-core/tests/video_tools_contract.rs
git commit -m "feat: add core video tool commands"
```

Expected: Commit succeeds.

---

### Task 2: mpv Translation For Screenshots, Frame Step, Picture, And Filters

**Files:**
- Modify: `crates/yoyo-mpv/src/translate.rs`
- Modify: `crates/yoyo-mpv/tests/translate_contract.rs`

**Interfaces:**
- Consumes: `BackendCommand::TakeScreenshot(PathBuf)`
- Consumes: `BackendCommand::StepFrame(FrameStepDirection)`
- Consumes: `BackendCommand::SetVideoAdjustment(VideoAdjustmentKind, i16)`
- Consumes: `BackendCommand::ResetVideoAdjustments`
- Consumes: `BackendCommand::SetVideoFilterPreset(VideoFilterPreset)`
- Produces: mpv `screenshot-to-file` command
- Produces: mpv `frame-step` and `frame-back-step` commands
- Produces: mpv `brightness`, `contrast`, `saturation`, `gamma`, and `hue` property writes
- Produces: app-owned `vf` label `@yoyovideo-preset`

- [ ] **Step 1: Write failing mpv translation tests**

Append to `crates/yoyo-mpv/tests/translate_contract.rs`:

```rust
use yoyo_core::{FrameStepDirection, VideoAdjustmentKind, VideoFilterPreset};

#[test]
fn screenshot_translates_to_screenshot_to_file_command() {
    assert_eq!(
        translate_command(&BackendCommand::TakeScreenshot(PathBuf::from("shot.png"))),
        vec![MpvAction::Command(vec![
            "screenshot-to-file".into(),
            "shot.png".into(),
            "subtitles".into(),
        ])]
    );
}

#[test]
fn frame_step_translates_to_mpv_frame_commands() {
    assert_eq!(
        translate_command(&BackendCommand::StepFrame(FrameStepDirection::Next)),
        vec![MpvAction::Command(vec!["frame-step".into()])]
    );
    assert_eq!(
        translate_command(&BackendCommand::StepFrame(FrameStepDirection::Previous)),
        vec![MpvAction::Command(vec!["frame-back-step".into()])]
    );
}

#[test]
fn video_adjustments_translate_to_matching_mpv_properties() {
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(
            VideoAdjustmentKind::Brightness,
            12,
        )),
        vec![MpvAction::SetDouble { name: "brightness".into(), value: 12.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(
            VideoAdjustmentKind::Contrast,
            -7,
        )),
        vec![MpvAction::SetDouble { name: "contrast".into(), value: -7.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(
            VideoAdjustmentKind::Saturation,
            22,
        )),
        vec![MpvAction::SetDouble { name: "saturation".into(), value: 22.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Gamma, 5)),
        vec![MpvAction::SetDouble { name: "gamma".into(), value: 5.0 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoAdjustment(VideoAdjustmentKind::Hue, -9)),
        vec![MpvAction::SetDouble { name: "hue".into(), value: -9.0 }]
    );
}

#[test]
fn reset_video_adjustments_translates_to_neutral_properties() {
    assert_eq!(
        translate_command(&BackendCommand::ResetVideoAdjustments),
        vec![
            MpvAction::SetDouble { name: "brightness".into(), value: 0.0 },
            MpvAction::SetDouble { name: "contrast".into(), value: 0.0 },
            MpvAction::SetDouble { name: "saturation".into(), value: 0.0 },
            MpvAction::SetDouble { name: "gamma".into(), value: 0.0 },
            MpvAction::SetDouble { name: "hue".into(), value: 0.0 },
        ]
    );
}

#[test]
fn filter_presets_translate_to_yoyovideo_owned_vf_slot() {
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::None)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "remove".into(),
            "@yoyovideo-preset".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Sharpen)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[unsharp=5:5:0.6:3:3:0.3]".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::LightDenoise)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[hqdn3d=1.5:1.5:6:6]".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Grayscale)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[format=gray]".into(),
        ])]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetVideoFilterPreset(VideoFilterPreset::Invert)),
        vec![MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[negate]".into(),
        ])]
    );
}
```

- [ ] **Step 2: Run the failing translation tests**

Run:

```powershell
cargo test -p yoyo-mpv --test translate_contract
```

Expected: FAIL because `translate_command` does not handle the new backend command variants.

- [ ] **Step 3: Add translation helpers**

Modify `crates/yoyo-mpv/src/translate.rs` imports:

```rust
use yoyo_core::{
    AudioChannelMode, BackendCommand, FrameStepDirection, MediaLocator, Rotation,
    VideoAdjustmentKind, VideoFilterPreset,
};
```

Add helper functions above `translate_command`:

```rust
fn video_adjustment_property(kind: VideoAdjustmentKind) -> &'static str {
    match kind {
        VideoAdjustmentKind::Brightness => "brightness",
        VideoAdjustmentKind::Contrast => "contrast",
        VideoAdjustmentKind::Saturation => "saturation",
        VideoAdjustmentKind::Gamma => "gamma",
        VideoAdjustmentKind::Hue => "hue",
    }
}

fn filter_preset_action(preset: VideoFilterPreset) -> MpvAction {
    match preset {
        VideoFilterPreset::None => MpvAction::Command(vec![
            "vf".into(),
            "remove".into(),
            "@yoyovideo-preset".into(),
        ]),
        VideoFilterPreset::Sharpen => MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[unsharp=5:5:0.6:3:3:0.3]".into(),
        ]),
        VideoFilterPreset::LightDenoise => MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[hqdn3d=1.5:1.5:6:6]".into(),
        ]),
        VideoFilterPreset::Grayscale => MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[format=gray]".into(),
        ]),
        VideoFilterPreset::Invert => MpvAction::Command(vec![
            "vf".into(),
            "add".into(),
            "@yoyovideo-preset:lavfi=[negate]".into(),
        ]),
    }
}
```

- [ ] **Step 4: Translate the new commands**

Add these match arms to `translate_command` in `crates/yoyo-mpv/src/translate.rs`:

```rust
BackendCommand::TakeScreenshot(path) => vec![MpvAction::Command(vec![
    "screenshot-to-file".into(),
    path.display().to_string(),
    "subtitles".into(),
])],
BackendCommand::StepFrame(direction) => {
    let command = match direction {
        FrameStepDirection::Previous => "frame-back-step",
        FrameStepDirection::Next => "frame-step",
    };
    vec![MpvAction::Command(vec![command.into()])]
}
BackendCommand::SetVideoAdjustment(kind, value) => vec![MpvAction::SetDouble {
    name: video_adjustment_property(*kind).into(),
    value: f64::from(*value),
}],
BackendCommand::ResetVideoAdjustments => VideoAdjustmentKind::ALL
    .iter()
    .map(|kind| MpvAction::SetDouble {
        name: video_adjustment_property(*kind).into(),
        value: 0.0,
    })
    .collect(),
BackendCommand::SetVideoFilterPreset(preset) => vec![filter_preset_action(*preset)],
```

- [ ] **Step 5: Run mpv tests and runtime check**

Run:

```powershell
cargo test -p yoyo-mpv --test translate_contract
cargo test -p yoyo-mpv --test dry_run_contract
cargo check -p yoyo-mpv --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add crates/yoyo-mpv/src/translate.rs crates/yoyo-mpv/tests/translate_contract.rs
git commit -m "feat: translate video tool mpv commands"
```

Expected: Commit succeeds.

---

### Task 3: Screenshot Path Policy

**Files:**
- Modify: `apps/yoyovideo-desktop/Cargo.toml`
- Create: `apps/yoyovideo-desktop/src/platform/screenshot.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Create: `apps/yoyovideo-desktop/tests/screenshot_paths_contract.rs`

**Interfaces:**
- Produces: `default_screenshot_dir(paths: Option<&AppPaths>) -> PathBuf`
- Produces: `next_screenshot_path(dir: &Path, timestamp: &str) -> PathBuf`
- Produces: `screenshot_timestamp_now() -> String`
- Produces: `prepare_screenshot_path(paths: Option<&AppPaths>) -> Result<PathBuf, std::io::Error>`

- [ ] **Step 1: Write failing screenshot path tests**

Create `apps/yoyovideo-desktop/tests/screenshot_paths_contract.rs`:

```rust
use tempfile::tempdir;
use yoyovideo_desktop::platform::{next_screenshot_path, prepare_screenshot_path_in_dir};

#[test]
fn screenshot_path_uses_timestamped_png_name() {
    let dir = tempdir().unwrap();

    let path = next_screenshot_path(dir.path(), "20260705-211530");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("yoyovideo-20260705-211530.png")
    );
}

#[test]
fn screenshot_path_adds_suffix_when_file_exists() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("yoyovideo-20260705-211530.png"), b"existing").unwrap();
    std::fs::write(dir.path().join("yoyovideo-20260705-211530-1.png"), b"existing").unwrap();

    let path = next_screenshot_path(dir.path(), "20260705-211530");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("yoyovideo-20260705-211530-2.png")
    );
}

#[test]
fn prepare_screenshot_path_in_dir_creates_directory() {
    let dir = tempdir().unwrap().path().join("nested").join("screens");

    let path = prepare_screenshot_path_in_dir(&dir, "20260705-211530").unwrap();

    assert!(dir.is_dir());
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("yoyovideo-20260705-211530.png")
    );
}
```

- [ ] **Step 2: Run the failing screenshot path tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test screenshot_paths_contract
```

Expected: FAIL because the screenshot path helper module does not exist.

- [ ] **Step 3: Add explicit chrono dependency**

Modify `apps/yoyovideo-desktop/Cargo.toml`:

```toml
chrono = "0.4.45"
```

- [ ] **Step 4: Implement screenshot path helpers**

Create `apps/yoyovideo-desktop/src/platform/screenshot.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use directories::UserDirs;

use super::AppPaths;

const SCREENSHOT_DIR_NAME: &str = "YoYoVideo Screenshots";

pub fn default_screenshot_dir(paths: Option<&AppPaths>) -> PathBuf {
    if let Some(pictures) = UserDirs::new().and_then(|dirs| dirs.picture_dir().map(Path::to_path_buf))
    {
        return pictures.join(SCREENSHOT_DIR_NAME);
    }

    paths
        .map(|paths| paths.data_dir.join("screenshots"))
        .unwrap_or_else(|| PathBuf::from("screenshots"))
}

pub fn screenshot_timestamp_now() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn next_screenshot_path(dir: &Path, timestamp: &str) -> PathBuf {
    let first = dir.join(format!("yoyovideo-{timestamp}.png"));
    if !first.exists() {
        return first;
    }

    for suffix in 1..=9999 {
        let candidate = dir.join(format!("yoyovideo-{timestamp}-{suffix}.png"));
        if !candidate.exists() {
            return candidate;
        }
    }

    dir.join(format!("yoyovideo-{timestamp}-overflow.png"))
}

pub fn prepare_screenshot_path_in_dir(
    dir: &Path,
    timestamp: &str,
) -> Result<PathBuf, std::io::Error> {
    fs::create_dir_all(dir)?;
    Ok(next_screenshot_path(dir, timestamp))
}

pub fn prepare_screenshot_path(paths: Option<&AppPaths>) -> Result<PathBuf, std::io::Error> {
    let dir = default_screenshot_dir(paths);
    prepare_screenshot_path_in_dir(&dir, &screenshot_timestamp_now())
}
```

Modify `apps/yoyovideo-desktop/src/platform/mod.rs`:

```rust
mod dialogs;
mod media_scan;
mod paths;
mod screenshot;

pub use dialogs::{DialogService, RfdDialogService};
pub use media_scan::scan_media_folder;
pub use paths::AppPaths;
pub use screenshot::{
    default_screenshot_dir, next_screenshot_path, prepare_screenshot_path,
    prepare_screenshot_path_in_dir, screenshot_timestamp_now,
};
```

- [ ] **Step 5: Run screenshot path tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test screenshot_paths_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/Cargo.toml apps/yoyovideo-desktop/src/platform/screenshot.rs apps/yoyovideo-desktop/src/platform/mod.rs apps/yoyovideo-desktop/tests/screenshot_paths_contract.rs Cargo.lock
git commit -m "feat: add screenshot path policy"
```

Expected: Commit succeeds.

---

### Task 4: Shortcut Actions And Desktop Shortcut Resolution

**Files:**
- Modify: `crates/yoyo-core/src/shortcut.rs`
- Modify: `apps/yoyovideo-desktop/src/keyboard.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/tests/shortcut_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/keyboard_contract.rs`
- Modify: `crates/yoyo-core/tests/config_shortcut_contract.rs`

**Interfaces:**
- Produces: `ShortcutAction::{TakeScreenshot, FrameStepBackward, FrameStepForward}`
- Produces: default bindings `S`, `,`, and `.`
- Produces: `pub enum ShortcutDispatch { Command(AppCommand), TakeScreenshot }`
- Produces: `resolve_shortcut(map: &ShortcutMap, gesture: &str) -> Option<ShortcutDispatch>`
- Preserves: `dispatch_shortcut(map: &ShortcutMap, gesture: &str) -> Option<AppCommand>` for existing tests and callers.

- [ ] **Step 1: Write failing shortcut tests**

Append to `apps/yoyovideo-desktop/tests/shortcut_contract.rs`:

```rust
use yoyo_core::FrameStepDirection;
use yoyovideo_desktop::{resolve_shortcut, ShortcutDispatch};

#[test]
fn video_tool_shortcuts_resolve_to_expected_dispatches() {
    let map = ShortcutMap::default();

    assert_eq!(
        resolve_shortcut(&map, "S"),
        Some(ShortcutDispatch::TakeScreenshot)
    );
    assert_eq!(
        resolve_shortcut(&map, ","),
        Some(ShortcutDispatch::Command(AppCommand::StepFrame(
            FrameStepDirection::Previous
        )))
    );
    assert_eq!(
        resolve_shortcut(&map, "."),
        Some(ShortcutDispatch::Command(AppCommand::StepFrame(
            FrameStepDirection::Next
        )))
    );
}

#[test]
fn legacy_dispatch_shortcut_returns_none_for_screenshot_requiring_desktop_path() {
    let map = ShortcutMap::default();

    assert_eq!(dispatch_shortcut(&map, "S"), None);
}
```

Append to `apps/yoyovideo-desktop/tests/keyboard_contract.rs`:

```rust
#[test]
fn keyboard_input_normalizes_video_tool_shortcuts() {
    assert_eq!(shortcut_gesture(KeyboardInput::character('s')), Some("S".to_string()));
    assert_eq!(shortcut_gesture(KeyboardInput::character(',')), Some(",".to_string()));
    assert_eq!(shortcut_gesture(KeyboardInput::character('.')), Some(".".to_string()));
}
```

Append to `crates/yoyo-core/tests/config_shortcut_contract.rs`:

```rust
#[test]
fn video_tool_default_shortcuts_are_registered() {
    let map = ShortcutMap::default();

    assert_eq!(
        map.action_for(&Shortcut::parse("S").unwrap()),
        Some(ShortcutAction::TakeScreenshot)
    );
    assert_eq!(
        map.action_for(&Shortcut::parse(",").unwrap()),
        Some(ShortcutAction::FrameStepBackward)
    );
    assert_eq!(
        map.action_for(&Shortcut::parse(".").unwrap()),
        Some(ShortcutAction::FrameStepForward)
    );
}
```

- [ ] **Step 2: Run the failing shortcut tests**

Run:

```powershell
cargo test -p yoyo-core --test config_shortcut_contract
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
```

Expected: FAIL because new shortcut actions and `ShortcutDispatch` do not exist.

- [ ] **Step 3: Add shortcut actions and defaults**

Modify `crates/yoyo-core/src/shortcut.rs`:

```rust
pub enum ShortcutAction {
    TogglePause,
    SeekBackwardSmall,
    SeekForwardSmall,
    VolumeUp,
    VolumeDown,
    SpeedDown,
    SpeedUp,
    ResetSpeed,
    SetABLoopPointA,
    SetABLoopPointB,
    ClearABLoop,
    RotateClockwise,
    ZoomOut,
    ZoomIn,
    CycleAudioChannel,
    ToggleFullscreen,
    OpenFile,
    OpenUrl,
    TakeScreenshot,
    FrameStepBackward,
    FrameStepForward,
}
```

Update `ShortcutAction::all()`:

```rust
const ACTIONS: [ShortcutAction; 21] = [
    ShortcutAction::TogglePause,
    ShortcutAction::SeekBackwardSmall,
    ShortcutAction::SeekForwardSmall,
    ShortcutAction::VolumeUp,
    ShortcutAction::VolumeDown,
    ShortcutAction::SpeedDown,
    ShortcutAction::SpeedUp,
    ShortcutAction::ResetSpeed,
    ShortcutAction::SetABLoopPointA,
    ShortcutAction::SetABLoopPointB,
    ShortcutAction::ClearABLoop,
    ShortcutAction::RotateClockwise,
    ShortcutAction::ZoomOut,
    ShortcutAction::ZoomIn,
    ShortcutAction::CycleAudioChannel,
    ShortcutAction::ToggleFullscreen,
    ShortcutAction::OpenFile,
    ShortcutAction::OpenUrl,
    ShortcutAction::TakeScreenshot,
    ShortcutAction::FrameStepBackward,
    ShortcutAction::FrameStepForward,
];
```

Add labels:

```rust
ShortcutAction::TakeScreenshot => "Take Screenshot",
ShortcutAction::FrameStepBackward => "Previous Frame",
ShortcutAction::FrameStepForward => "Next Frame",
```

Add default bindings:

```rust
bindings.insert(Shortcut("S".into()), ShortcutAction::TakeScreenshot);
bindings.insert(Shortcut(",".into()), ShortcutAction::FrameStepBackward);
bindings.insert(Shortcut(".".into()), ShortcutAction::FrameStepForward);
```

- [ ] **Step 4: Add shortcut dispatch resolution**

Modify `apps/yoyovideo-desktop/src/app.rs`:

```rust
use yoyo_core::{
    AppCommand, AppConfig, AppSession, FrameStepDirection, HistoryStore, MediaLocator,
    PlayerBackend, PlayerState, ShortcutAction, ShortcutMap,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutDispatch {
    Command(AppCommand),
    TakeScreenshot,
}

pub fn resolve_shortcut(map: &ShortcutMap, gesture: &str) -> Option<ShortcutDispatch> {
    let shortcut = yoyo_core::Shortcut::parse(gesture).ok()?;
    match map.action_for(&shortcut)? {
        ShortcutAction::TogglePause => Some(ShortcutDispatch::Command(AppCommand::TogglePause)),
        ShortcutAction::SeekBackwardSmall => {
            Some(ShortcutDispatch::Command(AppCommand::SeekRelative(-5.0)))
        }
        ShortcutAction::SeekForwardSmall => {
            Some(ShortcutDispatch::Command(AppCommand::SeekRelative(5.0)))
        }
        ShortcutAction::VolumeUp => Some(ShortcutDispatch::Command(AppCommand::AdjustVolume(5))),
        ShortcutAction::VolumeDown => {
            Some(ShortcutDispatch::Command(AppCommand::AdjustVolume(-5)))
        }
        ShortcutAction::SpeedDown => Some(ShortcutDispatch::Command(AppCommand::SetSpeed(0.75))),
        ShortcutAction::SpeedUp => Some(ShortcutDispatch::Command(AppCommand::SetSpeed(1.25))),
        ShortcutAction::ResetSpeed => Some(ShortcutDispatch::Command(AppCommand::ResetSpeed)),
        ShortcutAction::SetABLoopPointA => {
            Some(ShortcutDispatch::Command(AppCommand::SetABLoopPointA))
        }
        ShortcutAction::SetABLoopPointB => {
            Some(ShortcutDispatch::Command(AppCommand::SetABLoopPointB))
        }
        ShortcutAction::ClearABLoop => Some(ShortcutDispatch::Command(AppCommand::ClearABLoop)),
        ShortcutAction::RotateClockwise => {
            Some(ShortcutDispatch::Command(AppCommand::RotateClockwise))
        }
        ShortcutAction::ZoomOut => Some(ShortcutDispatch::Command(AppCommand::ZoomOut)),
        ShortcutAction::ZoomIn => Some(ShortcutDispatch::Command(AppCommand::ZoomIn)),
        ShortcutAction::CycleAudioChannel => {
            Some(ShortcutDispatch::Command(AppCommand::CycleAudioChannel))
        }
        ShortcutAction::ToggleFullscreen => {
            Some(ShortcutDispatch::Command(AppCommand::ToggleFullscreen))
        }
        ShortcutAction::TakeScreenshot => Some(ShortcutDispatch::TakeScreenshot),
        ShortcutAction::FrameStepBackward => Some(ShortcutDispatch::Command(
            AppCommand::StepFrame(FrameStepDirection::Previous),
        )),
        ShortcutAction::FrameStepForward => Some(ShortcutDispatch::Command(
            AppCommand::StepFrame(FrameStepDirection::Next),
        )),
        ShortcutAction::OpenFile | ShortcutAction::OpenUrl => None,
    }
}

pub fn dispatch_shortcut(map: &ShortcutMap, gesture: &str) -> Option<AppCommand> {
    match resolve_shortcut(map, gesture)? {
        ShortcutDispatch::Command(command) => Some(command),
        ShortcutDispatch::TakeScreenshot => None,
    }
}
```

Add a controller helper:

```rust
impl<B: PlayerBackend> DesktopController<B> {
    pub fn resolve_shortcut(&self, gesture: &str) -> Option<ShortcutDispatch> {
        resolve_shortcut(&self.shortcuts, gesture)
    }

    pub fn dispatch_shortcut(&mut self, gesture: &str) -> Result<(), yoyo_core::AppError> {
        if let Some(ShortcutDispatch::Command(command)) = self.resolve_shortcut(gesture) {
            self.dispatch(command)?;
        }
        Ok(())
    }
}
```

Modify `apps/yoyovideo-desktop/src/lib.rs` exports:

```rust
pub use app::{
    DesktopController, MainWindow, SettingsWindow, ShortcutDispatch, TrackPopupRowData,
    build_desktop_backend, build_desktop_backend_with_video_window, dispatch_shortcut,
    refresh_window, resolve_shortcut, run,
};
```

- [ ] **Step 5: Run shortcut tests**

Run:

```powershell
cargo test -p yoyo-core --test config_shortcut_contract
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
cargo test -p yoyovideo-desktop --test settings_contract
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add crates/yoyo-core/src/shortcut.rs crates/yoyo-core/tests/config_shortcut_contract.rs apps/yoyovideo-desktop/src/keyboard.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/shortcut_contract.rs apps/yoyovideo-desktop/tests/keyboard_contract.rs
git commit -m "feat: add video tool shortcuts"
```

Expected: Commit succeeds.

---

### Task 5: Video Tools Popup, Presenter Labels, And Runtime Wiring

**Files:**
- Modify: `apps/yoyovideo-desktop/src/presenter.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/presenter_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`
- Create: `apps/yoyovideo-desktop/tests/video_tools_window_contract.rs`

**Interfaces:**
- Consumes: `prepare_screenshot_path(paths: Option<&AppPaths>) -> Result<PathBuf, std::io::Error>`
- Consumes: `ShortcutDispatch`
- Produces: `format_video_adjustment_label(kind: VideoAdjustmentKind, value: i16) -> String`
- Produces: `format_video_filter_preset_label(preset: VideoFilterPreset) -> &'static str`
- Produces Slint callbacks: `screenshot_requested()`, `frame_step_previous_requested()`, `frame_step_next_requested()`, `brightness_changed(int)`, `contrast_changed(int)`, `saturation_changed(int)`, `gamma_changed(int)`, `hue_changed(int)`, `reset_video_adjustments_requested()`, `filter_none_requested()`, `filter_sharpen_requested()`, `filter_light_denoise_requested()`, `filter_grayscale_requested()`, `filter_invert_requested()`

- [ ] **Step 1: Write failing presenter and controller tests**

Append to `apps/yoyovideo-desktop/tests/presenter_contract.rs`:

```rust
use yoyo_core::{VideoAdjustmentKind, VideoFilterPreset};
use yoyovideo_desktop::{format_video_adjustment_label, format_video_filter_preset_label};

#[test]
fn video_tool_presenter_formats_adjustments_and_presets() {
    assert_eq!(
        format_video_adjustment_label(VideoAdjustmentKind::Brightness, 12),
        "Brightness +12"
    );
    assert_eq!(
        format_video_adjustment_label(VideoAdjustmentKind::Hue, -9),
        "Hue -9"
    );
    assert_eq!(
        format_video_filter_preset_label(VideoFilterPreset::None),
        "Filter: None"
    );
    assert_eq!(
        format_video_filter_preset_label(VideoFilterPreset::LightDenoise),
        "Filter: Light Denoise"
    );
}
```

Append to `apps/yoyovideo-desktop/tests/controller_contract.rs`:

```rust
use yoyo_core::{FrameStepDirection, VideoAdjustmentKind, VideoFilterPreset};

#[test]
fn controller_forwards_video_tool_commands() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::TakeScreenshot(PathBuf::from("shot.png"))).unwrap();
    controller.dispatch(AppCommand::StepFrame(FrameStepDirection::Next)).unwrap();
    controller
        .dispatch(AppCommand::SetVideoAdjustment(VideoAdjustmentKind::Gamma, 20))
        .unwrap();
    controller.dispatch(AppCommand::ResetVideoAdjustments).unwrap();
    controller
        .dispatch(AppCommand::SetVideoFilterPreset(VideoFilterPreset::Invert))
        .unwrap();

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
```

Create `apps/yoyovideo-desktop/tests/video_tools_window_contract.rs`:

```rust
use yoyovideo_desktop::MainWindow;

#[test]
fn main_window_with_video_tools_surface_compiles() {
    let constructor: fn() -> Result<MainWindow, slint::PlatformError> = MainWindow::new;
    let _ = constructor;
}
```

- [ ] **Step 2: Run the failing desktop tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test presenter_contract
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test video_tools_window_contract
```

Expected: FAIL because presenter functions and Slint callback surface are missing.

- [ ] **Step 3: Add presenter labels and exports**

Modify `apps/yoyovideo-desktop/src/presenter.rs`:

```rust
use yoyo_core::{AudioChannelMode, PlayerState, Rotation, VideoAdjustmentKind, VideoFilterPreset};

pub fn format_video_adjustment_label(kind: VideoAdjustmentKind, value: i16) -> String {
    let name = match kind {
        VideoAdjustmentKind::Brightness => "Brightness",
        VideoAdjustmentKind::Contrast => "Contrast",
        VideoAdjustmentKind::Saturation => "Saturation",
        VideoAdjustmentKind::Gamma => "Gamma",
        VideoAdjustmentKind::Hue => "Hue",
    };
    format!("{name} {value:+}")
}

pub fn format_video_filter_preset_label(preset: VideoFilterPreset) -> &'static str {
    match preset {
        VideoFilterPreset::None => "Filter: None",
        VideoFilterPreset::Sharpen => "Filter: Sharpen",
        VideoFilterPreset::LightDenoise => "Filter: Light Denoise",
        VideoFilterPreset::Grayscale => "Filter: Grayscale",
        VideoFilterPreset::Invert => "Filter: Invert",
    }
}
```

Modify `apps/yoyovideo-desktop/src/lib.rs` presenter exports:

```rust
pub use presenter::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_speed_label,
    format_time_label, format_transport_label, format_video_adjustment_label,
    format_video_filter_preset_label, format_volume_label, format_zoom_label, progress_ratio,
};
```

- [ ] **Step 4: Add Slint properties, callbacks, and popup**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` by adding properties near existing playback properties:

```slint
in-out property <int> brightness_value: 0;
in-out property <int> contrast_value: 0;
in-out property <int> saturation_value: 0;
in-out property <int> gamma_value: 0;
in-out property <int> hue_value: 0;
in-out property <string> brightness_label: "Brightness +0";
in-out property <string> contrast_label: "Contrast +0";
in-out property <string> saturation_label: "Saturation +0";
in-out property <string> gamma_label: "Gamma +0";
in-out property <string> hue_label: "Hue +0";
in-out property <string> video_filter_label: "Filter: None";
```

Add callbacks near existing playback callbacks:

```slint
callback screenshot_requested();
callback frame_step_previous_requested();
callback frame_step_next_requested();
callback brightness_changed(int);
callback contrast_changed(int);
callback saturation_changed(int);
callback gamma_changed(int);
callback hue_changed(int);
callback reset_video_adjustments_requested();
callback filter_none_requested();
callback filter_sharpen_requested();
callback filter_light_denoise_requested();
callback filter_grayscale_requested();
callback filter_invert_requested();
```

Add a popup after `tracks_popup`:

```slint
video_tools_popup := PopupWindow {
    close-policy: close-on-click-outside;
    width: 340px;
    height: 620px;

    ScrollView {
        VerticalBox {
            padding: 12px;
            spacing: 8px;

            Text { text: "Capture"; color: #f2f5f7; }
            Button { text: "Screenshot"; clicked => { root.screenshot_requested(); } }

            Text { text: "Frame Step"; color: #f2f5f7; }
            HorizontalBox {
                spacing: 8px;
                Button { text: "Prev Frame"; clicked => { root.frame_step_previous_requested(); } }
                Button { text: "Next Frame"; clicked => { root.frame_step_next_requested(); } }
            }

            Text { text: "Picture"; color: #f2f5f7; }
            Text { text: root.brightness_label; color: #c7d1d8; }
            Slider { minimum: -100; maximum: 100; value: root.brightness_value; changed(value) => { root.brightness_changed(value); } }
            Text { text: root.contrast_label; color: #c7d1d8; }
            Slider { minimum: -100; maximum: 100; value: root.contrast_value; changed(value) => { root.contrast_changed(value); } }
            Text { text: root.saturation_label; color: #c7d1d8; }
            Slider { minimum: -100; maximum: 100; value: root.saturation_value; changed(value) => { root.saturation_changed(value); } }
            Text { text: root.gamma_label; color: #c7d1d8; }
            Slider { minimum: -100; maximum: 100; value: root.gamma_value; changed(value) => { root.gamma_changed(value); } }
            Text { text: root.hue_label; color: #c7d1d8; }
            Slider { minimum: -100; maximum: 100; value: root.hue_value; changed(value) => { root.hue_changed(value); } }
            Button { text: "Reset Picture"; clicked => { root.reset_video_adjustments_requested(); } }

            Text { text: root.video_filter_label; color: #f2f5f7; }
            Button { text: "None"; clicked => { root.filter_none_requested(); } }
            Button { text: "Sharpen"; clicked => { root.filter_sharpen_requested(); } }
            Button { text: "Light Denoise"; clicked => { root.filter_light_denoise_requested(); } }
            Button { text: "Grayscale"; clicked => { root.filter_grayscale_requested(); } }
            Button { text: "Invert"; clicked => { root.filter_invert_requested(); } }

            Text { text: root.status_label; color: #7d8790; }
        }
    }
}
```

Add a toolbar button near `Tracks`:

```slint
Button { text: "Video Tools"; clicked => { video_tools_popup.show(); } }
```

Add menu entries inside `menu_popup`:

```slint
Button { text: "Screenshot"; clicked => { root.screenshot_requested(); menu_popup.close(); } }
Button { text: "Video Tools"; clicked => { video_tools_popup.show(); menu_popup.close(); } }
```

- [ ] **Step 5: Refresh new UI state from `PlayerState`**

Modify `apps/yoyovideo-desktop/src/app.rs` imports:

```rust
use yoyo_core::{
    AppCommand, AppConfig, AppSession, FrameStepDirection, HistoryStore, MediaLocator,
    PlayerBackend, PlayerState, ShortcutAction, ShortcutMap, VideoAdjustmentKind,
    VideoFilterPreset,
};
```

Add to `refresh_window`:

```rust
window.set_brightness_value(i32::from(state.video_adjustments.brightness));
window.set_contrast_value(i32::from(state.video_adjustments.contrast));
window.set_saturation_value(i32::from(state.video_adjustments.saturation));
window.set_gamma_value(i32::from(state.video_adjustments.gamma));
window.set_hue_value(i32::from(state.video_adjustments.hue));
window.set_brightness_label(
    crate::format_video_adjustment_label(
        VideoAdjustmentKind::Brightness,
        state.video_adjustments.brightness,
    )
    .into(),
);
window.set_contrast_label(
    crate::format_video_adjustment_label(
        VideoAdjustmentKind::Contrast,
        state.video_adjustments.contrast,
    )
    .into(),
);
window.set_saturation_label(
    crate::format_video_adjustment_label(
        VideoAdjustmentKind::Saturation,
        state.video_adjustments.saturation,
    )
    .into(),
);
window.set_gamma_label(
    crate::format_video_adjustment_label(VideoAdjustmentKind::Gamma, state.video_adjustments.gamma)
        .into(),
);
window.set_hue_label(
    crate::format_video_adjustment_label(VideoAdjustmentKind::Hue, state.video_adjustments.hue)
        .into(),
);
window.set_video_filter_label(crate::format_video_filter_preset_label(state.video_filter_preset).into());
```

- [ ] **Step 6: Wire screenshot dispatch, shortcuts, sliders, and filter callbacks**

Add helper functions in `apps/yoyovideo-desktop/src/app.rs`:

```rust
fn dispatch_screenshot(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    paths: Option<AppPaths>,
) {
    let path = match crate::platform::prepare_screenshot_path(paths.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(format!("Screenshot path failed: {error}").into());
            }
            return;
        }
    };

    with_runtime_controller(app_handle, runtime, move |controller| {
        controller.dispatch(AppCommand::TakeScreenshot(path))
    });
}

fn dispatch_video_adjustment(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    kind: VideoAdjustmentKind,
    value: i32,
) {
    with_runtime_controller(app_handle, runtime, move |controller| {
        controller.dispatch(AppCommand::SetVideoAdjustment(kind, value.clamp(-100, 100) as i16))
    });
}
```

Replace keyboard shortcut dispatch in the winit handler path:

```rust
let action = {
    let runtime_ref = runtime.borrow();
    runtime_ref
        .controller()
        .and_then(|controller| controller.resolve_shortcut(gesture.as_str()))
};

match action {
    Some(ShortcutDispatch::Command(command)) => {
        with_runtime_controller(&app_handle, &runtime, move |controller| controller.dispatch(command));
    }
    Some(ShortcutDispatch::TakeScreenshot) => {
        dispatch_screenshot(&app_handle, &runtime, paths.clone());
    }
    None => {}
}
```

Wire UI callbacks in `run()`:

```rust
app.on_screenshot_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    let paths = paths.clone();
    move || dispatch_screenshot(&app_handle, &runtime, paths.clone())
});

app.on_frame_step_previous_requested(command_callback(
    &app,
    &runtime,
    AppCommand::StepFrame(FrameStepDirection::Previous),
));
app.on_frame_step_next_requested(command_callback(
    &app,
    &runtime,
    AppCommand::StepFrame(FrameStepDirection::Next),
));

app.on_brightness_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |value| dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Brightness, value)
});
app.on_contrast_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |value| dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Contrast, value)
});
app.on_saturation_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |value| dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Saturation, value)
});
app.on_gamma_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |value| dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Gamma, value)
});
app.on_hue_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |value| dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Hue, value)
});

app.on_reset_video_adjustments_requested(command_callback(
    &app,
    &runtime,
    AppCommand::ResetVideoAdjustments,
));
app.on_filter_none_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SetVideoFilterPreset(VideoFilterPreset::None),
));
app.on_filter_sharpen_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SetVideoFilterPreset(VideoFilterPreset::Sharpen),
));
app.on_filter_light_denoise_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SetVideoFilterPreset(VideoFilterPreset::LightDenoise),
));
app.on_filter_grayscale_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SetVideoFilterPreset(VideoFilterPreset::Grayscale),
));
app.on_filter_invert_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SetVideoFilterPreset(VideoFilterPreset::Invert),
));
```

- [ ] **Step 7: Run desktop tests and checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test presenter_contract
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
cargo test -p yoyovideo-desktop --test video_tools_window_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/presenter.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/presenter_contract.rs apps/yoyovideo-desktop/tests/controller_contract.rs apps/yoyovideo-desktop/tests/video_tools_window_contract.rs
git commit -m "feat: add video tools popup"
```

Expected: Commit succeeds.

---

### Task 6: Smoke Checklist And Final Verification

**Files:**
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Produces: manual smoke coverage for screenshot, frame-step, picture parameters, filters, and shortcut focus suppression.

- [ ] **Step 1: Add manual smoke coverage**

Append these lines under `## UX` in `docs/testing/manual-smoke-checklist.md`:

```markdown
- Open a local video, open `Video Tools`, click `Screenshot`, and confirm a `.png` file appears in `Pictures/YoYoVideo Screenshots` or the fallback screenshots directory shown by the app.
- Pause playback, use `Prev Frame` and `Next Frame`, and confirm the displayed frame changes one frame at a time.
- Use the default `S` shortcut and confirm it saves a screenshot through the same status path as the button.
- Use the default `,` and `.` shortcuts and confirm previous-frame and next-frame stepping work.
- Type `s,.` into the URL input and confirm screenshot and frame-step shortcuts do not fire while the input is focused.
- Move brightness, contrast, saturation, gamma, and hue sliders and confirm visible picture changes.
- Click `Reset Picture` and confirm brightness, contrast, saturation, gamma, and hue return to neutral.
- Select `Sharpen`, `Light Denoise`, `Grayscale`, and `Invert` filter presets and confirm each preset applies.
- Select `None` and confirm the YoYoVideo preset filter is removed.
```

- [ ] **Step 2: Run a documentation coverage check**

Run:

```powershell
$content = Get-Content -Raw docs/testing/manual-smoke-checklist.md
$required = @(
  "Video Tools",
  "Screenshot",
  "Pictures/YoYoVideo Screenshots",
  "Prev Frame",
  "Next Frame",
  "brightness, contrast, saturation, gamma, and hue",
  "Reset Picture",
  "Sharpen",
  "Light Denoise",
  "Grayscale",
  "Invert"
)
$missing = $required | Where-Object { $content -notmatch [regex]::Escape($_) }
if ($missing.Count -gt 0) {
  Write-Error ("Missing video tools checklist coverage: " + ($missing -join ", "))
  exit 1
}
```

Expected: PASS.

- [ ] **Step 3: Run full verification**

Run:

```powershell
cargo fmt --check
cargo test
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
git status --short
```

Expected:

- `cargo fmt --check`: PASS
- `cargo test`: PASS
- `cargo check -p yoyo-mpv --features mpv-runtime`: PASS in the same environment where existing runtime checks pass
- `cargo check -p yoyovideo-desktop --features mpv-runtime`: PASS in the same environment where existing runtime checks pass
- `git status --short`: only `docs/testing/manual-smoke-checklist.md` remains modified before the final commit

- [ ] **Step 4: Commit**

Run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add video tools smoke checks"
```

Expected: Commit succeeds.

---

## Self-Review

**Spec coverage:** The plan covers the `Video Tools` popup, automatic screenshot path generation under the system pictures directory with fallback, screenshot success/failure status, previous/next frame stepping, brightness/contrast/saturation/gamma/hue controls, reset-to-neutral behavior, preset filters, screenshot/frame-step shortcuts, URL-focus suppression, automated tests, runtime feature checks, and manual smoke coverage.

**Intentional scope exclusions:** The plan does not add arbitrary mpv filter-chain editing, filter profiles, per-media picture persistence, screenshot format selection, gallery management, region capture, GIF export, shaders, LUT import, HDR tone mapping, or plugin scripting.

**Placeholder scan:** No placeholder markers remain. Each task names exact files, concrete type/function names, test code, implementation snippets, verification commands, expected outcomes, and commit messages.

**Type consistency:** The same names flow through the plan: `VideoAdjustmentKind`, `VideoAdjustments`, `VideoFilterPreset`, `FrameStepDirection`, `ShortcutDispatch`, `resolve_shortcut`, `prepare_screenshot_path`, `format_video_adjustment_label`, and `format_video_filter_preset_label`. Screenshot path creation remains desktop-owned; core receives only an explicit `PathBuf`.
