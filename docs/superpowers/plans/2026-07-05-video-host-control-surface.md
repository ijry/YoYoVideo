# Video Host Control Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make YoYoVideo visibly playable on supported native windowing backends and complete the primary control/shortcut surface.

**Architecture:** Keep `yoyo-core` as the command/state boundary, add mpv video-window initialization options in `yoyo-mpv`, and keep all Slint/winit/native-window work in `yoyovideo-desktop`. The implementation is staged so pure tests pass without libmpv, while runtime feature checks compile the real mpv path when development libraries are available.

**Tech Stack:** Rust 2024, Slint 1.17.0 with winit 0.30 integration, raw-window-handle 0.6, libmpv via `libmpv-sys`, PowerShell verification commands, GitHub Actions packaging foundation.

## Global Constraints

- Use the chosen native video host approach, not CPU frame-copy rendering and not deep Slint renderer composition through `mpv_render_context`.
- `yoyo-mpv` must not depend on Slint or winit.
- `yoyovideo-desktop` owns platform video host creation and keyboard event routing.
- Default `cargo test` must continue to pass without libmpv runtime files.
- Runtime feature checks are `cargo check -p yoyo-mpv --features mpv-runtime` and `cargo check -p yoyovideo-desktop --features mpv-runtime`.
- mpv must receive a native video window id before `mpv_initialize` when video embedding is available.
- If video host creation is unsupported, the app must stay open and show a clear status message instead of pretending video embedding works.
- Keyboard shortcuts must route through the same `DesktopController` command path as UI controls.
- Keyboard shortcuts must not fire while the URL input is focused.
- This phase does not implement playlist panel UI, history UI, complete settings UI, subtitle controls, signed installers, or public runtime redistribution.

---

## File Structure

- `crates/yoyo-mpv/src/options.rs`: pure mpv initialization option types and option formatting.
- `crates/yoyo-mpv/src/error.rs`: add video-output/runtime binding error variant.
- `crates/yoyo-mpv/src/client.rs`: apply `MpvClientOptions` before mpv initialization and expose `MpvBackend::new_runtime_with_options`.
- `crates/yoyo-mpv/src/lib.rs`: export option types.
- `crates/yoyo-mpv/tests/options_contract.rs`: non-runtime tests for option formatting.
- `apps/yoyovideo-desktop/src/presenter.rs`: add labels and ratios for progress, volume, speed, zoom, rotation, audio channel, and A-B loop.
- `apps/yoyovideo-desktop/tests/presenter_contract.rs`: expand presenter coverage.
- `apps/yoyovideo-desktop/ui/main-window.slint`: expose video geometry, focus state, and control callbacks.
- `apps/yoyovideo-desktop/src/keyboard.rs`: pure key/modifier to shortcut gesture mapping plus winit event adapter.
- `apps/yoyovideo-desktop/tests/keyboard_contract.rs`: test key mapping and URL focus suppression helpers.
- `apps/yoyovideo-desktop/src/video_host.rs`: platform-neutral video host traits, bounds, ids, unsupported host, and geometry conversion.
- `apps/yoyovideo-desktop/tests/video_host_contract.rs`: test bounds conversion and unsupported host behavior.
- `apps/yoyovideo-desktop/src/video_host_winit.rs`: winit native child host implementation and mpv-compatible id extraction.
- `apps/yoyovideo-desktop/src/app.rs`: deferred runtime startup, video host lifecycle, keyboard dispatch, UI callback wiring, fullscreen state.
- `apps/yoyovideo-desktop/src/lib.rs`: export new testable modules/functions.
- `apps/yoyovideo-desktop/Cargo.toml`: add direct `raw-window-handle = "0.6.2"` dependency if the implementation needs it outside Slint's re-exports.
- `docs/development/runtime-dependencies.md`: document video-host platform limitations.
- `docs/testing/manual-smoke-checklist.md`: add visible-video, resize, fullscreen, and shortcut smoke checks.

---

### Task 1: mpv Video Window Options

**Files:**
- Create: `crates/yoyo-mpv/src/options.rs`
- Modify: `crates/yoyo-mpv/src/error.rs`
- Modify: `crates/yoyo-mpv/src/client.rs`
- Modify: `crates/yoyo-mpv/src/lib.rs`
- Test: `crates/yoyo-mpv/tests/options_contract.rs`

**Interfaces:**
- Produces: `MpvVideoWindow::new(u64) -> Self`
- Produces: `MpvVideoWindow::id(&self) -> u64`
- Produces: `MpvClientOptions { video_window: Option<MpvVideoWindow>, force_window: bool, profile: Option<String> }`
- Produces: `MpvClientOptions::mpv_option_pairs(&self) -> Vec<(&'static str, String)>`
- Produces: `MpvClient::new_with_options(options: MpvClientOptions) -> Result<Self, MpvError>`
- Produces: `MpvBackend::new_runtime_with_options(options: MpvClientOptions) -> Result<Self, MpvError>`

- [ ] **Step 1: Write failing option formatting tests**

Create `crates/yoyo-mpv/tests/options_contract.rs`:

```rust
use yoyo_mpv::{MpvClientOptions, MpvVideoWindow};

#[test]
fn default_options_do_not_force_a_video_window() {
    let options = MpvClientOptions::default();

    assert!(options.video_window.is_none());
    assert!(!options.force_window);
    assert!(options.mpv_option_pairs().is_empty());
}

#[test]
fn video_window_options_are_formatted_for_mpv_before_runtime_init() {
    let options = MpvClientOptions {
        video_window: Some(MpvVideoWindow::new(42)),
        force_window: true,
        profile: Some("low-latency".into()),
    };

    assert_eq!(
        options.mpv_option_pairs(),
        vec![
            ("wid", "42".to_string()),
            ("force-window", "yes".to_string()),
            ("profile", "low-latency".to_string()),
        ]
    );
}
```

- [ ] **Step 2: Run the failing test**

Run:

```powershell
cargo test -p yoyo-mpv --test options_contract
```

Expected: FAIL because `MpvClientOptions` and `MpvVideoWindow` are not exported.

- [ ] **Step 3: Add pure option types**

Create `crates/yoyo-mpv/src/options.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpvVideoWindow {
    id: u64,
}

impl MpvVideoWindow {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MpvClientOptions {
    pub video_window: Option<MpvVideoWindow>,
    pub force_window: bool,
    pub profile: Option<String>,
}

impl MpvClientOptions {
    pub fn mpv_option_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(window) = self.video_window {
            pairs.push(("wid", window.id().to_string()));
        }
        if self.force_window {
            pairs.push(("force-window", "yes".to_string()));
        }
        if let Some(profile) = &self.profile {
            pairs.push(("profile", profile.clone()));
        }
        pairs
    }
}
```

- [ ] **Step 4: Export option types**

Modify `crates/yoyo-mpv/src/lib.rs`:

```rust
mod client;
mod error;
mod event;
mod options;
mod render;
mod translate;

pub use client::{DryRunMpvBackend, MpvActionSink, MpvBackend, MpvClient, execute_actions};
pub use error::MpvError;
pub use event::{MpvEvent, map_event};
pub use options::{MpvClientOptions, MpvVideoWindow};
pub use render::{MpvRenderBridge, RenderTarget};
pub use translate::{MpvAction, translate_command, translate_open};
```

- [ ] **Step 5: Add explicit video-output error**

Modify `crates/yoyo-mpv/src/error.rs` by adding this variant to `MpvError`:

```rust
#[error("mpv video output failed: {0}")]
VideoOutput(String),
```

- [ ] **Step 6: Wire options into runtime client creation**

Modify `crates/yoyo-mpv/src/client.rs`:

```rust
use crate::{MpvAction, MpvClientOptions, MpvError, MpvEvent, MpvRenderBridge, map_event, translate_command, translate_open};
```

Add `MpvBackend::new_runtime_with_options`:

```rust
pub fn new_runtime_with_options(options: MpvClientOptions) -> Result<Self, MpvError> {
    let mut client = MpvClient::new_with_options(options)?;
    client.observe_default_properties()?;
    Ok(Self { client, pending_events: Vec::new(), render_bridge: MpvRenderBridge::default() })
}
```

Change `MpvBackend::new_runtime()`:

```rust
pub fn new_runtime() -> Result<Self, MpvError> {
    Self::new_runtime_with_options(MpvClientOptions::default())
}
```

Add runtime-only client constructor:

```rust
pub fn new_with_options(options: MpvClientOptions) -> Result<Self, MpvError> {
    let handle = unsafe { libmpv_sys::mpv_create() };
    if handle.is_null() {
        return Err(MpvError::CreateHandle);
    }

    if let Err(error) = apply_client_options(handle, &options) {
        unsafe { libmpv_sys::mpv_terminate_destroy(handle) };
        return Err(error);
    }

    let init_result = unsafe { libmpv_sys::mpv_initialize(handle) };
    if init_result < 0 {
        unsafe { libmpv_sys::mpv_terminate_destroy(handle) };
        return Err(MpvError::Initialize(mpv_error_message(init_result)));
    }

    Ok(Self { handle })
}
```

Change runtime `MpvClient::new()`:

```rust
pub fn new() -> Result<Self, MpvError> {
    Self::new_with_options(MpvClientOptions::default())
}
```

Add helper:

```rust
#[cfg(feature = "mpv-runtime")]
fn apply_client_options(
    handle: *mut libmpv_sys::mpv_handle,
    options: &MpvClientOptions,
) -> Result<(), MpvError> {
    for (name, value) in options.mpv_option_pairs() {
        let name = cstring(name)?;
        let value = cstring(&value)?;
        let result =
            unsafe { libmpv_sys::mpv_set_option_string(handle, name.as_ptr(), value.as_ptr()) };
        if result < 0 {
            return Err(MpvError::VideoOutput(format!(
                "set option {}: {}",
                name.to_string_lossy(),
                mpv_error_message(result)
            )));
        }
    }
    Ok(())
}
```

Add non-runtime constructor:

```rust
pub fn new_with_options(_options: MpvClientOptions) -> Result<Self, MpvError> {
    Err(MpvError::RuntimeDisabled)
}
```

- [ ] **Step 7: Run option tests**

Run:

```powershell
cargo test -p yoyo-mpv --test options_contract
```

Expected: PASS.

- [ ] **Step 8: Run runtime type check**

Run:

```powershell
cargo check -p yoyo-mpv --features mpv-runtime
```

Expected: PASS in an environment where current runtime checks already pass.

- [ ] **Step 9: Commit**

Run:

```powershell
git add crates/yoyo-mpv/src/options.rs crates/yoyo-mpv/src/error.rs crates/yoyo-mpv/src/client.rs crates/yoyo-mpv/src/lib.rs crates/yoyo-mpv/tests/options_contract.rs
git commit -m "feat: add mpv video window options"
```

Expected: Commit succeeds.

---

### Task 2: Presenter Labels For Full Control Surface

**Files:**
- Modify: `apps/yoyovideo-desktop/src/presenter.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Test: `apps/yoyovideo-desktop/tests/presenter_contract.rs`

**Interfaces:**
- Produces: `format_volume_label(&PlayerState) -> String`
- Produces: `format_rotation_label(&PlayerState) -> String`
- Produces: `format_audio_channel_label(&PlayerState) -> String`
- Produces: `format_zoom_label(&PlayerState) -> String`
- Produces: `format_loop_label(&PlayerState) -> String`
- Produces: `progress_ratio(&PlayerState) -> f32`

- [ ] **Step 1: Add failing presenter tests**

Append to `apps/yoyovideo-desktop/tests/presenter_contract.rs`:

```rust
use yoyo_core::{AudioChannelMode, LoopState, Rotation};
use yoyovideo_desktop::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_volume_label,
    format_zoom_label, progress_ratio,
};

#[test]
fn presenter_formats_volume_rotation_audio_zoom_and_loop() {
    let mut state = yoyo_core::PlayerState::default();
    state.volume_percent = 73;
    state.rotation = Rotation::Deg90;
    state.audio_channel = AudioChannelMode::MonoLeft;
    state.zoom_step = 2;
    state.loop_state = LoopState { point_a: Some(12.4), point_b: Some(45.9) };

    assert_eq!(format_volume_label(&state), "Vol 73%");
    assert_eq!(format_rotation_label(&state), "90 deg");
    assert_eq!(format_audio_channel_label(&state), "Mono L");
    assert_eq!(format_zoom_label(&state), "Zoom +2");
    assert_eq!(format_loop_label(&state), "A 00:12 / B 00:45");
}

#[test]
fn progress_ratio_is_zero_without_duration_and_clamped_with_duration() {
    let mut state = yoyo_core::PlayerState::default();
    assert_eq!(progress_ratio(&state), 0.0);

    state.position_seconds = 25.0;
    state.duration_seconds = Some(100.0);
    assert_eq!(progress_ratio(&state), 0.25);

    state.position_seconds = 150.0;
    assert_eq!(progress_ratio(&state), 1.0);
}
```

- [ ] **Step 2: Run failing presenter tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test presenter_contract
```

Expected: FAIL because the new presenter functions are not exported.

- [ ] **Step 3: Implement presenter functions**

Modify `apps/yoyovideo-desktop/src/presenter.rs`:

```rust
use yoyo_core::{AudioChannelMode, PlayerState, Rotation};

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}
```

Change `format_time_label` to reuse `fmt_clock`, then add:

```rust
pub fn progress_ratio(state: &PlayerState) -> f32 {
    match state.duration_seconds {
        Some(duration) if duration > 0.0 => {
            (state.position_seconds / duration).clamp(0.0, 1.0) as f32
        }
        _ => 0.0,
    }
}

pub fn format_volume_label(state: &PlayerState) -> String {
    format!("Vol {}%", state.volume_percent)
}

pub fn format_rotation_label(state: &PlayerState) -> String {
    match state.rotation {
        Rotation::Deg0 => "0 deg".into(),
        Rotation::Deg90 => "90 deg".into(),
        Rotation::Deg180 => "180 deg".into(),
        Rotation::Deg270 => "270 deg".into(),
    }
}

pub fn format_audio_channel_label(state: &PlayerState) -> String {
    match state.audio_channel {
        AudioChannelMode::Stereo => "Stereo".into(),
        AudioChannelMode::MonoLeft => "Mono L".into(),
        AudioChannelMode::MonoRight => "Mono R".into(),
    }
}

pub fn format_zoom_label(state: &PlayerState) -> String {
    match state.zoom_step.cmp(&0) {
        std::cmp::Ordering::Greater => format!("Zoom +{}", state.zoom_step),
        std::cmp::Ordering::Less => format!("Zoom {}", state.zoom_step),
        std::cmp::Ordering::Equal => "Zoom 0".into(),
    }
}

pub fn format_loop_label(state: &PlayerState) -> String {
    match (state.loop_state.point_a, state.loop_state.point_b) {
        (Some(a), Some(b)) => format!("A {} / B {}", fmt_clock(a), fmt_clock(b)),
        (Some(a), None) => format!("A {} / B --:--", fmt_clock(a)),
        (None, Some(b)) => format!("A --:-- / B {}", fmt_clock(b)),
        (None, None) => "A --:-- / B --:--".into(),
    }
}
```

- [ ] **Step 4: Export presenter functions**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
pub use presenter::{
    format_audio_channel_label, format_loop_label, format_rotation_label, format_speed_label,
    format_time_label, format_transport_label, format_volume_label, format_zoom_label,
    progress_ratio,
};
```

- [ ] **Step 5: Run presenter tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test presenter_contract
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/presenter.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/presenter_contract.rs
git commit -m "feat: add player control presenter labels"
```

Expected: Commit succeeds.

---

### Task 3: Slint Control Surface And Geometry Exports

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/src/app.rs`

**Interfaces:**
- Produces Slint properties: `progress_value`, `volume_value`, `volume_label`, `rotation_label`, `audio_channel_label`, `zoom_label`, `loop_label`
- Produces Slint geometry properties: `video_area_x`, `video_area_y`, `video_area_width`, `video_area_height`
- Produces callbacks: `seek_percent_requested(float)`, `volume_changed(int)`, `reset_speed_requested()`, `zoom_in_requested()`, `zoom_out_requested()`

- [ ] **Step 1: Run current UI build check**

Run:

```powershell
cargo check -p yoyovideo-desktop
```

Expected: PASS before editing.

- [ ] **Step 2: Replace the basic UI with the expanded control surface**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` so the exported component contains these properties and callbacks:

```slint
import { Button, HorizontalBox, VerticalBox, LineEdit, Slider } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "YoYoVideo";
    width: 1200px;
    height: 760px;

    in-out property <string> transport_label: "Play";
    in-out property <string> speed_label: "1.00x";
    in-out property <string> time_label: "00:00 / --:--";
    in-out property <string> status_label: "";
    in-out property <string> volume_label: "Vol 100%";
    in-out property <string> rotation_label: "0 deg";
    in-out property <string> audio_channel_label: "Stereo";
    in-out property <string> zoom_label: "Zoom 0";
    in-out property <string> loop_label: "A --:-- / B --:--";
    in-out property <float> progress_value: 0;
    in-out property <int> volume_value: 100;
    in-out property <bool> url_focused: false;

    out property <length> video_area_x: video_area.x;
    out property <length> video_area_y: video_area.y;
    out property <length> video_area_width: video_area.width;
    out property <length> video_area_height: video_area.height;
```

Add these callbacks:

```slint
    callback seek_percent_requested(float);
    callback volume_changed(int);
    callback reset_speed_requested();
    callback zoom_in_requested();
    callback zoom_out_requested();
```

Replace the body with a dark video area and compact two-row control bar:

```slint
    VerticalBox {
        spacing: 8px;
        padding: 10px;
        video_area := Rectangle {
            background: #090b0d;
            border-color: #22272d;
            border-width: 1px;
            border-radius: 8px;
            min-height: 560px;
            Text {
                text: status_label == "" ? "Video host initializing..." : status_label;
                color: #7d8790;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
        VerticalBox {
            spacing: 6px;
            Slider {
                minimum: 0;
                maximum: 1;
                value: progress_value;
                changed(value) => { root.seek_percent_requested(value); }
            }
            HorizontalBox {
                spacing: 8px;
                Button { text: transport_label; clicked => { root.toggle_pause_requested(); } }
                Button { text: "Open"; clicked => { root.open_file_requested(); } }
                Button { text: "Folder"; clicked => { root.open_folder_requested(); } }
                Button { text: "Menu"; clicked => { menu_popup.show(); } }
                LineEdit {
                    placeholder-text: "Open URL";
                    has-focus <=> root.url_focused;
                    accepted => { root.open_url_requested(self.text); }
                }
                Text { text: time_label; }
                Text { text: speed_label; }
                Button { text: "-"; clicked => { root.speed_down_requested(); } }
                Button { text: "+"; clicked => { root.speed_up_requested(); } }
                Button { text: "1x"; clicked => { root.reset_speed_requested(); } }
                Text { text: volume_label; }
                Slider {
                    minimum: 0;
                    maximum: 100;
                    value: volume_value;
                    changed(value) => { root.volume_changed(value); }
                }
            }
            HorizontalBox {
                spacing: 8px;
                Button { text: "Zoom -"; clicked => { root.zoom_out_requested(); } }
                Button { text: "Zoom +"; clicked => { root.zoom_in_requested(); } }
                Text { text: zoom_label; }
                Button { text: "Rotate"; clicked => { root.rotate_requested(); } }
                Text { text: rotation_label; }
                Button { text: "Audio"; clicked => { root.cycle_audio_requested(); } }
                Text { text: audio_channel_label; }
                Button { text: "Set A"; clicked => { root.set_ab_point_a_requested(); } }
                Button { text: "Set B"; clicked => { root.set_ab_point_b_requested(); } }
                Button { text: "Clear A-B"; clicked => { root.clear_ab_loop_requested(); } }
                Text { text: loop_label; }
                Button { text: "Fullscreen"; clicked => { root.toggle_fullscreen_requested(); } }
            }
        }
    }
```

- [ ] **Step 3: Refresh all new UI properties**

Modify `refresh_window` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
pub fn refresh_window(window: &MainWindow, state: &PlayerState) {
    window.set_transport_label(crate::format_transport_label(state).into());
    window.set_speed_label(crate::format_speed_label(state).into());
    window.set_time_label(crate::format_time_label(state).into());
    window.set_volume_label(crate::format_volume_label(state).into());
    window.set_rotation_label(crate::format_rotation_label(state).into());
    window.set_audio_channel_label(crate::format_audio_channel_label(state).into());
    window.set_zoom_label(crate::format_zoom_label(state).into());
    window.set_loop_label(crate::format_loop_label(state).into());
    window.set_progress_value(crate::progress_ratio(state));
    window.set_volume_value(state.volume_percent as i32);
    window.set_status_label(
        state
            .last_error
            .clone()
            .or_else(|| state.status_message.clone())
            .unwrap_or_default()
            .into(),
    );
}
```

- [ ] **Step 4: Wire new callbacks to existing commands**

In `run()`, add handlers:

```rust
app.on_reset_speed_requested(command_callback(&app, &controller, AppCommand::ResetSpeed));
app.on_zoom_in_requested(command_callback(&app, &controller, AppCommand::ZoomIn));
app.on_zoom_out_requested(command_callback(&app, &controller, AppCommand::ZoomOut));
```

Add seek and volume handlers:

```rust
app.on_seek_percent_requested({
    let app_handle = app.as_weak();
    let controller = Rc::clone(&controller);
    move |percent| {
        let mut controller = controller.borrow_mut();
        let duration = controller.session().state().duration_seconds;
        if let Some(duration) = duration {
            if controller.dispatch(AppCommand::SeekAbsolute(duration * percent as f64)).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    }
});

app.on_volume_changed({
    let app_handle = app.as_weak();
    let controller = Rc::clone(&controller);
    move |value| {
        let volume = value.clamp(0, 100) as u8;
        let mut controller = controller.borrow_mut();
        if controller.dispatch(AppCommand::SetVolume(volume)).is_ok() {
            if let Some(app) = app_handle.upgrade() {
                refresh_window(&app, controller.session().state());
            }
        }
    }
});
```

- [ ] **Step 5: Run desktop check**

Run:

```powershell
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/src/app.rs
git commit -m "feat: expand desktop control surface"
```

Expected: Commit succeeds.

---

### Task 4: Keyboard Mapping And Shortcut Suppression

**Files:**
- Create: `apps/yoyovideo-desktop/src/keyboard.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Test: `apps/yoyovideo-desktop/tests/keyboard_contract.rs`

**Interfaces:**
- Produces: `DesktopKey`
- Produces: `KeyboardInput { key: DesktopKey, ctrl: bool, repeat: bool, pressed: bool }`
- Produces: `shortcut_gesture(input: KeyboardInput) -> Option<&'static str>`
- Produces: `shortcut_allowed(url_focused: bool) -> bool`

- [ ] **Step 1: Write failing keyboard tests**

Create `apps/yoyovideo-desktop/tests/keyboard_contract.rs`:

```rust
use yoyovideo_desktop::{
    DesktopKey, KeyboardInput, shortcut_allowed, shortcut_gesture,
};

#[test]
fn keyboard_input_maps_to_existing_shortcut_gestures() {
    assert_eq!(
        shortcut_gesture(KeyboardInput::pressed(DesktopKey::Space)),
        Some("Space")
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput::pressed(DesktopKey::Right)),
        Some("Right")
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput { key: DesktopKey::A, ctrl: true, repeat: false, pressed: true }),
        Some("Ctrl+A")
    );
}

#[test]
fn key_release_is_ignored() {
    assert_eq!(
        shortcut_gesture(KeyboardInput { key: DesktopKey::Space, ctrl: false, repeat: false, pressed: false }),
        None
    );
}

#[test]
fn url_focus_suppresses_player_shortcuts() {
    assert!(shortcut_allowed(false));
    assert!(!shortcut_allowed(true));
}
```

- [ ] **Step 2: Run failing keyboard tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test keyboard_contract
```

Expected: FAIL because keyboard types are not exported.

- [ ] **Step 3: Implement pure keyboard mapper**

Create `apps/yoyovideo-desktop/src/keyboard.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopKey {
    Space,
    Left,
    Right,
    Up,
    Down,
    LeftBracket,
    RightBracket,
    Digit0,
    A,
    B,
    R,
    Z,
    X,
    C,
    F,
    O,
    U,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInput {
    pub key: DesktopKey,
    pub ctrl: bool,
    pub repeat: bool,
    pub pressed: bool,
}

impl KeyboardInput {
    pub fn pressed(key: DesktopKey) -> Self {
        Self { key, ctrl: false, repeat: false, pressed: true }
    }
}

pub fn shortcut_allowed(url_focused: bool) -> bool {
    !url_focused
}

pub fn shortcut_gesture(input: KeyboardInput) -> Option<&'static str> {
    if !input.pressed {
        return None;
    }

    match (input.key, input.ctrl) {
        (DesktopKey::Space, false) => Some("Space"),
        (DesktopKey::Left, false) => Some("Left"),
        (DesktopKey::Right, false) => Some("Right"),
        (DesktopKey::Up, false) => Some("Up"),
        (DesktopKey::Down, false) => Some("Down"),
        (DesktopKey::LeftBracket, false) => Some("["),
        (DesktopKey::RightBracket, false) => Some("]"),
        (DesktopKey::Digit0, false) => Some("0"),
        (DesktopKey::A, false) => Some("A"),
        (DesktopKey::B, false) => Some("B"),
        (DesktopKey::A, true) => Some("Ctrl+A"),
        (DesktopKey::R, false) => Some("R"),
        (DesktopKey::Z, false) => Some("Z"),
        (DesktopKey::X, false) => Some("X"),
        (DesktopKey::C, false) => Some("C"),
        (DesktopKey::F, false) => Some("F"),
        (DesktopKey::O, false) => Some("O"),
        (DesktopKey::U, false) => Some("U"),
        _ => None,
    }
}
```

- [ ] **Step 4: Export keyboard helpers**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
mod keyboard;
pub use keyboard::{DesktopKey, KeyboardInput, shortcut_allowed, shortcut_gesture};
```

- [ ] **Step 5: Add winit event adapter**

Append to `apps/yoyovideo-desktop/src/keyboard.rs`:

```rust
#[cfg(feature = "mpv-runtime")]
pub mod winit_adapter {
    use slint::winit_030::winit::{
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{Key, ModifiersState, NamedKey},
    };

    use super::{DesktopKey, KeyboardInput};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct WinitKeyboardState {
        modifiers: ModifiersState,
    }

    impl WinitKeyboardState {
        pub fn update(&mut self, event: &WindowEvent) -> Option<KeyboardInput> {
            match event {
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = modifiers.state();
                    None
                }
                WindowEvent::KeyboardInput { event, .. } => self.map_key_event(event),
                _ => None,
            }
        }

        fn map_key_event(&self, event: &KeyEvent) -> Option<KeyboardInput> {
            let key = match &event.logical_key {
                Key::Named(NamedKey::Space) => DesktopKey::Space,
                Key::Named(NamedKey::ArrowLeft) => DesktopKey::Left,
                Key::Named(NamedKey::ArrowRight) => DesktopKey::Right,
                Key::Named(NamedKey::ArrowUp) => DesktopKey::Up,
                Key::Named(NamedKey::ArrowDown) => DesktopKey::Down,
                Key::Character(value) if value == "[" => DesktopKey::LeftBracket,
                Key::Character(value) if value == "]" => DesktopKey::RightBracket,
                Key::Character(value) if value == "0" => DesktopKey::Digit0,
                Key::Character(value) if value.eq_ignore_ascii_case("a") => DesktopKey::A,
                Key::Character(value) if value.eq_ignore_ascii_case("b") => DesktopKey::B,
                Key::Character(value) if value.eq_ignore_ascii_case("r") => DesktopKey::R,
                Key::Character(value) if value.eq_ignore_ascii_case("z") => DesktopKey::Z,
                Key::Character(value) if value.eq_ignore_ascii_case("x") => DesktopKey::X,
                Key::Character(value) if value.eq_ignore_ascii_case("c") => DesktopKey::C,
                Key::Character(value) if value.eq_ignore_ascii_case("f") => DesktopKey::F,
                Key::Character(value) if value.eq_ignore_ascii_case("o") => DesktopKey::O,
                Key::Character(value) if value.eq_ignore_ascii_case("u") => DesktopKey::U,
                _ => return None,
            };

            Some(KeyboardInput {
                key,
                ctrl: self.modifiers.control_key(),
                repeat: event.repeat,
                pressed: event.state == ElementState::Pressed,
            })
        }
    }
}
```

- [ ] **Step 6: Wire real window shortcuts**

Modify `apps/yoyovideo-desktop/src/app.rs` to call `app.window().on_winit_window_event(...)` after callback wiring. Use `shortcut_allowed(app.get_url_focused())`, `WinitKeyboardState::update`, and `shortcut_gesture` to call `DesktopController::dispatch_shortcut`.

Use this closure body:

```rust
let keyboard_state = Rc::new(RefCell::new(crate::keyboard::winit_adapter::WinitKeyboardState::default()));
app.window().on_winit_window_event({
    let app_handle = app.as_weak();
    let controller = Rc::clone(&controller);
    let keyboard_state = Rc::clone(&keyboard_state);
    move |_window, event| {
        let Some(app) = app_handle.upgrade() else {
            return slint::winit_030::EventResult::Propagate;
        };
        if !crate::shortcut_allowed(app.get_url_focused()) {
            return slint::winit_030::EventResult::Propagate;
        }
        let Some(input) = keyboard_state.borrow_mut().update(event) else {
            return slint::winit_030::EventResult::Propagate;
        };
        let Some(gesture) = crate::shortcut_gesture(input) else {
            return slint::winit_030::EventResult::Propagate;
        };
        let mut controller = controller.borrow_mut();
        if controller.dispatch_shortcut(gesture).is_ok() {
            refresh_window(&app, controller.session().state());
        }
        slint::winit_030::EventResult::PreventDefault
    }
});
```

- [ ] **Step 7: Run keyboard tests and desktop check**

Run:

```powershell
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/keyboard.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/tests/keyboard_contract.rs
git commit -m "feat: route window keyboard shortcuts"
```

Expected: Commit succeeds.

---

### Task 5: Video Host Core Abstraction

**Files:**
- Create: `apps/yoyovideo-desktop/src/video_host.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Test: `apps/yoyovideo-desktop/tests/video_host_contract.rs`

**Interfaces:**
- Produces: `NativeVideoWindowId(u64)`
- Produces: `VideoHostBounds`
- Produces: `LogicalVideoRect`
- Produces: `VideoHost` trait
- Produces: `UnsupportedVideoHost`

- [ ] **Step 1: Write failing video host tests**

Create `apps/yoyovideo-desktop/tests/video_host_contract.rs`:

```rust
use yoyovideo_desktop::{
    LogicalVideoRect, UnsupportedVideoHost, VideoHost, VideoHostBounds,
};

#[test]
fn logical_rect_converts_to_physical_bounds() {
    let rect = LogicalVideoRect { x: 10.0, y: 20.0, width: 300.0, height: 200.0 };

    assert_eq!(
        rect.to_physical(1.5),
        VideoHostBounds { x: 15, y: 30, width: 450, height: 300 }
    );
}

#[test]
fn unsupported_host_reports_clear_failure() {
    let mut host = UnsupportedVideoHost::new("Video embedding is not supported on this windowing backend yet");

    assert!(!host.is_available());
    assert!(host.mpv_window_id().is_err());
    assert!(host.show().is_err());
}
```

- [ ] **Step 2: Run failing video host tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test video_host_contract
```

Expected: FAIL because video host types are not exported.

- [ ] **Step 3: Implement video host core**

Create `apps/yoyovideo-desktop/src/video_host.rs`:

```rust
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeVideoWindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoHostBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalVideoRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LogicalVideoRect {
    pub fn to_physical(self, scale_factor: f64) -> VideoHostBounds {
        VideoHostBounds {
            x: (self.x as f64 * scale_factor).round() as i32,
            y: (self.y as f64 * scale_factor).round() as i32,
            width: (self.width as f64 * scale_factor).round().max(1.0) as u32,
            height: (self.height as f64 * scale_factor).round().max(1.0) as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoHostError {
    message: String,
}

impl VideoHostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for VideoHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for VideoHostError {}

pub trait VideoHost {
    fn mpv_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError>;
    fn set_bounds(&mut self, bounds: VideoHostBounds) -> Result<(), VideoHostError>;
    fn show(&mut self) -> Result<(), VideoHostError>;
    fn hide(&mut self) -> Result<(), VideoHostError>;
    fn is_available(&self) -> bool;
}

pub struct UnsupportedVideoHost {
    message: String,
}

impl UnsupportedVideoHost {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    fn error(&self) -> VideoHostError {
        VideoHostError::new(self.message.clone())
    }
}

impl VideoHost for UnsupportedVideoHost {
    fn mpv_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError> {
        Err(self.error())
    }

    fn set_bounds(&mut self, _bounds: VideoHostBounds) -> Result<(), VideoHostError> {
        Err(self.error())
    }

    fn show(&mut self) -> Result<(), VideoHostError> {
        Err(self.error())
    }

    fn hide(&mut self) -> Result<(), VideoHostError> {
        Err(self.error())
    }

    fn is_available(&self) -> bool {
        false
    }
}
```

- [ ] **Step 4: Export video host types**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
mod video_host;
pub use video_host::{
    LogicalVideoRect, NativeVideoWindowId, UnsupportedVideoHost, VideoHost, VideoHostBounds,
    VideoHostError,
};
```

- [ ] **Step 5: Run video host tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test video_host_contract
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/video_host.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/video_host_contract.rs
git commit -m "feat: add video host abstraction"
```

Expected: Commit succeeds.

---

### Task 6: Winit Video Host And Deferred Runtime Startup

**Files:**
- Create: `apps/yoyovideo-desktop/src/video_host_winit.rs`
- Modify: `apps/yoyovideo-desktop/Cargo.toml`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`

**Interfaces:**
- Produces: `WinitVideoHost::new_child(event_loop, parent_window) -> Result<Self, VideoHostError>`
- Produces: `WinitVideoHost` implementing `VideoHost`
- Produces: `DesktopRuntime` that creates `MpvBackend` after a video host exists or reports unsupported host status.

- [ ] **Step 1: Add direct raw-window-handle dependency**

Modify `apps/yoyovideo-desktop/Cargo.toml`:

```toml
raw-window-handle = "0.6.2"
```

The adapter uses `raw_window_handle::{HasWindowHandle, RawWindowHandle}` directly through winit's re-exported raw-window-handle traits. Add the direct dependency so imports stay stable and explicit.

- [ ] **Step 2: Create winit video host adapter**

Create `apps/yoyovideo-desktop/src/video_host_winit.rs` with Windows and X11 id extraction plus unsupported fallback:

```rust
use std::sync::Arc;

use slint::winit_030::winit::{
    event_loop::ActiveEventLoop,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowAttributes},
};

use crate::{NativeVideoWindowId, VideoHost, VideoHostBounds, VideoHostError};

pub struct WinitVideoHost {
    window: Arc<Window>,
}

impl WinitVideoHost {
    pub fn new_child(
        event_loop: &ActiveEventLoop,
        parent: &Window,
    ) -> Result<Self, VideoHostError> {
        let parent_handle = parent
            .window_handle()
            .map_err(|error| VideoHostError::new(format!("parent window handle unavailable: {error}")))?
            .as_raw();
        let attributes = unsafe {
            WindowAttributes::default()
                .with_title("YoYoVideo Video Host")
                .with_visible(false)
                .with_decorations(false)
                .with_parent_window(Some(parent_handle))
        };
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| VideoHostError::new(format!("create video host window: {error}")))?;
        Ok(Self { window: Arc::new(window) })
    }

    fn raw_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError> {
        let handle = self
            .window
            .window_handle()
            .map_err(|error| VideoHostError::new(format!("video host handle unavailable: {error}")))?
            .as_raw();
        match handle {
            RawWindowHandle::Win32(handle) => {
                Ok(NativeVideoWindowId(handle.hwnd.get() as u64))
            }
            RawWindowHandle::Xlib(handle) => {
                Ok(NativeVideoWindowId(handle.window))
            }
            _ => Err(VideoHostError::new(
                "Video embedding is not supported on this windowing backend yet",
            )),
        }
    }
}
```

Implement `VideoHost`:

```rust
impl VideoHost for WinitVideoHost {
    fn mpv_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError> {
        self.raw_window_id()
    }

    fn set_bounds(&mut self, bounds: VideoHostBounds) -> Result<(), VideoHostError> {
        self.window.set_outer_position(slint::winit_030::winit::dpi::PhysicalPosition::new(
            bounds.x,
            bounds.y,
        ));
        let _ = self.window.request_inner_size(
            slint::winit_030::winit::dpi::PhysicalSize::new(bounds.width, bounds.height),
        );
        Ok(())
    }

    fn show(&mut self) -> Result<(), VideoHostError> {
        self.window.set_visible(true);
        Ok(())
    }

    fn hide(&mut self) -> Result<(), VideoHostError> {
        self.window.set_visible(false);
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.raw_window_id().is_ok()
    }
}
```

- [ ] **Step 3: Export runtime-only winit host module**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
#[cfg(feature = "mpv-runtime")]
mod video_host_winit;
```

- [ ] **Step 4: Add deferred backend creation helper**

Modify `apps/yoyovideo-desktop/src/app.rs` so `build_desktop_backend()` remains for tests and simple runtime paths, then add:

```rust
pub fn build_desktop_backend_with_video_window(
    window_id: crate::NativeVideoWindowId,
) -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime_with_options(yoyo_mpv::MpvClientOptions {
        video_window: Some(yoyo_mpv::MpvVideoWindow::new(window_id.0)),
        force_window: true,
        profile: None,
    })
}
```

- [ ] **Step 5: Refactor startup around runtime state**

Refactor `run()` so it creates the Slint window first and creates `AppSession` after video host initialization succeeds. Use a concrete runtime state:

```rust
struct DesktopRuntime {
    controller: Option<DesktopController<MpvBackend>>,
    video_host_error: Option<String>,
}

impl DesktopRuntime {
    fn controller(&self) -> Option<&DesktopController<MpvBackend>> {
        self.controller.as_ref()
    }

    fn controller_mut(&mut self) -> Option<&mut DesktopController<MpvBackend>> {
        self.controller.as_mut()
    }
}
```

When no controller exists yet, button callbacks set `status_label` to `Playback runtime is still initializing`. After initialization succeeds, callbacks dispatch as before.

- [ ] **Step 6: Create video host in winit custom handler**

Configure backend selection:

```rust
slint::BackendSelector::new()
    .backend_name("winit".into())
    .with_winit_custom_application_handler(DesktopWinitHandler::new(Rc::clone(&runtime)))
    .select()?;
```

Implement `DesktopWinitHandler` in `app.rs` behind `#[cfg(feature = "mpv-runtime")]`:

```rust
struct DesktopWinitHandler {
    runtime: Rc<RefCell<DesktopRuntime>>,
}

impl DesktopWinitHandler {
    fn new(runtime: Rc<RefCell<DesktopRuntime>>) -> Self {
        Self { runtime }
    }
}
```

In `window_event`, create `WinitVideoHost` on the first event that provides `winit_window`. Use `build_desktop_backend_with_video_window(host.mpv_window_id()?)`, then create `AppSession::new(AppConfig::default(), backend)` and `DesktopController::new(session)`.

If the host cannot be created or cannot provide a supported id, store the error in `runtime.video_host_error` and leave the controller unset.

- [ ] **Step 7: Run default tests and runtime check**

Run:

```powershell
cargo test -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS. If Windows linking fails only for a linked test binary, keep runtime validation to `cargo check`.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/Cargo.toml apps/yoyovideo-desktop/src/video_host_winit.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs
git commit -m "feat: add winit video host startup"
```

Expected: Commit succeeds.

---

### Task 7: Geometry Sync, Fullscreen, And Runtime Control Wiring

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`

**Interfaces:**
- Produces: `current_video_rect(window: &MainWindow) -> LogicalVideoRect`
- Produces: `sync_video_host_bounds(app, host, scale_factor)`
- Produces: real winit fullscreen state change for `ToggleFullscreen`

- [ ] **Step 1: Add geometry helper in `app.rs`**

Add:

```rust
fn current_video_rect(window: &MainWindow) -> crate::LogicalVideoRect {
    crate::LogicalVideoRect {
        x: window.get_video_area_x() as f32,
        y: window.get_video_area_y() as f32,
        width: window.get_video_area_width() as f32,
        height: window.get_video_area_height() as f32,
    }
}
```

Add:

```rust
fn sync_video_host_bounds<H: crate::VideoHost>(
    window: &MainWindow,
    host: &mut H,
    scale_factor: f64,
) -> Result<(), crate::VideoHostError> {
    let bounds = current_video_rect(window).to_physical(scale_factor);
    host.set_bounds(bounds)?;
    host.show()
}
```

- [ ] **Step 2: Sync host bounds on timer**

Extend the existing 250 ms `poll_timer` callback so that after `refresh_window`, it also reads `app.window().with_winit_window(|w| w.scale_factor())` and calls `sync_video_host_bounds`.

Expected behavior: resize and scale changes converge even if one specific winit event is missed.

- [ ] **Step 3: Sync host bounds after startup**

Immediately after creating `WinitVideoHost` and `DesktopController`, call `sync_video_host_bounds`. If sync fails, store the error and hide the host.

- [ ] **Step 4: Apply actual fullscreen state**

In the `toggle_fullscreen_requested` callback, after dispatching `AppCommand::ToggleFullscreen`, apply winit fullscreen:

```rust
app.window().with_winit_window(|winit_window| {
    if controller.session().state().fullscreen {
        winit_window.set_fullscreen(Some(slint::winit_030::winit::window::Fullscreen::Borderless(None)));
    } else {
        winit_window.set_fullscreen(None);
    }
});
```

- [ ] **Step 5: Run desktop checks**

Run:

```powershell
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/ui/main-window.slint
git commit -m "feat: sync video host geometry and fullscreen"
```

Expected: Commit succeeds.

---

### Task 8: Documentation And Smoke Checklist

**Files:**
- Modify: `docs/development/runtime-dependencies.md`
- Modify: `docs/testing/manual-smoke-checklist.md`
- Modify: `README.md`

**Interfaces:**
- Produces: documented video-host platform behavior and manual visible-video validation.

- [ ] **Step 1: Add failing documentation coverage check**

Run:

```powershell
$checks = @{
  "README.md" = @("visible video", "mpv-runtime")
  "docs/development/runtime-dependencies.md" = @("video host", "wid", "Wayland")
  "docs/testing/manual-smoke-checklist.md" = @("visible inside the video area", "URL input", "resize")
}
$missing = @()
foreach ($file in $checks.Keys) {
  $content = Get-Content -Raw $file
  foreach ($pattern in $checks[$file]) {
    if ($content -notmatch [regex]::Escape($pattern)) {
      $missing += "$file missing $pattern"
    }
  }
}
if ($missing) {
  Write-Error ($missing -join "; ")
  exit 1
}
```

Expected: FAIL before docs are updated.

- [ ] **Step 2: Update README**

Append to `README.md`:

```markdown
## Visible Video Runtime

The desktop app uses a native video host for visible video when built with `mpv-runtime` and when the current windowing backend can provide an mpv-compatible window id. If video embedding is unavailable, the app stays open and reports the limitation in the status label.

Run:

```powershell
cargo run -p yoyovideo-desktop --features mpv-runtime
```
```

- [ ] **Step 3: Update runtime dependency docs**

Append to `docs/development/runtime-dependencies.md`:

```markdown
## Video Host Requirements

Visible video uses mpv's `wid` window binding. The desktop app creates a native video host and passes that id to mpv before initialization.

- Windows: required first target for native video host embedding.
- Linux X11: required design target using an X11 window id.
- Wayland: reports unsupported embedding unless a verified host path is implemented.
- macOS: reports unsupported embedding unless a verified host path is implemented.
```

- [ ] **Step 4: Update smoke checklist**

Append to `docs/testing/manual-smoke-checklist.md`:

```markdown
## Visible Video Host

- Launch with `cargo run -p yoyovideo-desktop --features mpv-runtime`.
- Open a local video and confirm video is visible inside the video area.
- Confirm the video does not cover controls.
- Resize the window and confirm the video area tracks the UI.
- Toggle fullscreen and confirm the video host resizes.
- Type in the URL input and confirm player shortcuts do not fire while it is focused.
- Use keyboard shortcuts for play/pause, seek, volume, speed, zoom, rotation, audio channel, A-B loop, and fullscreen.
```

- [ ] **Step 5: Run documentation coverage check again**

Run the command from Step 1.

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add README.md docs/development/runtime-dependencies.md docs/testing/manual-smoke-checklist.md
git commit -m "docs: document visible video host smoke tests"
```

Expected: Commit succeeds.

---

### Task 9: Final Verification

**Files:**
- Read: all files changed by Tasks 1-8.

**Interfaces:**
- Produces: verified branch ready for manual runtime smoke testing with libmpv files.

- [ ] **Step 1: Run default formatting and tests**

Run:

```powershell
cargo fmt --check
cargo test
```

Expected: PASS without libmpv runtime files.

- [ ] **Step 2: Run runtime feature checks**

Run:

```powershell
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS in the same local environment where current runtime checks pass.

- [ ] **Step 3: Run package script without runtime**

Run:

```powershell
cargo build -p yoyovideo-desktop
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -SkipBuild
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64
```

Expected: PASS and produces ignored `dist/YoYoVideo-windows-x64.zip`.

- [ ] **Step 4: Confirm runtime-required package still fails clearly without staged runtime**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -SkipBuild
```

Expected: FAIL with `Missing Windows mpv import library` or another clear missing-runtime message.

- [ ] **Step 5: Inspect git status**

Run:

```powershell
git status --short
```

Expected: no source changes. Ignored `dist/` output may exist but must not appear.

- [ ] **Step 6: Report outcome**

Report:

```text
Implemented:
- mpv video-window initialization options
- expanded player control surface
- real keyboard event routing
- video host abstraction
- winit child video host path
- geometry/fullscreen sync
- docs and smoke checklist updates

Verified:
- cargo fmt --check
- cargo test
- cargo check -p yoyo-mpv --features mpv-runtime
- cargo check -p yoyovideo-desktop --features mpv-runtime
- package script without runtime
- missing-runtime package failure

Still requires manual smoke with staged libmpv runtime:
- visible video inside the native host
- resize/fullscreen behavior
- platform-specific video-host support
```

Expected: The user understands that this phase makes visible playback technically available on supported backends, while real runtime smoke testing still requires staged libmpv files.

---

## Self-Review

**Spec coverage:** The plan covers mpv video-window options, video host abstraction, winit native host, deferred runtime startup, Slint geometry exports, expanded controls, keyboard routing, URL focus suppression, fullscreen sync, docs, and verification. It explicitly leaves playlist/history/settings/subtitles/release signing outside this phase.

**Placeholder scan:** The plan does not contain deferred implementation placeholders or vague error-handling tasks. Unsupported platform behavior is implemented as explicit capability reporting, not hidden follow-up work.

**Type consistency:** The named types are consistent across tasks: `MpvClientOptions`, `MpvVideoWindow`, `NativeVideoWindowId`, `VideoHostBounds`, `LogicalVideoRect`, `VideoHost`, `UnsupportedVideoHost`, `DesktopKey`, and `KeyboardInput`.
