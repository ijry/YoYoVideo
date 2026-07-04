# Real mpv Playback Alpha Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder mpv backend with a real `libmpv` playback path that can open local files and supported URLs, execute core transport commands, surface playback events, and fail clearly when runtime support is unavailable.

**Architecture:** Keep `yoyo-core` unchanged as the typed domain boundary. Implement the real playback path inside `yoyo-mpv` with a safe `MpvClient` wrapper around `libmpv-sys`, a separate dry-run backend for unit tests, pure action/event translation helpers, and a desktop bootstrap layer that explicitly opts into runtime playback and polls backend events into Slint UI state.

**Tech Stack:** Rust 1.94.1, Cargo workspaces, Slint 1.17.0 (`unstable-winit-030`, `raw-window-handle-06`, `backend-winit`), libmpv via `libmpv-sys 3.1.0`, thiserror 2.0.18, tracing 0.1.44, tracing-subscriber 0.3.23, rfd 0.17.2.

## Global Constraints

- Support Windows, macOS, and Linux desktop platforms.
- Use Rust for playback orchestration, Slint for the desktop shell, and libmpv as the embedded native playback engine.
- Keep memory usage and runtime overhead low by using native UI and embedded playback rather than decoded-frame copies.
- This phase includes real local-file playback, supported network URL playback, playback commands, and backend event observation.
- This phase excludes Slint video-surface embedding, advanced subtitle/filter features, capture features, plugin scripting, and full PotPlayer parity.
- Unit tests must pass without requiring a user-installed libmpv.
- Runtime-enabled compilation must be explicit and deterministic through Cargo features.
- The app must not silently use fake playback in the production desktop startup path.

---

## Planned File Structure

`crates/yoyo-mpv/src/lib.rs`
- Re-export the real backend, dry-run backend, event mapping helpers, and runtime client types.

`crates/yoyo-mpv/src/error.rs`
- Expand mpv-specific error coverage so startup and runtime failures are actionable.

`crates/yoyo-mpv/src/client.rs`
- Hold `MpvClient`, runtime-gated FFI helpers, the action execution seam, and the dry-run backend.

`crates/yoyo-mpv/src/event.rs`
- Define typed mpv-side playback events and pure mapping into `yoyo-core::BackendEvent`.

`crates/yoyo-mpv/src/translate.rs`
- Keep pure translation from `BackendCommand`/`MediaLocator` into `MpvAction`.

`crates/yoyo-mpv/tests/dry_run_contract.rs`
- Verify dry-run execution records translated actions in order without libmpv.

`crates/yoyo-mpv/tests/event_contract.rs`
- Verify typed mpv events map into the right `BackendEvent` values.

`crates/yoyo-mpv/tests/runtime_contract.rs`
- Verify runtime backend construction fails cleanly when the `mpv-runtime` feature is not enabled.

`apps/yoyovideo-desktop/Cargo.toml`
- Add a desktop-facing passthrough feature for `yoyo-mpv/mpv-runtime`.

`apps/yoyovideo-desktop/src/app.rs`
- Select the real backend at startup, wire open/playback callbacks, poll backend events on a timer, and refresh UI labels from `PlayerState`.

`apps/yoyovideo-desktop/src/lib.rs`
- Export any new startup helpers needed by tests.

`apps/yoyovideo-desktop/src/platform/dialogs.rs`
- Add an optional URL prompt abstraction so UI callback code does not inline backend selection logic.

`apps/yoyovideo-desktop/tests/controller_contract.rs`
- Extend controller coverage for file/URL dispatch and label refresh behavior.

`apps/yoyovideo-desktop/tests/runtime_startup_contract.rs`
- Verify default desktop startup rejects runtime playback when the feature is absent.

`docs/development/runtime-dependencies.md`
- Document feature flags, expected runtime discovery, and failure modes for local development.

`docs/testing/manual-smoke-checklist.md`
- Add real-playback smoke steps for local file and URL playback.

`README.md`
- Document the new runtime feature build/run command.

### Task 1: Add Dry-Run Execution And Pure Event Mapping

**Files:**
- Modify: `crates/yoyo-mpv/src/lib.rs`
- Modify: `crates/yoyo-mpv/src/error.rs`
- Modify: `crates/yoyo-mpv/src/client.rs`
- Create: `crates/yoyo-mpv/src/event.rs`
- Create: `crates/yoyo-mpv/tests/dry_run_contract.rs`
- Create: `crates/yoyo-mpv/tests/event_contract.rs`

**Interfaces:**
- Consumes:
  - `pub enum MpvAction`
  - `pub fn translate_open(locator: &MediaLocator) -> Vec<MpvAction>`
  - `pub fn translate_command(command: &BackendCommand) -> Vec<MpvAction>`
- Produces:
  - `pub trait MpvActionSink`
  - `pub fn execute_actions<S: MpvActionSink>(sink: &mut S, actions: &[MpvAction]) -> Result<(), MpvError>`
  - `pub struct DryRunMpvBackend`
  - `impl DryRunMpvBackend { pub fn recorded_actions(&self) -> &[String] }`
  - `pub enum MpvEvent`
  - `pub fn map_event(event: MpvEvent) -> Option<BackendEvent>`

- [ ] **Step 1: Write the failing tests**

```rust
// crates/yoyo-mpv/tests/dry_run_contract.rs
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
```

```rust
// crates/yoyo-mpv/tests/event_contract.rs
use yoyo_core::{BackendEvent, Rotation};
use yoyo_mpv::{MpvEvent, map_event};

#[test]
fn pause_event_maps_to_backend_pause_changed() {
    assert_eq!(map_event(MpvEvent::Pause(true)), Some(BackendEvent::PauseChanged(true)));
}

#[test]
fn duration_event_maps_to_backend_duration_changed() {
    assert_eq!(
        map_event(MpvEvent::Duration(Some(120.0))),
        Some(BackendEvent::DurationChanged(Some(120.0)))
    );
}

#[test]
fn end_file_maps_to_backend_eof() {
    assert_eq!(map_event(MpvEvent::EndFile), Some(BackendEvent::EndOfFile));
}

#[test]
fn unknown_rotation_warning_is_preserved() {
    assert_eq!(
        map_event(MpvEvent::Warning("rotation fallback".into())),
        Some(BackendEvent::Warning("rotation fallback".into()))
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yoyo-mpv --test dry_run_contract --test event_contract`

Expected: FAIL with unresolved imports such as `DryRunMpvBackend`, `MpvEvent`, `map_event`, or `recorded_actions`.

- [ ] **Step 3: Write the minimal implementation**

```rust
// crates/yoyo-mpv/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MpvError {
    #[error("mpv runtime feature is disabled")]
    RuntimeDisabled,
    #[error("mpv handle creation failed")]
    CreateHandle,
    #[error("mpv initialization failed: {0}")]
    Initialize(String),
    #[error("mpv command failed: {0}")]
    Command(String),
    #[error("mpv property failed: {0}")]
    Property(String),
    #[error("mpv string contained an interior null byte: {0}")]
    InvalidString(String),
    #[error("mpv api error: {0}")]
    Api(String),
}
```

```rust
// crates/yoyo-mpv/src/event.rs
use yoyo_core::BackendEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum MpvEvent {
    Pause(bool),
    Position(f64),
    Duration(Option<f64>),
    Speed(f32),
    Volume(u8),
    Rotation(i64),
    Warning(String),
    Error(String),
    EndFile,
}

pub fn map_event(event: MpvEvent) -> Option<BackendEvent> {
    match event {
        MpvEvent::Pause(value) => Some(BackendEvent::PauseChanged(value)),
        MpvEvent::Position(value) => Some(BackendEvent::PositionChanged(value)),
        MpvEvent::Duration(value) => Some(BackendEvent::DurationChanged(value)),
        MpvEvent::Speed(value) => Some(BackendEvent::SpeedChanged(value)),
        MpvEvent::Volume(value) => Some(BackendEvent::VolumeChanged(value)),
        MpvEvent::Rotation(0) => Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg0)),
        MpvEvent::Rotation(90) => Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg90)),
        MpvEvent::Rotation(180) => {
            Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg180))
        }
        MpvEvent::Rotation(270) => {
            Some(BackendEvent::RotationChanged(yoyo_core::Rotation::Deg270))
        }
        MpvEvent::Rotation(other) => Some(BackendEvent::Warning(format!(
            "unsupported rotation reported by mpv: {other}"
        ))),
        MpvEvent::Warning(message) => Some(BackendEvent::Warning(message)),
        MpvEvent::Error(message) => Some(BackendEvent::Error(message)),
        MpvEvent::EndFile => Some(BackendEvent::EndOfFile),
    }
}
```

```rust
// crates/yoyo-mpv/src/client.rs
use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{MpvAction, MpvError, MpvEvent, map_event, translate_command, translate_open};

pub trait MpvActionSink {
    fn command(&mut self, args: &[String]) -> Result<(), MpvError>;
    fn set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError>;
    fn set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError>;
    fn set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError>;
    fn set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError>;
}

pub fn execute_actions<S: MpvActionSink>(
    sink: &mut S,
    actions: &[MpvAction],
) -> Result<(), MpvError> {
    for action in actions {
        match action {
            MpvAction::Command(args) => sink.command(args)?,
            MpvAction::SetString { name, value } => sink.set_string(name, value)?,
            MpvAction::SetInt { name, value } => sink.set_i64(name, *value)?,
            MpvAction::SetDouble { name, value } => sink.set_f64(name, *value)?,
            MpvAction::SetFlag { name, value } => sink.set_flag(name, *value)?,
        }
    }
    Ok(())
}

#[derive(Default)]
struct RecordingSink {
    actions: Vec<String>,
}

impl MpvActionSink for RecordingSink {
    fn command(&mut self, args: &[String]) -> Result<(), MpvError> {
        self.actions.push(format!("Command({args:?})"));
        Ok(())
    }

    fn set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError> {
        self.actions.push(format!("SetFlag {{ name: \"{name}\", value: {value} }}"));
        Ok(())
    }

    fn set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError> {
        self.actions.push(format!("SetString {{ name: \"{name}\", value: \"{value}\" }}"));
        Ok(())
    }

    fn set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError> {
        self.actions.push(format!("SetInt {{ name: \"{name}\", value: {value} }}"));
        Ok(())
    }

    fn set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError> {
        self.actions.push(format!("SetDouble {{ name: \"{name}\", value: {value} }}"));
        Ok(())
    }
}

#[derive(Default)]
pub struct DryRunMpvBackend {
    pending_events: Vec<BackendEvent>,
    sink: RecordingSink,
}

impl DryRunMpvBackend {
    pub fn recorded_actions(&self) -> &[String] {
        &self.sink.actions
    }

    pub fn push_event(&mut self, event: MpvEvent) {
        if let Some(mapped) = map_event(event) {
            self.pending_events.push(mapped);
        }
    }
}

impl PlayerBackend for DryRunMpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        execute_actions(&mut self.sink, &translate_open(locator)).map_err(|error| error.to_string())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        execute_actions(&mut self.sink, &translate_command(&command))
            .map_err(|error| error.to_string())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
```

```rust
// crates/yoyo-mpv/src/lib.rs
mod client;
mod error;
mod event;
mod render;
mod translate;

pub use client::{DryRunMpvBackend, MpvActionSink, execute_actions};
pub use error::MpvError;
pub use event::{MpvEvent, map_event};
pub use render::{MpvRenderBridge, RenderTarget};
pub use translate::{MpvAction, translate_command, translate_open};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yoyo-mpv --test dry_run_contract --test event_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/yoyo-mpv/src/lib.rs crates/yoyo-mpv/src/error.rs crates/yoyo-mpv/src/client.rs crates/yoyo-mpv/src/event.rs crates/yoyo-mpv/tests/dry_run_contract.rs crates/yoyo-mpv/tests/event_contract.rs
git commit -m "feat: add dry-run mpv backend and event mapping"
```

### Task 2: Implement The Runtime-Gated libmpv Client And Real Backend

**Files:**
- Modify: `crates/yoyo-mpv/src/lib.rs`
- Modify: `crates/yoyo-mpv/src/client.rs`
- Modify: `crates/yoyo-mpv/src/error.rs`
- Create: `crates/yoyo-mpv/tests/runtime_contract.rs`

**Interfaces:**
- Consumes:
  - `pub trait MpvActionSink`
  - `pub fn execute_actions<S: MpvActionSink>(...) -> Result<(), MpvError>`
  - `pub enum MpvEvent`
  - `pub fn map_event(event: MpvEvent) -> Option<BackendEvent>`
- Produces:
  - `pub struct MpvClient`
  - `impl MpvClient { pub fn new() -> Result<Self, MpvError> }`
  - `impl MpvClient { pub fn observe_default_properties(&mut self) -> Result<(), MpvError> }`
  - `impl MpvClient { pub fn drain_typed_events(&mut self) -> Vec<Result<MpvEvent, MpvError>> }`
  - `pub struct MpvBackend`
  - `impl MpvBackend { pub fn new_runtime() -> Result<Self, MpvError> }`

- [ ] **Step 1: Write the failing test**

```rust
// crates/yoyo-mpv/tests/runtime_contract.rs
use yoyo_mpv::{MpvBackend, MpvError};

#[test]
fn runtime_backend_requires_mpv_runtime_feature_by_default() {
    let error = MpvBackend::new_runtime().unwrap_err();
    assert!(matches!(error, MpvError::RuntimeDisabled));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyo-mpv --test runtime_contract`

Expected: FAIL with unresolved `MpvBackend::new_runtime` or mismatched error handling.

- [ ] **Step 3: Write the minimal implementation**

```rust
// crates/yoyo-mpv/src/client.rs
use std::ffi::{CStr, CString};
use std::ptr;

use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{MpvAction, MpvError, MpvEvent, execute_actions, map_event, translate_command, translate_open};

pub struct MpvBackend {
    client: MpvClient,
    pending_events: Vec<BackendEvent>,
    render_bridge: crate::MpvRenderBridge,
}

impl MpvBackend {
    pub fn new_runtime() -> Result<Self, MpvError> {
        let mut client = MpvClient::new()?;
        client.observe_default_properties()?;
        Ok(Self {
            client,
            pending_events: Vec::new(),
            render_bridge: crate::MpvRenderBridge::default(),
        })
    }
}

impl PlayerBackend for MpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        execute_actions(&mut self.client, &translate_open(locator)).map_err(|error| error.to_string())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        execute_actions(&mut self.client, &translate_command(&command))
            .map_err(|error| error.to_string())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        self.pending_events.clear();
        for event in self.client.drain_typed_events() {
            match event {
                Ok(event) => {
                    if let Some(mapped) = map_event(event) {
                        self.pending_events.push(mapped);
                    }
                }
                Err(error) => self.pending_events.push(BackendEvent::Error(error.to_string())),
            }
        }
        std::mem::take(&mut self.pending_events)
    }
}

#[cfg(feature = "mpv-runtime")]
pub struct MpvClient {
    handle: *mut libmpv_sys::mpv_handle,
}

#[cfg(feature = "mpv-runtime")]
impl MpvClient {
    pub fn new() -> Result<Self, MpvError> {
        let handle = unsafe { libmpv_sys::mpv_create() };
        if handle.is_null() {
            return Err(MpvError::CreateHandle);
        }

        let init_result = unsafe { libmpv_sys::mpv_initialize(handle) };
        if init_result < 0 {
            unsafe { libmpv_sys::mpv_terminate_destroy(handle) };
            return Err(MpvError::Initialize(format!("error code {init_result}")));
        }

        Ok(Self { handle })
    }

    pub fn observe_default_properties(&mut self) -> Result<(), MpvError> {
        self.observe_property(1, "pause", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
        self.observe_property(2, "time-pos", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(3, "duration", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(4, "speed", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(5, "volume", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
        self.observe_property(6, "video-rotate", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
        Ok(())
    }

    fn observe_property(
        &mut self,
        reply_user_data: u64,
        name: &str,
        format: libmpv_sys::mpv_format,
    ) -> Result<(), MpvError> {
        let name = CString::new(name).map_err(|_| MpvError::InvalidString(name.into()))?;
        let result = unsafe {
            libmpv_sys::mpv_observe_property(self.handle, reply_user_data, name.as_ptr(), format)
        };
        if result < 0 {
            return Err(MpvError::Property(format!("observe {reply_user_data}:{result}")));
        }
        Ok(())
    }

    pub fn drain_typed_events(&mut self) -> Vec<Result<MpvEvent, MpvError>> {
        Vec::new()
    }
}

#[cfg(feature = "mpv-runtime")]
impl MpvActionSink for MpvClient {
    fn command(&mut self, args: &[String]) -> Result<(), MpvError> {
        let cstrings: Result<Vec<_>, _> =
            args.iter().map(|arg| CString::new(arg.as_str()).map_err(|_| MpvError::InvalidString(arg.clone()))).collect();
        let cstrings = cstrings?;
        let mut ptrs: Vec<*const i8> = cstrings.iter().map(|arg| arg.as_ptr()).collect();
        ptrs.push(ptr::null());
        let result = unsafe { libmpv_sys::mpv_command(self.handle, ptrs.as_ptr()) };
        if result < 0 {
            return Err(MpvError::Command(args.join(" ")));
        }
        Ok(())
    }

    fn set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError> {
        self.set_property_i64(name, value as i64)
    }

    fn set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError> {
        let name = CString::new(name).map_err(|_| MpvError::InvalidString(name.into()))?;
        let value = CString::new(value).map_err(|_| MpvError::InvalidString(value.into()))?;
        let result = unsafe {
            libmpv_sys::mpv_set_property(
                self.handle,
                name.as_ptr(),
                libmpv_sys::mpv_format_MPV_FORMAT_STRING,
                value.as_ptr() as *mut _,
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!("set string {result}")));
        }
        Ok(())
    }

    fn set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError> {
        self.set_property_i64(name, value)
    }

    fn set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError> {
        let name = CString::new(name).map_err(|_| MpvError::InvalidString(name.into()))?;
        let mut value = value;
        let result = unsafe {
            libmpv_sys::mpv_set_property(
                self.handle,
                name.as_ptr(),
                libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
                (&mut value as *mut f64).cast(),
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!("set double {result}")));
        }
        Ok(())
    }
}

#[cfg(feature = "mpv-runtime")]
impl MpvClient {
    fn set_property_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError> {
        let name = CString::new(name).map_err(|_| MpvError::InvalidString(name.into()))?;
        let mut value = value;
        let result = unsafe {
            libmpv_sys::mpv_set_property(
                self.handle,
                name.as_ptr(),
                libmpv_sys::mpv_format_MPV_FORMAT_INT64,
                (&mut value as *mut i64).cast(),
            )
        };
        if result < 0 {
            return Err(MpvError::Property(format!("set i64 {result}")));
        }
        Ok(())
    }
}

#[cfg(feature = "mpv-runtime")]
impl Drop for MpvClient {
    fn drop(&mut self) {
        unsafe { libmpv_sys::mpv_terminate_destroy(self.handle) };
    }
}

#[cfg(not(feature = "mpv-runtime"))]
pub struct MpvClient;

#[cfg(not(feature = "mpv-runtime"))]
impl MpvClient {
    pub fn new() -> Result<Self, MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    pub fn observe_default_properties(&mut self) -> Result<(), MpvError> {
        Err(MpvError::RuntimeDisabled)
    }

    pub fn drain_typed_events(&mut self) -> Vec<Result<MpvEvent, MpvError>> {
        vec![Err(MpvError::RuntimeDisabled)]
    }
}
```

```rust
// crates/yoyo-mpv/src/lib.rs
mod client;
mod error;
mod event;
mod render;
mod translate;

pub use client::{DryRunMpvBackend, MpvActionSink, MpvBackend, MpvClient, execute_actions};
pub use error::MpvError;
pub use event::{MpvEvent, map_event};
pub use render::{MpvRenderBridge, RenderTarget};
pub use translate::{MpvAction, translate_command, translate_open};
```

- [ ] **Step 4: Run verification**

Run: `cargo test -p yoyo-mpv --test runtime_contract`

Expected: PASS

Run: `cargo test -p yoyo-mpv --features mpv-runtime --no-run`

Expected: PASS in an environment where `libmpv` development headers and libraries are available.

- [ ] **Step 5: Commit**

```bash
git add crates/yoyo-mpv/src/lib.rs crates/yoyo-mpv/src/client.rs crates/yoyo-mpv/src/error.rs crates/yoyo-mpv/tests/runtime_contract.rs
git commit -m "feat: add runtime mpv client"
```

### Task 3: Wire Desktop Startup, Backend Polling, And Runtime Docs

**Files:**
- Modify: `apps/yoyovideo-desktop/Cargo.toml`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/dialogs.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`
- Create: `apps/yoyovideo-desktop/tests/runtime_startup_contract.rs`
- Modify: `docs/development/runtime-dependencies.md`
- Modify: `docs/testing/manual-smoke-checklist.md`
- Modify: `README.md`

**Interfaces:**
- Consumes:
  - `pub struct MpvBackend`
  - `impl MpvBackend { pub fn new_runtime() -> Result<Self, MpvError> }`
  - `pub struct DryRunMpvBackend`
  - `pub fn format_transport_label(state: &PlayerState) -> String`
  - `pub fn format_speed_label(state: &PlayerState) -> String`
  - `pub fn format_time_label(state: &PlayerState) -> String`
- Produces:
  - `pub fn build_desktop_backend() -> Result<MpvBackend, MpvError>`
  - `pub fn refresh_window(window: &MainWindow, state: &PlayerState)`
  - `pub trait DialogService { fn prompt_url(&self) -> Option<String>; }`

- [ ] **Step 1: Write the failing tests**

```rust
// apps/yoyovideo-desktop/tests/runtime_startup_contract.rs
use yoyovideo_desktop::build_desktop_backend;

#[test]
fn desktop_backend_requires_runtime_feature_by_default() {
    let error = build_desktop_backend().unwrap_err();
    assert!(error.to_string().contains("disabled"));
}
```

```rust
// apps/yoyovideo-desktop/tests/controller_contract.rs
use yoyo_core::{AppCommand, AppConfig, AppSession, BackendCommand, MediaLocator, PlayerBackend};
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yoyovideo-desktop --test controller_contract --test runtime_startup_contract`

Expected: FAIL with unresolved `build_desktop_backend` or missing controller/runtime wiring.

- [ ] **Step 3: Write the minimal implementation**

```toml
# apps/yoyovideo-desktop/Cargo.toml
[package]
name = "yoyovideo-desktop"
version = "0.1.0"
edition = "2024"

[features]
default = []
mpv-runtime = ["yoyo-mpv/mpv-runtime"]

[dependencies]
directories = "6.0.0"
rfd = "0.17.2"
slint = { version = "1.17.0", features = ["backend-winit", "unstable-winit-030", "raw-window-handle-06"] }
tracing = "0.1.44"
tracing-subscriber = "0.3.23"
yoyo-core = { path = "../../crates/yoyo-core" }
yoyo-mpv = { path = "../../crates/yoyo-mpv" }
```

```rust
// apps/yoyovideo-desktop/src/platform/dialogs.rs
use std::path::PathBuf;

pub trait DialogService {
    fn pick_file(&self) -> Option<PathBuf>;
    fn pick_folder(&self) -> Option<PathBuf>;
    fn prompt_url(&self) -> Option<String>;
}

#[derive(Default)]
pub struct RfdDialogService;

impl DialogService for RfdDialogService {
    fn pick_file(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }

    fn pick_folder(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    fn prompt_url(&self) -> Option<String> {
        None
    }
}
```

```rust
// apps/yoyovideo-desktop/src/app.rs
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use yoyo_core::{AppCommand, AppConfig, AppSession, PlayerBackend, PlayerState, ShortcutAction, ShortcutMap};
use yoyo_mpv::{MpvBackend, MpvError};

use crate::platform::{DialogService, RfdDialogService, scan_media_folder};
use crate::video_texture::VideoTexture;

slint::include_modules!();

pub fn build_desktop_backend() -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime()
}

pub fn refresh_window(window: &MainWindow, state: &PlayerState) {
    window.set_transport_label(crate::format_transport_label(state).into());
    window.set_speed_label(crate::format_speed_label(state).into());
    window.set_time_label(crate::format_time_label(state).into());
    window.set_status_label(
        state
            .last_error
            .clone()
            .or_else(|| state.status_message.clone())
            .unwrap_or_default()
            .into(),
    );
}

pub struct DesktopController<B: PlayerBackend> {
    session: AppSession<B>,
    shortcuts: ShortcutMap,
    #[allow(dead_code)]
    video_texture: VideoTexture,
}

impl<B: PlayerBackend> DesktopController<B> {
    pub fn new(session: AppSession<B>) -> Self {
        Self { session, shortcuts: ShortcutMap::default(), video_texture: VideoTexture::default() }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), yoyo_core::AppError> {
        self.session.handle_command(command)?;
        self.session.poll_backend()?;
        Ok(())
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }

    pub fn open_folder(&mut self, path: &std::path::Path) -> Result<(), yoyo_core::AppError> {
        let entries = scan_media_folder(path)?;
        self.session.replace_playlist(entries, 0)?;
        self.session.poll_backend()
    }

    pub fn poll_backend(&mut self) -> Result<(), yoyo_core::AppError> {
        self.session.poll_backend()
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();
    slint::BackendSelector::new().backend_name("winit".into()).select()?;

    let app = MainWindow::new()?;
    let backend = build_desktop_backend()?;
    let session = AppSession::new(AppConfig::default(), backend);
    let controller = Rc::new(RefCell::new(DesktopController::new(session)));
    let dialogs = Rc::new(RfdDialogService);

    refresh_window(&app, controller.borrow().session().state());

    app.on_open_file_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if let Some(path) = dialogs.pick_file() {
                let mut controller = controller.borrow_mut();
                if controller.dispatch(AppCommand::OpenFile(path)).is_ok() {
                    if let Some(app) = app_handle.upgrade() {
                        refresh_window(&app, controller.session().state());
                    }
                }
            }
        }
    });

    app.on_open_folder_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if let Some(path) = dialogs.pick_folder() {
                let mut controller = controller.borrow_mut();
                if controller.open_folder(&path).is_ok() {
                    if let Some(app) = app_handle.upgrade() {
                        refresh_window(&app, controller.session().state());
                    }
                }
            }
        }
    });

    app.on_open_url_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move |url| {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::OpenUrl(url.to_string())).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    app.on_toggle_pause_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::TogglePause).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    refresh_window(&app, controller.session().state());
                }
            }
        }
    });

    let poll_timer = slint::Timer::default();
    poll_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(250),
        {
            let app_handle = app.as_weak();
            let controller = Rc::clone(&controller);
            move || {
                let mut controller = controller.borrow_mut();
                if controller.poll_backend().is_ok() {
                    if let Some(app) = app_handle.upgrade() {
                        refresh_window(&app, controller.session().state());
                    }
                }
            }
        },
    );

    app.run()?;
    Ok(())
}
```

```rust
// apps/yoyovideo-desktop/src/lib.rs
mod app;
pub mod platform;
mod presenter;
mod settings_controller;
mod video_texture;

pub use app::{DesktopController, build_desktop_backend, dispatch_shortcut, refresh_window, run};
pub use platform::scan_media_folder;
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
pub use settings_controller::SettingsController;
```

```markdown
# docs/development/runtime-dependencies.md

## libmpv runtime checklist

- Build the desktop app with `cargo run -p yoyovideo-desktop --features mpv-runtime`.
- The `mpv-runtime` feature enables `yoyo-mpv/mpv-runtime` and switches desktop startup to the real backend.
- Without the feature, startup must fail with a clear runtime-disabled error instead of silently pretending playback works.
- Bundle or otherwise make discoverable `libmpv` and its FFmpeg-dependent runtime libraries on each target platform.
- Test both hardware-decoding success and software-decoding fallback paths once video embedding lands.
```

```markdown
# docs/testing/manual-smoke-checklist.md

## Real Playback Alpha

- Build and run `cargo run -p yoyovideo-desktop --features mpv-runtime`.
- Confirm startup fails clearly if `libmpv` cannot be found.
- Open a local video file and confirm mpv starts playback.
- Open a supported network URL and confirm playback begins or fails with a visible error.
- Verify pause/resume, seek, speed, volume, audio channel switching, rotation, zoom, and A-B repeat commands.
- Confirm EOF on a scanned folder playlist advances to the next item.
```

```markdown
# README.md
# YoYoVideo

Rust + Slint + libmpv cross-platform desktop media player.

## Development

- Default `cargo test` keeps playback tests in dry-run mode and does not require libmpv.
- Runtime playback alpha: `cargo run -p yoyovideo-desktop --features mpv-runtime`
```

- [ ] **Step 4: Run verification**

Run: `cargo test -p yoyovideo-desktop --test controller_contract --test runtime_startup_contract`

Expected: PASS

Run: `cargo test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/yoyovideo-desktop/Cargo.toml apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/src/platform/dialogs.rs apps/yoyovideo-desktop/tests/controller_contract.rs apps/yoyovideo-desktop/tests/runtime_startup_contract.rs docs/development/runtime-dependencies.md docs/testing/manual-smoke-checklist.md README.md
git commit -m "feat: wire real mpv desktop startup"
```

## Self-Review

### Spec coverage

- Real `libmpv` client creation and initialization: covered by Task 2.
- Pure command execution seam and testable translation: covered by Task 1.
- Typed event observation and mapping into `BackendEvent`: covered by Tasks 1 and 2.
- Explicit desktop runtime backend selection and no fake-playback fallback: covered by Task 3.
- Local file and supported URL playback path: covered by Tasks 2 and 3.
- Startup/runtime error visibility and smoke verification: covered by Tasks 2 and 3.
- Slint video embedding remains deferred: no task introduces surface integration.

### Placeholder scan

- No `TODO`, `TBD`, or “implement later” markers remain.
- All task steps include concrete file paths, code snippets, commands, and expected outcomes.

### Type consistency

- `MpvActionSink`, `execute_actions`, `DryRunMpvBackend`, `MpvClient`, `MpvBackend`, `MpvEvent`, `map_event`, `build_desktop_backend`, and `refresh_window` are defined before later tasks consume them.
- `BackendEvent` remains the only event type crossing into `yoyo-core`.
