# Playlist History Sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a right-side collapsible `Playlist / History` sidebar to YoYoVideo, backed by persistent playback history and queue selection, without breaking the existing controller/runtime boundary.

**Architecture:** Keep playlist and history semantics inside `yoyo-core`, add two focused desktop helper modules for sidebar row mapping and history persistence/resume state, and keep Slint declarative by feeding it simple row arrays and click callbacks. Desktop startup loads config/history once from `AppPaths`, initializes sidebar visibility from config plus startup width, and keeps history flushing plus pending resume seek inside the desktop runtime.

**Tech Stack:** Rust 2024, Slint 1.17.0, `directories` 6.0, existing `yoyo-core` / `yoyovideo-desktop` crates, PowerShell verification commands.

## Global Constraints

- Startup visibility follows `ui.show_playlist_on_startup`, except windows narrower than `1050px`, which start collapsed.
- Collapsed state keeps a `36px` edge strip with a visible affordance to reopen.
- Expanded state uses a `320px` target width with two tabs at the top, `Playlist` and `History`.
- Narrow windows keep the same layout model, but use a reduced `260px` sidebar width instead of introducing an overlay mode in this phase.
- Opening a single file or URL replaces the current playlist with one item.
- Opening a folder replaces the current playlist with the scanned media entries and starts from the first playable item.
- Clicking a playlist row switches playback to that item and updates current-row highlight.
- Clicking a history row reopens that media and resumes from the stored progress position.
- History activation restores a single media item only. It does not restore the original historical queue.
- History is loaded once during startup.
- History writes are throttled to at most once every 2 seconds during active playback, with an immediate flush on pause, media switch, and app shutdown.
- After a successful history write, the desktop history view is refreshed from the current in-memory model.
- If a history entry points to a missing local file, activation fails gracefully and reports a clear status-bar error.
- If reopening a historical URL fails, the existing session/backend error path is reused.
- If a stored resume position exceeds the playable duration, the restored seek position is clamped to the playable range.
- If playlist snapshot data is temporarily inconsistent, the sidebar falls back to no active highlight rather than panicking.
- Slint stays declarative and mostly dumb. Row mapping and activation rules live in Rust, not in complex Slint logic.

---

## File Structure

- `crates/yoyo-core/src/playlist.rs`: add playlist snapshot and safe index-selection helpers.
- `crates/yoyo-core/src/history.rs`: add history upsert/read helpers so `yoyo-core` owns persistence semantics.
- `crates/yoyo-core/src/session.rs`: make single-file/URL opens replace the playlist, expose playlist snapshots, and add safe queue-index activation.
- `crates/yoyo-core/src/lib.rs`: export the new playlist snapshot type.
- `crates/yoyo-core/tests/playlist_history_contract.rs`: core playlist/history regression coverage.
- `apps/yoyovideo-desktop/src/sidebar.rs`: pure sidebar state, width policy, and row mapping.
- `apps/yoyovideo-desktop/src/history_runtime.rs`: pure desktop history persistence, activation generation, throttle policy, and pending resume seek clamp.
- `apps/yoyovideo-desktop/src/app.rs`: integrate config/history load, sidebar callbacks, row refresh, pending resume application, and shutdown flush.
- `apps/yoyovideo-desktop/src/lib.rs`: export new pure helpers for tests.
- `apps/yoyovideo-desktop/ui/main-window.slint`: add the right sidebar surface, row arrays, and click callbacks.
- `apps/yoyovideo-desktop/tests/sidebar_contract.rs`: pure sidebar-state and row-mapping tests.
- `apps/yoyovideo-desktop/tests/history_runtime_contract.rs`: history throttling, activation, and resume-clamp tests.
- `apps/yoyovideo-desktop/tests/controller_contract.rs`: controller-level queue-index activation regression coverage.
- `docs/testing/manual-smoke-checklist.md`: add sidebar and history-resume manual checks.

---

### Task 1: Core Playlist Snapshot And Safe Queue Activation

**Files:**
- Create: `crates/yoyo-core/tests/playlist_history_contract.rs`
- Modify: `crates/yoyo-core/src/playlist.rs`
- Modify: `crates/yoyo-core/src/session.rs`
- Modify: `crates/yoyo-core/src/lib.rs`

**Interfaces:**
- Produces: `PlaylistSnapshot { entries: Vec<PlaylistEntry>, current_index: Option<usize> }`
- Produces: `Playlist::select(&mut self, index: usize) -> Option<&PlaylistEntry>`
- Produces: `Playlist::snapshot(&self) -> PlaylistSnapshot`
- Produces: `AppSession::playlist_snapshot(&self) -> PlaylistSnapshot`
- Produces: `AppSession::open_playlist_index(&mut self, index: usize) -> Result<(), AppError>`

- [ ] **Step 1: Write the failing core playlist tests**

Create `crates/yoyo-core/tests/playlist_history_contract.rs`:

```rust
use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, BackendEvent, MediaLocator, PlayerBackend,
    PlaylistEntry,
};

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

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        Vec::new()
    }
}

#[test]
fn open_file_replaces_playlist_with_a_single_entry_snapshot() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("first.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("second.mp4"))),
            ],
            1,
        )
        .unwrap();

    session
        .handle_command(AppCommand::OpenFile(PathBuf::from("solo.mkv")))
        .unwrap();

    let snapshot = session.playlist_snapshot();
    assert_eq!(snapshot.current_index, Some(0));
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(
        snapshot.entries[0].locator,
        MediaLocator::File(PathBuf::from("solo.mkv"))
    );
    assert_eq!(
        session.state().current,
        Some(MediaLocator::File(PathBuf::from("solo.mkv")))
    );
}

#[test]
fn open_playlist_index_switches_to_the_requested_queue_entry() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("alpha.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("beta.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("gamma.mp4"))),
            ],
            0,
        )
        .unwrap();

    session.open_playlist_index(2).unwrap();

    let snapshot = session.playlist_snapshot();
    assert_eq!(snapshot.current_index, Some(2));
    assert_eq!(
        session.backend().opened.last(),
        Some(&MediaLocator::File(PathBuf::from("gamma.mp4")))
    );
    assert_eq!(
        session.state().current,
        Some(MediaLocator::File(PathBuf::from("gamma.mp4")))
    );
}
```

- [ ] **Step 2: Run the failing core playlist tests**

Run:

```powershell
cargo test -p yoyo-core --test playlist_history_contract
```

Expected: FAIL because `PlaylistSnapshot`, `playlist_snapshot()`, and `open_playlist_index()` do not exist yet.

- [ ] **Step 3: Add playlist snapshot and safe index selection**

Modify `crates/yoyo-core/src/playlist.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistSnapshot {
    pub entries: Vec<PlaylistEntry>,
    pub current_index: Option<usize>,
}

impl Playlist {
    pub fn replace(&mut self, entries: Vec<PlaylistEntry>, start_index: usize) {
        self.entries = entries;
        self.current_index = if self.entries.is_empty() {
            None
        } else {
            Some(start_index.min(self.entries.len() - 1))
        };
    }

    pub fn select(&mut self, index: usize) -> Option<&PlaylistEntry> {
        if index >= self.entries.len() {
            return None;
        }
        self.current_index = Some(index);
        self.entries.get(index)
    }

    pub fn snapshot(&self) -> PlaylistSnapshot {
        PlaylistSnapshot {
            entries: self.entries.clone(),
            current_index: self.current_index,
        }
    }
}
```

- [ ] **Step 4: Make session single-item opens replace the playlist and expose queue activation**

Modify `crates/yoyo-core/src/session.rs`:

```rust
use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent, MediaLocator,
    PlayerBackend, PlayerState, Playlist, PlaylistEntry, PlaylistSnapshot, Rotation,
};
```

Add these helpers inside `impl<B: PlayerBackend> AppSession<B>`:

```rust
pub fn playlist_snapshot(&self) -> PlaylistSnapshot {
    self.playlist.snapshot()
}

pub fn open_playlist_index(&mut self, index: usize) -> Result<(), AppError> {
    let Some(entry) = self.playlist.select(index).cloned() else {
        return Ok(());
    };
    self.backend.open(&entry.locator).map_err(AppError::Message)?;
    self.state.current = Some(entry.locator.clone());
    self.state.paused = false;
    Ok(())
}

fn open_single_locator(&mut self, locator: MediaLocator) -> Result<(), AppError> {
    let entry = PlaylistEntry::new(locator.clone());
    self.playlist.replace(vec![entry.clone()], 0);
    self.backend.open(&entry.locator).map_err(AppError::Message)?;
    self.state.current = Some(locator);
    self.state.paused = false;
    Ok(())
}
```

Change the `OpenFile` and `OpenUrl` match arms:

```rust
AppCommand::OpenFile(path) => {
    self.open_single_locator(MediaLocator::File(path))?;
}
AppCommand::OpenUrl(url) => {
    let locator = MediaLocator::from_url(&url)?;
    self.open_single_locator(locator)?;
}
```

Change the queue-navigation match arms to reuse the new helper:

```rust
AppCommand::NextItem => {
    if let Some(index) = self.playlist.current_index.and_then(|current| {
        let next = current.saturating_add(1);
        (next < self.playlist.entries.len()).then_some(next)
    }) {
        self.open_playlist_index(index)?;
    }
}
AppCommand::PreviousItem => {
    if let Some(index) = self
        .playlist
        .current_index
        .and_then(|current| current.checked_sub(1))
    {
        self.open_playlist_index(index)?;
    }
}
```

Change the EOF handling branch to reuse the safe index open:

```rust
BackendEvent::EndOfFile => {
    if let Some(index) = self.playlist.current_index.and_then(|current| {
        let next = current.saturating_add(1);
        (next < self.playlist.entries.len()).then_some(next)
    }) {
        self.open_playlist_index(index)?;
    }
}
```

- [ ] **Step 5: Export the snapshot type**

Modify `crates/yoyo-core/src/lib.rs`:

```rust
pub use playlist::{Playlist, PlaylistEntry, PlaylistSnapshot};
```

- [ ] **Step 6: Run the core playlist tests again**

Run:

```powershell
cargo test -p yoyo-core --test playlist_history_contract
```

Expected: PASS for the new playlist snapshot and queue-activation tests.

- [ ] **Step 7: Commit**

Run:

```powershell
git add crates/yoyo-core/src/playlist.rs crates/yoyo-core/src/session.rs crates/yoyo-core/src/lib.rs crates/yoyo-core/tests/playlist_history_contract.rs
git commit -m "feat: add playlist snapshot and queue activation"
```

Expected: Commit succeeds.

---

### Task 2: Core History Upsert Semantics

**Files:**
- Modify: `crates/yoyo-core/src/history.rs`
- Modify: `crates/yoyo-core/tests/playlist_history_contract.rs`

**Interfaces:**
- Produces: `HistoryStore::items(&self) -> &[HistoryEntry]`
- Produces: `HistoryStore::entry(&self, index: usize) -> Option<&HistoryEntry>`
- Produces: `HistoryStore::remember(&mut self, locator: MediaLocator, title: String, last_position_seconds: Option<f64>)`

- [ ] **Step 1: Add failing history semantics tests**

Append to `crates/yoyo-core/tests/playlist_history_contract.rs`:

```rust
use yoyo_core::HistoryStore;

#[test]
fn remember_moves_an_existing_locator_to_the_front() {
    let mut store = HistoryStore::default();

    store.remember(
        MediaLocator::Url("https://example.com/first.mp4".into()),
        "First".into(),
        Some(12.0),
    );
    store.remember(
        MediaLocator::Url("https://example.com/second.mp4".into()),
        "Second".into(),
        Some(48.0),
    );
    store.remember(
        MediaLocator::Url("https://example.com/first.mp4".into()),
        "First renamed".into(),
        Some(99.0),
    );

    assert_eq!(store.items().len(), 2);
    assert_eq!(store.items()[0].title, "First renamed");
    assert_eq!(store.items()[0].last_position_seconds, Some(99.0));
    assert_eq!(
        store.items()[0].locator,
        MediaLocator::Url("https://example.com/first.mp4".into())
    );
}

#[test]
fn history_entry_lookup_is_bounds_checked() {
    let mut store = HistoryStore::default();
    store.remember(
        MediaLocator::Url("https://example.com/video.mp4".into()),
        "Video".into(),
        Some(35.0),
    );

    assert!(store.entry(0).is_some());
    assert!(store.entry(5).is_none());
}
```

- [ ] **Step 2: Run the failing history tests**

Run:

```powershell
cargo test -p yoyo-core --test playlist_history_contract
```

Expected: FAIL because `items()`, `entry()`, and `remember()` are not implemented.

- [ ] **Step 3: Add history upsert helpers owned by `yoyo-core`**

Modify `crates/yoyo-core/src/history.rs`:

```rust
impl HistoryStore {
    pub fn items(&self) -> &[HistoryEntry] {
        &self.items
    }

    pub fn entry(&self, index: usize) -> Option<&HistoryEntry> {
        self.items.get(index)
    }

    pub fn remember(
        &mut self,
        locator: MediaLocator,
        title: String,
        last_position_seconds: Option<f64>,
    ) {
        let last_position_seconds =
            last_position_seconds.filter(|seconds| seconds.is_finite() && *seconds > 0.0);

        self.items.retain(|item| item.locator != locator);
        self.items.insert(
            0,
            HistoryEntry {
                locator,
                title,
                last_position_seconds,
            },
        );
    }
}
```

- [ ] **Step 4: Run the core history tests again**

Run:

```powershell
cargo test -p yoyo-core --test playlist_history_contract
```

Expected: PASS for both playlist and history core coverage.

- [ ] **Step 5: Commit**

Run:

```powershell
git add crates/yoyo-core/src/history.rs crates/yoyo-core/tests/playlist_history_contract.rs
git commit -m "feat: add core history remember helpers"
```

Expected: Commit succeeds.

---

### Task 3: Pure Desktop Sidebar State And Row Mapping

**Files:**
- Create: `apps/yoyovideo-desktop/src/sidebar.rs`
- Create: `apps/yoyovideo-desktop/tests/sidebar_contract.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`

**Interfaces:**
- Produces: `SidebarTab`
- Produces: `SidebarState { visible: bool, active_tab: SidebarTab }`
- Produces: `SidebarState::toggle(&mut self)`
- Produces: `SidebarState::show_tab(&mut self, tab: SidebarTab)`
- Produces: `SidebarState::tab_index(&self) -> i32`
- Produces: `initial_sidebar_state(show_playlist_on_startup: bool, window_width: f32) -> SidebarState`
- Produces: `expanded_sidebar_width(window_width: f32) -> f32`
- Produces: `PlaylistSidebarRow { title: String, is_current: bool }`
- Produces: `HistorySidebarRow { title: String, subtitle: String }`
- Produces: `build_playlist_rows(snapshot: &yoyo_core::PlaylistSnapshot) -> Vec<PlaylistSidebarRow>`
- Produces: `build_history_rows(store: &yoyo_core::HistoryStore) -> Vec<HistorySidebarRow>`

- [ ] **Step 1: Write the failing sidebar tests**

Create `apps/yoyovideo-desktop/tests/sidebar_contract.rs`:

```rust
use yoyo_core::{HistoryEntry, HistoryStore, MediaLocator, PlaylistEntry, PlaylistSnapshot};
use yoyovideo_desktop::{
    SidebarTab, build_history_rows, build_playlist_rows, expanded_sidebar_width,
    initial_sidebar_state,
};

#[test]
fn startup_visibility_uses_config_but_forces_narrow_windows_collapsed() {
    let wide = initial_sidebar_state(true, 1280.0);
    let narrow = initial_sidebar_state(true, 980.0);

    assert!(wide.visible);
    assert_eq!(wide.active_tab, SidebarTab::Playlist);
    assert!(!narrow.visible);
    assert_eq!(expanded_sidebar_width(980.0), 260.0);
    assert_eq!(expanded_sidebar_width(1280.0), 320.0);
}

#[test]
fn playlist_rows_highlight_only_the_valid_current_index() {
    let snapshot = PlaylistSnapshot {
        entries: vec![
            PlaylistEntry::new(MediaLocator::Url("https://example.com/a.mp4".into())),
            PlaylistEntry::new(MediaLocator::Url("https://example.com/b.mp4".into())),
        ],
        current_index: Some(1),
    };

    let rows = build_playlist_rows(&snapshot);

    assert_eq!(rows.len(), 2);
    assert!(!rows[0].is_current);
    assert!(rows[1].is_current);
}

#[test]
fn history_rows_format_resume_metadata() {
    let store = HistoryStore {
        items: vec![
            HistoryEntry {
                locator: MediaLocator::Url("https://example.com/movie.mp4".into()),
                title: "Movie".into(),
                last_position_seconds: Some(95.0),
            },
            HistoryEntry {
                locator: MediaLocator::Url("https://example.com/fresh.mp4".into()),
                title: "Fresh".into(),
                last_position_seconds: None,
            },
        ],
    };

    let rows = build_history_rows(&store);

    assert_eq!(rows[0].subtitle, "Resume 01:35");
    assert_eq!(rows[1].subtitle, "Resume start");
}
```

- [ ] **Step 2: Run the failing sidebar tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test sidebar_contract
```

Expected: FAIL because the sidebar module and exports do not exist.

- [ ] **Step 3: Add the pure sidebar module**

Create `apps/yoyovideo-desktop/src/sidebar.rs`:

```rust
use yoyo_core::{HistoryStore, PlaylistSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Playlist,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarState {
    pub visible: bool,
    pub active_tab: SidebarTab,
}

impl SidebarState {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show_tab(&mut self, tab: SidebarTab) {
        self.active_tab = tab;
        self.visible = true;
    }

    pub fn tab_index(&self) -> i32 {
        match self.active_tab {
            SidebarTab::Playlist => 0,
            SidebarTab::History => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistSidebarRow {
    pub title: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySidebarRow {
    pub title: String,
    pub subtitle: String,
}

pub fn initial_sidebar_state(
    show_playlist_on_startup: bool,
    window_width: f32,
) -> SidebarState {
    SidebarState {
        visible: show_playlist_on_startup && window_width >= 1050.0,
        active_tab: SidebarTab::Playlist,
    }
}

pub fn expanded_sidebar_width(window_width: f32) -> f32 {
    if window_width < 1050.0 { 260.0 } else { 320.0 }
}

pub fn build_playlist_rows(snapshot: &PlaylistSnapshot) -> Vec<PlaylistSidebarRow> {
    let valid_current =
        snapshot.current_index.filter(|index| *index < snapshot.entries.len());

    snapshot
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| PlaylistSidebarRow {
            title: entry.title.clone(),
            is_current: valid_current == Some(index),
        })
        .collect()
}

pub fn build_history_rows(store: &HistoryStore) -> Vec<HistorySidebarRow> {
    store
        .items()
        .iter()
        .map(|entry| HistorySidebarRow {
            title: entry.title.clone(),
            subtitle: format_history_resume(entry.last_position_seconds),
        })
        .collect()
}

fn format_history_resume(seconds: Option<f64>) -> String {
    match seconds {
        Some(seconds) => {
            let total = seconds.max(0.0) as u64;
            format!("Resume {:02}:{:02}", total / 60, total % 60)
        }
        None => "Resume start".into(),
    }
}
```

- [ ] **Step 4: Export sidebar helpers**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
mod sidebar;
```

Add these exports:

```rust
pub use sidebar::{
    HistorySidebarRow, PlaylistSidebarRow, SidebarState, SidebarTab, build_history_rows,
    build_playlist_rows, expanded_sidebar_width, initial_sidebar_state,
};
```

- [ ] **Step 5: Run the sidebar tests again**

Run:

```powershell
cargo test -p yoyovideo-desktop --test sidebar_contract
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/sidebar.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/sidebar_contract.rs
git commit -m "feat: add sidebar state and row mapping"
```

Expected: Commit succeeds.

---

### Task 4: Desktop History Runtime, Activation, And Resume Clamp

**Files:**
- Create: `apps/yoyovideo-desktop/src/history_runtime.rs`
- Create: `apps/yoyovideo-desktop/tests/history_runtime_contract.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`

**Interfaces:**
- Produces: `FlushReason`
- Produces: `PendingResumeSeek::new(target_seconds: f64) -> Option<Self>`
- Produces: `PendingResumeSeek::try_resolve(&self, duration_seconds: Option<f64>) -> Option<f64>`
- Produces: `HistoryActivation { command: yoyo_core::AppCommand, pending_seek: Option<PendingResumeSeek> }`
- Produces: `HistoryActivationError::MissingLocalFile(std::path::PathBuf)`
- Produces: `HistoryRuntime::new(path: Option<std::path::PathBuf>, store: yoyo_core::HistoryStore, enabled: bool) -> Self`
- Produces: `HistoryRuntime::load(path: Option<std::path::PathBuf>, enabled: bool) -> Result<Self, yoyo_core::StorageError>`
- Produces: `HistoryRuntime::store(&self) -> &yoyo_core::HistoryStore`
- Produces: `HistoryRuntime::remember_playback(&mut self, locator: &yoyo_core::MediaLocator, title: &str, position_seconds: Option<f64>)`
- Produces: `HistoryRuntime::activation_for(&self, index: usize) -> Result<Option<HistoryActivation>, HistoryActivationError>`
- Produces: `HistoryRuntime::flush_if_needed(&mut self, now: std::time::Duration, reason: FlushReason) -> Result<bool, yoyo_core::StorageError>`

- [ ] **Step 1: Write the failing history runtime tests**

Create `apps/yoyovideo-desktop/tests/history_runtime_contract.rs`:

```rust
use std::path::PathBuf;
use std::time::Duration;

use tempfile::tempdir;
use yoyo_core::{HistoryEntry, HistoryStore, MediaLocator};
use yoyovideo_desktop::{
    FlushReason, HistoryActivationError, HistoryRuntime, PendingResumeSeek,
};

#[test]
fn periodic_flush_is_throttled_to_two_seconds() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), HistoryStore::default(), true);

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(10.0),
    );
    assert!(runtime
        .flush_if_needed(Duration::from_secs(0), FlushReason::PeriodicTick)
        .unwrap());

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(12.0),
    );
    assert!(!runtime
        .flush_if_needed(Duration::from_secs(1), FlushReason::PeriodicTick)
        .unwrap());
    assert!(runtime
        .flush_if_needed(Duration::from_secs(2), FlushReason::PeriodicTick)
        .unwrap());
}

#[test]
fn pause_flush_bypasses_the_periodic_throttle_window() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), HistoryStore::default(), true);

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(10.0),
    );
    runtime
        .flush_if_needed(Duration::from_secs(0), FlushReason::PeriodicTick)
        .unwrap();

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(11.0),
    );
    assert!(runtime
        .flush_if_needed(Duration::from_secs(1), FlushReason::Pause)
        .unwrap());
}

#[test]
fn activation_rejects_missing_files_and_resume_seek_clamps_to_duration() {
    let missing = PathBuf::from("Z:/missing/movie.mp4");
    let runtime = HistoryRuntime::new(
        None,
        HistoryStore {
            items: vec![HistoryEntry {
                locator: MediaLocator::File(missing.clone()),
                title: "Missing".into(),
                last_position_seconds: Some(999.0),
            }],
        },
        true,
    );

    let error = runtime.activation_for(0).unwrap_err();
    assert_eq!(error, HistoryActivationError::MissingLocalFile(missing));

    let pending = PendingResumeSeek::new(999.0).unwrap();
    assert_eq!(pending.try_resolve(Some(120.0)), Some(120.0));
    assert_eq!(pending.try_resolve(None), None);
}
```

- [ ] **Step 2: Run the failing history runtime tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test history_runtime_contract
```

Expected: FAIL because the history runtime module and exports do not exist.

- [ ] **Step 3: Add the pure history runtime module**

Create `apps/yoyovideo-desktop/src/history_runtime.rs`:

```rust
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use yoyo_core::{
    AppCommand, HistoryStore, MediaLocator, StorageError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushReason {
    PeriodicTick,
    Pause,
    MediaSwitch,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingResumeSeek {
    target_seconds: f64,
}

impl PendingResumeSeek {
    pub fn new(target_seconds: f64) -> Option<Self> {
        (target_seconds.is_finite() && target_seconds > 0.0)
            .then_some(Self { target_seconds })
    }

    pub fn try_resolve(&self, duration_seconds: Option<f64>) -> Option<f64> {
        duration_seconds
            .filter(|duration| *duration > 0.0)
            .map(|duration| self.target_seconds.clamp(0.0, duration))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryActivation {
    pub command: AppCommand,
    pub pending_seek: Option<PendingResumeSeek>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryActivationError {
    MissingLocalFile(PathBuf),
}

pub struct HistoryRuntime {
    path: Option<PathBuf>,
    store: HistoryStore,
    enabled: bool,
    dirty: bool,
    last_flush_at: Option<Duration>,
}

impl HistoryRuntime {
    pub fn new(path: Option<PathBuf>, store: HistoryStore, enabled: bool) -> Self {
        Self {
            path,
            store,
            enabled,
            dirty: false,
            last_flush_at: None,
        }
    }

    pub fn load(path: Option<PathBuf>, enabled: bool) -> Result<Self, StorageError> {
        let store = if enabled {
            match path.as_ref() {
                Some(path) => HistoryStore::load(path)?,
                None => HistoryStore::default(),
            }
        } else {
            HistoryStore::default()
        };

        Ok(Self::new(path, store, enabled))
    }

    pub fn store(&self) -> &HistoryStore {
        &self.store
    }

    pub fn remember_playback(
        &mut self,
        locator: &MediaLocator,
        title: &str,
        position_seconds: Option<f64>,
    ) {
        if !self.enabled {
            return;
        }

        self.store
            .remember(locator.clone(), title.to_string(), position_seconds);
        self.dirty = true;
    }

    pub fn activation_for(
        &self,
        index: usize,
    ) -> Result<Option<HistoryActivation>, HistoryActivationError> {
        let Some(entry) = self.store.entry(index) else {
            return Ok(None);
        };

        let command = match &entry.locator {
            MediaLocator::File(path) => {
                if !path.exists() {
                    return Err(HistoryActivationError::MissingLocalFile(path.clone()));
                }
                AppCommand::OpenFile(path.clone())
            }
            MediaLocator::Url(url) => AppCommand::OpenUrl(url.clone()),
        };

        Ok(Some(HistoryActivation {
            command,
            pending_seek: entry.last_position_seconds.and_then(PendingResumeSeek::new),
        }))
    }

    pub fn flush_if_needed(
        &mut self,
        now: Duration,
        reason: FlushReason,
    ) -> Result<bool, StorageError> {
        if !self.enabled || !self.dirty {
            return Ok(false);
        }

        if matches!(reason, FlushReason::PeriodicTick) {
            if let Some(last) = self.last_flush_at {
                if now < last + Duration::from_secs(2) {
                    return Ok(false);
                }
            }
        }

        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            self.store.save(path)?;
        }

        self.dirty = false;
        self.last_flush_at = Some(now);
        Ok(true)
    }
}
```

- [ ] **Step 4: Export the history runtime helpers**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
mod history_runtime;
```

Add these exports:

```rust
pub use history_runtime::{
    FlushReason, HistoryActivation, HistoryActivationError, HistoryRuntime, PendingResumeSeek,
};
```

- [ ] **Step 5: Run the history runtime tests again**

Run:

```powershell
cargo test -p yoyovideo-desktop --test history_runtime_contract
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/history_runtime.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/history_runtime_contract.rs
git commit -m "feat: add desktop history runtime helpers"
```

Expected: Commit succeeds.

---

### Task 5: Slint Sidebar Surface

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`

**Interfaces:**
- Produces Slint structs: `PlaylistSidebarRowData`, `HistorySidebarRowData`
- Produces Slint properties: `sidebar_visible`, `sidebar_tab_index`, `sidebar_expanded_width_px`, `playlist_rows`, `history_rows`
- Produces Slint callbacks: `toggle_sidebar_requested()`, `show_playlist_tab_requested()`, `show_history_tab_requested()`, `playlist_item_requested(int)`, `history_item_requested(int)`

- [ ] **Step 1: Run the current desktop UI build check**

Run:

```powershell
cargo check -p yoyovideo-desktop
```

Expected: PASS before editing `main-window.slint`.

- [ ] **Step 2: Add sidebar row structs, properties, callbacks, and the right-hand layout**

Modify the import line in `apps/yoyovideo-desktop/ui/main-window.slint`:

```slint
import { Button, HorizontalBox, VerticalBox, LineEdit, Slider, ScrollView } from "std-widgets.slint";
```

Add these Slint structs and properties near the top of the file:

```slint
export struct PlaylistSidebarRowData {
    title: string,
    is_current: bool,
}

export struct HistorySidebarRowData {
    title: string,
    subtitle: string,
}
```

Inside `MainWindow`, add:

```slint
    in-out property <bool> sidebar_visible: true;
    in-out property <int> sidebar_tab_index: 0;
    in-out property <float> sidebar_expanded_width_px: 320.0;
    in-out property <[PlaylistSidebarRowData]> playlist_rows: [];
    in-out property <[HistorySidebarRowData]> history_rows: [];

    callback toggle_sidebar_requested();
    callback show_playlist_tab_requested();
    callback show_history_tab_requested();
    callback playlist_item_requested(int);
    callback history_item_requested(int);
```

Add a dedicated sidebar toggle button to the first control row:

```slint
                Button {
                    text: root.sidebar_visible ? "List <<" : "List >>";
                    clicked => { root.toggle_sidebar_requested(); }
                }
```

Replace the top-level `VerticalBox` body with a `HorizontalBox` that keeps the current player on the left and adds the sidebar on the right:

```slint
    HorizontalBox {
        spacing: 8px;
        padding: 10px;

        VerticalBox {
            spacing: 8px;

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
                    Button { text: root.sidebar_visible ? "List <<" : "List >>"; clicked => { root.toggle_sidebar_requested(); } }
                    Button { text: "Menu"; clicked => { menu_popup.show(); } }
                    url_input := LineEdit {
                        placeholder-text: "Open URL";
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
                    Button { text: "A"; clicked => { root.set_ab_point_a_requested(); } }
                    Button { text: "B"; clicked => { root.set_ab_point_b_requested(); } }
                    Button { text: "Clear"; clicked => { root.clear_ab_loop_requested(); } }
                    Text { text: loop_label; }
                    Button { text: "Fullscreen"; clicked => { root.toggle_fullscreen_requested(); } }
                    Text { text: status_label; color: #7d8790; }
                }
            }
        }

        Rectangle {
            width: root.sidebar_visible ? root.sidebar_expanded_width_px * 1px : 36px;
            min-width: self.width;
            background: #101418;
            border-color: #26313a;
            border-width: 1px;
            border-radius: 8px;

            if root.sidebar_visible : VerticalBox {
                padding: 10px;
                spacing: 8px;

                HorizontalBox {
                    spacing: 6px;
                    Button { text: "Playlist"; clicked => { root.show_playlist_tab_requested(); } }
                    Button { text: "History"; clicked => { root.show_history_tab_requested(); } }
                    Button { text: "Close"; clicked => { root.toggle_sidebar_requested(); } }
                }

                ScrollView {
                    if root.sidebar_tab_index == 0 : VerticalBox {
                        spacing: 4px;
                        for row[idx] in root.playlist_rows : Rectangle {
                            min-height: 44px;
                            border-radius: 6px;
                            background: row.is_current ? #1d2d38 : #141b20;

                            TouchArea {
                                clicked => { root.playlist_item_requested(idx); }
                            }

                            Text {
                                text: row.title;
                                color: row.is_current ? #f2f5f7 : #c7d1d8;
                                vertical-alignment: center;
                                x: 10px;
                                y: 12px;
                            }
                        }
                    }

                    if root.sidebar_tab_index == 1 : VerticalBox {
                        spacing: 4px;
                        for row[idx] in root.history_rows : Rectangle {
                            min-height: 56px;
                            border-radius: 6px;
                            background: #141b20;

                            TouchArea {
                                clicked => { root.history_item_requested(idx); }
                            }

                            VerticalBox {
                                padding-left: 10px;
                                padding-top: 8px;
                                spacing: 2px;
                                Text { text: row.title; color: #f2f5f7; }
                                Text { text: row.subtitle; color: #7d8790; }
                            }
                        }
                    }
                }
            }

            if !root.sidebar_visible : Rectangle {
                background: transparent;
                TouchArea {
                    clicked => { root.toggle_sidebar_requested(); }
                }
                Text {
                    text: ">>";
                    color: #c7d1d8;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
            }
        }
    }
```

- [ ] **Step 3: Run the desktop build check again**

Run:

```powershell
cargo check -p yoyovideo-desktop
```

Expected: PASS with the new Slint properties and callbacks available to Rust.

- [ ] **Step 4: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint
git commit -m "feat: add playlist history sidebar surface"
```

Expected: Commit succeeds.

---

### Task 6: Desktop Runtime Integration, Startup Load, And Row Activation

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`

**Interfaces:**
- Produces: `DesktopController::open_playlist_index(&mut self, index: usize) -> Result<(), yoyo_core::AppError>`
- Produces: `refresh_sidebar(window: &MainWindow, runtime: &DesktopRuntime)`
- Produces: `history_now(runtime: &DesktopRuntime) -> std::time::Duration`
- Produces: `PlaybackHistorySnapshot { current: Option<yoyo_core::MediaLocator>, title: Option<String>, position_seconds: f64, paused: bool }`
- Produces: `capture_history_snapshot(session: &yoyo_core::AppSession<yoyo_mpv::MpvBackend>) -> PlaybackHistorySnapshot`
- Produces: `sync_history_from_snapshot(runtime: &mut DesktopRuntime, snapshot: &PlaybackHistorySnapshot) -> Result<(), yoyo_core::StorageError>`
- Produces: `apply_pending_resume(controller: &mut DesktopController<yoyo_mpv::MpvBackend>, pending: Option<crate::PendingResumeSeek>) -> Result<Option<crate::PendingResumeSeek>, yoyo_core::AppError>`

- [ ] **Step 1: Add the failing controller queue-selection regression test**

Append to `apps/yoyovideo-desktop/tests/controller_contract.rs`:

```rust
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
    assert_eq!(
        controller.session().state().current,
        Some(MediaLocator::File("b.mp4".into()))
    );
}
```

- [ ] **Step 2: Run the failing controller test**

Run:

```powershell
cargo test -p yoyovideo-desktop --test controller_contract
```

Expected: FAIL because `DesktopController::open_playlist_index()` does not exist.

- [ ] **Step 3: Add controller queue-selection and export the new helper modules**

Modify `apps/yoyovideo-desktop/src/app.rs` inside `impl<B: PlayerBackend> DesktopController<B>`:

```rust
pub fn open_playlist_index(&mut self, index: usize) -> Result<(), yoyo_core::AppError> {
    self.session.open_playlist_index(index)?;
    self.session.poll_backend()
}
```

Modify `apps/yoyovideo-desktop/src/lib.rs` to keep the new modules reachable from tests:

```rust
pub use app::{
    DesktopController, build_desktop_backend, build_desktop_backend_with_video_window,
    dispatch_shortcut, refresh_window, run,
};
```

This export block stays, but after this task it must coexist with the new sidebar and history-runtime exports added in Tasks 3 and 4.

- [ ] **Step 4: Load config/history once during startup and extend runtime state**

Modify `apps/yoyovideo-desktop/src/app.rs` imports:

```rust
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use yoyo_core::{AppCommand, AppConfig, AppSession, MediaLocator, PlayerBackend, PlayerState, ShortcutAction, ShortcutMap};
```

Change `DesktopRuntime` so it owns startup-loaded config, history runtime, sidebar state, pending resume state, and a startup clock:

```rust
struct DesktopRuntime {
    controller: Option<DesktopController<MpvBackend>>,
    video_host_error: Option<String>,
    app_handle: Option<slint::Weak<MainWindow>>,
    config: AppConfig,
    history: crate::HistoryRuntime,
    sidebar: crate::SidebarState,
    pending_resume: Option<crate::PendingResumeSeek>,
    last_seen_locator: Option<MediaLocator>,
    started_at: Instant,
    #[cfg(feature = "mpv-runtime")]
    video_host: Option<WinitVideoHost>,
}
```

Add storage helpers above `run()`:

```rust
fn config_file_path(paths: &crate::platform::AppPaths) -> PathBuf {
    paths.config_dir.join("config.toml")
}

fn history_file_path(paths: &crate::platform::AppPaths) -> PathBuf {
    paths.data_dir.join("history.json")
}

fn load_boot_config(paths: Option<&crate::platform::AppPaths>) -> AppConfig {
    paths
        .map(config_file_path)
        .and_then(|path| AppConfig::load(&path).ok())
        .unwrap_or_default()
}

fn load_history_runtime(
    paths: Option<&crate::platform::AppPaths>,
    config: &AppConfig,
) -> crate::HistoryRuntime {
    let history_path = paths.map(history_file_path);
    crate::HistoryRuntime::load(history_path, config.ui.remember_history)
        .unwrap_or_else(|_| crate::HistoryRuntime::new(None, yoyo_core::HistoryStore::default(), false))
}
```

Change `DesktopRuntime::new()` to:

```rust
fn new(config: AppConfig, history: crate::HistoryRuntime, sidebar: crate::SidebarState) -> Self {
    Self {
        controller: None,
        video_host_error: initial_runtime_error(),
        app_handle: None,
        config,
        history,
        sidebar,
        pending_resume: None,
        last_seen_locator: None,
        started_at: Instant::now(),
        #[cfg(feature = "mpv-runtime")]
        video_host: None,
    }
}
```

- [ ] **Step 5: Add sidebar refresh, history sync, and pending-resume helpers**

Still in `apps/yoyovideo-desktop/src/app.rs`, add:

```rust
fn history_now(runtime: &DesktopRuntime) -> Duration {
    runtime.started_at.elapsed()
}

#[derive(Debug, Clone)]
struct PlaybackHistorySnapshot {
    current: Option<MediaLocator>,
    title: Option<String>,
    position_seconds: f64,
    paused: bool,
}

fn current_playlist_title(session: &AppSession<MpvBackend>) -> Option<String> {
    let snapshot = session.playlist_snapshot();
    let index = snapshot.current_index?;
    snapshot.entries.get(index).map(|entry| entry.title.clone())
}

fn capture_history_snapshot(session: &AppSession<MpvBackend>) -> PlaybackHistorySnapshot {
    PlaybackHistorySnapshot {
        current: session.state().current.clone(),
        title: current_playlist_title(session),
        position_seconds: session.state().position_seconds,
        paused: session.state().paused,
    }
}

fn refresh_sidebar(window: &MainWindow, runtime: &DesktopRuntime) {
    window.set_sidebar_visible(runtime.sidebar.visible);
    window.set_sidebar_tab_index(runtime.sidebar.tab_index());
    window.set_sidebar_expanded_width_px(crate::expanded_sidebar_width(window.get_width() as f32));

    let playlist_rows = runtime
        .controller()
        .map(|controller| crate::build_playlist_rows(&controller.session().playlist_snapshot()))
        .unwrap_or_default()
        .into_iter()
        .map(|row| PlaylistSidebarRowData {
            title: row.title.into(),
            is_current: row.is_current,
        })
        .collect();

    let history_rows = crate::build_history_rows(runtime.history.store())
        .into_iter()
        .map(|row| HistorySidebarRowData {
            title: row.title.into(),
            subtitle: row.subtitle.into(),
        })
        .collect();

    window.set_playlist_rows(playlist_rows);
    window.set_history_rows(history_rows);
}

fn sync_history_from_snapshot(
    runtime: &mut DesktopRuntime,
    snapshot: &PlaybackHistorySnapshot,
) -> Result<(), yoyo_core::StorageError> {
    let now = history_now(runtime);
    let current = snapshot.current.clone();

    if current != runtime.last_seen_locator {
        runtime
            .history
            .flush_if_needed(now, crate::FlushReason::MediaSwitch)?;
        runtime.last_seen_locator = current.clone();
    }

    if let (Some(locator), Some(title)) = (current.as_ref(), snapshot.title.as_ref()) {
        runtime.history.remember_playback(
            locator,
            title,
            Some(snapshot.position_seconds),
        );
    }

    if snapshot.paused {
        runtime.history.flush_if_needed(now, crate::FlushReason::Pause)?;
    } else {
        runtime
            .history
            .flush_if_needed(now, crate::FlushReason::PeriodicTick)?;
    }

    Ok(())
}

fn apply_pending_resume(
    controller: &mut DesktopController<MpvBackend>,
    pending: Option<crate::PendingResumeSeek>,
) -> Result<Option<crate::PendingResumeSeek>, yoyo_core::AppError> {
    let Some(seek) = pending else {
        return Ok(None);
    };
    let Some(position) = seek.try_resolve(controller.session().state().duration_seconds) else {
        return Ok(Some(seek));
    };

    controller.dispatch(AppCommand::SeekAbsolute(position))?;
    Ok(None)
}
```

- [ ] **Step 6: Wire sidebar callbacks, startup state, history row activation, and shutdown flush**

Modify `run()` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
let paths = crate::platform::AppPaths::discover();
let config = load_boot_config(paths.as_ref());
let history = load_history_runtime(paths.as_ref(), &config);
let sidebar = crate::initial_sidebar_state(config.ui.show_playlist_on_startup, 1200.0);
let runtime = Rc::new(RefCell::new(DesktopRuntime::new(config, history, sidebar)));
configure_backend(Rc::clone(&runtime))?;

let app = MainWindow::new()?;
runtime.borrow_mut().app_handle = Some(app.as_weak());
{
    let mut runtime = runtime.borrow_mut();
    runtime.sidebar = crate::initial_sidebar_state(
        runtime.config.ui.show_playlist_on_startup,
        app.get_width() as f32,
    );
}
```

Inside `DesktopWinitHandler::initialize_runtime`, replace `AppConfig::default()` with the loaded config:

```rust
let session = AppSession::new(runtime.config.clone(), backend);
```

Change `with_runtime_controller()` so it returns `bool`. After each successful controller action inside that helper, copy the post-action state out of the controller, let the controller-field borrow end, then sync history and refresh the sidebar:

```rust
let pending = runtime.pending_resume.take();
let outcome = {
    let Some(controller) = runtime.controller_mut() else {
        if let Some(app) = app_handle.upgrade() {
            app.set_status_label(runtime.status_message().into());
        }
        return false;
    };

    match action(controller) {
        Ok(()) => {
            let pending_resume = match apply_pending_resume(controller, pending) {
                Ok(pending_resume) => pending_resume,
                Err(error) => {
                    if let Some(app) = app_handle.upgrade() {
                        app.set_status_label(error.to_string().into());
                    }
                    return false;
                }
            };
            let state = controller.session().state().clone();
            let history_snapshot = capture_history_snapshot(controller.session());
            Ok((state, history_snapshot, pending_resume))
        }
        Err(error) => Err(error),
    }
};

match outcome {
    Ok((state, history_snapshot, pending_resume)) => {
        runtime.pending_resume = pending_resume;
        if let Err(error) = sync_history_from_snapshot(&mut runtime, &history_snapshot) {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(error.to_string().into());
            }
            return false;
        }
        if let Some(app) = app_handle.upgrade() {
            refresh_window(&app, &state);
            refresh_sidebar(&app, &runtime);
            apply_fullscreen_state(&app, &state);
        }
        true
    }
    Err(error) => {
        if let Some(app) = app_handle.upgrade() {
            app.set_status_label(error.to_string().into());
        }
        false
    }
}
```

Add the new sidebar callbacks in `run()`:

```rust
app.on_toggle_sidebar_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move || {
        let mut runtime = runtime.borrow_mut();
        runtime.sidebar.toggle();
        if let Some(app) = app_handle.upgrade() {
            refresh_sidebar(&app, &runtime);
        }
    }
});

app.on_show_playlist_tab_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move || {
        let mut runtime = runtime.borrow_mut();
        runtime.sidebar.show_tab(crate::SidebarTab::Playlist);
        if let Some(app) = app_handle.upgrade() {
            refresh_sidebar(&app, &runtime);
        }
    }
});

app.on_show_history_tab_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move || {
        let mut runtime = runtime.borrow_mut();
        runtime.sidebar.show_tab(crate::SidebarTab::History);
        if let Some(app) = app_handle.upgrade() {
            refresh_sidebar(&app, &runtime);
        }
    }
});

app.on_playlist_item_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |index| {
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.open_playlist_index(index as usize)
        });
    }
});

app.on_history_item_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |index| {
        let activation = {
            let runtime = runtime.borrow();
            runtime.history.activation_for(index as usize)
        };

        match activation {
            Ok(Some(activation)) => {
                let pending_seek = activation.pending_seek;
                let dispatched = with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.dispatch(activation.command.clone())
                });
                if dispatched {
                    runtime.borrow_mut().pending_resume = pending_seek;
                }
            }
            Ok(None) => {}
            Err(crate::HistoryActivationError::MissingLocalFile(path)) => {
                if let Some(app) = app_handle.upgrade() {
                    app.set_status_label(
                        format!("History file is missing: {}", path.display()).into(),
                    );
                }
            }
        }
    }
});
```

Inside the poll timer, after `controller.poll_backend()`, keep history and pending resume in sync while respecting field-borrow order:

```rust
let pending = runtime.pending_resume.take();
if let Some(controller) = runtime.controller_mut() {
    match controller.poll_backend() {
        Ok(()) => {
            let next_pending = match apply_pending_resume(controller, pending) {
                Ok(next_pending) => next_pending,
                Err(error) => {
                    app.set_status_label(error.to_string().into());
                    None
                }
            };
            let state = controller.session().state().clone();
            let history_snapshot = capture_history_snapshot(controller.session());

            runtime.pending_resume = next_pending;
            if let Err(error) = sync_history_from_snapshot(&mut runtime, &history_snapshot) {
                app.set_status_label(error.to_string().into());
            }
            refresh_window(&app, &state);
            refresh_sidebar(&app, &runtime);
            #[cfg(feature = "mpv-runtime")]
            sync_runtime_video_host(&app, &mut runtime);
        }
        Err(error) => app.set_status_label(error.to_string().into()),
    }
} else {
    refresh_runtime_window(&app, &runtime);
    refresh_sidebar(&app, &runtime);
}
```

After `app.run()?`, flush history on shutdown:

```rust
{
    let mut runtime = runtime.borrow_mut();
    if let Some(snapshot) = runtime
        .controller()
        .map(|controller| capture_history_snapshot(controller.session()))
    {
        let _ = sync_history_from_snapshot(&mut runtime, &snapshot);
    }
    let _ = runtime
        .history
        .flush_if_needed(history_now(&runtime), crate::FlushReason::Shutdown);
}
```

- [ ] **Step 7: Run the desktop tests and build checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test sidebar_contract
cargo test -p yoyovideo-desktop --test history_runtime_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS. If `mpv-runtime` is unavailable locally, stop and resolve the compile issue before moving on.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/controller_contract.rs
git commit -m "feat: wire playlist history sidebar runtime"
```

Expected: Commit succeeds.

---

### Task 7: Manual Smoke Checklist And End-To-End Verification

**Files:**
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Produces: updated sidebar/history smoke coverage in the shared manual checklist

- [ ] **Step 1: Add the new manual smoke checklist coverage**

Append to `docs/testing/manual-smoke-checklist.md` under `## UX`:

```markdown
- Confirm the right sidebar honors the startup preference on wide windows.
- Launch the app with a window narrower than `1050px` and confirm the sidebar starts collapsed.
- Toggle the sidebar from the control surface and confirm the collapsed strip can reopen it.
- Open a folder and confirm the `Playlist` tab shows the scanned queue with the active item highlighted.
- Click a playlist item in the sidebar and confirm playback switches to that item.
- Restart the app, open the `History` tab, and confirm recent items show resume metadata.
- Click a history item and confirm playback resumes near the stored position.
- Click a history item pointing to a removed file and confirm the app shows a clear error without crashing.
```

- [ ] **Step 2: Run a quick documentation coverage check**

Run:

```powershell
$content = Get-Content -Raw docs/testing/manual-smoke-checklist.md
$required = @(
  "sidebar starts collapsed",
  "Playlist",
  "History",
  "resume metadata",
  "removed file"
)
$missing = $required | Where-Object { $content -notmatch [regex]::Escape($_) }
if ($missing.Count -gt 0) {
  Write-Error ("Missing checklist coverage: " + ($missing -join ", "))
  exit 1
}
```

Expected: PASS.

- [ ] **Step 3: Run final formatting and test verification**

Run:

```powershell
cargo fmt --check
cargo test -p yoyo-core --test playlist_history_contract
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test sidebar_contract
cargo test -p yoyovideo-desktop --test history_runtime_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
git status --short
```

Expected:
- `cargo fmt --check`: PASS
- all targeted tests: PASS
- both `cargo check` commands: PASS
- `git status --short`: only the planned source/doc changes remain before commit

- [ ] **Step 4: Commit**

Run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add playlist history sidebar smoke checks"
```

Expected: Commit succeeds.

---

## Self-Review

**Spec coverage:** The plan covers the right-side collapsible sidebar, startup visibility rules, `320px` / `260px` width policy, `36px` collapsed strip, playlist row highlight, history row resume metadata, playlist row activation, history row reopen + pending resume seek, missing-file error handling, history load-once startup behavior, throttled writes, immediate pause/media-switch/shutdown flushes, and manual smoke updates. It intentionally leaves subtitles, settings UI, playlist editing, history search/filter/bulk clear, and full session restore out of scope.

**Placeholder scan:** The plan does not contain `TBD`, `TODO`, “implement later”, vague “handle edge cases”, or “similar to Task N” placeholders. Each task names exact files, concrete test commands, and concrete code snippets.

**Type consistency:** The plan uses one consistent set of names across tasks: `PlaylistSnapshot`, `SidebarState`, `SidebarTab`, `PlaylistSidebarRow`, `HistorySidebarRow`, `FlushReason`, `HistoryRuntime`, `HistoryActivation`, `HistoryActivationError`, and `PendingResumeSeek`. The Slint bridge types are `PlaylistSidebarRowData` and `HistorySidebarRowData`, and the app integration task is the only place that converts Rust-side rows into Slint-side rows.
