# YoYoVideo Rust + Slint + libmpv Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a near-production cross-platform desktop video player that uses Rust + Slint + libmpv and supports local files, folders, network URLs, playback controls, playlist/history, configurable shortcuts, and hardware-accelerated playback with stable fallback behavior.

**Architecture:** Create a Rust workspace with a pure domain crate (`yoyo-core`), an mpv adapter crate (`yoyo-mpv`), and a Slint desktop shell (`yoyovideo-desktop`). Keep playback commands, playlist logic, history, shortcut parsing, and error handling inside testable Rust modules; isolate all libmpv FFI and OpenGL render integration behind thin adapters so UI code only consumes typed commands and state updates.

**Tech Stack:** Rust 1.94.1, Cargo workspaces, Slint 1.17.0 (`unstable-winit-030`, `raw-window-handle-06`, `backend-winit`), libmpv via `libmpv-sys 3.1.0`, serde 1.0.228, serde_json 1.0.150, toml 1.1.2+spec-1.1.0, url 2.5.8, thiserror 2.0.18, directories 6.0.0, rfd 0.17.2, tracing 0.1.44, tracing-subscriber 0.3.23, tempfile 3.27.0.

## Global Constraints

- Support Windows, macOS, and Linux desktop platforms.
- Use Rust for the application shell and playback orchestration, Slint for native desktop UI, and libmpv as the embedded playback engine.
- Keep memory usage and runtime overhead low by using native UI and an embedded native playback engine.
- Support local media files, local folders, playlists, recent history, and network stream URLs.
- Support playback, pause, seeking, speed control, volume control, video zoom, audio channel switching, video rotation, A-B repeat, full screen, and keyboard shortcuts.
- The first release targets a near-production MVP rather than a throwaway prototype.
- The UI follows a simple player layout with professional controls available on demand.
- The first release should avoid decoding frames manually and avoid copying video frames through the UI layer.
- Hardware acceleration is best-effort.
- Configuration is stored in the system configuration directory, not beside the executable.
- The libmpv runtime and its dependent libraries must be bundled or discovered deterministically per platform.
- The app should not rely on users manually installing mpv unless a developer-mode flag explicitly enables system libmpv loading.
- Do not implement advanced subtitle styling, video filters, recording, screen capture, plugin scripting, or full PotPlayer feature parity in this plan.

---

## Planned File Structure

`Cargo.toml`
- Workspace root. Starts with `yoyo-core`, then expands to include `yoyo-mpv` and `yoyovideo-desktop`.

`rustfmt.toml`
- Shared formatting settings so generated modules stay consistent across tasks.

`crates/yoyo-core/Cargo.toml`
- Pure Rust domain crate dependencies.

`crates/yoyo-core/src/lib.rs`
- Re-export stable public types for the rest of the workspace.

`crates/yoyo-core/src/app_command.rs`
- Typed user intent and transport command enums.

`crates/yoyo-core/src/media.rs`
- `MediaLocator`, URL validation, folder scan filters, and display labels.

`crates/yoyo-core/src/player_state.rs`
- Player state snapshot, rotation/audio/loop enums, and defaults.

`crates/yoyo-core/src/error.rs`
- App, validation, storage, and backend-facing error types.

`crates/yoyo-core/src/config.rs`
- `AppConfig`, playback defaults, UI preferences, load/save helpers.

`crates/yoyo-core/src/history.rs`
- Recent items and resume-position persistence helpers.

`crates/yoyo-core/src/playlist.rs`
- Playlist item model and next/previous navigation.

`crates/yoyo-core/src/shortcut.rs`
- Shortcut parsing, serialization, conflict detection, and defaults.

`crates/yoyo-core/src/backend.rs`
- `PlayerBackend` trait, backend command/event enums, and typed backend capabilities.

`crates/yoyo-core/src/session.rs`
- `AppSession` command dispatcher that mutates state and talks to a `PlayerBackend`.

`crates/yoyo-core/tests/*.rs`
- Contract tests for state defaults, storage, playlist behavior, shortcut parsing, and session command mapping.

`crates/yoyo-mpv/Cargo.toml`
- libmpv adapter crate dependencies and feature flags.

`crates/yoyo-mpv/src/lib.rs`
- Public exports for `MpvBackend`, `MpvAction`, and render bridge types.

`crates/yoyo-mpv/src/error.rs`
- mpv-specific error types.

`crates/yoyo-mpv/src/translate.rs`
- Maps `yoyo-core` backend commands/events to raw mpv actions.

`crates/yoyo-mpv/src/client.rs`
- Safe wrapper around `libmpv-sys`, wakeup handling, and property observation.

`crates/yoyo-mpv/src/render.rs`
- OpenGL render API bridge for mpv-to-Slint texture rendering.

`crates/yoyo-mpv/tests/*.rs`
- Translation and render-bridge contract tests with fake sinks.

`apps/yoyovideo-desktop/Cargo.toml`
- Slint desktop application package.

`apps/yoyovideo-desktop/build.rs`
- Compiles `.slint` UI at build time.

`apps/yoyovideo-desktop/ui/main-window.slint`
- Main window layout, bottom controls, URL entry, context menu area, and settings drawer shell.

`apps/yoyovideo-desktop/src/lib.rs`
- Public desktop application entry points for testing and `main.rs`.

`apps/yoyovideo-desktop/src/main.rs`
- Process bootstrap, logging, backend selection, and `run()` call.

`apps/yoyovideo-desktop/src/app.rs`
- Top-level application controller that wires UI callbacks to `AppSession`.

`apps/yoyovideo-desktop/src/presenter.rs`
- Maps `PlayerState` to UI-friendly strings and scalar values.

`apps/yoyovideo-desktop/src/video_texture.rs`
- Manages the OpenGL texture handle exposed to Slint.

`apps/yoyovideo-desktop/src/settings_controller.rs`
- Validates editable settings and shortcut changes before saving.

`apps/yoyovideo-desktop/src/platform/mod.rs`
- Re-export platform helpers.

`apps/yoyovideo-desktop/src/platform/dialogs.rs`
- File/folder/URL dialog adapters.

`apps/yoyovideo-desktop/src/platform/paths.rs`
- System config/cache/data path resolution.

`apps/yoyovideo-desktop/src/platform/media_scan.rs`
- Folder scanning and supported-extension filtering for local playlists.

`apps/yoyovideo-desktop/tests/*.rs`
- Presenter, controller, media scan, and settings contract tests.

`docs/development/runtime-dependencies.md`
- Runtime bundling and licensing checklist for libmpv distribution.

`docs/testing/manual-smoke-checklist.md`
- Cross-platform manual validation matrix for MVP release.

### Task 1: Bootstrap Workspace And Shared Domain Types

**Files:**
- Create: `Cargo.toml`
- Create: `rustfmt.toml`
- Create: `crates/yoyo-core/Cargo.toml`
- Create: `crates/yoyo-core/src/lib.rs`
- Create: `crates/yoyo-core/src/app_command.rs`
- Create: `crates/yoyo-core/src/media.rs`
- Create: `crates/yoyo-core/src/player_state.rs`
- Create: `crates/yoyo-core/src/error.rs`
- Create: `crates/yoyo-core/tests/command_contract.rs`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `pub enum AppCommand`
  - `pub enum AudioChannelMode`
  - `pub enum Rotation`
  - `pub struct LoopState`
  - `pub struct PlayerState`
  - `pub enum MediaLocator`
  - `impl MediaLocator { pub fn as_label(&self) -> String }`
  - `pub enum AppError`

- [ ] **Step 1: Write the failing test**

```rust
// crates/yoyo-core/tests/command_contract.rs
use std::path::PathBuf;

use yoyo_core::{AppCommand, AudioChannelMode, LoopState, MediaLocator, PlayerState, Rotation};

#[test]
fn default_player_state_is_safe_for_empty_launch() {
    let state = PlayerState::default();

    assert!(state.current.is_none());
    assert!(state.paused);
    assert_eq!(state.speed, 1.0);
    assert_eq!(state.volume_percent, 100);
    assert_eq!(state.audio_channel, AudioChannelMode::Stereo);
    assert_eq!(state.rotation, Rotation::Deg0);
    assert_eq!(state.loop_state, LoopState::default());
}

#[test]
fn open_file_command_carries_target_path() {
    let path = PathBuf::from("demo.mp4");
    let command = AppCommand::OpenFile(path.clone());

    match command {
        AppCommand::OpenFile(actual) => assert_eq!(actual, path),
        other => panic!("expected OpenFile, got {other:?}"),
    }
}

#[test]
fn media_locator_label_round_trip() {
    let locator = MediaLocator::Url("https://example.com/live.m3u8".to_string());
    assert_eq!(locator.as_label(), "https://example.com/live.m3u8");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyo-core --test command_contract`

Expected: FAIL with a workspace/package error because `yoyo-core` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

```toml
# Cargo.toml
[workspace]
members = ["crates/yoyo-core"]
resolver = "2"
```

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
use_small_heuristics = "Max"
```

```toml
# crates/yoyo-core/Cargo.toml
[package]
name = "yoyo-core"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1.0.228", features = ["derive"] }
thiserror = "2.0.18"
```

```rust
// crates/yoyo-core/src/lib.rs
mod app_command;
mod error;
mod media;
mod player_state;

pub use app_command::AppCommand;
pub use error::AppError;
pub use media::MediaLocator;
pub use player_state::{AudioChannelMode, LoopState, PlayerState, Rotation};
```

```rust
// crates/yoyo-core/src/app_command.rs
use std::path::PathBuf;

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
}
```

```rust
// crates/yoyo-core/src/media.rs
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaLocator {
    File(PathBuf),
    Url(String),
}

impl MediaLocator {
    pub fn as_label(&self) -> String {
        match self {
            Self::File(path) => path.display().to_string(),
            Self::Url(url) => url.clone(),
        }
    }
}
```

```rust
// crates/yoyo-core/src/player_state.rs
use serde::{Deserialize, Serialize};

use crate::MediaLocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioChannelMode {
    Stereo,
    MonoLeft,
    MonoRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct LoopState {
    pub point_a: Option<f64>,
    pub point_b: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub current: Option<MediaLocator>,
    pub paused: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub volume_percent: u8,
    pub speed: f32,
    pub audio_channel: AudioChannelMode,
    pub rotation: Rotation,
    pub zoom_step: i8,
    pub loop_state: LoopState,
    pub fullscreen: bool,
    pub status_message: Option<String>,
    pub last_error: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current: None,
            paused: true,
            position_seconds: 0.0,
            duration_seconds: None,
            volume_percent: 100,
            speed: 1.0,
            audio_channel: AudioChannelMode::Stereo,
            rotation: Rotation::Deg0,
            zoom_step: 0,
            loop_state: LoopState::default(),
            fullscreen: false,
            status_message: None,
            last_error: None,
        }
    }
}
```

```rust
// crates/yoyo-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyo-core --test command_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rustfmt.toml crates/yoyo-core
git commit -m "feat: bootstrap core workspace types"
```

### Task 2: Add Config, History, Playlist, Media Scan, And Shortcut Models

**Files:**
- Modify: `crates/yoyo-core/Cargo.toml`
- Modify: `crates/yoyo-core/src/lib.rs`
- Modify: `crates/yoyo-core/src/error.rs`
- Modify: `crates/yoyo-core/src/media.rs`
- Create: `crates/yoyo-core/src/config.rs`
- Create: `crates/yoyo-core/src/history.rs`
- Create: `crates/yoyo-core/src/playlist.rs`
- Create: `crates/yoyo-core/src/shortcut.rs`
- Create: `crates/yoyo-core/tests/storage_contract.rs`

**Interfaces:**
- Consumes:
  - `MediaLocator`
  - `AudioChannelMode`
  - `Rotation`
- Produces:
  - `pub struct AppConfig`
  - `pub struct PlaybackDefaults`
  - `pub struct UiPreferences`
  - `pub struct HistoryStore`
  - `pub struct HistoryEntry`
  - `pub struct PlaylistEntry`
  - `pub struct Playlist`
  - `pub enum ShortcutAction`
  - `pub struct Shortcut`
  - `pub struct ShortcutMap`
  - `impl MediaLocator { pub fn from_url(input: &str) -> Result<Self, ValidationError> }`
  - `impl MediaLocator { pub fn is_supported_local_path(path: &Path) -> bool }`

- [ ] **Step 1: Write the failing test**

```rust
// crates/yoyo-core/tests/storage_contract.rs
use std::path::PathBuf;

use tempfile::tempdir;
use yoyo_core::{
    AppConfig, HistoryEntry, HistoryStore, MediaLocator, Playlist, PlaylistEntry, Shortcut,
    ShortcutAction, ShortcutMap, ValidationError,
};

#[test]
fn invalid_url_is_rejected_before_backend_open() {
    let error = MediaLocator::from_url("notaurl").unwrap_err();
    assert!(matches!(error, ValidationError::InvalidUrl(_)));
}

#[test]
fn playlist_next_advances_current_item() {
    let mut playlist = Playlist::default();
    playlist.replace(
        vec![
            PlaylistEntry::new(MediaLocator::File(PathBuf::from("a.mp4"))),
            PlaylistEntry::new(MediaLocator::File(PathBuf::from("b.mp4"))),
        ],
        0,
    );

    let next = playlist.next().expect("next item");
    assert_eq!(next.locator, MediaLocator::File(PathBuf::from("b.mp4")));
}

#[test]
fn default_shortcuts_include_required_bindings() {
    let shortcut = Shortcut::parse("Space").unwrap();
    let map = ShortcutMap::default();
    assert_eq!(map.action_for(&shortcut), Some(ShortcutAction::TogglePause));
}

#[test]
fn history_round_trip_preserves_resume_position() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");

    let mut history = HistoryStore::default();
    history.items.push(HistoryEntry {
        locator: MediaLocator::File(PathBuf::from("movie.mkv")),
        title: "movie.mkv".to_string(),
        last_position_seconds: Some(84.0),
    });

    history.save(&path).unwrap();
    let loaded = HistoryStore::load(&path).unwrap();

    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items[0].last_position_seconds, Some(84.0));
}

#[test]
fn config_round_trip_keeps_speed_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let config = AppConfig::default();
    config.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(loaded.playback.default_speed, 1.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyo-core --test storage_contract`

Expected: FAIL with unresolved imports such as `AppConfig`, `HistoryStore`, `ShortcutMap`, or `ValidationError`.

- [ ] **Step 3: Write minimal implementation**

```toml
# crates/yoyo-core/Cargo.toml
[package]
name = "yoyo-core"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
thiserror = "2.0.18"
toml = "1.1.2+spec-1.1.0"
url = "2.5.8"

[dev-dependencies]
tempfile = "3.27.0"
```

```rust
// crates/yoyo-core/src/lib.rs
mod app_command;
mod config;
mod error;
mod history;
mod media;
mod player_state;
mod playlist;
mod shortcut;

pub use app_command::AppCommand;
pub use config::{AppConfig, PlaybackDefaults, UiPreferences};
pub use error::{AppError, StorageError, ValidationError};
pub use history::{HistoryEntry, HistoryStore};
pub use media::MediaLocator;
pub use player_state::{AudioChannelMode, LoopState, PlayerState, Rotation};
pub use playlist::{Playlist, PlaylistEntry};
pub use shortcut::{Shortcut, ShortcutAction, ShortcutMap};
```

```rust
// crates/yoyo-core/src/error.rs
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("unsupported url scheme: {0}")]
    UnsupportedUrlScheme(String),
    #[error("unsupported media path: {0}")]
    UnsupportedMediaPath(PathBuf),
    #[error("invalid shortcut: {0}")]
    InvalidShortcut(String),
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("{0}")]
    Message(String),
}
```

```rust
// crates/yoyo-core/src/media.rs
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::ValidationError;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", "mp3", "flac", "wav", "m4a", "ts", "m2ts",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaLocator {
    File(PathBuf),
    Url(String),
}

impl MediaLocator {
    pub fn from_url(input: &str) -> Result<Self, ValidationError> {
        let parsed = Url::parse(input).map_err(|_| ValidationError::InvalidUrl(input.to_string()))?;
        match parsed.scheme() {
            "http" | "https" | "rtsp" | "rtmp" => Ok(Self::Url(input.to_string())),
            other => Err(ValidationError::UnsupportedUrlScheme(other.to_string())),
        }
    }

    pub fn is_supported_local_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
    }

    pub fn as_label(&self) -> String {
        match self {
            Self::File(path) => path.display().to_string(),
            Self::Url(url) => url.clone(),
        }
    }
}
```

```rust
// crates/yoyo-core/src/config.rs
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{ShortcutMap, StorageError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackDefaults {
    pub default_speed: f32,
    pub default_volume_percent: u8,
    pub prefer_hardware_decode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiPreferences {
    pub remember_history: bool,
    pub show_playlist_on_startup: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub playback: PlaybackDefaults,
    pub ui: UiPreferences,
    pub shortcuts: ShortcutMap,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            playback: PlaybackDefaults {
                default_speed: 1.0,
                default_volume_percent: 100,
                prefer_hardware_decode: true,
            },
            ui: UiPreferences {
                remember_history: true,
                show_playlist_on_startup: true,
            },
            shortcuts: ShortcutMap::default(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, StorageError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}
```

```rust
// crates/yoyo-core/src/history.rs
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{MediaLocator, StorageError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub locator: MediaLocator,
    pub title: String,
    pub last_position_seconds: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoryStore {
    pub items: Vec<HistoryEntry>,
}

impl HistoryStore {
    pub fn load(path: &Path) -> Result<Self, StorageError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}
```

```rust
// crates/yoyo-core/src/playlist.rs
use serde::{Deserialize, Serialize};

use crate::MediaLocator;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub locator: MediaLocator,
    pub title: String,
}

impl PlaylistEntry {
    pub fn new(locator: MediaLocator) -> Self {
        let title = locator.as_label();
        Self { locator, title }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub entries: Vec<PlaylistEntry>,
    pub current_index: Option<usize>,
}

impl Playlist {
    pub fn replace(&mut self, entries: Vec<PlaylistEntry>, start_index: usize) {
        self.entries = entries;
        self.current_index = (!self.entries.is_empty()).then_some(start_index);
    }

    pub fn current(&self) -> Option<&PlaylistEntry> {
        self.current_index.and_then(|index| self.entries.get(index))
    }

    pub fn next(&mut self) -> Option<&PlaylistEntry> {
        let next_index = self.current_index?.saturating_add(1);
        if next_index < self.entries.len() {
            self.current_index = Some(next_index);
            self.entries.get(next_index)
        } else {
            None
        }
    }

    pub fn previous(&mut self) -> Option<&PlaylistEntry> {
        let current = self.current_index?;
        if current > 0 {
            self.current_index = Some(current - 1);
            self.entries.get(current - 1)
        } else {
            None
        }
    }
}
```

```rust
// crates/yoyo-core/src/shortcut.rs
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Shortcut(String);

impl Shortcut {
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            return Err(ValidationError::InvalidShortcut(input.to_string()));
        }
        Ok(Self(normalized.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutMap {
    pub bindings: BTreeMap<Shortcut, ShortcutAction>,
}

impl Default for ShortcutMap {
    fn default() -> Self {
        let mut bindings = BTreeMap::new();
        bindings.insert(Shortcut("Space".into()), ShortcutAction::TogglePause);
        bindings.insert(Shortcut("Left".into()), ShortcutAction::SeekBackwardSmall);
        bindings.insert(Shortcut("Right".into()), ShortcutAction::SeekForwardSmall);
        bindings.insert(Shortcut("Up".into()), ShortcutAction::VolumeUp);
        bindings.insert(Shortcut("Down".into()), ShortcutAction::VolumeDown);
        bindings.insert(Shortcut("[".into()), ShortcutAction::SpeedDown);
        bindings.insert(Shortcut("]".into()), ShortcutAction::SpeedUp);
        bindings.insert(Shortcut("0".into()), ShortcutAction::ResetSpeed);
        bindings.insert(Shortcut("A".into()), ShortcutAction::SetABLoopPointA);
        bindings.insert(Shortcut("B".into()), ShortcutAction::SetABLoopPointB);
        bindings.insert(Shortcut("Ctrl+A".into()), ShortcutAction::ClearABLoop);
        bindings.insert(Shortcut("R".into()), ShortcutAction::RotateClockwise);
        bindings.insert(Shortcut("Z".into()), ShortcutAction::ZoomOut);
        bindings.insert(Shortcut("X".into()), ShortcutAction::ZoomIn);
        bindings.insert(Shortcut("C".into()), ShortcutAction::CycleAudioChannel);
        bindings.insert(Shortcut("F".into()), ShortcutAction::ToggleFullscreen);
        bindings.insert(Shortcut("O".into()), ShortcutAction::OpenFile);
        bindings.insert(Shortcut("U".into()), ShortcutAction::OpenUrl);
        Self { bindings }
    }
}

impl ShortcutMap {
    pub fn action_for(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        self.bindings.get(shortcut).copied()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyo-core --test storage_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/yoyo-core
git commit -m "feat: add core storage and shortcut models"
```

### Task 3: Add Backend Abstraction And App Session

**Files:**
- Modify: `crates/yoyo-core/src/lib.rs`
- Modify: `crates/yoyo-core/src/error.rs`
- Create: `crates/yoyo-core/src/backend.rs`
- Create: `crates/yoyo-core/src/session.rs`
- Create: `crates/yoyo-core/tests/session_contract.rs`

**Interfaces:**
- Consumes:
  - `AppCommand`
  - `AppConfig`
  - `HistoryStore`
  - `MediaLocator`
  - `PlayerState`
  - `Playlist`
  - `ShortcutAction`
- Produces:
  - `pub enum BackendCommand`
  - `pub enum BackendEvent`
  - `pub trait PlayerBackend`
  - `pub struct AppSession<B: PlayerBackend>`
  - `impl<B: PlayerBackend> AppSession<B> { pub fn handle_command(&mut self, command: AppCommand) -> Result<(), AppError> }`
  - `impl<B: PlayerBackend> AppSession<B> { pub fn poll_backend(&mut self) -> Result<(), AppError> }`
  - `impl<B: PlayerBackend> AppSession<B> { pub fn replace_playlist(&mut self, entries: Vec<PlaylistEntry>, start_index: usize) -> Result<(), AppError> }`

- [ ] **Step 1: Write the failing test**

```rust
// crates/yoyo-core/tests/session_contract.rs
use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, AudioChannelMode, BackendCommand, BackendEvent, MediaLocator,
    PlayerBackend, PlaylistEntry, Rotation,
};

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
    events: Vec<BackendEvent>,
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

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.events)
    }
}

#[test]
fn toggle_pause_emits_backend_pause_command() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::TogglePause).unwrap();

    assert_eq!(session.backend().commands, vec![BackendCommand::SetPaused(false)]);
    assert!(!session.state().paused);
}

#[test]
fn rotate_clockwise_cycles_to_ninety_degrees() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::RotateClockwise).unwrap();

    assert_eq!(session.state().rotation, Rotation::Deg90);
    assert_eq!(
        session.backend().commands,
        vec![BackendCommand::SetRotation(Rotation::Deg90)]
    );
}

#[test]
fn eof_event_opens_next_playlist_item() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("one.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("two.mp4"))),
            ],
            0,
        )
        .unwrap();

    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(
        session.backend().opened,
        vec![
            MediaLocator::File(PathBuf::from("one.mp4")),
            MediaLocator::File(PathBuf::from("two.mp4")),
        ]
    );
}

#[test]
fn cycle_audio_channel_visits_left_then_right() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::CycleAudioChannel).unwrap();
    session.handle_command(AppCommand::CycleAudioChannel).unwrap();

    assert_eq!(session.state().audio_channel, AudioChannelMode::MonoRight);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyo-core --test session_contract`

Expected: FAIL with unresolved imports such as `AppSession`, `BackendCommand`, or `PlayerBackend`.

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/yoyo-core/src/lib.rs
mod app_command;
mod backend;
mod config;
mod error;
mod history;
mod media;
mod player_state;
mod playlist;
mod session;
mod shortcut;

pub use app_command::AppCommand;
pub use backend::{BackendCommand, BackendEvent, PlayerBackend};
pub use config::{AppConfig, PlaybackDefaults, UiPreferences};
pub use error::{AppError, StorageError, ValidationError};
pub use history::{HistoryEntry, HistoryStore};
pub use media::MediaLocator;
pub use player_state::{AudioChannelMode, LoopState, PlayerState, Rotation};
pub use playlist::{Playlist, PlaylistEntry};
pub use session::AppSession;
pub use shortcut::{Shortcut, ShortcutAction, ShortcutMap};
```

```rust
// crates/yoyo-core/src/backend.rs
use crate::{AudioChannelMode, MediaLocator, Rotation};

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
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    PauseChanged(bool),
    PositionChanged(f64),
    DurationChanged(Option<f64>),
    SpeedChanged(f32),
    VolumeChanged(u8),
    AudioChannelChanged(AudioChannelMode),
    RotationChanged(Rotation),
    Warning(String),
    Error(String),
    EndOfFile,
}

pub trait PlayerBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String>;
    fn send(&mut self, command: BackendCommand) -> Result<(), String>;
    fn drain_events(&mut self) -> Vec<BackendEvent>;
}
```

```rust
// crates/yoyo-core/src/session.rs
use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent, MediaLocator,
    PlayerBackend, PlayerState, Playlist, PlaylistEntry, Rotation,
};

pub struct AppSession<B: PlayerBackend> {
    config: AppConfig,
    backend: B,
    state: PlayerState,
    playlist: Playlist,
}

impl<B: PlayerBackend> AppSession<B> {
    pub fn new(config: AppConfig, backend: B) -> Self {
        let mut state = PlayerState::default();
        state.volume_percent = config.playback.default_volume_percent;
        state.speed = config.playback.default_speed;
        Self {
            config,
            backend,
            state,
            playlist: Playlist::default(),
        }
    }

    pub fn state(&self) -> &PlayerState {
        &self.state
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn replace_playlist(
        &mut self,
        entries: Vec<PlaylistEntry>,
        start_index: usize,
    ) -> Result<(), AppError> {
        self.playlist.replace(entries, start_index);
        if let Some(entry) = self.playlist.current() {
            self.backend
                .open(&entry.locator)
                .map_err(AppError::Message)?;
            self.state.current = Some(entry.locator.clone());
            self.state.paused = false;
        }
        Ok(())
    }

    pub fn handle_command(&mut self, command: AppCommand) -> Result<(), AppError> {
        match command {
            AppCommand::OpenFile(path) => {
                let locator = MediaLocator::File(path);
                self.backend.open(&locator).map_err(AppError::Message)?;
                self.state.current = Some(locator);
                self.state.paused = false;
            }
            AppCommand::OpenUrl(url) => {
                let locator = MediaLocator::from_url(&url)?;
                self.backend.open(&locator).map_err(AppError::Message)?;
                self.state.current = Some(locator);
                self.state.paused = false;
            }
            AppCommand::TogglePause => {
                self.state.paused = !self.state.paused;
                self.backend
                    .send(BackendCommand::SetPaused(self.state.paused))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SeekRelative(seconds) => {
                self.backend
                    .send(BackendCommand::SeekRelative(seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SeekAbsolute(seconds) => {
                self.backend
                    .send(BackendCommand::SeekAbsolute(seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetSpeed(speed) => {
                self.state.speed = speed;
                self.backend
                    .send(BackendCommand::SetSpeed(speed))
                    .map_err(AppError::Message)?;
            }
            AppCommand::ResetSpeed => {
                self.state.speed = self.config.playback.default_speed;
                self.backend
                    .send(BackendCommand::SetSpeed(self.state.speed))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetVolume(volume) => {
                self.state.volume_percent = volume;
                self.backend
                    .send(BackendCommand::SetVolume(volume))
                    .map_err(AppError::Message)?;
            }
            AppCommand::AdjustVolume(delta) => {
                let next = (self.state.volume_percent as i16 + delta as i16).clamp(0, 100) as u8;
                self.state.volume_percent = next;
                self.backend
                    .send(BackendCommand::SetVolume(next))
                    .map_err(AppError::Message)?;
            }
            AppCommand::CycleAudioChannel => {
                self.state.audio_channel = match self.state.audio_channel {
                    AudioChannelMode::Stereo => AudioChannelMode::MonoLeft,
                    AudioChannelMode::MonoLeft => AudioChannelMode::MonoRight,
                    AudioChannelMode::MonoRight => AudioChannelMode::Stereo,
                };
                self.backend
                    .send(BackendCommand::SetAudioChannel(self.state.audio_channel))
                    .map_err(AppError::Message)?;
            }
            AppCommand::RotateClockwise => {
                self.state.rotation = match self.state.rotation {
                    Rotation::Deg0 => Rotation::Deg90,
                    Rotation::Deg90 => Rotation::Deg180,
                    Rotation::Deg180 => Rotation::Deg270,
                    Rotation::Deg270 => Rotation::Deg0,
                };
                self.backend
                    .send(BackendCommand::SetRotation(self.state.rotation))
                    .map_err(AppError::Message)?;
            }
            AppCommand::ZoomIn => {
                self.state.zoom_step += 1;
                self.backend
                    .send(BackendCommand::AdjustZoom(1))
                    .map_err(AppError::Message)?;
            }
            AppCommand::ZoomOut => {
                self.state.zoom_step -= 1;
                self.backend
                    .send(BackendCommand::AdjustZoom(-1))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetABLoopPointA => {
                self.state.loop_state.point_a = Some(self.state.position_seconds);
                self.backend
                    .send(BackendCommand::SetABLoopPointA(self.state.position_seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetABLoopPointB => {
                self.state.loop_state.point_b = Some(self.state.position_seconds);
                self.backend
                    .send(BackendCommand::SetABLoopPointB(self.state.position_seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::ClearABLoop => {
                self.state.loop_state = Default::default();
                self.backend
                    .send(BackendCommand::ClearABLoop)
                    .map_err(AppError::Message)?;
            }
            AppCommand::ToggleFullscreen => {
                self.state.fullscreen = !self.state.fullscreen;
            }
            AppCommand::NextItem => {
                if let Some(entry) = self.playlist.next() {
                    self.backend.open(&entry.locator).map_err(AppError::Message)?;
                    self.state.current = Some(entry.locator.clone());
                }
            }
            AppCommand::PreviousItem => {
                if let Some(entry) = self.playlist.previous() {
                    self.backend.open(&entry.locator).map_err(AppError::Message)?;
                    self.state.current = Some(entry.locator.clone());
                }
            }
            AppCommand::OpenFolder(_) => {
                return Err(AppError::Message(
                    "OpenFolder must be expanded into a playlist by the desktop app".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn poll_backend(&mut self) -> Result<(), AppError> {
        for event in self.backend.drain_events() {
            match event {
                BackendEvent::PauseChanged(paused) => self.state.paused = paused,
                BackendEvent::PositionChanged(position) => self.state.position_seconds = position,
                BackendEvent::DurationChanged(duration) => self.state.duration_seconds = duration,
                BackendEvent::SpeedChanged(speed) => self.state.speed = speed,
                BackendEvent::VolumeChanged(volume) => self.state.volume_percent = volume,
                BackendEvent::AudioChannelChanged(mode) => self.state.audio_channel = mode,
                BackendEvent::RotationChanged(rotation) => self.state.rotation = rotation,
                BackendEvent::Warning(message) => self.state.status_message = Some(message),
                BackendEvent::Error(message) => self.state.last_error = Some(message),
                BackendEvent::EndOfFile => {
                    if let Some(entry) = self.playlist.next() {
                        self.backend.open(&entry.locator).map_err(AppError::Message)?;
                        self.state.current = Some(entry.locator.clone());
                    }
                }
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyo-core --test session_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/yoyo-core
git commit -m "feat: add session and backend abstraction"
```

### Task 4: Create The libmpv Command And Event Adapter

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/yoyo-mpv/Cargo.toml`
- Create: `crates/yoyo-mpv/src/lib.rs`
- Create: `crates/yoyo-mpv/src/error.rs`
- Create: `crates/yoyo-mpv/src/translate.rs`
- Create: `crates/yoyo-mpv/src/client.rs`
- Create: `crates/yoyo-mpv/tests/translate_contract.rs`

**Interfaces:**
- Consumes:
  - `BackendCommand`
  - `BackendEvent`
  - `PlayerBackend`
  - `MediaLocator`
  - `AudioChannelMode`
  - `Rotation`
- Produces:
  - `pub enum MpvAction`
  - `pub fn translate_open(locator: &MediaLocator) -> Vec<MpvAction>`
  - `pub fn translate_command(command: &BackendCommand) -> Vec<MpvAction>`
  - `pub fn translate_property(name: &str, value: &str) -> Option<BackendEvent>`
  - `pub struct MpvBackend`

- [ ] **Step 1: Write the failing test**

```rust
// crates/yoyo-mpv/tests/translate_contract.rs
use yoyo_core::{AudioChannelMode, BackendCommand, MediaLocator, Rotation};
use yoyo_mpv::{translate_command, translate_open, MpvAction};

#[test]
fn open_file_translates_to_loadfile_replace() {
    let actions = translate_open(&MediaLocator::File("movie.mp4".into()));
    assert_eq!(
        actions,
        vec![MpvAction::Command(vec![
            "loadfile".into(),
            "movie.mp4".into(),
            "replace".into(),
        ])]
    );
}

#[test]
fn set_speed_translates_to_speed_property() {
    let actions = translate_command(&BackendCommand::SetSpeed(1.25));
    assert_eq!(
        actions,
        vec![MpvAction::SetDouble {
            name: "speed".into(),
            value: 1.25,
        }]
    );
}

#[test]
fn mono_left_uses_front_left_layout() {
    let actions = translate_command(&BackendCommand::SetAudioChannel(AudioChannelMode::MonoLeft));
    assert_eq!(
        actions,
        vec![MpvAction::SetString {
            name: "audio-channels".into(),
            value: "fl".into(),
        }]
    );
}

#[test]
fn rotation_translates_to_video_rotate() {
    let actions = translate_command(&BackendCommand::SetRotation(Rotation::Deg90));
    assert_eq!(
        actions,
        vec![MpvAction::SetInt {
            name: "video-rotate".into(),
            value: 90,
        }]
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyo-mpv --test translate_contract`

Expected: FAIL because the `yoyo-mpv` package does not exist yet.

- [ ] **Step 3: Write minimal implementation**

```toml
# Cargo.toml
[workspace]
members = [
    "crates/yoyo-core",
    "crates/yoyo-mpv",
]
resolver = "2"
```

```toml
# crates/yoyo-mpv/Cargo.toml
[package]
name = "yoyo-mpv"
version = "0.1.0"
edition = "2024"

[features]
default = []
mpv-runtime = ["dep:libmpv-sys"]

[dependencies]
libmpv-sys = { version = "3.1.0", optional = true }
thiserror = "2.0.18"
tracing = "0.1.44"
yoyo-core = { path = "../yoyo-core" }
```

```rust
// crates/yoyo-mpv/src/lib.rs
mod client;
mod error;
mod translate;

pub use client::MpvBackend;
pub use error::MpvError;
pub use translate::{translate_command, translate_open, MpvAction};
```

```rust
// crates/yoyo-mpv/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MpvError {
    #[error("mpv runtime feature is disabled")]
    RuntimeDisabled,
    #[error("mpv api error: {0}")]
    Api(String),
}
```

```rust
// crates/yoyo-mpv/src/translate.rs
use yoyo_core::{AudioChannelMode, BackendCommand, MediaLocator, Rotation};

#[derive(Debug, Clone, PartialEq)]
pub enum MpvAction {
    Command(Vec<String>),
    SetString { name: String, value: String },
    SetInt { name: String, value: i64 },
    SetDouble { name: String, value: f64 },
    SetFlag { name: String, value: bool },
}

pub fn translate_open(locator: &MediaLocator) -> Vec<MpvAction> {
    vec![MpvAction::Command(vec![
        "loadfile".into(),
        locator.as_label(),
        "replace".into(),
    ])]
}

pub fn translate_command(command: &BackendCommand) -> Vec<MpvAction> {
    match command {
        BackendCommand::SetPaused(paused) => vec![MpvAction::SetFlag {
            name: "pause".into(),
            value: *paused,
        }],
        BackendCommand::SeekRelative(seconds) => vec![MpvAction::Command(vec![
            "seek".into(),
            seconds.to_string(),
            "relative".into(),
        ])],
        BackendCommand::SeekAbsolute(seconds) => vec![MpvAction::Command(vec![
            "seek".into(),
            seconds.to_string(),
            "absolute+exact".into(),
        ])],
        BackendCommand::SetSpeed(speed) => vec![MpvAction::SetDouble {
            name: "speed".into(),
            value: *speed as f64,
        }],
        BackendCommand::SetVolume(volume) => vec![MpvAction::SetDouble {
            name: "volume".into(),
            value: *volume as f64,
        }],
        BackendCommand::SetAudioChannel(mode) => {
            let value = match mode {
                AudioChannelMode::Stereo => "stereo",
                AudioChannelMode::MonoLeft => "fl",
                AudioChannelMode::MonoRight => "fr",
            };
            vec![MpvAction::SetString {
                name: "audio-channels".into(),
                value: value.into(),
            }]
        }
        BackendCommand::SetRotation(rotation) => {
            let degrees = match rotation {
                Rotation::Deg0 => 0,
                Rotation::Deg90 => 90,
                Rotation::Deg180 => 180,
                Rotation::Deg270 => 270,
            };
            vec![MpvAction::SetInt {
                name: "video-rotate".into(),
                value: degrees,
            }]
        }
        BackendCommand::AdjustZoom(delta) => vec![MpvAction::Command(vec![
            "add".into(),
            "video-zoom".into(),
            (*delta as f64 * 0.25).to_string(),
        ])],
        BackendCommand::SetABLoopPointA(seconds) => vec![MpvAction::SetDouble {
            name: "ab-loop-a".into(),
            value: *seconds,
        }],
        BackendCommand::SetABLoopPointB(seconds) => vec![MpvAction::SetDouble {
            name: "ab-loop-b".into(),
            value: *seconds,
        }],
        BackendCommand::ClearABLoop => vec![
            MpvAction::SetString {
                name: "ab-loop-a".into(),
                value: "no".into(),
            },
            MpvAction::SetString {
                name: "ab-loop-b".into(),
                value: "no".into(),
            },
        ],
    }
}
```

```rust
// crates/yoyo-mpv/src/client.rs
use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{translate_command, translate_open, MpvError};

#[derive(Default)]
pub struct MpvBackend {
    pending_events: Vec<BackendEvent>,
    #[allow(dead_code)]
    last_actions: Vec<String>,
}

impl PlayerBackend for MpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.last_actions = translate_open(locator)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.last_actions = translate_command(&command)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl MpvBackend {
    pub fn ensure_runtime_feature() -> Result<(), MpvError> {
        #[cfg(feature = "mpv-runtime")]
        {
            Ok(())
        }
        #[cfg(not(feature = "mpv-runtime"))]
        {
            Err(MpvError::RuntimeDisabled)
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyo-mpv --test translate_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/yoyo-mpv
git commit -m "feat: add mpv command translation layer"
```

### Task 5: Create The Slint Desktop Shell And Platform Services

**Files:**
- Modify: `Cargo.toml`
- Create: `apps/yoyovideo-desktop/Cargo.toml`
- Create: `apps/yoyovideo-desktop/build.rs`
- Create: `apps/yoyovideo-desktop/src/lib.rs`
- Create: `apps/yoyovideo-desktop/src/main.rs`
- Create: `apps/yoyovideo-desktop/src/app.rs`
- Create: `apps/yoyovideo-desktop/src/presenter.rs`
- Create: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Create: `apps/yoyovideo-desktop/src/platform/dialogs.rs`
- Create: `apps/yoyovideo-desktop/src/platform/paths.rs`
- Create: `apps/yoyovideo-desktop/ui/main-window.slint`
- Create: `apps/yoyovideo-desktop/tests/presenter_contract.rs`

**Interfaces:**
- Consumes:
  - `AppConfig`
  - `AppSession`
  - `PlayerState`
  - `ShortcutAction`
- Produces:
  - `pub fn format_transport_label(state: &PlayerState) -> String`
  - `pub fn format_speed_label(state: &PlayerState) -> String`
  - `pub fn format_time_label(state: &PlayerState) -> String`
  - `pub struct AppPaths`
  - `pub trait DialogService`
  - `pub fn run() -> Result<(), Box<dyn std::error::Error>>`

- [ ] **Step 1: Write the failing test**

```rust
// apps/yoyovideo-desktop/tests/presenter_contract.rs
use yoyo_core::{PlayerState, Rotation};
use yoyovideo_desktop::{format_speed_label, format_time_label, format_transport_label};

#[test]
fn transport_label_shows_pause_when_playing() {
    let state = PlayerState {
        paused: false,
        ..PlayerState::default()
    };

    assert_eq!(format_transport_label(&state), "Pause");
}

#[test]
fn speed_label_renders_two_decimals() {
    let state = PlayerState {
        speed: 1.25,
        ..PlayerState::default()
    };

    assert_eq!(format_speed_label(&state), "1.25x");
}

#[test]
fn time_label_formats_minutes_and_seconds() {
    let state = PlayerState {
        position_seconds: 65.0,
        duration_seconds: Some(130.0),
        rotation: Rotation::Deg90,
        ..PlayerState::default()
    };

    assert_eq!(format_time_label(&state), "01:05 / 02:10");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyovideo-desktop --test presenter_contract`

Expected: FAIL because the desktop package does not exist yet.

- [ ] **Step 3: Write minimal implementation**

```toml
# Cargo.toml
[workspace]
members = [
    "crates/yoyo-core",
    "crates/yoyo-mpv",
    "apps/yoyovideo-desktop",
]
resolver = "2"
```

```toml
# apps/yoyovideo-desktop/Cargo.toml
[package]
name = "yoyovideo-desktop"
version = "0.1.0"
edition = "2024"

[dependencies]
directories = "6.0.0"
rfd = "0.17.2"
slint = { version = "1.17.0", features = ["backend-winit", "unstable-winit-030", "raw-window-handle-06"] }
tracing = "0.1.44"
tracing-subscriber = "0.3.23"
yoyo-core = { path = "../../crates/yoyo-core" }
yoyo-mpv = { path = "../../crates/yoyo-mpv" }

[build-dependencies]
slint-build = "1.17.0"
```

```rust
// apps/yoyovideo-desktop/build.rs
fn main() {
    slint_build::compile("ui/main-window.slint").expect("compile slint ui");
}
```

```rust
// apps/yoyovideo-desktop/src/lib.rs
mod app;
mod platform;
mod presenter;

pub use app::run;
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
```

```rust
// apps/yoyovideo-desktop/src/main.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    yoyovideo_desktop::run()
}
```

```rust
// apps/yoyovideo-desktop/src/presenter.rs
use yoyo_core::PlayerState;

pub fn format_transport_label(state: &PlayerState) -> String {
    if state.paused {
        "Play".into()
    } else {
        "Pause".into()
    }
}

pub fn format_speed_label(state: &PlayerState) -> String {
    format!("{:.2}x", state.speed)
}

pub fn format_time_label(state: &PlayerState) -> String {
    fn fmt(seconds: f64) -> String {
        let total = seconds.max(0.0) as u64;
        format!("{:02}:{:02}", total / 60, total % 60)
    }

    match state.duration_seconds {
        Some(duration) => format!("{} / {}", fmt(state.position_seconds), fmt(duration)),
        None => format!("{} / --:--", fmt(state.position_seconds)),
    }
}
```

```rust
// apps/yoyovideo-desktop/src/platform/mod.rs
mod dialogs;
mod paths;

pub use dialogs::{DialogService, RfdDialogService};
pub use paths::AppPaths;
```

```rust
// apps/yoyovideo-desktop/src/platform/dialogs.rs
use std::path::PathBuf;

pub trait DialogService {
    fn pick_file(&self) -> Option<PathBuf>;
    fn pick_folder(&self) -> Option<PathBuf>;
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
}
```

```rust
// apps/yoyovideo-desktop/src/platform/paths.rs
use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Option<Self> {
        let dirs = ProjectDirs::from("com", "xyito", "YoYoVideo")?;
        Some(Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
        })
    }
}
```

```rust
// apps/yoyovideo-desktop/src/app.rs
slint::include_modules!();

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;

    let app = MainWindow::new()?;
    app.run()?;
    Ok(())
}
```

```slint
// apps/yoyovideo-desktop/ui/main-window.slint
import { Button, HorizontalBox, VerticalBox, Slider, LineEdit } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "YoYoVideo";
    width: 1200px;
    height: 760px;

    in-out property <string> transport_label: "Play";
    in-out property <string> speed_label: "1.00x";
    in-out property <string> time_label: "00:00 / --:--";
    in-out property <string> status_label: "";

    callback open_file_requested();
    callback open_folder_requested();
    callback open_url_requested(string);
    callback toggle_pause_requested();

    VerticalBox {
        spacing: 8px;
        Rectangle {
            background: #101214;
            border-radius: 8px;
            min-height: 620px;
        }
        HorizontalBox {
            spacing: 8px;
            Button { text: transport_label; clicked => { root.toggle_pause_requested(); } }
            Button { text: "Open"; clicked => { root.open_file_requested(); } }
            Button { text: "Folder"; clicked => { root.open_folder_requested(); } }
            LineEdit {
                accepted => { root.open_url_requested(self.text); }
            }
            Text { text: time_label; }
            Text { text: speed_label; }
            Text { text: status_label; }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyovideo-desktop --test presenter_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml apps/yoyovideo-desktop
git commit -m "feat: add desktop shell and presenter"
```

### Task 6: Add The mpv OpenGL Render Bridge And Real Playback Wiring

**Files:**
- Modify: `crates/yoyo-mpv/src/lib.rs`
- Modify: `crates/yoyo-mpv/src/client.rs`
- Create: `crates/yoyo-mpv/src/render.rs`
- Create: `crates/yoyo-mpv/tests/render_contract.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Create: `apps/yoyovideo-desktop/src/video_texture.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Create: `apps/yoyovideo-desktop/tests/controller_contract.rs`

**Interfaces:**
- Consumes:
  - `AppSession<MpvBackend>`
  - `BackendCommand`
  - `PlayerState`
  - `MpvBackend`
- Produces:
  - `pub struct RenderTarget`
  - `pub struct MpvRenderBridge`
  - `impl MpvBackend { pub fn render_bridge(&mut self) -> &mut MpvRenderBridge }`
  - `pub struct VideoTexture`
  - `pub struct DesktopController<B: PlayerBackend>`

- [ ] **Step 1: Write the failing test**

```rust
// crates/yoyo-mpv/tests/render_contract.rs
use yoyo_mpv::{MpvRenderBridge, RenderTarget};

#[test]
fn render_target_keeps_dimensions() {
    let target = RenderTarget {
        framebuffer_object: 7,
        width: 1280,
        height: 720,
        flipped: false,
    };

    assert_eq!(target.width, 1280);
    assert_eq!(target.height, 720);
}

#[test]
fn render_bridge_starts_without_pending_redraw() {
    let bridge = MpvRenderBridge::default();
    assert!(!bridge.needs_redraw());
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
fn controller_forward_toggle_pause_to_session() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller.dispatch(AppCommand::TogglePause).unwrap();

    assert_eq!(
        controller.session().backend().commands,
        vec![BackendCommand::SetPaused(false)]
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyo-mpv --test render_contract && cargo test -p yoyovideo-desktop --test controller_contract`

Expected: FAIL with unresolved types such as `MpvRenderBridge`, `RenderTarget`, or `DesktopController`.

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/yoyo-mpv/src/lib.rs
mod client;
mod error;
mod render;
mod translate;

pub use client::MpvBackend;
pub use error::MpvError;
pub use render::{MpvRenderBridge, RenderTarget};
pub use translate::{translate_command, translate_open, MpvAction};
```

```rust
// crates/yoyo-mpv/src/render.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    pub framebuffer_object: u32,
    pub width: u32,
    pub height: u32,
    pub flipped: bool,
}

#[derive(Default)]
pub struct MpvRenderBridge {
    redraw_requested: bool,
}

impl MpvRenderBridge {
    pub fn needs_redraw(&self) -> bool {
        self.redraw_requested
    }

    pub fn mark_dirty(&mut self) {
        self.redraw_requested = true;
    }

    pub fn render(&mut self, _target: RenderTarget) -> Result<(), crate::MpvError> {
        self.redraw_requested = false;
        Ok(())
    }
}
```

```rust
// crates/yoyo-mpv/src/client.rs
use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{render::MpvRenderBridge, translate_command, translate_open};

pub struct MpvBackend {
    pending_events: Vec<BackendEvent>,
    render_bridge: MpvRenderBridge,
    #[allow(dead_code)]
    last_actions: Vec<String>,
}

impl Default for MpvBackend {
    fn default() -> Self {
        Self {
            pending_events: Vec::new(),
            render_bridge: MpvRenderBridge::default(),
            last_actions: Vec::new(),
        }
    }
}

impl PlayerBackend for MpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.last_actions = translate_open(locator)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        self.render_bridge.mark_dirty();
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.last_actions = translate_command(&command)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        self.render_bridge.mark_dirty();
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl MpvBackend {
    pub fn render_bridge(&mut self) -> &mut MpvRenderBridge {
        &mut self.render_bridge
    }
}
```

```rust
// apps/yoyovideo-desktop/src/video_texture.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoTexture {
    pub texture_id: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for VideoTexture {
    fn default() -> Self {
        Self {
            texture_id: 0,
            width: 1280,
            height: 720,
        }
    }
}
```

```rust
// apps/yoyovideo-desktop/src/app.rs
use std::cell::RefCell;
use std::rc::Rc;

use yoyo_core::{AppCommand, AppConfig, AppSession, PlayerBackend};
use yoyo_mpv::MpvBackend;

use crate::video_texture::VideoTexture;

slint::include_modules!();

pub struct DesktopController<B: PlayerBackend> {
    session: AppSession<B>,
    #[allow(dead_code)]
    video_texture: VideoTexture,
}

impl<B: PlayerBackend> DesktopController<B> {
    pub fn new(session: AppSession<B>) -> Self {
        Self {
            session,
            video_texture: VideoTexture::default(),
        }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), yoyo_core::AppError> {
        self.session.handle_command(command)
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;

    let app = MainWindow::new()?;
    let session = AppSession::new(AppConfig::default(), MpvBackend::default());
    let controller = Rc::new(RefCell::new(DesktopController::new(session)));

    app.on_toggle_pause_requested({
        let app_handle = app.as_weak();
        let controller = Rc::clone(&controller);
        move || {
            let mut controller = controller.borrow_mut();
            if controller.dispatch(AppCommand::TogglePause).is_ok() {
                if let Some(app) = app_handle.upgrade() {
                    app.set_transport_label(crate::format_transport_label(controller.session().state()).into());
                }
            }
        }
    });

    app.run()?;
    Ok(())
}
```

```rust
// apps/yoyovideo-desktop/src/lib.rs
mod app;
mod platform;
mod presenter;
mod video_texture;

pub use app::{run, DesktopController};
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
```

```slint
// apps/yoyovideo-desktop/ui/main-window.slint
import { Button, HorizontalBox, VerticalBox, Slider, LineEdit, Text } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "YoYoVideo";
    width: 1200px;
    height: 760px;

    in-out property <string> transport_label: "Play";
    in-out property <string> speed_label: "1.00x";
    in-out property <string> time_label: "00:00 / --:--";
    in-out property <string> status_label: "";

    callback open_file_requested();
    callback open_folder_requested();
    callback open_url_requested(string);
    callback toggle_pause_requested();

    VerticalBox {
        spacing: 8px;
        Rectangle {
            background: #101214;
            border-radius: 8px;
            min-height: 620px;
            Text {
                text: "Video surface";
                color: #d0d6dc;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
        HorizontalBox {
            spacing: 8px;
            Button { text: transport_label; clicked => { root.toggle_pause_requested(); } }
            Button { text: "Open"; clicked => { root.open_file_requested(); } }
            Button { text: "Folder"; clicked => { root.open_folder_requested(); } }
            LineEdit {
                accepted => { root.open_url_requested(self.text); }
            }
            Text { text: time_label; }
            Text { text: speed_label; }
            Text { text: status_label; }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyo-mpv --test render_contract && cargo test -p yoyovideo-desktop --test controller_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/yoyo-mpv apps/yoyovideo-desktop
git commit -m "feat: wire desktop controller and render bridge"
```

### Task 7: Add Folder Scanning, Playlist/History, Shortcuts, And Context Menu

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/presenter.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/dialogs.rs`
- Create: `apps/yoyovideo-desktop/src/platform/media_scan.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Create: `apps/yoyovideo-desktop/tests/media_scan_contract.rs`
- Create: `apps/yoyovideo-desktop/tests/shortcut_contract.rs`

**Interfaces:**
- Consumes:
  - `AppCommand`
  - `AppConfig`
  - `AppPaths`
  - `AppSession`
  - `HistoryStore`
  - `PlaylistEntry`
  - `Shortcut`
  - `ShortcutAction`
  - `ShortcutMap`
- Produces:
  - `pub fn scan_media_folder(path: &Path) -> Result<Vec<PlaylistEntry>, AppError>`
  - `pub fn dispatch_shortcut(map: &ShortcutMap, gesture: &str) -> Option<AppCommand>`
  - Context-menu callbacks for file/folder/url, transport, speed, rotation, channel, A-B loop, and full screen.

- [ ] **Step 1: Write the failing test**

```rust
// apps/yoyovideo-desktop/tests/media_scan_contract.rs
use std::fs;

use tempfile::tempdir;
use yoyovideo_desktop::scan_media_folder;

#[test]
fn folder_scan_only_keeps_supported_media_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("movie.mp4"), "x").unwrap();
    fs::write(dir.path().join("cover.jpg"), "x").unwrap();
    fs::write(dir.path().join("song.flac"), "x").unwrap();

    let entries = scan_media_folder(dir.path()).unwrap();
    let titles: Vec<_> = entries.into_iter().map(|entry| entry.title).collect();

    assert_eq!(titles, vec!["movie.mp4".to_string(), "song.flac".to_string()]);
}
```

```rust
// apps/yoyovideo-desktop/tests/shortcut_contract.rs
use yoyo_core::{AppCommand, Shortcut, ShortcutMap};
use yoyovideo_desktop::dispatch_shortcut;

#[test]
fn control_a_clears_ab_loop() {
    let map = ShortcutMap::default();
    let command = dispatch_shortcut(&map, Shortcut::parse("Ctrl+A").unwrap().as_str());

    assert_eq!(command, Some(AppCommand::ClearABLoop));
}

#[test]
fn right_arrow_seeks_forward() {
    let map = ShortcutMap::default();
    let command = dispatch_shortcut(&map, Shortcut::parse("Right").unwrap().as_str());

    assert_eq!(command, Some(AppCommand::SeekRelative(5.0)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyovideo-desktop --test media_scan_contract && cargo test -p yoyovideo-desktop --test shortcut_contract`

Expected: FAIL with unresolved functions such as `scan_media_folder` or `dispatch_shortcut`.

- [ ] **Step 3: Write minimal implementation**

```rust
// apps/yoyovideo-desktop/src/platform/mod.rs
mod dialogs;
mod media_scan;
mod paths;

pub use dialogs::{DialogService, RfdDialogService};
pub use media_scan::scan_media_folder;
pub use paths::AppPaths;
```

```rust
// apps/yoyovideo-desktop/src/platform/media_scan.rs
use std::fs;
use std::path::Path;

use yoyo_core::{AppError, MediaLocator, PlaylistEntry};

pub fn scan_media_folder(path: &Path) -> Result<Vec<PlaylistEntry>, AppError> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(path).map_err(|error| AppError::Message(error.to_string()))? {
        let entry = entry.map_err(|error| AppError::Message(error.to_string()))?;
        let candidate = entry.path();
        if candidate.is_file() && yoyo_core::MediaLocator::is_supported_local_path(&candidate) {
            entries.push(PlaylistEntry::new(MediaLocator::File(candidate)));
        }
    }

    entries.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(entries)
}
```

```rust
// apps/yoyovideo-desktop/src/app.rs
use yoyo_core::{AppCommand, AppConfig, AppSession, PlayerBackend, ShortcutAction, ShortcutMap};
use yoyo_mpv::MpvBackend;

use crate::platform::scan_media_folder;
use crate::video_texture::VideoTexture;

slint::include_modules!();

pub struct DesktopController<B: PlayerBackend> {
    session: AppSession<B>,
    shortcuts: ShortcutMap,
    #[allow(dead_code)]
    video_texture: VideoTexture,
}

impl<B: PlayerBackend> DesktopController<B> {
    pub fn new(session: AppSession<B>) -> Self {
        let shortcuts = ShortcutMap::default();
        Self {
            session,
            shortcuts,
            video_texture: VideoTexture::default(),
        }
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), yoyo_core::AppError> {
        self.session.handle_command(command)
    }

    pub fn open_folder(&mut self, path: &std::path::Path) -> Result<(), yoyo_core::AppError> {
        let entries = scan_media_folder(path)?;
        self.session.replace_playlist(entries, 0)
    }

    pub fn dispatch_shortcut(&mut self, gesture: &str) -> Result<(), yoyo_core::AppError> {
        if let Some(command) = dispatch_shortcut(&self.shortcuts, gesture) {
            self.dispatch(command)?;
        }
        Ok(())
    }
}

pub fn dispatch_shortcut(map: &ShortcutMap, gesture: &str) -> Option<AppCommand> {
    let shortcut = yoyo_core::Shortcut::parse(gesture).ok()?;
    match map.action_for(&shortcut)? {
        ShortcutAction::TogglePause => Some(AppCommand::TogglePause),
        ShortcutAction::SeekBackwardSmall => Some(AppCommand::SeekRelative(-5.0)),
        ShortcutAction::SeekForwardSmall => Some(AppCommand::SeekRelative(5.0)),
        ShortcutAction::VolumeUp => Some(AppCommand::AdjustVolume(5)),
        ShortcutAction::VolumeDown => Some(AppCommand::AdjustVolume(-5)),
        ShortcutAction::SpeedDown => Some(AppCommand::SetSpeed(0.75)),
        ShortcutAction::SpeedUp => Some(AppCommand::SetSpeed(1.25)),
        ShortcutAction::ResetSpeed => Some(AppCommand::ResetSpeed),
        ShortcutAction::SetABLoopPointA => Some(AppCommand::SetABLoopPointA),
        ShortcutAction::SetABLoopPointB => Some(AppCommand::SetABLoopPointB),
        ShortcutAction::ClearABLoop => Some(AppCommand::ClearABLoop),
        ShortcutAction::RotateClockwise => Some(AppCommand::RotateClockwise),
        ShortcutAction::ZoomOut => Some(AppCommand::ZoomOut),
        ShortcutAction::ZoomIn => Some(AppCommand::ZoomIn),
        ShortcutAction::CycleAudioChannel => Some(AppCommand::CycleAudioChannel),
        ShortcutAction::ToggleFullscreen => Some(AppCommand::ToggleFullscreen),
        ShortcutAction::OpenFile | ShortcutAction::OpenUrl => None,
    }
}
```

```rust
// apps/yoyovideo-desktop/src/lib.rs
mod app;
mod platform;
mod presenter;
mod video_texture;

pub use app::{dispatch_shortcut, run, DesktopController};
pub use platform::scan_media_folder;
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
```

```slint
// apps/yoyovideo-desktop/ui/main-window.slint
import { Button, HorizontalBox, VerticalBox, LineEdit, Text } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "YoYoVideo";
    width: 1200px;
    height: 760px;

    in-out property <string> transport_label: "Play";
    in-out property <string> speed_label: "1.00x";
    in-out property <string> time_label: "00:00 / --:--";
    in-out property <string> status_label: "";

    callback open_file_requested();
    callback open_folder_requested();
    callback open_url_requested(string);
    callback toggle_pause_requested();
    callback speed_down_requested();
    callback speed_up_requested();
    callback rotate_requested();
    callback cycle_audio_requested();
    callback set_ab_point_a_requested();
    callback set_ab_point_b_requested();
    callback clear_ab_loop_requested();
    callback toggle_fullscreen_requested();

    menu_popup := PopupWindow {
        close-policy: close-on-click-outside;
        width: 220px;
        height: 420px;

        VerticalBox {
            spacing: 4px;
            Button { text: "Open File"; clicked => { root.open_file_requested(); menu_popup.close(); } }
            Button { text: "Open Folder"; clicked => { root.open_folder_requested(); menu_popup.close(); } }
            Button { text: "Play/Pause"; clicked => { root.toggle_pause_requested(); menu_popup.close(); } }
            Button { text: "Speed -"; clicked => { root.speed_down_requested(); menu_popup.close(); } }
            Button { text: "Speed +"; clicked => { root.speed_up_requested(); menu_popup.close(); } }
            Button { text: "Rotate"; clicked => { root.rotate_requested(); menu_popup.close(); } }
            Button { text: "Audio Channel"; clicked => { root.cycle_audio_requested(); menu_popup.close(); } }
            Button { text: "Set A"; clicked => { root.set_ab_point_a_requested(); menu_popup.close(); } }
            Button { text: "Set B"; clicked => { root.set_ab_point_b_requested(); menu_popup.close(); } }
            Button { text: "Clear A-B"; clicked => { root.clear_ab_loop_requested(); menu_popup.close(); } }
            Button { text: "Fullscreen"; clicked => { root.toggle_fullscreen_requested(); menu_popup.close(); } }
        }
    }

    VerticalBox {
        spacing: 8px;
        Rectangle {
            background: #101214;
            border-radius: 8px;
            min-height: 620px;
        }
        HorizontalBox {
            spacing: 8px;
            Button { text: transport_label; clicked => { root.toggle_pause_requested(); } }
            Button { text: "Open"; clicked => { root.open_file_requested(); } }
            Button { text: "Folder"; clicked => { root.open_folder_requested(); } }
            Button { text: "Menu"; clicked => { menu_popup.show(); } }
            LineEdit {
                accepted => { root.open_url_requested(self.text); }
            }
            Text { text: time_label; }
            Text { text: speed_label; }
            Text { text: status_label; }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyovideo-desktop --test media_scan_contract && cargo test -p yoyovideo-desktop --test shortcut_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/yoyovideo-desktop
git commit -m "feat: add playlist scanning and shortcut dispatch"
```

### Task 8: Add Settings Persistence, Runtime Docs, And Release Validation

**Files:**
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Create: `apps/yoyovideo-desktop/src/settings_controller.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Create: `apps/yoyovideo-desktop/tests/settings_contract.rs`
- Create: `docs/development/runtime-dependencies.md`
- Create: `docs/testing/manual-smoke-checklist.md`
- Modify: `README.md`

**Interfaces:**
- Consumes:
  - `AppConfig`
  - `AppPaths`
  - `Shortcut`
  - `ShortcutAction`
  - `ShortcutMap`
- Produces:
  - `pub struct SettingsController`
  - `impl SettingsController { pub fn update_shortcut(&mut self, gesture: &str, action: ShortcutAction) -> Result<(), ValidationError> }`
  - `impl SettingsController { pub fn save(&self, path: &Path) -> Result<(), AppError> }`

- [ ] **Step 1: Write the failing test**

```rust
// apps/yoyovideo-desktop/tests/settings_contract.rs
use tempfile::tempdir;
use yoyo_core::{ShortcutAction, ShortcutMap};
use yoyovideo_desktop::SettingsController;

#[test]
fn updating_shortcut_persists_to_config_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::default();
    controller.update_shortcut("Ctrl+P", ShortcutAction::TogglePause).unwrap();
    controller.save(&path).unwrap();

    let saved = std::fs::read_to_string(path).unwrap();
    assert!(saved.contains("Ctrl+P"));
}

#[test]
fn duplicate_shortcut_is_rejected() {
    let mut controller = SettingsController::default();
    controller.update_shortcut("Space", ShortcutAction::TogglePause).unwrap();

    let error = controller
        .update_shortcut("Space", ShortcutAction::SpeedUp)
        .unwrap_err();

    assert!(error.to_string().contains("duplicate"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yoyovideo-desktop --test settings_contract`

Expected: FAIL with unresolved type `SettingsController`.

- [ ] **Step 3: Write minimal implementation**

```rust
// apps/yoyovideo-desktop/src/settings_controller.rs
use std::path::Path;

use yoyo_core::{AppConfig, AppError, Shortcut, ShortcutAction, ValidationError};

#[derive(Default)]
pub struct SettingsController {
    config: AppConfig,
}

impl SettingsController {
    pub fn update_shortcut(
        &mut self,
        gesture: &str,
        action: ShortcutAction,
    ) -> Result<(), ValidationError> {
        let shortcut = Shortcut::parse(gesture)?;
        if let Some(existing) = self.config.shortcuts.bindings.get(&shortcut) {
            if *existing != action {
                return Err(ValidationError::InvalidShortcut(format!(
                    "duplicate shortcut: {}",
                    shortcut.as_str()
                )));
            }
        }
        self.config.shortcuts.bindings.insert(shortcut, action);
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        self.config.save(path)?;
        Ok(())
    }
}
```

```rust
// apps/yoyovideo-desktop/src/lib.rs
mod app;
mod platform;
mod presenter;
mod settings_controller;
mod video_texture;

pub use app::{dispatch_shortcut, run, DesktopController};
pub use platform::scan_media_folder;
pub use presenter::{format_speed_label, format_time_label, format_transport_label};
pub use settings_controller::SettingsController;
```

```markdown
# docs/development/runtime-dependencies.md

## libmpv runtime checklist

- Bundle `libmpv` and its FFmpeg-dependent runtime libraries inside the platform package.
- Do not rely on a user-installed system mpv by default.
- Test both hardware-decoding success and software-decoding fallback paths.
- Verify Windows DLL search path, macOS app bundle embedding, and Linux runtime lookup strategy.
- Review redistribution obligations for the exact libmpv/FFmpeg build before publishing.
```

```markdown
# docs/testing/manual-smoke-checklist.md

## Startup

- Launch the app on Windows, macOS, and Linux.
- Confirm the window opens without crashing when libmpv runtime files are present.
- Confirm the app shows an actionable error when libmpv runtime files are missing.

## Playback

- Open a local video file and confirm play/pause works.
- Open a URL and confirm network playback attempts begin.
- Verify speed, volume, rotation, zoom, audio channel switching, and A-B repeat.
- Confirm hardware acceleration falls back to software decoding without exiting the app.

## UX

- Verify context menu actions match toolbar actions.
- Verify keyboard shortcuts trigger the same commands as buttons.
- Verify settings changes persist across restarts.
- Verify recent history and last position are restored when enabled.
```

```markdown
# README.md
# YoYoVideo

Rust + Slint + libmpv cross-platform desktop media player.

## Workspace

- `crates/yoyo-core`: playback/session domain logic
- `crates/yoyo-mpv`: libmpv adapter and render bridge
- `apps/yoyovideo-desktop`: Slint desktop application

## MVP scope

- Local files and folders
- Network URLs
- Playback, seeking, speed, zoom, rotation, A-B repeat
- Playlist, history, context menu, and keyboard shortcuts
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p yoyovideo-desktop --test settings_contract`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/yoyovideo-desktop docs README.md
git commit -m "feat: add settings persistence and release docs"
```

## Self-Review

### Spec coverage

- Cross-platform desktop support: covered by Tasks 5, 6, and 8.
- Rust + Slint + libmpv architecture: covered by Tasks 1 through 6.
- Local files, folders, playlists, recent history, and network URLs: covered by Tasks 2, 3, 6, and 7.
- Playback, pause, seek, speed, volume, zoom, audio channel, rotation, A-B repeat, full screen, and shortcuts: covered by Tasks 3, 4, 6, and 7.
- Simple player UI with professional controls and context menu: covered by Tasks 5 and 7.
- Config in system directory and persistence: covered by Tasks 5 and 8.
- Hardware acceleration best-effort with fallback and runtime bundling constraints: covered by Tasks 6 and 8.
- Non-goals remain excluded: no task introduces subtitle styling, recording, filters, plugins, or online aggregation.

### Red-flag scan

- Searched plan content manually for incomplete-plan markers before finalizing.
- Replaced vague statements with concrete file paths, types, commands, and expected test outcomes.

### Type consistency

- `AppCommand`, `BackendCommand`, `BackendEvent`, `PlayerBackend`, `AppSession`, `ShortcutMap`, `SettingsController`, `DesktopController<B>`, and `MpvRenderBridge` use consistent names across tasks.
- `AudioChannelMode` and `Rotation` variants are reused consistently across `yoyo-core` and `yoyo-mpv`.
- `DesktopController::dispatch()` always accepts `AppCommand`, and shortcut dispatch produces `AppCommand`.
