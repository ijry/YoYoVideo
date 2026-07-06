# Lightweight Playback Experience Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add window state restore, a recent-open menu, and configurable playback-end behavior while preserving the current default player behavior.

**Architecture:** Put durable playback-end semantics in `yoyo-core`, keep desktop persistence for recent-open and window state inside `apps/yoyovideo-desktop/src/platform`, and expose only small callback/property additions through Slint. Settings update the desktop runtime and the active `AppSession` without rebuilding the mpv backend.

**Tech Stack:** Rust 2024, Slint 1.17.0, libmpv via `yoyo-mpv`, `serde`, `toml`, `chrono`, `directories`, `tempfile`, PowerShell smoke scripts.

## Global Constraints

- No full media library, tagging, thumbnails, indexing, or database.
- No playlist save/load format in this phase.
- No global hotkeys or OS media key integration.
- No UI redesign beyond small settings/menu additions.
- No cloud sync or cross-device recent list.
- Existing default behavior remains `PlayNext` for users with no saved config.
- Existing config files must continue to load through serde defaults.
- Recent-open must remain independent from `remember_history`.
- Missing recent items and window-state persistence failures must never crash or clear playback.
- Settings save must apply playback-end behavior for future EOF handling without interrupting current playback.

---

## File Structure

- Modify `crates/yoyo-core/src/config.rs`: add `PlaybackEndBehavior`, serde defaults, and config compatibility.
- Modify `crates/yoyo-core/src/lib.rs`: export `PlaybackEndBehavior`.
- Modify `crates/yoyo-core/src/session.rs`: add config replacement and EOF behavior handling.
- Modify `crates/yoyo-core/tests/config_shortcut_contract.rs`: cover legacy config compatibility and default EOF behavior.
- Modify `crates/yoyo-core/tests/session_contract.rs`: cover EOF stop, loop-current, and loop-playlist behavior.
- Modify `apps/yoyovideo-desktop/src/settings_controller.rs`: expose playback-end behavior in settings snapshots, drafts, save, and restore.
- Modify `apps/yoyovideo-desktop/src/app.rs`: refresh settings UI, wire settings callback, sync saved config into the active session, load recent/window state, and route recent menu actions.
- Modify `apps/yoyovideo-desktop/ui/main-window.slint`: add settings playback-end controls and recent menu rows.
- Modify `apps/yoyovideo-desktop/tests/settings_contract.rs`: cover saving playback-end behavior.
- Modify `apps/yoyovideo-desktop/tests/settings_runtime_contract.rs`: cover runtime config sync.
- Modify `apps/yoyovideo-desktop/tests/context_menu_contract.rs`: cover recent callback/property surface.
- Create `apps/yoyovideo-desktop/src/platform/recent.rs`: recent-open item model, load/save, dedupe, cap, missing-path validation.
- Create `apps/yoyovideo-desktop/tests/recent_contract.rs`: recent-open store and dispatch behavior.
- Modify `apps/yoyovideo-desktop/src/platform/mod.rs`: export recent helpers.
- Create `apps/yoyovideo-desktop/src/platform/window_state.rs`: window state model, load/save, clamp helpers.
- Create `apps/yoyovideo-desktop/tests/window_state_contract.rs`: model tests for clamping, corrupt files, and missing files.
- Modify `docs/testing/manual-smoke-checklist.md`: add manual coverage for window restore, recent menu, and playback-end behavior.

---

### Task 1: Playback-End Behavior In Core

**Files:**
- Modify: `crates/yoyo-core/src/config.rs`
- Modify: `crates/yoyo-core/src/lib.rs`
- Modify: `crates/yoyo-core/src/session.rs`
- Modify: `crates/yoyo-core/tests/config_shortcut_contract.rs`
- Modify: `crates/yoyo-core/tests/session_contract.rs`

**Interfaces:**
- Produces: `pub enum PlaybackEndBehavior { PlayNext, Stop, LoopCurrent, LoopPlaylist }`
- Produces: `PlaybackDefaults::end_behavior: PlaybackEndBehavior`
- Produces: `AppSession<B>::set_config(&mut self, config: AppConfig)`
- Consumes: `BackendEvent::EndOfFile`

- [ ] **Step 1: Add failing config compatibility tests**

Append to `crates/yoyo-core/tests/config_shortcut_contract.rs`:

```rust
#[test]
fn default_playback_end_behavior_is_play_next() {
    let config = AppConfig::default();

    assert_eq!(config.playback.end_behavior, yoyo_core::PlaybackEndBehavior::PlayNext);
}

#[test]
fn legacy_config_without_playback_end_behavior_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[playback]
default_speed = 1.25
default_volume_percent = 80
prefer_hardware_decode = true

[ui]
remember_history = true
show_playlist_on_startup = false

[shortcuts.bindings]
"#,
    )
    .unwrap();

    let config = AppConfig::load(&path).unwrap();

    assert_eq!(config.playback.default_speed, 1.25);
    assert_eq!(config.playback.default_volume_percent, 80);
    assert_eq!(config.playback.end_behavior, yoyo_core::PlaybackEndBehavior::PlayNext);
    assert!(!config.ui.show_playlist_on_startup);
}
```

- [ ] **Step 2: Add failing EOF behavior tests**

Append to `crates/yoyo-core/tests/session_contract.rs`:

```rust
#[test]
fn eof_stop_behavior_does_not_advance_playlist() {
    let backend = MockBackend::default();
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::Stop;
    let mut session = AppSession::new(config, backend);
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

    assert_eq!(session.backend().opened, vec![MediaLocator::File(PathBuf::from("one.mp4"))]);
    assert!(session.state().paused);
    assert_eq!(session.state().status_message.as_deref(), Some("Playback ended"));
}

#[test]
fn eof_loop_current_reopens_current_playlist_item() {
    let backend = MockBackend::default();
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::LoopCurrent;
    let mut session = AppSession::new(config, backend);
    session
        .replace_playlist(vec![PlaylistEntry::new(MediaLocator::File(PathBuf::from("one.mp4")))], 0)
        .unwrap();

    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(
        session.backend().opened,
        vec![
            MediaLocator::File(PathBuf::from("one.mp4")),
            MediaLocator::File(PathBuf::from("one.mp4")),
        ]
    );
    assert!(!session.state().paused);
}

#[test]
fn eof_loop_playlist_wraps_from_last_item_to_first() {
    let backend = MockBackend::default();
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::LoopPlaylist;
    let mut session = AppSession::new(config, backend);
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("one.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("two.mp4"))),
            ],
            1,
        )
        .unwrap();

    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(
        session.backend().opened,
        vec![
            MediaLocator::File(PathBuf::from("two.mp4")),
            MediaLocator::File(PathBuf::from("one.mp4")),
        ]
    );
    assert_eq!(session.playlist_snapshot().current_index, Some(0));
}

#[test]
fn replacing_session_config_changes_future_eof_behavior() {
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
    let mut config = AppConfig::default();
    config.playback.end_behavior = yoyo_core::PlaybackEndBehavior::Stop;

    session.set_config(config);
    session.backend_mut().events.push(BackendEvent::EndOfFile);
    session.poll_backend().unwrap();

    assert_eq!(session.backend().opened, vec![MediaLocator::File(PathBuf::from("one.mp4"))]);
}
```

- [ ] **Step 3: Run failing core tests**

Run:

```powershell
cargo test -p yoyo-core --test config_shortcut_contract default_playback_end_behavior_is_play_next
cargo test -p yoyo-core --test config_shortcut_contract legacy_config_without_playback_end_behavior_still_loads
cargo test -p yoyo-core --test session_contract eof_stop_behavior_does_not_advance_playlist
```

Expected:

- Config tests fail because `PlaybackEndBehavior` and `PlaybackDefaults::end_behavior` do not exist.
- Session tests fail because EOF handling has no configurable behavior and `set_config` does not exist.

- [ ] **Step 4: Add playback-end config type and serde default**

Modify `crates/yoyo-core/src/config.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaybackEndBehavior {
    PlayNext,
    Stop,
    LoopCurrent,
    LoopPlaylist,
}

impl Default for PlaybackEndBehavior {
    fn default() -> Self {
        Self::PlayNext
    }
}
```

Then add the field to `PlaybackDefaults`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackDefaults {
    pub default_speed: f32,
    pub default_volume_percent: u8,
    pub prefer_hardware_decode: bool,
    #[serde(default)]
    pub end_behavior: PlaybackEndBehavior,
}
```

Update `AppConfig::default()` playback construction:

```rust
playback: PlaybackDefaults {
    default_speed: 1.0,
    default_volume_percent: 100,
    prefer_hardware_decode: true,
    end_behavior: PlaybackEndBehavior::PlayNext,
},
```

- [ ] **Step 5: Export the enum**

Modify `crates/yoyo-core/src/lib.rs` config exports:

```rust
pub use config::{
    AppConfig, MAX_DEFAULT_SPEED, MIN_DEFAULT_SPEED, PlaybackDefaults, PlaybackEndBehavior,
    UiPreferences,
};
```

- [ ] **Step 6: Implement configurable EOF behavior**

Modify `crates/yoyo-core/src/session.rs` imports to include `PlaybackEndBehavior`.

Add these helpers inside `impl<B: PlayerBackend> AppSession<B>` after `previous_playlist_index`:

```rust
fn current_playlist_index(&self) -> Option<usize> {
    self.playlist.current_index
}

fn first_playlist_index(&self) -> Option<usize> {
    (!self.playlist.entries.is_empty()).then_some(0)
}

pub fn set_config(&mut self, config: AppConfig) {
    self.config = config;
}

fn handle_end_of_file(&mut self) -> Result<(), AppError> {
    match self.config.playback.end_behavior {
        PlaybackEndBehavior::PlayNext => {
            if let Some(index) = self.next_playlist_index() {
                self.open_playlist_index(index)?;
            }
        }
        PlaybackEndBehavior::Stop => {
            self.state.paused = true;
            self.state.status_message = Some("Playback ended".to_string());
        }
        PlaybackEndBehavior::LoopCurrent => {
            if let Some(index) = self.current_playlist_index() {
                self.open_playlist_index(index)?;
            }
        }
        PlaybackEndBehavior::LoopPlaylist => {
            if let Some(index) = self.next_playlist_index().or_else(|| self.first_playlist_index())
            {
                self.open_playlist_index(index)?;
            }
        }
    }
    Ok(())
}
```

Replace the `BackendEvent::EndOfFile` branch in `poll_backend` with:

```rust
BackendEvent::EndOfFile => self.handle_end_of_file()?,
```

- [ ] **Step 7: Run core tests**

Run:

```powershell
cargo test -p yoyo-core --test config_shortcut_contract
cargo test -p yoyo-core --test session_contract
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add crates/yoyo-core/src/config.rs crates/yoyo-core/src/lib.rs crates/yoyo-core/src/session.rs crates/yoyo-core/tests/config_shortcut_contract.rs crates/yoyo-core/tests/session_contract.rs
git commit -m "feat: add playback end behavior"
```

Expected: Commit succeeds.

---

### Task 2: Settings UI For Playback-End Behavior

**Files:**
- Modify: `apps/yoyovideo-desktop/src/settings_controller.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/settings_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/settings_runtime_contract.rs`

**Interfaces:**
- Consumes: `yoyo_core::PlaybackEndBehavior`
- Consumes: `AppSession<B>::set_config(AppConfig)`
- Produces: `SettingsSnapshot::playback_end_behavior_index: i32`
- Produces: `SettingsController::set_playback_end_behavior(PlaybackEndBehavior)`
- Produces: `SettingsController::set_playback_end_behavior_index(i32)`
- Produces Slint property `playback_end_behavior_index` and callback `playback_end_behavior_changed(int)`

- [ ] **Step 1: Add failing settings persistence test**

Append to `apps/yoyovideo-desktop/tests/settings_contract.rs`:

```rust
#[test]
fn save_persists_playback_end_behavior() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.set_playback_end_behavior(yoyo_core::PlaybackEndBehavior::LoopPlaylist);

    let saved = controller.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(saved.playback.end_behavior, yoyo_core::PlaybackEndBehavior::LoopPlaylist);
    assert_eq!(loaded.playback.end_behavior, yoyo_core::PlaybackEndBehavior::LoopPlaylist);
    assert_eq!(controller.snapshot().playback_end_behavior_index, 3);
}
```

- [ ] **Step 2: Add failing runtime settings test**

Modify `apps/yoyovideo-desktop/tests/settings_runtime_contract.rs` imports:

```rust
use yoyo_core::{
    AppConfig, AppSession, BackendCommand, BackendEvent, MediaLocator, PlaybackEndBehavior,
    PlayerBackend, PlaylistEntry, Shortcut, ShortcutAction, ShortcutMap,
};
```

Add `events` to the local `MockBackend`:

```rust
#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
    events: Vec<BackendEvent>,
}
```

Replace its `drain_events` implementation:

```rust
fn drain_events(&mut self) -> Vec<BackendEvent> {
    std::mem::take(&mut self.events)
}
```

Append the test:

```rust
#[test]
fn saved_playback_end_behavior_updates_active_session() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);
    controller
        .open_playlist_entries(vec![
            PlaylistEntry::new(MediaLocator::File("one.mp4".into())),
            PlaylistEntry::new(MediaLocator::File("two.mp4".into())),
        ])
        .unwrap();
    let mut config = AppConfig::default();
    config.playback.end_behavior = PlaybackEndBehavior::Stop;

    controller.set_config(config);
    controller.session_mut().backend_mut().events.push(BackendEvent::EndOfFile);
    controller.poll_backend().unwrap();

    assert_eq!(
        controller.session().backend().opened,
        vec![MediaLocator::File("one.mp4".into())]
    );
}
```

- [ ] **Step 3: Add failing Slint callback contract**

Modify `apps/yoyovideo-desktop/tests/context_menu_contract.rs` or create `apps/yoyovideo-desktop/tests/settings_window_contract.rs` extension:

```rust
#[test]
fn settings_window_playback_end_behavior_surface_compiles() {
    let window = yoyovideo_desktop::SettingsWindow::new().unwrap();

    window.set_playback_end_behavior_index(2);
    assert_eq!(window.get_playback_end_behavior_index(), 2);
    window.on_playback_end_behavior_changed(|_| {});
}
```

- [ ] **Step 4: Run failing settings tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_contract save_persists_playback_end_behavior
cargo test -p yoyovideo-desktop --test settings_runtime_contract saved_playback_end_behavior_updates_active_session
cargo test -p yoyovideo-desktop --test settings_window_contract settings_window_playback_end_behavior_surface_compiles
```

Expected:

- Settings test fails because `SettingsController::set_playback_end_behavior` and snapshot field do not exist.
- Runtime test fails because `DesktopController::set_config` or `session_mut` does not exist.
- Slint test fails because the property/callback does not exist.

- [ ] **Step 5: Add settings controller state and mapping**

Modify `apps/yoyovideo-desktop/src/settings_controller.rs`.

Update imports:

```rust
use yoyo_core::{
    AppConfig, AppError, PlaybackEndBehavior, Shortcut, ShortcutAction, ShortcutMap, StorageError,
    ValidationError,
};
```

Add to `SettingsSnapshot`:

```rust
pub playback_end_behavior_index: i32,
```

Add to `SettingsDraft`:

```rust
playback_end_behavior: PlaybackEndBehavior,
```

In `SettingsDraft::from_config`:

```rust
playback_end_behavior: config.playback.end_behavior,
```

In `SettingsDraft::to_config`:

```rust
config.playback.end_behavior = self.playback_end_behavior;
```

Add helper functions near `impl SettingsDraft`:

```rust
fn playback_end_behavior_index(value: PlaybackEndBehavior) -> i32 {
    match value {
        PlaybackEndBehavior::PlayNext => 0,
        PlaybackEndBehavior::Stop => 1,
        PlaybackEndBehavior::LoopCurrent => 2,
        PlaybackEndBehavior::LoopPlaylist => 3,
    }
}

fn playback_end_behavior_from_index(index: i32) -> PlaybackEndBehavior {
    match index {
        1 => PlaybackEndBehavior::Stop,
        2 => PlaybackEndBehavior::LoopCurrent,
        3 => PlaybackEndBehavior::LoopPlaylist,
        _ => PlaybackEndBehavior::PlayNext,
    }
}
```

In `SettingsController::snapshot`:

```rust
playback_end_behavior_index: playback_end_behavior_index(self.draft.playback_end_behavior),
```

Add public setters:

```rust
pub fn set_playback_end_behavior(&mut self, value: PlaybackEndBehavior) {
    self.draft.playback_end_behavior = value;
}

pub fn set_playback_end_behavior_index(&mut self, index: i32) {
    self.draft.playback_end_behavior = playback_end_behavior_from_index(index);
}
```

- [ ] **Step 6: Add DesktopController config sync**

Modify `apps/yoyovideo-desktop/src/app.rs` in `impl<B: PlayerBackend> DesktopController<B>`:

```rust
pub fn session_mut(&mut self) -> &mut AppSession<B> {
    &mut self.session
}

pub fn set_config(&mut self, config: AppConfig) {
    self.session.set_config(config);
}
```

Modify `apply_saved_settings`:

```rust
fn apply_saved_settings(runtime: &mut DesktopRuntime, saved: AppConfig) {
    if let Some(controller) = runtime.controller_mut() {
        controller.set_shortcuts(saved.shortcuts.clone());
        controller.set_config(saved.clone());
    }
    runtime.history.set_enabled(saved.ui.remember_history);
    runtime.config = saved;
}
```

- [ ] **Step 7: Wire settings window Rust callbacks**

Modify `refresh_settings_window` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
window.set_playback_end_behavior_index(snapshot.playback_end_behavior_index);
```

In the Settings window setup block, after `window.on_default_volume_changed`, add:

```rust
window.on_playback_end_behavior_changed({
    let runtime = Rc::clone(&runtime);
    let app_handle = app_handle.clone();
    move |index| {
        mutate_settings_controller(&mut runtime.borrow_mut(), |controller| {
            controller.set_playback_end_behavior_index(index);
        });
        if let Some(app) = app_handle.upgrade() {
            app.set_status_label("Settings changed".into());
        }
    }
});
```

- [ ] **Step 8: Add Slint settings controls**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` in `SettingsWindow` properties:

```slint
in-out property <int> playback_end_behavior_index: 0;
```

Add callback:

```slint
callback playback_end_behavior_changed(int);
```

In the Playback settings section near default speed/volume controls, add:

```slint
Text { text: "Playback End"; color: #f2f5f7; }
HorizontalBox {
    spacing: 6px;
    Button {
        text: root.playback_end_behavior_index == 0 ? "Play Next *" : "Play Next";
        clicked => { root.playback_end_behavior_changed(0); }
    }
    Button {
        text: root.playback_end_behavior_index == 1 ? "Stop *" : "Stop";
        clicked => { root.playback_end_behavior_changed(1); }
    }
    Button {
        text: root.playback_end_behavior_index == 2 ? "Loop Current *" : "Loop Current";
        clicked => { root.playback_end_behavior_changed(2); }
    }
    Button {
        text: root.playback_end_behavior_index == 3 ? "Loop Playlist *" : "Loop Playlist";
        clicked => { root.playback_end_behavior_changed(3); }
    }
}
```

- [ ] **Step 9: Run settings tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_contract
cargo test -p yoyovideo-desktop --test settings_runtime_contract
cargo test -p yoyovideo-desktop --test settings_window_contract
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/settings_controller.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/settings_contract.rs apps/yoyovideo-desktop/tests/settings_runtime_contract.rs apps/yoyovideo-desktop/tests/settings_window_contract.rs
git commit -m "feat: expose playback end setting"
```

Expected: Commit succeeds.

---

### Task 3: Recent-Open Store

**Files:**
- Create: `apps/yoyovideo-desktop/src/platform/recent.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Create: `apps/yoyovideo-desktop/tests/recent_contract.rs`

**Interfaces:**
- Produces: `pub enum RecentOpenKind { File, Folder, Url }`
- Produces: `pub struct RecentOpenItem { kind, target, title, opened_at }`
- Produces: `pub struct RecentOpenStore { items: Vec<RecentOpenItem> }`
- Produces: `pub const MAX_RECENT_OPEN_ITEMS: usize = 10`
- Produces: `RecentOpenStore::remember(&mut self, item: RecentOpenItem)`
- Produces: `RecentOpenStore::load(path: Option<PathBuf>) -> Result<Self, StorageError>`
- Produces: `RecentOpenStore::save(&self) -> Result<(), StorageError>`
- Produces: `recent_open_path(paths: Option<&AppPaths>) -> Option<PathBuf>`

- [ ] **Step 1: Write failing recent store tests**

Create `apps/yoyovideo-desktop/tests/recent_contract.rs`:

```rust
use tempfile::tempdir;
use yoyovideo_desktop::platform::{
    RecentOpenItem, RecentOpenKind, RecentOpenStore, MAX_RECENT_OPEN_ITEMS,
};

fn item(kind: RecentOpenKind, target: &str, title: &str, opened_at: &str) -> RecentOpenItem {
    RecentOpenItem {
        kind,
        target: target.to_string(),
        title: title.to_string(),
        opened_at: opened_at.to_string(),
    }
}

#[test]
fn recent_store_deduplicates_newest_first_and_caps_at_ten() {
    let mut store = RecentOpenStore::default();

    for index in 0..12 {
        store.remember(item(
            RecentOpenKind::File,
            &format!("movie-{index}.mp4"),
            &format!("movie-{index}.mp4"),
            &format!("2026-07-06T10:{index:02}:00+08:00"),
        ));
    }
    store.remember(item(
        RecentOpenKind::File,
        "movie-5.mp4",
        "movie-5.mp4",
        "2026-07-06T11:00:00+08:00",
    ));

    assert_eq!(store.items.len(), MAX_RECENT_OPEN_ITEMS);
    assert_eq!(store.items[0].target, "movie-5.mp4");
    assert_eq!(
        store.items.iter().filter(|entry| entry.target == "movie-5.mp4").count(),
        1
    );
}

#[test]
fn recent_store_round_trips_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recent.toml");
    let mut store = RecentOpenStore::with_path(Some(path.clone()));
    store.remember(item(
        RecentOpenKind::Folder,
        "D:/Media",
        "Media",
        "2026-07-06T10:00:00+08:00",
    ));

    store.save().unwrap();
    let loaded = RecentOpenStore::load(Some(path)).unwrap();

    assert_eq!(loaded.items, store.items);
}

#[test]
fn recent_store_missing_file_loads_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing.toml");

    let store = RecentOpenStore::load(Some(path)).unwrap();

    assert!(store.items.is_empty());
}

#[test]
fn recent_store_corrupt_file_loads_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recent.toml");
    std::fs::write(&path, "not valid toml").unwrap();

    let store = RecentOpenStore::load(Some(path)).unwrap();

    assert!(store.items.is_empty());
}
```

- [ ] **Step 2: Run failing recent tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test recent_contract
```

Expected: FAIL because recent-open types do not exist.

- [ ] **Step 3: Implement recent-open store**

Create `apps/yoyovideo-desktop/src/platform/recent.rs`:

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::StorageError;

use super::AppPaths;

pub const MAX_RECENT_OPEN_ITEMS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecentOpenKind {
    File,
    Folder,
    Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentOpenItem {
    pub kind: RecentOpenKind,
    pub target: String,
    pub title: String,
    pub opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecentOpenStore {
    #[serde(skip)]
    path: Option<PathBuf>,
    pub items: Vec<RecentOpenItem>,
}

impl RecentOpenStore {
    pub fn with_path(path: Option<PathBuf>) -> Self {
        Self { path, items: Vec::new() }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let Some(path_value) = path else {
            return Ok(Self::default());
        };
        if !path_value.exists() {
            return Ok(Self::with_path(Some(path_value)));
        }
        let raw = fs::read_to_string(&path_value)?;
        let mut store = toml::from_str::<RecentOpenStore>(&raw).unwrap_or_default();
        store.path = Some(path_value);
        store.items.truncate(MAX_RECENT_OPEN_ITEMS);
        Ok(store)
    }

    pub fn remember(&mut self, item: RecentOpenItem) {
        self.items
            .retain(|existing| existing.kind != item.kind || existing.target != item.target);
        self.items.insert(0, item);
        self.items.truncate(MAX_RECENT_OPEN_ITEMS);
    }

    pub fn save(&self) -> Result<(), StorageError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}

pub fn recent_open_path(paths: Option<&AppPaths>) -> Option<PathBuf> {
    paths.map(|paths| paths.data_dir.join("recent-open.toml"))
}
```

- [ ] **Step 4: Export recent helpers**

Modify `apps/yoyovideo-desktop/src/platform/mod.rs`:

```rust
mod recent;
```

Add exports:

```rust
pub use recent::{
    MAX_RECENT_OPEN_ITEMS, RecentOpenItem, RecentOpenKind, RecentOpenStore, recent_open_path,
};
```

- [ ] **Step 5: Run recent tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test recent_contract
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/platform/recent.rs apps/yoyovideo-desktop/src/platform/mod.rs apps/yoyovideo-desktop/tests/recent_contract.rs
git commit -m "feat: add recent open store"
```

Expected: Commit succeeds.

---

### Task 4: Recent Menu And Dispatch Wiring

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/recent_contract.rs`

**Interfaces:**
- Consumes: `RecentOpenStore`, `RecentOpenItem`, `RecentOpenKind`
- Produces: Slint struct `RecentOpenRowData { title: string, subtitle: string }`
- Produces: Slint property `recent_open_rows: [RecentOpenRowData]`
- Produces: Slint callback `recent_open_item_requested(int)`
- Produces: `pub fn recent_item_status(item: &RecentOpenItem) -> String`

- [ ] **Step 1: Add failing Slint recent menu contract**

Append to `apps/yoyovideo-desktop/tests/context_menu_contract.rs`:

```rust
#[test]
fn main_window_recent_open_menu_surface_compiles() {
    let window = MainWindow::new().unwrap();

    window.on_recent_open_item_requested(|_| {});
    assert_eq!(window.get_recent_open_rows().row_count(), 0);
}
```

- [ ] **Step 2: Add failing recent status test**

Append to `apps/yoyovideo-desktop/tests/recent_contract.rs`:

```rust
#[test]
fn recent_item_status_describes_selected_item() {
    let row = item(
        RecentOpenKind::Url,
        "https://example.test/movie.mp4",
        "movie.mp4",
        "2026-07-06T10:00:00+08:00",
    );

    assert_eq!(
        yoyovideo_desktop::recent_item_status(&row),
        "Opening recent URL: https://example.test/movie.mp4"
    );
}
```

- [ ] **Step 3: Run failing recent UI tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract main_window_recent_open_menu_surface_compiles
cargo test -p yoyovideo-desktop --test recent_contract recent_item_status_describes_selected_item
```

Expected:

- Slint contract fails because recent row surface does not exist.
- Status test fails because `recent_item_status` is not exported.

- [ ] **Step 4: Add Slint recent row surface and menu entries**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` near other exported structs:

```slint
export struct RecentOpenRowData {
    title: string,
    subtitle: string,
}
```

Add property and callback to `MainWindow`:

```slint
in-out property <[RecentOpenRowData]> recent_open_rows: [];
callback recent_open_item_requested(int);
```

Increase `menu_popup` height enough for recent rows:

```slint
height: 660px;
```

Add this block after `Open Folder`:

```slint
Text { text: "Recent"; color: #9ba6ad; }
if root.recent_open_rows.length == 0: Text {
    text: "No Recent Items";
    color: #66737c;
}
for row[index] in root.recent_open_rows: Button {
    text: row.title;
    clicked => {
        root.recent_open_item_requested(index);
        menu_popup.close();
    }
}
```

- [ ] **Step 5: Add Rust recent row mapping and status helper**

Modify `apps/yoyovideo-desktop/src/app.rs`.

Import recent types where needed:

```rust
use crate::platform::{RecentOpenItem, RecentOpenKind, RecentOpenStore};
```

Add public helper near `dropped_media_status`:

```rust
pub fn recent_item_status(item: &RecentOpenItem) -> String {
    match item.kind {
        RecentOpenKind::File => format!("Opening recent file: {}", item.target),
        RecentOpenKind::Folder => format!("Opening recent folder: {}", item.target),
        RecentOpenKind::Url => format!("Opening recent URL: {}", item.target),
    }
}
```

Add `RecentOpenStore` to `DesktopRuntime`:

```rust
recent_open: RecentOpenStore,
```

Update `DesktopRuntime::new` parameters and call sites to include `recent_open`.

Add a refresh helper:

```rust
fn refresh_recent_open_menu(window: &MainWindow, runtime: &DesktopRuntime) {
    let rows = runtime
        .recent_open
        .items
        .iter()
        .map(|item| RecentOpenRowData {
            title: item.title.clone().into(),
            subtitle: item.target.clone().into(),
        })
        .collect::<Vec<_>>();
    window.set_recent_open_rows(model_from_vec(rows));
}
```

Call `refresh_recent_open_menu(&app, &runtime.borrow())` after creating the runtime and after successful recent updates.

Export the helper in `apps/yoyovideo-desktop/src/lib.rs`:

```rust
recent_item_status,
```

- [ ] **Step 6: Load recent store at startup**

Modify `run()` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
let recent_open = crate::platform::RecentOpenStore::load(crate::platform::recent_open_path(paths.as_ref()))
    .unwrap_or_else(|_| crate::platform::RecentOpenStore::default());
```

Pass `recent_open` into `DesktopRuntime::new`.

- [ ] **Step 7: Add recent recording helpers**

Add to `apps/yoyovideo-desktop/src/app.rs`:

```rust
fn recent_title_for_target(kind: RecentOpenKind, target: &str) -> String {
    match kind {
        RecentOpenKind::File | RecentOpenKind::Folder => std::path::Path::new(target)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(target)
            .to_string(),
        RecentOpenKind::Url => target.to_string(),
    }
}

fn remember_recent_open(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    kind: RecentOpenKind,
    target: String,
) {
    let opened_at = chrono::Local::now().to_rfc3339();
    let title = recent_title_for_target(kind, &target);
    let mut runtime = runtime.borrow_mut();
    runtime.recent_open.remember(RecentOpenItem { kind, target, title, opened_at });
    if let Err(error) = runtime.recent_open.save() {
        runtime.record_diagnostic("WARN", format!("Recent open save failed: {error}"));
    }
    if let Some(app) = app_handle.upgrade() {
        refresh_recent_open_menu(&app, &runtime);
    }
}
```

After successful file, folder, and URL opens in their existing callbacks, call:

```rust
remember_recent_open(&app_handle, &runtime, RecentOpenKind::File, path.display().to_string());
```

Use `RecentOpenKind::Folder` for folder callbacks and `RecentOpenKind::Url` for URL callbacks. For dropped single file, add the same file recent item after successful dispatch. For dropped playlist from a single folder, remember the folder path in the drop dispatch branch only when the original drop input contains exactly one folder.

- [ ] **Step 8: Add recent dispatch callback**

Add callback wiring in `run()`:

```rust
app.on_recent_open_item_requested({
    let runtime = Rc::clone(&runtime);
    let app_handle = app_handle.clone();
    move |index| {
        let item = {
            let runtime = runtime.borrow();
            runtime.recent_open.items.get(index as usize).cloned()
        };
        let Some(item) = item else {
            return;
        };
        if let Some(app) = app_handle.upgrade() {
            app.set_status_label(recent_item_status(&item).into());
        }
        match item.kind {
            RecentOpenKind::File => {
                let path = PathBuf::from(&item.target);
                if !path.is_file() {
                    if let Some(app) = app_handle.upgrade() {
                        app.set_status_label(format!("Recent item is missing: {}", path.display()).into());
                    }
                    return;
                }
                with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.dispatch(AppCommand::OpenFile(path))
                });
            }
            RecentOpenKind::Folder => {
                let path = PathBuf::from(&item.target);
                if !path.is_dir() {
                    if let Some(app) = app_handle.upgrade() {
                        app.set_status_label(format!("Recent item is missing: {}", path.display()).into());
                    }
                    return;
                }
                with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.open_folder(path)
                });
            }
            RecentOpenKind::Url => {
                let target = item.target.clone();
                with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.dispatch(AppCommand::OpenUrl(target))
                });
            }
        }
    }
});
```

- [ ] **Step 9: Run recent UI tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo test -p yoyovideo-desktop --test recent_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/context_menu_contract.rs apps/yoyovideo-desktop/tests/recent_contract.rs
git commit -m "feat: wire recent open menu"
```

Expected: Commit succeeds.

---

### Task 5: Window State Persistence

**Files:**
- Create: `apps/yoyovideo-desktop/src/platform/window_state.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Create: `apps/yoyovideo-desktop/tests/window_state_contract.rs`

**Interfaces:**
- Produces: `pub struct WindowState { width, height, x, y, maximized }`
- Produces: `pub const MIN_WINDOW_WIDTH: u32 = 900`
- Produces: `pub const MIN_WINDOW_HEIGHT: u32 = 560`
- Produces: `WindowState::clamped(self) -> Self`
- Produces: `load_window_state(path: Option<PathBuf>) -> Result<Option<WindowState>, StorageError>`
- Produces: `save_window_state(path: Option<PathBuf>, state: &WindowState) -> Result<(), StorageError>`
- Produces: `window_state_path(paths: Option<&AppPaths>) -> Option<PathBuf>`

- [ ] **Step 1: Write failing window state tests**

Create `apps/yoyovideo-desktop/tests/window_state_contract.rs`:

```rust
use tempfile::tempdir;
use yoyovideo_desktop::platform::{
    load_window_state, save_window_state, WindowState, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH,
};

#[test]
fn window_state_clamps_too_small_sizes() {
    let state = WindowState {
        width: 100,
        height: 100,
        x: Some(20),
        y: Some(30),
        maximized: false,
    }
    .clamped();

    assert_eq!(state.width, MIN_WINDOW_WIDTH);
    assert_eq!(state.height, MIN_WINDOW_HEIGHT);
    assert_eq!(state.x, Some(20));
    assert_eq!(state.y, Some(30));
}

#[test]
fn window_state_round_trips_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("window-state.toml");
    let state = WindowState {
        width: 1280,
        height: 720,
        x: Some(10),
        y: Some(20),
        maximized: true,
    };

    save_window_state(Some(path.clone()), &state).unwrap();
    let loaded = load_window_state(Some(path)).unwrap().unwrap();

    assert_eq!(loaded, state);
}

#[test]
fn window_state_missing_file_returns_none() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing.toml");

    assert_eq!(load_window_state(Some(path)).unwrap(), None);
}

#[test]
fn window_state_corrupt_file_returns_none() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("window-state.toml");
    std::fs::write(&path, "not valid toml").unwrap();

    assert_eq!(load_window_state(Some(path)).unwrap(), None);
}
```

- [ ] **Step 2: Run failing window state tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test window_state_contract
```

Expected: FAIL because window-state helpers do not exist.

- [ ] **Step 3: Implement window state model and persistence**

Create `apps/yoyovideo-desktop/src/platform/window_state.rs`:

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::StorageError;

use super::AppPaths;

pub const MIN_WINDOW_WIDTH: u32 = 900;
pub const MIN_WINDOW_HEIGHT: u32 = 560;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

impl WindowState {
    pub fn clamped(self) -> Self {
        Self {
            width: self.width.max(MIN_WINDOW_WIDTH),
            height: self.height.max(MIN_WINDOW_HEIGHT),
            x: self.x,
            y: self.y,
            maximized: self.maximized,
        }
    }
}

pub fn window_state_path(paths: Option<&AppPaths>) -> Option<PathBuf> {
    paths.map(|paths| paths.config_dir.join("window-state.toml"))
}

pub fn load_window_state(path: Option<PathBuf>) -> Result<Option<WindowState>, StorageError> {
    let Some(path) = path else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    Ok(toml::from_str::<WindowState>(&raw).ok().map(WindowState::clamped))
}

pub fn save_window_state(path: Option<PathBuf>, state: &WindowState) -> Result<(), StorageError> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(state)?;
    fs::write(path, raw)?;
    Ok(())
}
```

- [ ] **Step 4: Export window state helpers**

Modify `apps/yoyovideo-desktop/src/platform/mod.rs`:

```rust
mod window_state;
```

Add exports:

```rust
pub use window_state::{
    MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, WindowState, load_window_state, save_window_state,
    window_state_path,
};
```

- [ ] **Step 5: Wire best-effort restore and save**

Modify `apps/yoyovideo-desktop/src/app.rs`.

Add to `DesktopRuntime`:

```rust
window_state_path: Option<PathBuf>,
```

Pass `crate::platform::window_state_path(paths.as_ref())` into `DesktopRuntime::new`.

After `let app = MainWindow::new()?;`, load and apply:

```rust
if let Ok(Some(state)) = crate::platform::load_window_state(crate::platform::window_state_path(paths.as_ref())) {
    app.window().set_size(slint::PhysicalSize::new(state.width, state.height));
    if let (Some(x), Some(y)) = (state.x, state.y) {
        app.window().set_position(slint::PhysicalPosition::new(x, y));
    }
    app.window().set_maximized(state.maximized);
}
```

Add this helper near the other desktop runtime helpers:

```rust
fn save_current_window_state(
    runtime: &Rc<RefCell<DesktopRuntime>>,
    window: &slint::winit_030::winit::window::Window,
) {
    let size = window.inner_size();
    let position = window.outer_position().ok();
    let state = crate::platform::WindowState {
        width: size.width,
        height: size.height,
        x: position.map(|position| position.x),
        y: position.map(|position| position.y),
        maximized: window.is_maximized(),
    }
    .clamped();
    let mut runtime = runtime.borrow_mut();
    if let Err(error) =
        crate::platform::save_window_state(runtime.window_state_path.clone(), &state)
    {
        runtime.record_diagnostic("WARN", format!("Window state save failed: {error}"));
    }
}
```

Inside the existing `app.window().on_winit_window_event` closure, rename the closure argument from `_window` to `window` and save state for move, resize, and close events before shortcut handling:

```rust
match event {
    slint::winit_030::winit::event::WindowEvent::Moved(_)
    | slint::winit_030::winit::event::WindowEvent::Resized(_)
    | slint::winit_030::winit::event::WindowEvent::CloseRequested => {
        save_current_window_state(&runtime, window);
    }
    _ => {}
}
```

- [ ] **Step 6: Run window state tests and desktop check**

Run:

```powershell
cargo test -p yoyovideo-desktop --test window_state_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/platform/window_state.rs apps/yoyovideo-desktop/src/platform/mod.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/tests/window_state_contract.rs
git commit -m "feat: remember window state"
```

Expected: Commit succeeds.

---

### Task 6: Final Documentation And Verification

**Files:**
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes: all prior tasks.
- Produces: manual coverage for playback-end behavior, recent menu, and window state restore.

- [ ] **Step 1: Add manual smoke coverage**

Modify `docs/testing/manual-smoke-checklist.md` under `## UX` and add:

```markdown
- Resize and move the window, close the app, restart, and confirm size and position are restored.
- Maximize the window, close the app, restart, and confirm the maximized state is restored.
- Open several files, folders, and URLs, then confirm the `Recent` section in the menu shows newest entries first.
- Select a recent file, folder, and URL, and confirm each opens through the normal playback path.
- Remove a recent local file, select it from the menu, and confirm the app shows a non-fatal missing item message without clearing current playback.
- Change playback-end behavior in settings to `Stop`, `Loop Current`, `Loop Playlist`, and `Play Next`, then confirm EOF handling matches each mode.
```

- [ ] **Step 2: Run full verification**

Run:

```powershell
cargo fmt --check
cargo test
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
pwsh -NoProfile -File scripts/test-package-smoke.ps1
```

Expected:

- `cargo fmt --check`: PASS
- `cargo test`: PASS
- `cargo check -p yoyo-mpv --features mpv-runtime`: PASS
- `cargo check -p yoyovideo-desktop --features mpv-runtime`: PASS
- `scripts/test-package-smoke.ps1`: PASS

- [ ] **Step 3: Optional runtime smoke when Windows runtime is staged**

Run if `third_party/mpv/windows-x64/bin/mpv-2.dll` exists:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64 -TimeoutSeconds 8
```

Expected: PASS with `runtime_smoke=ok`.

- [ ] **Step 4: Confirm git status**

Run:

```powershell
git status --short
```

Expected: Only planned documentation or formatting changes remain before the final commit.

- [ ] **Step 5: Commit final verification docs**

Run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add lightweight playback smoke checks"
```

Expected: Commit succeeds.

---

## Self-Review

**Spec coverage:** Task 1 covers playback-end behavior and config compatibility. Task 2 covers settings UI and runtime config application. Task 3 covers recent-open storage independent from history. Task 4 covers recent menu display, successful open recording, missing item safety, and dispatch. Task 5 covers window state persistence and best-effort restore/save. Task 6 covers manual smoke and full verification.

**Directive scan:** All task instructions are concrete. Each task includes exact files, expected interfaces, test code, implementation snippets, commands, expected outcomes, and commit messages.

**Type consistency:** `PlaybackEndBehavior`, `SettingsSnapshot::playback_end_behavior_index`, `RecentOpenItem`, `RecentOpenStore`, `WindowState`, and Slint callback/property names are introduced before later tasks consume them. Desktop persistence remains in `platform`; core playback semantics remain in `yoyo-core`.
