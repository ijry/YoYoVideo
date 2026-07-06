# Compact Frameless Player UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a compact frameless Chinese-first YoYoVideo UI with categorized menus, working custom window controls, double-click fullscreen, and drag-to-pan video.

**Architecture:** Keep the existing Rust + Slint + libmpv split. `yoyo-core` owns durable playback state and new video pan commands, `yoyo-mpv` translates those commands to mpv properties, and `apps/yoyovideo-desktop` owns UI language, window control callbacks, and Slint layout. The Slint surface remains one generated `MainWindow` but gets tighter local components and callback boundaries.

**Tech Stack:** Rust workspace, Slint 1.17, winit 0.30 through Slint, libmpv command translation, cargo tests.

## Global Constraints

- Main window must be Slint frameless with `no-frame: true`.
- Default visible UI language is Chinese.
- English remains available through an in-session menu switch.
- Do not add a new UI framework or heavy dependency.
- Do not use emoji icons; use simple glyphs or Slint geometry.
- Bottom deck must show only core playback controls.
- Advanced features must remain available through categorized menus.
- Volume slider width must be fixed to 96-120 px.
- Existing playback, shortcuts, playlist/history, tracks, subtitles, screenshot, frame step, filters, markers, chapters, packaging, and runtime smoke behavior must continue to pass.
- Language persistence is intentionally out of scope for this implementation; startup always defaults to Chinese.

---

## File Structure

- Create `apps/yoyovideo-desktop/src/i18n.rs`: desktop UI language enum and Rust-side label formatting helpers.
- Modify `apps/yoyovideo-desktop/src/lib.rs`: export `UiLanguage` and language-aware presenter helpers needed by tests.
- Modify `apps/yoyovideo-desktop/src/presenter.rs`: keep existing public presenter functions as Chinese defaults, add `_for_language` variants.
- Modify `apps/yoyovideo-desktop/src/osd.rs`: keep existing OSD formatter as Chinese default, add language-aware OSD formatting.
- Modify `crates/yoyo-core/src/player_state.rs`: add video pan state to `PlayerState`.
- Modify `crates/yoyo-core/src/app_command.rs`: add `AdjustVideoPan` and `ResetVideoPan`.
- Modify `crates/yoyo-core/src/backend.rs`: add `SetVideoPan`.
- Modify `crates/yoyo-core/src/session.rs`: update pan state and send backend pan command.
- Modify `crates/yoyo-mpv/src/translate.rs`: translate pan commands to `video-pan-x` and `video-pan-y`.
- Modify `apps/yoyovideo-desktop/src/app.rs`: track UI language in runtime, connect window callbacks, connect video drag/double-click callbacks.
- Modify `apps/yoyovideo-desktop/ui/main-window.slint`: frameless compact UI, Chinese/English ternary labels, top-left menu, bottom core deck, video `TouchArea`.
- Modify tests:
  - `apps/yoyovideo-desktop/tests/presenter_contract.rs`
  - `apps/yoyovideo-desktop/tests/context_menu_contract.rs`
  - `crates/yoyo-core/tests/command_contract.rs`
  - `crates/yoyo-core/tests/video_tools_contract.rs`
  - `crates/yoyo-mpv/tests/translate_contract.rs`

---

### Task 1: Chinese-First Presenter And OSD Labels

**Files:**
- Create: `apps/yoyovideo-desktop/src/i18n.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/src/presenter.rs`
- Modify: `apps/yoyovideo-desktop/src/osd.rs`
- Test: `apps/yoyovideo-desktop/tests/presenter_contract.rs`

**Interfaces:**
- Produces: `UiLanguage`, `UiLanguage::parse(&str) -> UiLanguage`, `UiLanguage::code(self) -> &'static str`
- Produces: `format_transport_label_for_language(&PlayerState, UiLanguage) -> String`
- Produces: `format_volume_label_for_language(&PlayerState, UiLanguage) -> String`
- Produces: `format_osd_message_for_language(OsdKind, UiLanguage) -> String`
- Consumed by Task 3: `DesktopRuntime.ui_language`

- [ ] **Step 1: Add failing presenter tests for Chinese default and English alternate**

Replace the existing presenter expectations for text labels in `apps/yoyovideo-desktop/tests/presenter_contract.rs` with Chinese defaults, and add English alternate assertions:

```rust
use yoyo_core::{
    AudioChannelMode, LoopState, PlayerState, Rotation, VideoAdjustmentKind, VideoFilterPreset,
};
use yoyovideo_desktop::{
    UiLanguage, format_audio_channel_label, format_audio_channel_label_for_language,
    format_loop_label, format_osd_message, format_osd_message_for_language,
    format_rotation_label, format_speed_label, format_time_label, format_transport_label,
    format_transport_label_for_language, format_video_adjustment_label,
    format_video_adjustment_label_for_language, format_video_filter_preset_label,
    format_video_filter_preset_label_for_language, format_volume_label,
    format_volume_label_for_language, format_zoom_label, format_zoom_label_for_language,
    progress_ratio,
};

#[test]
fn transport_label_defaults_to_chinese_and_supports_english() {
    let playing = PlayerState { paused: false, ..PlayerState::default() };
    let paused = PlayerState { paused: true, ..PlayerState::default() };

    assert_eq!(format_transport_label(&playing), "暂停");
    assert_eq!(format_transport_label(&paused), "播放");
    assert_eq!(
        format_transport_label_for_language(&playing, UiLanguage::English),
        "Pause"
    );
}
```

Keep the existing clock, progress, parse, and progress tick tests. Update the combined label test to assert:

```rust
assert_eq!(format_volume_label(&state), "音量 73%");
assert_eq!(format_rotation_label(&state), "90°");
assert_eq!(format_audio_channel_label(&state), "左声道");
assert_eq!(format_zoom_label(&state), "缩放 +2");
assert_eq!(format_loop_label(&state), "A 00:12 / B 00:45");
assert_eq!(
    format_volume_label_for_language(&state, UiLanguage::English),
    "Vol 73%"
);
assert_eq!(
    format_audio_channel_label_for_language(&state, UiLanguage::English),
    "Mono L"
);
```

Update video label assertions:

```rust
assert_eq!(
    format_video_adjustment_label(VideoAdjustmentKind::Brightness, 12),
    "亮度 +12"
);
assert_eq!(
    format_video_adjustment_label_for_language(
        VideoAdjustmentKind::Brightness,
        12,
        UiLanguage::English,
    ),
    "Brightness +12"
);
assert_eq!(
    format_video_filter_preset_label(VideoFilterPreset::None),
    "滤镜: 无"
);
assert_eq!(
    format_video_filter_preset_label_for_language(
        VideoFilterPreset::LightDenoise,
        UiLanguage::English,
    ),
    "Filter: Light Denoise"
);
```

Update OSD assertions:

```rust
assert_eq!(
    format_osd_message(yoyovideo_desktop::OsdKind::Muted(true)),
    "已静音"
);
assert_eq!(
    format_osd_message_for_language(
        yoyovideo_desktop::OsdKind::JumpedTo(75.0),
        UiLanguage::English,
    ),
    "Jumped to 01:15"
);
```

- [ ] **Step 2: Run presenter test and verify it fails**

Run:

```powershell
cargo test -p yoyovideo-desktop --test presenter_contract
```

Expected: FAIL with unresolved `UiLanguage` and `_for_language` functions or old English label values.

- [ ] **Step 3: Add the UI language enum**

Create `apps/yoyovideo-desktop/src/i18n.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiLanguage {
    #[default]
    Chinese,
    English,
}

impl UiLanguage {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" => Self::English,
            _ => Self::Chinese,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Chinese => "zh",
            Self::English => "en",
        }
    }
}
```

- [ ] **Step 4: Wire `i18n` into exports**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
mod i18n;
```

Add exports:

```rust
pub use i18n::UiLanguage;
pub use osd::{OsdKind, OsdState, format_osd_message, format_osd_message_for_language};
pub use presenter::{
    format_audio_channel_label, format_audio_channel_label_for_language, format_loop_label,
    format_loop_label_for_language, format_rotation_label, format_rotation_label_for_language,
    format_speed_label, format_time_label, format_transport_label,
    format_transport_label_for_language, format_video_adjustment_label,
    format_video_adjustment_label_for_language, format_video_filter_preset_label,
    format_video_filter_preset_label_for_language, format_volume_label,
    format_volume_label_for_language, format_zoom_label, format_zoom_label_for_language,
    progress_ratio,
};
```

Remove the old duplicate `pub use osd` and `pub use presenter` blocks so each item is exported once.

- [ ] **Step 5: Implement language-aware presenter functions**

Modify `apps/yoyovideo-desktop/src/presenter.rs` so existing functions call Chinese defaults and `_for_language` variants accept `UiLanguage`:

```rust
use crate::UiLanguage;
use yoyo_core::{AudioChannelMode, PlayerState, Rotation, VideoAdjustmentKind, VideoFilterPreset};

pub fn format_transport_label(state: &PlayerState) -> String {
    format_transport_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_transport_label_for_language(state: &PlayerState, language: UiLanguage) -> String {
    match (language, state.paused) {
        (UiLanguage::Chinese, true) => "播放".into(),
        (UiLanguage::Chinese, false) => "暂停".into(),
        (UiLanguage::English, true) => "Play".into(),
        (UiLanguage::English, false) => "Pause".into(),
    }
}

pub fn format_volume_label(state: &PlayerState) -> String {
    format_volume_label_for_language(state, UiLanguage::Chinese)
}

pub fn format_volume_label_for_language(state: &PlayerState, language: UiLanguage) -> String {
    match language {
        UiLanguage::Chinese => format!("音量 {}%", state.volume_percent),
        UiLanguage::English => format!("Vol {}%", state.volume_percent),
    }
}
```

Apply the same pattern to rotation, audio channel, zoom, loop, video adjustment, and video filter labels. Use these mappings:

```rust
// Chinese labels
"0°", "90°", "180°", "270°"
"立体声", "左声道", "右声道"
"缩放 +{n}", "缩放 {n}", "缩放 0"
"亮度", "对比度", "饱和度", "伽马", "色调"
"滤镜: 无", "滤镜: 锐化", "滤镜: 轻降噪", "滤镜: 灰度", "滤镜: 反色"
```

Keep `format_speed_label` and `format_time_label` language-neutral.

- [ ] **Step 6: Implement language-aware OSD functions**

Modify `apps/yoyovideo-desktop/src/osd.rs`:

```rust
use crate::UiLanguage;

pub fn format_osd_message(kind: OsdKind) -> String {
    format_osd_message_for_language(kind, UiLanguage::Chinese)
}

pub fn format_osd_message_for_language(kind: OsdKind, language: UiLanguage) -> String {
    match (language, kind) {
        (UiLanguage::Chinese, OsdKind::Muted(true)) => "已静音".into(),
        (UiLanguage::Chinese, OsdKind::Muted(false)) => "声音开启".into(),
        (UiLanguage::Chinese, OsdKind::JumpedTo(seconds)) => {
            format!("跳转到 {}", fmt_clock(seconds))
        }
        (UiLanguage::Chinese, OsdKind::SeekedTo(seconds)) => format!("定位 {}", fmt_clock(seconds)),
        (UiLanguage::Chinese, OsdKind::Volume(volume)) => format!("音量 {volume}%"),
        (UiLanguage::Chinese, OsdKind::Speed(speed)) => format!("{speed:.2}x"),
        (UiLanguage::Chinese, OsdKind::MarkerAdded) => "已添加标记".into(),
        (UiLanguage::Chinese, OsdKind::MarkerRemoved) => "已移除标记".into(),
        (UiLanguage::Chinese, OsdKind::Chapter(title)) => title,
        (UiLanguage::Chinese, OsdKind::Screenshot(path)) => format!("截图已保存: {path}"),
        (UiLanguage::Chinese, OsdKind::Message(message)) => message,
        (UiLanguage::English, OsdKind::Muted(true)) => "Muted".into(),
        (UiLanguage::English, OsdKind::Muted(false)) => "Sound On".into(),
        (UiLanguage::English, OsdKind::JumpedTo(seconds)) => {
            format!("Jumped to {}", fmt_clock(seconds))
        }
        (UiLanguage::English, OsdKind::SeekedTo(seconds)) => format!("Seek {}", fmt_clock(seconds)),
        (UiLanguage::English, OsdKind::Volume(volume)) => format!("Volume {volume}%"),
        (UiLanguage::English, OsdKind::Speed(speed)) => format!("{speed:.2}x"),
        (UiLanguage::English, OsdKind::MarkerAdded) => "Marker added".into(),
        (UiLanguage::English, OsdKind::MarkerRemoved) => "Marker removed".into(),
        (UiLanguage::English, OsdKind::Chapter(title)) => title,
        (UiLanguage::English, OsdKind::Screenshot(path)) => format!("Screenshot saved: {path}"),
        (UiLanguage::English, OsdKind::Message(message)) => message,
    }
}
```

- [ ] **Step 7: Run presenter test and verify it passes**

Run:

```powershell
cargo test -p yoyovideo-desktop --test presenter_contract
```

Expected: PASS.

- [ ] **Step 8: Commit Task 1**

Run:

```powershell
git add apps/yoyovideo-desktop/src/i18n.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/src/presenter.rs apps/yoyovideo-desktop/src/osd.rs apps/yoyovideo-desktop/tests/presenter_contract.rs
git commit -m "feat: add chinese first ui labels"
```

---

### Task 2: Video Pan State And mpv Translation

**Files:**
- Modify: `crates/yoyo-core/src/player_state.rs`
- Modify: `crates/yoyo-core/src/app_command.rs`
- Modify: `crates/yoyo-core/src/backend.rs`
- Modify: `crates/yoyo-core/src/session.rs`
- Modify: `crates/yoyo-core/tests/command_contract.rs`
- Modify: `crates/yoyo-core/tests/video_tools_contract.rs`
- Modify: `crates/yoyo-mpv/src/translate.rs`
- Modify: `crates/yoyo-mpv/tests/translate_contract.rs`

**Interfaces:**
- Produces: `PlayerState.video_pan_x: f64`
- Produces: `PlayerState.video_pan_y: f64`
- Produces: `AppCommand::AdjustVideoPan { delta_x: f64, delta_y: f64 }`
- Produces: `AppCommand::ResetVideoPan`
- Produces: `BackendCommand::SetVideoPan { x: f64, y: f64 }`
- Consumed by Task 3: `AppCommand::AdjustVideoPan` from video drag callback.

- [ ] **Step 1: Add failing core tests for default pan, adjust pan, reset pan**

Modify `crates/yoyo-core/tests/command_contract.rs` default state test:

```rust
assert_eq!(state.video_pan_x, 0.0);
assert_eq!(state.video_pan_y, 0.0);
```

Add tests to `crates/yoyo-core/tests/video_tools_contract.rs`:

```rust
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
```

- [ ] **Step 2: Add failing mpv translation test**

Add to `crates/yoyo-mpv/tests/translate_contract.rs`:

```rust
#[test]
fn video_pan_translates_to_mpv_pan_properties() {
    assert_eq!(
        translate_command(&BackendCommand::SetVideoPan { x: 0.25, y: -0.5 }),
        vec![
            MpvAction::SetDouble { name: "video-pan-x".into(), value: 0.25 },
            MpvAction::SetDouble { name: "video-pan-y".into(), value: -0.5 },
        ]
    );
}
```

- [ ] **Step 3: Run targeted tests and verify they fail**

Run:

```powershell
cargo test -p yoyo-core --test command_contract
cargo test -p yoyo-core --test video_tools_contract
cargo test -p yoyo-mpv --test translate_contract
```

Expected: FAIL with missing fields and enum variants.

- [ ] **Step 4: Add pan fields to `PlayerState`**

Modify `crates/yoyo-core/src/player_state.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub current: Option<MediaLocator>,
    pub paused: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub volume_percent: u8,
    pub muted: bool,
    pub speed: f32,
    pub audio_channel: AudioChannelMode,
    pub rotation: Rotation,
    pub zoom_step: i8,
    #[serde(default)]
    pub video_pan_x: f64,
    #[serde(default)]
    pub video_pan_y: f64,
    pub loop_state: LoopState,
    // keep remaining existing fields unchanged
}
```

Add defaults:

```rust
video_pan_x: 0.0,
video_pan_y: 0.0,
```

- [ ] **Step 5: Add command variants**

Modify `crates/yoyo-core/src/app_command.rs`:

```rust
    ZoomIn,
    ZoomOut,
    AdjustVideoPan { delta_x: f64, delta_y: f64 },
    ResetVideoPan,
    SetABLoopPointA,
```

Modify `crates/yoyo-core/src/backend.rs`:

```rust
    AdjustZoom(i8),
    SetVideoPan { x: f64, y: f64 },
    SetABLoopPointA(f64),
```

- [ ] **Step 6: Implement session pan handling**

Modify `crates/yoyo-core/src/session.rs` near the zoom handling:

```rust
const MIN_VIDEO_PAN: f64 = -3.0;
const MAX_VIDEO_PAN: f64 = 3.0;
```

Add match arms after `ZoomOut`:

```rust
AppCommand::AdjustVideoPan { delta_x, delta_y } => {
    let next_x = (self.state.video_pan_x + delta_x).clamp(MIN_VIDEO_PAN, MAX_VIDEO_PAN);
    let next_y = (self.state.video_pan_y + delta_y).clamp(MIN_VIDEO_PAN, MAX_VIDEO_PAN);
    self.backend
        .send(BackendCommand::SetVideoPan { x: next_x, y: next_y })
        .map_err(AppError::Message)?;
    self.state.video_pan_x = next_x;
    self.state.video_pan_y = next_y;
}
AppCommand::ResetVideoPan => {
    self.backend
        .send(BackendCommand::SetVideoPan { x: 0.0, y: 0.0 })
        .map_err(AppError::Message)?;
    self.state.video_pan_x = 0.0;
    self.state.video_pan_y = 0.0;
}
```

- [ ] **Step 7: Translate pan commands to mpv**

Modify `crates/yoyo-mpv/src/translate.rs` after `AdjustZoom`:

```rust
BackendCommand::SetVideoPan { x, y } => vec![
    MpvAction::SetDouble { name: "video-pan-x".into(), value: *x },
    MpvAction::SetDouble { name: "video-pan-y".into(), value: *y },
],
```

- [ ] **Step 8: Run targeted tests and verify they pass**

Run:

```powershell
cargo test -p yoyo-core --test command_contract
cargo test -p yoyo-core --test video_tools_contract
cargo test -p yoyo-mpv --test translate_contract
```

Expected: PASS.

- [ ] **Step 9: Commit Task 2**

Run:

```powershell
git add crates/yoyo-core/src/player_state.rs crates/yoyo-core/src/app_command.rs crates/yoyo-core/src/backend.rs crates/yoyo-core/src/session.rs crates/yoyo-core/tests/command_contract.rs crates/yoyo-core/tests/video_tools_contract.rs crates/yoyo-mpv/src/translate.rs crates/yoyo-mpv/tests/translate_contract.rs
git commit -m "feat: add video pan command path"
```

---

### Task 3: Runtime Language, Window Controls, And Video Event Callbacks

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`

**Interfaces:**
- Consumes: `UiLanguage` from Task 1.
- Consumes: `AppCommand::AdjustVideoPan` and `AppCommand::ResetVideoPan` from Task 2.
- Produces Slint callbacks:
  - `window_drag_requested()`
  - `window_minimize_requested()`
  - `window_maximize_restore_requested()`
  - `window_close_requested()`
  - `language_changed(string)`
  - `video_double_clicked()`
  - `video_dragged(float, float)`
  - `reset_video_pan_requested()`
- Produces Slint property: `ui_language_code: string`

- [ ] **Step 1: Add failing Slint compile-contract callback checks**

Modify `apps/yoyovideo-desktop/tests/context_menu_contract.rs`:

```rust
window.set_ui_language_code("zh".into());
assert_eq!(window.get_ui_language_code().as_str(), "zh");
window.on_window_drag_requested(|| {});
window.on_window_minimize_requested(|| {});
window.on_window_maximize_restore_requested(|| {});
window.on_window_close_requested(|| {});
window.on_language_changed(|_| {});
window.on_video_double_clicked(|| {});
window.on_video_dragged(|_, _| {});
window.on_reset_video_pan_requested(|| {});
```

- [ ] **Step 2: Run compile-contract test and verify it fails**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
```

Expected: FAIL with missing generated Slint methods.

- [ ] **Step 3: Add Slint properties and placeholder callback declarations**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` inside `MainWindow` property/callback area:

```slint
in-out property <string> ui_language_code: "zh";

callback window_drag_requested();
callback window_minimize_requested();
callback window_maximize_restore_requested();
callback window_close_requested();
callback language_changed(string);
callback video_double_clicked();
callback video_dragged(float, float);
callback reset_video_pan_requested();
```

Do not restructure the UI in this task. The callbacks only need to compile; Task 4 wires them to the actual layout.

- [ ] **Step 4: Add runtime language state and language-aware refresh**

Modify `apps/yoyovideo-desktop/src/app.rs` imports to use `UiLanguage`.

Add to `DesktopRuntime`:

```rust
ui_language: crate::UiLanguage,
```

Initialize in `DesktopRuntime::new`:

```rust
ui_language: crate::UiLanguage::Chinese,
```

Change `refresh_window` to keep public Chinese default and add a runtime-aware helper:

```rust
pub fn refresh_window(window: &MainWindow, state: &PlayerState) {
    refresh_window_with_language(window, state, crate::UiLanguage::Chinese);
}

fn refresh_window_with_language(
    window: &MainWindow,
    state: &PlayerState,
    language: crate::UiLanguage,
) {
    window.set_ui_language_code(language.code().into());
    window.set_transport_label(crate::format_transport_label_for_language(state, language).into());
    window.set_speed_label(crate::format_speed_label(state).into());
    window.set_time_label(crate::format_time_label(state).into());
    window.set_volume_label(crate::format_volume_label_for_language(state, language).into());
    window.set_volume_value(i32::from(state.volume_percent));
    window.set_muted(state.muted);
    window.set_mute_label(
        match (language, state.muted) {
            (crate::UiLanguage::Chinese, true) => "静音",
            (crate::UiLanguage::Chinese, false) => "声音",
            (crate::UiLanguage::English, true) => "Muted",
            (crate::UiLanguage::English, false) => "Sound",
        }
        .into(),
    );
    window.set_rotation_label(crate::format_rotation_label_for_language(state, language).into());
    window
        .set_audio_channel_label(crate::format_audio_channel_label_for_language(state, language).into());
    window.set_zoom_label(crate::format_zoom_label_for_language(state, language).into());
    window.set_loop_label(crate::format_loop_label_for_language(state, language).into());
    window.set_progress_value(crate::progress_ratio(state));
    window.set_brightness_value(i32::from(state.video_adjustments.brightness));
    window.set_brightness_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Brightness,
            state.video_adjustments.brightness,
            language,
        )
        .into(),
    );
    window.set_contrast_value(i32::from(state.video_adjustments.contrast));
    window.set_contrast_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Contrast,
            state.video_adjustments.contrast,
            language,
        )
        .into(),
    );
    window.set_saturation_value(i32::from(state.video_adjustments.saturation));
    window.set_saturation_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Saturation,
            state.video_adjustments.saturation,
            language,
        )
        .into(),
    );
    window.set_gamma_value(i32::from(state.video_adjustments.gamma));
    window.set_gamma_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Gamma,
            state.video_adjustments.gamma,
            language,
        )
        .into(),
    );
    window.set_hue_value(i32::from(state.video_adjustments.hue));
    window.set_hue_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Hue,
            state.video_adjustments.hue,
            language,
        )
        .into(),
    );
    window.set_video_filter_label(
        crate::format_video_filter_preset_label_for_language(state.video_filter_preset, language)
            .into(),
    );
    window.set_status_label(state.status_message.clone().unwrap_or_default().into());
    refresh_navigation_surfaces(window, state);
}
```

Update `refresh_runtime_window`:

```rust
if let Some(controller) = runtime.controller() {
    refresh_window_with_language(window, controller.session().state(), runtime.ui_language);
} else {
    window.set_ui_language_code(runtime.ui_language.code().into());
    window.set_status_label(runtime.status_message().into());
    refresh_navigation_surfaces(window, &PlayerState::default());
}
```

- [ ] **Step 5: Use language-aware OSD**

Modify `set_osd` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
runtime.osd.message = crate::format_osd_message_for_language(kind, runtime.ui_language);
```

- [ ] **Step 6: Add window control callback handlers**

In `run()` after existing callback registration block starts, add:

```rust
app.on_window_drag_requested({
    let runtime = Rc::clone(&runtime);
    let app_handle = app.as_weak();
    move || {
        let Some(app) = app_handle.upgrade() else {
            return;
        };
        let result = app.window().with_winit_window(|winit_window| winit_window.drag_window());
        if let Some(Err(error)) = result {
            runtime
                .borrow_mut()
                .record_diagnostic("WARN", format!("Window drag failed: {error}"));
        }
    }
});

app.on_window_minimize_requested({
    let app_handle = app.as_weak();
    move || {
        if let Some(app) = app_handle.upgrade() {
            app.window().with_winit_window(|winit_window| winit_window.set_minimized(true));
        }
    }
});

app.on_window_maximize_restore_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move || {
        if let Some(app) = app_handle.upgrade() {
            app.window().set_maximized(!app.window().is_maximized());
            save_current_window_state(&runtime, app.window());
        }
    }
});

app.on_window_close_requested({
    let app_handle = app.as_weak();
    move || {
        if let Some(app) = app_handle.upgrade()
            && app.window().request_close()
        {
            let _ = app.hide();
        }
    }
});
```

- [ ] **Step 7: Add language change callback**

Add in `run()`:

```rust
app.on_language_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |language_code| {
        let language = crate::UiLanguage::parse(language_code.as_str());
        let Some(app) = app_handle.upgrade() else {
            return;
        };
        let mut runtime = runtime.borrow_mut();
        runtime.ui_language = language;
        if let Some(controller) = runtime.controller() {
            refresh_window_with_language(&app, controller.session().state(), language);
        } else {
            app.set_ui_language_code(language.code().into());
            app.set_status_label(runtime.status_message().into());
        }
    }
});
```

- [ ] **Step 8: Add video double-click and drag callbacks**

Add in `run()`:

```rust
app.on_video_double_clicked(command_callback(
    &app,
    &runtime,
    AppCommand::ToggleFullscreen,
));

app.on_video_dragged({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |delta_x, delta_y| {
        let Some(app) = app_handle.upgrade() else {
            return;
        };
        let width = (app.get_video_area_width() as f64).max(1.0);
        let height = (app.get_video_area_height() as f64).max(1.0);
        let pan_delta_x = f64::from(delta_x) / width;
        let pan_delta_y = f64::from(delta_y) / height;
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::AdjustVideoPan {
                delta_x: pan_delta_x,
                delta_y: pan_delta_y,
            })
        });
    }
});

app.on_reset_video_pan_requested(command_callback(
    &app,
    &runtime,
    AppCommand::ResetVideoPan,
));
```

- [ ] **Step 9: Run compile-contract test and verify it passes**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
```

Expected: PASS.

- [ ] **Step 10: Commit Task 3**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/context_menu_contract.rs
git commit -m "feat: wire frameless player runtime controls"
```

---

### Task 4: Compact Frameless Slint Layout

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`

**Interfaces:**
- Consumes callbacks and `ui_language_code` from Task 3.
- Preserves existing callbacks for open, recent, tracks, subtitles, picture tools, markers, progress, and sidebar.
- Produces centered `video_area` geometry with no inner host padding.

- [ ] **Step 1: Add a compile-contract assertion for default Chinese language**

Add to `main_window_context_menu_daily_actions_compile` in `apps/yoyovideo-desktop/tests/context_menu_contract.rs`:

```rust
assert_eq!(window.get_ui_language_code().as_str(), "zh");
```

- [ ] **Step 2: Run compile-contract test before layout work**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
```

Expected: PASS from Task 3.

- [ ] **Step 3: Set frameless window properties**

Modify the top of `MainWindow` in `apps/yoyovideo-desktop/ui/main-window.slint`:

```slint
export component MainWindow inherits Window {
    title: "YoYoVideo";
    width: 1200px;
    height: 760px;
    min-width: 860px;
    min-height: 520px;
    background: #000000;
    no-frame: true;
    resize-border-width: 6px;
```

- [ ] **Step 4: Add compact icon button components**

Add near existing component definitions:

```slint
component IconButton inherits Rectangle {
    in property <string> icon;
    in property <bool> selected: false;
    in property <color> accent: #d5dde8;
    callback clicked();
    width: 34px;
    height: 34px;
    border-radius: 6px;
    background: selected ? #172434 : (touch.has-hover ? #121923 : transparent);
    border-width: selected ? 1px : 0px;
    border-color: selected ? #38bdf8 : transparent;

    Text {
        text: root.icon;
        color: root.selected ? #f8fafc : root.accent;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 16px;
        font-weight: 800;
    }

    touch := TouchArea {
        clicked => { root.clicked(); }
    }
}

component MenuRow inherits Rectangle {
    in property <string> text;
    callback clicked();
    min-height: 32px;
    border-radius: 6px;
    background: touch.has-hover ? #121923 : transparent;

    Text {
        x: 10px;
        width: parent.width - 20px;
        text: root.text;
        color: #d5dde8;
        vertical-alignment: center;
        font-size: 12px;
        font-weight: 650;
    }

    touch := TouchArea {
        clicked => { root.clicked(); }
    }
}
```

- [ ] **Step 5: Rebuild the top-left categorized menu**

Replace `menu_popup` content with a Chinese/English-aware categorized popup:

```slint
menu_popup := PopupWindow {
    close-policy: close-on-click-outside;
    width: 320px;
    height: 620px;

    Rectangle {
        width: parent.width;
        height: parent.height;
        background: #05070bee;
        border-width: 1px;
        border-color: #263241;
    }

    ScrollView {
        VerticalBox {
            padding: 12px;
            spacing: 6px;

            DeckSectionTitle { text: root.ui_language_code == "zh" ? "文件" : "File"; }
            MenuRow { text: root.ui_language_code == "zh" ? "打开文件" : "Open File"; clicked => { root.open_file_requested(); menu_popup.close(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "打开文件夹" : "Open Folder"; clicked => { root.open_folder_requested(); menu_popup.close(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "打开链接" : "Open URL"; clicked => { jump_panel.show(); root.jump_panel_requested(); } }
            if root.recent_open_rows.length == 0: Text {
                text: root.ui_language_code == "zh" ? "暂无最近项目" : "No Recent Items";
                color: #64748b;
                font-size: 12px;
            }
            for row[index] in root.recent_open_rows: MenuRow {
                text: row.title;
                clicked => { root.recent_open_item_requested(index); menu_popup.close(); }
            }

            DeckDivider { }
            DeckSectionTitle { text: root.ui_language_code == "zh" ? "播放" : "Playback"; }
            MenuRow { text: root.ui_language_code == "zh" ? "跳转到时间" : "Jump To Time"; clicked => { jump_panel.show(); root.jump_panel_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "速度降低" : "Speed Down"; clicked => { root.speed_down_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "速度提高" : "Speed Up"; clicked => { root.speed_up_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "恢复 1x" : "Reset 1x"; clicked => { root.reset_speed_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "设置 A 点" : "Set A"; clicked => { root.set_ab_point_a_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "设置 B 点" : "Set B"; clicked => { root.set_ab_point_b_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "清除 AB 重复" : "Clear AB Loop"; clicked => { root.clear_ab_loop_requested(); } }

            DeckDivider { }
            DeckSectionTitle { text: root.ui_language_code == "zh" ? "音轨/字幕" : "Tracks"; }
            MenuRow { text: root.ui_language_code == "zh" ? "音轨和字幕" : "Audio and Subtitles"; clicked => { tracks_popup.show(); menu_popup.close(); } }

            DeckDivider { }
            DeckSectionTitle { text: root.ui_language_code == "zh" ? "画面" : "Picture"; }
            MenuRow { text: root.ui_language_code == "zh" ? "截图" : "Screenshot"; clicked => { root.screenshot_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "上一帧" : "Previous Frame"; clicked => { root.frame_step_previous_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "下一帧" : "Next Frame"; clicked => { root.frame_step_next_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "缩小" : "Zoom Out"; clicked => { root.zoom_out_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "放大" : "Zoom In"; clicked => { root.zoom_in_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "旋转" : "Rotate"; clicked => { root.rotate_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "重置画面位置" : "Reset Picture Position"; clicked => { root.reset_video_pan_requested(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "画面参数/滤镜" : "Adjustments / Filters"; clicked => { video_tools_popup.show(); menu_popup.close(); } }

            DeckDivider { }
            DeckSectionTitle { text: root.ui_language_code == "zh" ? "视图/设置" : "View / Settings"; }
            MenuRow { text: root.ui_language_code == "zh" ? "播放列表" : "Playlist"; clicked => { root.show_playlist_tab_requested(); menu_popup.close(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "历史记录" : "History"; clicked => { root.show_history_tab_requested(); menu_popup.close(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "全屏" : "Fullscreen"; clicked => { root.toggle_fullscreen_requested(); menu_popup.close(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "设置" : "Settings"; clicked => { root.settings_requested(); menu_popup.close(); } }
            MenuRow { text: root.ui_language_code == "zh" ? "English" : "中文"; clicked => { root.language_changed(root.ui_language_code == "zh" ? "en" : "zh"); } }
        }
    }
}
```

Keep `tracks_popup`, `video_tools_popup`, `action_panel`, and `jump_panel` available, but update their visible headers to use `ui_language_code` ternaries as part of this task.

- [ ] **Step 6: Replace native-looking top chrome**

Replace the current 44 px rounded top bar with:

```slint
Rectangle {
    height: 38px;
    background: #030405;

    HorizontalBox {
        padding-left: 8px;
        padding-right: 8px;
        spacing: 8px;

        IconButton { icon: "☰"; clicked => { menu_popup.show(); } }

        Text {
            text: "YoYoVideo";
            color: #f8fafc;
            font-size: 13px;
            font-weight: 850;
            vertical-alignment: center;
        }

        Text {
            text: status_label == "" ? (root.ui_language_code == "zh" ? "就绪" : "Ready") : status_label;
            color: #7b8794;
            font-size: 11px;
            vertical-alignment: center;
        }

        drag_space := Rectangle {
            TouchArea {
                clicked => { }
                moved => {
                    if self.pressed {
                        root.window_drag_requested();
                    }
                }
                double-clicked => { root.window_maximize_restore_requested(); }
            }
        }

        IconButton { icon: "—"; clicked => { root.window_minimize_requested(); } }
        IconButton { icon: "□"; clicked => { root.window_maximize_restore_requested(); } }
        IconButton { icon: "×"; accent: #fda4af; clicked => { root.window_close_requested(); } }
    }
}
```

If Slint rejects the `☰` character in this environment, replace it with `"≡"` before committing. Keep the close button as `"×"` only if the file already compiles; otherwise use `"x"`.

- [ ] **Step 7: Flatten and center the video stage**

Replace the current rounded `video_area` card internals with a true host rectangle:

```slint
video_area := Rectangle {
    background: #000000;
    min-height: 360px;

    TouchArea {
        width: parent.width;
        height: parent.height;
        double-clicked => { root.video_double_clicked(); }
        moved => {
            if self.pressed {
                root.video_dragged(
                    (self.mouse-x - self.pressed-x) / 1px,
                    (self.mouse-y - self.pressed-y) / 1px
                );
            }
        }
    }

    VerticalBox {
        x: (parent.width - 360px) / 2;
        y: (parent.height - 116px) / 2;
        width: 360px;
        height: 116px;
        spacing: 8px;

        Text {
            text: root.ui_language_code == "zh" ? "拖入视频或打开文件" : "Drop video here";
            color: #f8fafc;
            horizontal-alignment: center;
            font-size: 22px;
            font-weight: 850;
        }

        Text {
            text: root.ui_language_code == "zh" ? "支持播放、字幕、截图、逐帧、滤镜和快捷键" : "Open file, folder, URL, or use shortcuts";
            color: #64748b;
            horizontal-alignment: center;
            font-size: 12px;
        }
    }

    if root.osd_visible: Rectangle {
        width: 260px;
        height: 58px;
        border-radius: 8px;
        background: #020817e6;
        border-width: 1px;
        border-color: #38bdf866;
        x: (parent.width - self.width) / 2;
        y: (parent.height - self.height) / 2;

        Text {
            text: root.osd_message;
            color: #f8fafc;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-size: 16px;
            font-weight: 800;
        }
    }
}
```

This removes inner padding from the native video host bounds because `current_video_rect()` reads `video_area` directly.

- [ ] **Step 8: Collapse bottom deck to core controls**

Replace the current two-row bottom deck with:

```slint
Rectangle {
    height: 66px;
    background: #030405ee;
    border-width: 1px;
    border-color: #151d29;

    HorizontalBox {
        padding-left: 10px;
        padding-right: 10px;
        spacing: 8px;

        IconButton { icon: "⏮"; clicked => { root.previous_chapter_marker_requested(); } }
        IconButton {
            icon: root.transport_label == "播放" || root.transport_label == "Play" ? "▶" : "Ⅱ";
            selected: true;
            clicked => { root.toggle_pause_requested(); }
        }
        IconButton { icon: "⏭"; clicked => { root.next_chapter_marker_requested(); } }

        Text {
            text: root.time_label;
            color: #cbd5e1;
            font-size: 12px;
            font-weight: 700;
            vertical-alignment: center;
        }

        progress_rail := Rectangle {
            height: 34px;
            background: transparent;

            Rectangle {
                x: 0;
                y: 14px;
                width: parent.width;
                height: 6px;
                border-radius: 3px;
                background: #101722;
                border-width: 1px;
                border-color: #1f2c3a;
            }

            Rectangle {
                x: 0;
                y: 14px;
                width: root.progress_value * parent.width;
                height: 6px;
                border-radius: 3px;
                background: #38bdf8;
            }

            for tick in root.progress_tick_rows: Rectangle {
                x: tick.percent * parent.width - 1px;
                y: tick.is_marker ? 8px : 10px;
                width: tick.is_marker ? 4px : 2px;
                height: tick.is_marker ? 20px : 16px;
                border-radius: 2px;
                background: tick.is_marker ? #f59e0b : #7dd3fc;
            }

            if root.progress_preview_visible: Rectangle {
                x: root.progress_preview_value * parent.width - 44px;
                y: -20px;
                width: 88px;
                height: 24px;
                border-radius: 12px;
                background: #020817ee;
                border-width: 1px;
                border-color: #38bdf866;

                Text {
                    text: root.progress_preview_label;
                    color: #f8fafc;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                    font-size: 11px;
                    font-weight: 700;
                }
            }

            touch := TouchArea {
                moved => {
                    root.progress_preview_requested(self.mouse-x / progress_rail.width);
                }
                clicked => {
                    root.progress_commit_requested(self.mouse-x / progress_rail.width);
                }
            }
        }

        IconButton {
            icon: root.muted ? "M" : "V";
            selected: root.muted;
            clicked => { root.toggle_mute_requested(); }
        }
        Slider {
            width: 110px;
            minimum: 0;
            maximum: 100;
            value: volume_value;
            changed(value) => { root.volume_changed(value); }
        }
        IconButton { icon: "⛶"; clicked => { root.toggle_fullscreen_requested(); } }
    }
}
```

Copy the existing `progress_rail` implementation from the current first deck row into the indicated location. Keep `progress_preview_requested`, `progress_commit_requested`, ticks, and preview bubble unchanged except for colors if needed.

- [ ] **Step 9: Keep advanced panels reachable but not visible in bottom deck**

Remove these controls from the bottom deck only:

```slint
Open, Folder, url_input, Jump, Actions, Tracks, Picture, Mark, Prev, Next,
speed pill/buttons, zoom pill/buttons, Rot, Audio, audio_channel_label
```

Do not remove their callbacks or popups. They are reachable from `menu_popup`.

- [ ] **Step 10: Run Slint compile tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo test -p yoyovideo-desktop --test main_window_tracks_contract
cargo test -p yoyovideo-desktop --test video_tools_window_contract
```

Expected: PASS. If Slint fails on a glyph, replace that glyph with ASCII fallback listed in Step 6 or Step 8 and rerun.

- [ ] **Step 11: Commit Task 4**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/context_menu_contract.rs
git commit -m "feat: compact frameless player surface"
```

---

### Task 5: Full Verification And Test Install

**Files:**
- Modify only if verification exposes a defect in files touched by Tasks 1-4.
- Test/package scripts only.

**Interfaces:**
- Consumes all previous task outputs.
- Produces an installed local test build for user validation.

- [ ] **Step 1: Run formatting check**

Run:

```powershell
cargo fmt --check
```

Expected: PASS. If it fails, run:

```powershell
cargo fmt
```

Then rerun `cargo fmt --check`.

- [ ] **Step 2: Run full tests**

Run:

```powershell
cargo test
```

Expected: PASS.

- [ ] **Step 3: Run runtime feature checks**

Run:

```powershell
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 4: Run package smoke**

Run:

```powershell
pwsh -NoProfile -File scripts/test-package-smoke.ps1
```

Expected: PASS.

- [ ] **Step 5: Build release package with runtime required**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
```

Expected: PASS and release package artifacts under the existing package output directory.

- [ ] **Step 6: Run runtime smoke**

Run:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64 -TimeoutSeconds 8
```

Expected: output includes:

```text
runtime_smoke=ok
```

- [ ] **Step 7: Reinstall the test build if package scripts do not already update it**

If the prior packaging script did not update `C:\Users\Admin\AppData\Local\YoYoVideo-Test\bin\yoyovideo-desktop.exe`, use the existing installer/package flow from the repository scripts. Do not manually copy arbitrary files unless the package script output clearly identifies the release executable and existing test install layout.

- [ ] **Step 8: Manual smoke checklist**

Launch:

```powershell
Start-Process "$env:LOCALAPPDATA\\YoYoVideo-Test\\bin\\yoyovideo-desktop.exe"
```

Verify:

```text
1. No native OS title bar is visible.
2. Top black chrome has menu, minimize, maximize/restore, close.
3. Default UI text is Chinese.
4. Bottom deck only contains core playback controls.
5. Volume slider is compact and does not stretch.
6. Menu exposes file, playback, tracks, picture, view, settings/language entries.
7. Double-clicking the video stage toggles fullscreen.
8. Dragging top chrome moves the window.
9. Opening a video still plays through libmpv.
10. Zoom in, drag the video, and verify pan changes picture position.
```

- [ ] **Step 9: Commit verification fixes or leave tree clean**

If fixes were required:

```powershell
git add <fixed-files>
git commit -m "fix: stabilize compact player ui"
```

If no fixes were required:

```powershell
git status --short --branch
```

Expected: clean worktree.

---

## Self-Review

- Spec coverage: frameless window is Task 4; custom window controls are Task 3 and Task 4; Chinese default and English switch are Task 1, Task 3, and Task 4; compact bottom deck and fixed volume width are Task 4; categorized menu is Task 4; centered video bounds are Task 4; double-click fullscreen is Task 3 and Task 4; drag-to-pan is Task 2, Task 3, and Task 4; verification and install are Task 5.
- Placeholder scan: no unresolved placeholder markers or undefined task names remain.
- Type consistency: planned Rust variants are `AdjustVideoPan { delta_x: f64, delta_y: f64 }`, `ResetVideoPan`, and `SetVideoPan { x: f64, y: f64 }`; planned Slint callbacks use the exact generated callback names consumed by `app.rs`.
