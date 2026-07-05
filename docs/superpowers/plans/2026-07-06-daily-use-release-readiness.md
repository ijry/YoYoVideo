# Daily Use Release Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make YoYoVideo reliable enough for daily local use and safer to package for testers by adding drag-and-drop media opening, a fuller context menu, actionable diagnostics, local logging, and stronger package smoke checks.

**Architecture:** Keep playback/session logic in `yoyo-core`, mpv translation/runtime work in `yoyo-mpv`, desktop-only UX and diagnostics in `apps/yoyovideo-desktop`, and release/package verification in `scripts/`. Drag-and-drop is implemented as a desktop path classifier plus controller dispatch helpers; UI event handlers only collect OS events and call those helpers.

**Tech Stack:** Rust 2024, Slint 1.17.0, winit 0.30 through Slint, libmpv via `yoyo-mpv`, `directories` 6.0, `chrono` 0.4.45, PowerShell package scripts, `tempfile` test fixtures.

## Global Constraints

- No automatic updater.
- No code signing or notarization workflow.
- No remote crash reporting service.
- No online subtitle matching.
- No LUT, HDR, shader, or color-management work.
- No large UI redesign beyond menu and status-message improvements.
- Dragging only unsupported files must not crash or clear the current playlist.
- Folder scans that produce no supported media must leave current playback untouched and show a status message.
- Screenshot path failures must remain non-fatal and must be logged after logging exists.
- Missing runtime files must fail early with a focused diagnostic instead of a generic backend initialization failure.
- Logging failures must never block playback.
- Default Rust tests must not require libmpv runtime files.

---

## File Structure

- Create `apps/yoyovideo-desktop/src/platform/drop.rs`: classify dropped paths into a typed desktop action using existing media support and folder scanning rules.
- Modify `apps/yoyovideo-desktop/src/platform/mod.rs`: export drop classification helpers.
- Modify `apps/yoyovideo-desktop/src/app.rs`: add controller playlist helper, drag-drop debounce dispatch, menu wiring, diagnostics integration, and actionable runtime errors.
- Modify `apps/yoyovideo-desktop/src/lib.rs`: export new helpers needed by contract tests.
- Modify `apps/yoyovideo-desktop/ui/main-window.slint`: add context-menu entries for playlist/history.
- Create `apps/yoyovideo-desktop/tests/drop_contract.rs`: cover path classification and dispatch behavior.
- Modify `apps/yoyovideo-desktop/tests/controller_contract.rs`: cover opening multiple playlist entries through the controller.
- Create `apps/yoyovideo-desktop/tests/context_menu_contract.rs`: compile-level checks for menu callback surface.
- Create `apps/yoyovideo-desktop/src/platform/logging.rs`: resolve log file paths and append timestamped diagnostic lines.
- Modify `apps/yoyovideo-desktop/src/platform/mod.rs`: export logging helpers.
- Create `apps/yoyovideo-desktop/tests/logging_contract.rs`: cover log path creation and append behavior.
- Create `apps/yoyovideo-desktop/tests/runtime_diagnostics_contract.rs`: cover missing-runtime message formatting.
- Modify `apps/yoyovideo-desktop/src/main.rs`: write fatal startup errors to the diagnostic log before exiting.
- Create `scripts/smoke-package.ps1`: verify package layout, optionally launch the packaged binary briefly, and run runtime playback smoke with the package runtime files on `PATH`.
- Create `scripts/test-package-smoke.ps1`: fixture tests for package smoke failure messages and log creation.
- Modify `scripts/smoke-runtime.ps1`: allow package smoke to override runtime bin/lib directories.
- Modify `docs/testing/manual-smoke-checklist.md`: add drag-drop, context menu, diagnostics, and package smoke checks.
- Modify `README.md`: document package smoke command.

---

### Task 1: Dropped Media Classification And Playlist Dispatch

**Files:**
- Create: `apps/yoyovideo-desktop/src/platform/drop.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Create: `apps/yoyovideo-desktop/tests/drop_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`

**Interfaces:**
- Consumes: `yoyo_core::MediaLocator::is_supported_local_path(&Path) -> bool`
- Consumes: `scan_media_folder(path: &Path) -> Result<Vec<PlaylistEntry>, AppError>`
- Produces: `pub enum DroppedMediaAction { NoPlayableMedia { ignored_count: usize }, OpenFile(PathBuf), ReplacePlaylist(Vec<PlaylistEntry>) }`
- Produces: `pub fn classify_dropped_paths(paths: &[PathBuf]) -> Result<DroppedMediaAction, AppError>`
- Produces: `DesktopController::open_playlist_entries(&mut self, entries: Vec<PlaylistEntry>) -> Result<(), yoyo_core::AppError>`

- [ ] **Step 1: Write failing drop classification tests**

Create `apps/yoyovideo-desktop/tests/drop_contract.rs`:

```rust
use std::fs;

use tempfile::tempdir;
use yoyo_core::{MediaLocator, PlaylistEntry};
use yoyovideo_desktop::platform::{DroppedMediaAction, classify_dropped_paths};

#[test]
fn single_supported_file_drop_opens_that_file() {
    let dir = tempdir().unwrap();
    let movie = dir.path().join("movie.mp4");
    fs::write(&movie, "media").unwrap();

    let action = classify_dropped_paths(&[movie.clone()]).unwrap();

    assert_eq!(action, DroppedMediaAction::OpenFile(movie));
}

#[test]
fn multiple_supported_files_drop_replaces_playlist_in_drop_order() {
    let dir = tempdir().unwrap();
    let first = dir.path().join("b.mp4");
    let second = dir.path().join("a.mkv");
    fs::write(&first, "media").unwrap();
    fs::write(&second, "media").unwrap();

    let action = classify_dropped_paths(&[first.clone(), second.clone()]).unwrap();

    assert_eq!(
        action,
        DroppedMediaAction::ReplacePlaylist(vec![
            PlaylistEntry::new(MediaLocator::File(first)),
            PlaylistEntry::new(MediaLocator::File(second)),
        ])
    );
}

#[test]
fn folder_drop_replaces_playlist_with_sorted_supported_media() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.webm"), "media").unwrap();
    fs::write(dir.path().join("cover.jpg"), "image").unwrap();
    fs::write(dir.path().join("a.mp4"), "media").unwrap();

    let action = classify_dropped_paths(&[dir.path().to_path_buf()]).unwrap();

    let DroppedMediaAction::ReplacePlaylist(entries) = action else {
        panic!("expected playlist replacement");
    };
    let titles: Vec<_> = entries.into_iter().map(|entry| entry.title).collect();
    assert_eq!(titles, vec!["a.mp4".to_string(), "z.webm".to_string()]);
}

#[test]
fn mixed_drop_ignores_unsupported_paths_when_supported_media_exists() {
    let dir = tempdir().unwrap();
    let unsupported = dir.path().join("notes.txt");
    let movie = dir.path().join("movie.mp4");
    fs::write(&unsupported, "notes").unwrap();
    fs::write(&movie, "media").unwrap();

    let action = classify_dropped_paths(&[unsupported, movie.clone()]).unwrap();

    assert_eq!(action, DroppedMediaAction::OpenFile(movie));
}

#[test]
fn unsupported_only_drop_reports_no_playable_media() {
    let dir = tempdir().unwrap();
    let unsupported = dir.path().join("cover.jpg");
    fs::write(&unsupported, "image").unwrap();

    let action = classify_dropped_paths(&[unsupported]).unwrap();

    assert_eq!(action, DroppedMediaAction::NoPlayableMedia { ignored_count: 1 });
}
```

- [ ] **Step 2: Write failing controller playlist dispatch test**

Append to `apps/yoyovideo-desktop/tests/controller_contract.rs`:

```rust
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
```

- [ ] **Step 3: Run failing tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test drop_contract
cargo test -p yoyovideo-desktop --test controller_contract controller_can_open_multiple_playlist_entries_from_drop
```

Expected:

- `drop_contract` fails because `DroppedMediaAction` and `classify_dropped_paths` do not exist.
- controller test fails because `DesktopController::open_playlist_entries` does not exist.

- [ ] **Step 4: Add dropped media classification module**

Create `apps/yoyovideo-desktop/src/platform/drop.rs`:

```rust
use std::path::PathBuf;

use yoyo_core::{AppError, MediaLocator, PlaylistEntry};

use super::scan_media_folder;

#[derive(Debug, Clone, PartialEq)]
pub enum DroppedMediaAction {
    NoPlayableMedia { ignored_count: usize },
    OpenFile(PathBuf),
    ReplacePlaylist(Vec<PlaylistEntry>),
}

pub fn classify_dropped_paths(paths: &[PathBuf]) -> Result<DroppedMediaAction, AppError> {
    let mut entries = Vec::new();
    let mut ignored_count = 0usize;
    let mut saw_folder = false;

    for path in paths {
        if path.is_dir() {
            saw_folder = true;
            let scanned = scan_media_folder(path)?;
            if scanned.is_empty() {
                ignored_count += 1;
            } else {
                entries.extend(scanned);
            }
        } else if path.is_file() && MediaLocator::is_supported_local_path(path) {
            entries.push(PlaylistEntry::new(MediaLocator::File(path.clone())));
        } else {
            ignored_count += 1;
        }
    }

    match entries.len() {
        0 => Ok(DroppedMediaAction::NoPlayableMedia { ignored_count }),
        1 if !saw_folder => match &entries[0].locator {
            MediaLocator::File(path) => Ok(DroppedMediaAction::OpenFile(path.clone())),
            MediaLocator::Url(_) => Ok(DroppedMediaAction::ReplacePlaylist(entries)),
        },
        _ => Ok(DroppedMediaAction::ReplacePlaylist(entries)),
    }
}
```

- [ ] **Step 5: Export drop helpers**

Modify `apps/yoyovideo-desktop/src/platform/mod.rs`:

```rust
mod dialogs;
mod drop;
mod media_scan;
mod paths;
mod screenshot;

pub use dialogs::{DialogService, RfdDialogService};
pub use drop::{DroppedMediaAction, classify_dropped_paths};
pub use media_scan::scan_media_folder;
pub use paths::AppPaths;
pub use screenshot::{
    default_screenshot_dir, next_screenshot_path, prepare_screenshot_path,
    prepare_screenshot_path_in_dir, screenshot_timestamp_now,
};
```

No `apps/yoyovideo-desktop/src/lib.rs` change is required in this task because `pub mod platform;` already exposes `platform::DroppedMediaAction` and `platform::classify_dropped_paths`.

- [ ] **Step 6: Add controller playlist helper**

Modify `apps/yoyovideo-desktop/src/app.rs` inside `impl<B: PlayerBackend> DesktopController<B>` after `open_folder`:

```rust
    pub fn open_playlist_entries(
        &mut self,
        entries: Vec<yoyo_core::PlaylistEntry>,
    ) -> Result<(), yoyo_core::AppError> {
        self.session.replace_playlist(entries, 0)?;
        self.session.poll_backend()
    }
```

- [ ] **Step 7: Run task tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test drop_contract
cargo test -p yoyovideo-desktop --test controller_contract controller_can_open_multiple_playlist_entries_from_drop
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/platform/drop.rs apps/yoyovideo-desktop/src/platform/mod.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/tests/drop_contract.rs apps/yoyovideo-desktop/tests/controller_contract.rs
git commit -m "feat: classify dropped media paths"
```

Expected: Commit succeeds.

---

### Task 2: Drag-And-Drop Wiring And Context Menu Completion

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Create: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/drop_contract.rs`

**Interfaces:**
- Consumes: `classify_dropped_paths(paths: &[PathBuf]) -> Result<DroppedMediaAction, AppError>`
- Consumes: `DesktopController::open_playlist_entries(Vec<PlaylistEntry>)`
- Produces: `pub fn dropped_media_status(action: &DroppedMediaAction) -> String`
- Produces: `fn dispatch_dropped_paths(app_handle: &slint::Weak<MainWindow>, runtime: &Rc<RefCell<DesktopRuntime>>, paths: Vec<PathBuf>)`
- Produces Slint callback/property surface for context menu playlist/history visibility actions through existing `show_playlist_tab_requested()` and `show_history_tab_requested()`

- [ ] **Step 1: Write failing status and context menu tests**

Append to `apps/yoyovideo-desktop/tests/drop_contract.rs`:

```rust
use yoyovideo_desktop::dropped_media_status;

#[test]
fn dropped_media_status_explains_unsupported_only_drop() {
    let status = dropped_media_status(&DroppedMediaAction::NoPlayableMedia { ignored_count: 2 });

    assert_eq!(status, "No playable media found in dropped items");
}

#[test]
fn dropped_media_status_reports_playlist_replacement_count() {
    let status = dropped_media_status(&DroppedMediaAction::ReplacePlaylist(vec![
        PlaylistEntry::new(MediaLocator::File("a.mp4".into())),
        PlaylistEntry::new(MediaLocator::File("b.mp4".into())),
    ]));

    assert_eq!(status, "Opened dropped playlist: 2 items");
}
```

Create `apps/yoyovideo-desktop/tests/context_menu_contract.rs`:

```rust
use yoyovideo_desktop::MainWindow;

#[test]
fn main_window_context_menu_daily_actions_compile() {
    let window = MainWindow::new().unwrap();

    window.on_open_file_requested(|| {});
    window.on_open_folder_requested(|| {});
    window.on_screenshot_requested(|| {});
    window.on_settings_requested(|| {});
    window.on_toggle_fullscreen_requested(|| {});
    window.on_show_playlist_tab_requested(|| {});
    window.on_show_history_tab_requested(|| {});
}
```

- [ ] **Step 2: Run failing tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test drop_contract dropped_media_status
cargo test -p yoyovideo-desktop --test context_menu_contract
```

Expected:

- drop status tests fail because `dropped_media_status` does not exist.
- context menu contract may pass callback existence but is still run before UI changes to guard callback names.

- [ ] **Step 3: Add dropped media status helper**

Modify `apps/yoyovideo-desktop/src/app.rs` near `dispatch_shortcut`:

```rust
pub fn dropped_media_status(action: &crate::platform::DroppedMediaAction) -> String {
    match action {
        crate::platform::DroppedMediaAction::NoPlayableMedia { .. } => {
            "No playable media found in dropped items".to_string()
        }
        crate::platform::DroppedMediaAction::OpenFile(path) => {
            format!("Opened dropped file: {}", path.display())
        }
        crate::platform::DroppedMediaAction::ReplacePlaylist(entries) => {
            format!("Opened dropped playlist: {} items", entries.len())
        }
    }
}
```

Modify `apps/yoyovideo-desktop/src/lib.rs` root exports:

```rust
pub use app::{
    DesktopController, MainWindow, SettingsWindow, ShortcutDispatch, TrackPopupRowData,
    build_desktop_backend, build_desktop_backend_with_video_window, dispatch_shortcut,
    dropped_media_status, refresh_window, resolve_shortcut, run,
};
```

- [ ] **Step 4: Add drop dispatch helper**

Modify `apps/yoyovideo-desktop/src/app.rs` after `dispatch_video_adjustment`:

```rust
fn dispatch_dropped_paths(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    paths: Vec<PathBuf>,
) {
    if paths.is_empty() {
        return;
    }

    let action = match crate::platform::classify_dropped_paths(&paths) {
        Ok(action) => action,
        Err(error) => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(format!("Drop failed: {error}").into());
            }
            return;
        }
    };
    let status = dropped_media_status(&action);

    match action {
        crate::platform::DroppedMediaAction::NoPlayableMedia { .. } => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(status.into());
            }
        }
        crate::platform::DroppedMediaAction::OpenFile(path) => {
            let dispatched = with_runtime_controller(app_handle, runtime, move |controller| {
                controller.dispatch(AppCommand::OpenFile(path))
            });
            if dispatched && let Some(app) = app_handle.upgrade() {
                app.set_status_label(status.into());
            }
        }
        crate::platform::DroppedMediaAction::ReplacePlaylist(entries) => {
            let dispatched = with_runtime_controller(app_handle, runtime, move |controller| {
                controller.open_playlist_entries(entries)
            });
            if dispatched && let Some(app) = app_handle.upgrade() {
                app.set_status_label(status.into());
            }
        }
    }
}
```

- [ ] **Step 5: Wire winit dropped-file debounce**

Modify `apps/yoyovideo-desktop/src/app.rs` in `run()` before `app.window().on_winit_window_event`:

```rust
    let keyboard_state =
        Rc::new(RefCell::new(crate::keyboard::winit_adapter::WinitKeyboardState::default()));
    let dropped_paths = Rc::new(RefCell::new(Vec::<PathBuf>::new()));
    let drop_timer = Rc::new(slint::Timer::default());
```

Modify the `app.window().on_winit_window_event` capture list to include:

```rust
        let dropped_paths = Rc::clone(&dropped_paths);
        let drop_timer = Rc::clone(&drop_timer);
```

At the start of the event closure, before shortcut handling, add:

```rust
            if let slint::winit_030::winit::event::WindowEvent::DroppedFile(path) = event {
                dropped_paths.borrow_mut().push(path.clone());
                drop_timer.stop();
                drop_timer.start(slint::TimerMode::SingleShot, Duration::from_millis(120), {
                    let app_handle = app_handle.clone();
                    let runtime = Rc::clone(&runtime);
                    let dropped_paths = Rc::clone(&dropped_paths);
                    move || {
                        let paths = std::mem::take(&mut *dropped_paths.borrow_mut());
                        dispatch_dropped_paths(&app_handle, &runtime, paths);
                    }
                });
                return slint::winit_030::EventResult::PreventDefault;
            }
```

Keep the existing shortcut handling below this block unchanged.

- [ ] **Step 6: Complete context menu entries**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` `menu_popup` height and entries:

```slint
    menu_popup := PopupWindow {
        close-policy: close-on-click-outside;
        width: 230px;
        height: 560px;

        VerticalBox {
            spacing: 4px;
            Button { text: "Open File"; clicked => { root.open_file_requested(); menu_popup.close(); } }
            Button { text: "Open Folder"; clicked => { root.open_folder_requested(); menu_popup.close(); } }
            Button { text: "Playlist"; clicked => { root.show_playlist_tab_requested(); menu_popup.close(); } }
            Button { text: "History"; clicked => { root.show_history_tab_requested(); menu_popup.close(); } }
            Button { text: "Play/Pause"; clicked => { root.toggle_pause_requested(); menu_popup.close(); } }
            Button { text: "Speed -"; clicked => { root.speed_down_requested(); menu_popup.close(); } }
            Button { text: "Speed +"; clicked => { root.speed_up_requested(); menu_popup.close(); } }
            Button { text: "Speed 1x"; clicked => { root.reset_speed_requested(); menu_popup.close(); } }
            Button { text: "Zoom -"; clicked => { root.zoom_out_requested(); menu_popup.close(); } }
            Button { text: "Zoom +"; clicked => { root.zoom_in_requested(); menu_popup.close(); } }
            Button { text: "Rotate"; clicked => { root.rotate_requested(); menu_popup.close(); } }
            Button { text: "Audio Channel"; clicked => { root.cycle_audio_requested(); menu_popup.close(); } }
            Button { text: "Set A"; clicked => { root.set_ab_point_a_requested(); menu_popup.close(); } }
            Button { text: "Set B"; clicked => { root.set_ab_point_b_requested(); menu_popup.close(); } }
            Button { text: "Clear A-B"; clicked => { root.clear_ab_loop_requested(); menu_popup.close(); } }
            Button { text: "Screenshot"; clicked => { root.screenshot_requested(); menu_popup.close(); } }
            Button { text: "Video Tools"; clicked => { video_tools_popup.show(); menu_popup.close(); } }
            Button { text: "Fullscreen"; clicked => { root.toggle_fullscreen_requested(); menu_popup.close(); } }
            Button { text: "Settings"; clicked => { root.settings_requested(); menu_popup.close(); } }
        }
    }
```

- [ ] **Step 7: Run task tests and desktop check**

Run:

```powershell
cargo test -p yoyovideo-desktop --test drop_contract
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/drop_contract.rs apps/yoyovideo-desktop/tests/context_menu_contract.rs
git commit -m "feat: wire drag drop and daily menu actions"
```

Expected: Commit succeeds.

---

### Task 3: Local Diagnostic Logging

**Files:**
- Create: `apps/yoyovideo-desktop/src/platform/logging.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/main.rs`
- Create: `apps/yoyovideo-desktop/tests/logging_contract.rs`

**Interfaces:**
- Produces: `pub fn default_log_file(paths: Option<&AppPaths>) -> PathBuf`
- Produces: `pub fn diagnostic_timestamp_now() -> String`
- Produces: `pub fn append_diagnostic_line(path: &Path, timestamp: &str, level: &str, message: &str) -> Result<(), std::io::Error>`
- Produces: `pub fn append_diagnostic(paths: Option<&AppPaths>, level: &str, message: &str) -> Result<PathBuf, std::io::Error>`
- Produces: `DesktopRuntime::record_diagnostic(&mut self, level: &str, message: impl AsRef<str>)`

- [ ] **Step 1: Write failing logging tests**

Create `apps/yoyovideo-desktop/tests/logging_contract.rs`:

```rust
use tempfile::tempdir;
use yoyovideo_desktop::platform::{
    AppPaths, append_diagnostic, append_diagnostic_line, default_log_file,
};

#[test]
fn default_log_file_uses_app_data_logs_directory() {
    let dir = tempdir().unwrap();
    let paths = AppPaths {
        config_dir: dir.path().join("config"),
        data_dir: dir.path().join("data"),
        cache_dir: dir.path().join("cache"),
    };

    let log = default_log_file(Some(&paths));

    assert_eq!(log, dir.path().join("data").join("logs").join("yoyovideo.log"));
}

#[test]
fn append_diagnostic_line_creates_parent_and_appends_text() {
    let dir = tempdir().unwrap();
    let log = dir.path().join("logs").join("yoyovideo.log");

    append_diagnostic_line(&log, "2026-07-06T10:11:12+08:00", "ERROR", "backend failed")
        .unwrap();
    append_diagnostic_line(&log, "2026-07-06T10:11:13+08:00", "WARN", "retrying").unwrap();

    let content = std::fs::read_to_string(log).unwrap();
    assert!(content.contains("2026-07-06T10:11:12+08:00 ERROR backend failed"));
    assert!(content.contains("2026-07-06T10:11:13+08:00 WARN retrying"));
}

#[test]
fn append_diagnostic_returns_actual_log_path() {
    let dir = tempdir().unwrap();
    let paths = AppPaths {
        config_dir: dir.path().join("config"),
        data_dir: dir.path().join("data"),
        cache_dir: dir.path().join("cache"),
    };

    let path = append_diagnostic(Some(&paths), "INFO", "startup").unwrap();

    assert_eq!(path, dir.path().join("data").join("logs").join("yoyovideo.log"));
    assert!(std::fs::read_to_string(path).unwrap().contains("INFO startup"));
}
```

- [ ] **Step 2: Run failing logging tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test logging_contract
```

Expected: FAIL with unresolved logging helpers.

- [ ] **Step 3: Add logging module**

Create `apps/yoyovideo-desktop/src/platform/logging.rs`:

```rust
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;

use super::AppPaths;

pub fn default_log_file(paths: Option<&AppPaths>) -> PathBuf {
    paths
        .map(|paths| paths.data_dir.join("logs").join("yoyovideo.log"))
        .unwrap_or_else(|| PathBuf::from("logs").join("yoyovideo.log"))
}

pub fn diagnostic_timestamp_now() -> String {
    Local::now().to_rfc3339()
}

pub fn append_diagnostic_line(
    path: &Path,
    timestamp: &str,
    level: &str,
    message: &str,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let sanitized = message.replace('\r', " ").replace('\n', " ");
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{timestamp} {level} {sanitized}")?;
    Ok(())
}

pub fn append_diagnostic(
    paths: Option<&AppPaths>,
    level: &str,
    message: &str,
) -> Result<PathBuf, std::io::Error> {
    let path = default_log_file(paths);
    append_diagnostic_line(&path, &diagnostic_timestamp_now(), level, message)?;
    Ok(path)
}
```

- [ ] **Step 4: Export logging helpers**

Modify `apps/yoyovideo-desktop/src/platform/mod.rs`:

```rust
mod dialogs;
mod drop;
mod logging;
mod media_scan;
mod paths;
mod screenshot;

pub use dialogs::{DialogService, RfdDialogService};
pub use drop::{DroppedMediaAction, classify_dropped_paths};
pub use logging::{
    append_diagnostic, append_diagnostic_line, default_log_file, diagnostic_timestamp_now,
};
pub use media_scan::scan_media_folder;
pub use paths::AppPaths;
pub use screenshot::{
    default_screenshot_dir, next_screenshot_path, prepare_screenshot_path,
    prepare_screenshot_path_in_dir, screenshot_timestamp_now,
};
```

- [ ] **Step 5: Add runtime diagnostic state**

Modify `apps/yoyovideo-desktop/src/app.rs` `DesktopRuntime` fields:

```rust
    diagnostic_log_path: PathBuf,
    diagnostic_log_failed: bool,
```

Modify `DesktopRuntime::new` signature and construction:

```rust
    fn new(
        config: AppConfig,
        history: crate::HistoryRuntime,
        subtitle_prefs: crate::SubtitlePrefsRuntime,
        sidebar: crate::SidebarState,
        diagnostic_log_path: PathBuf,
    ) -> Self {
        Self {
            controller: None,
            video_host_error: initial_runtime_error(),
            app_handle: None,
            config,
            history,
            subtitle_prefs,
            sidebar,
            settings_window: None,
            settings_controller: None,
            pending_resume: None,
            last_seen_locator: None,
            last_seen_subtitle_locator: None,
            started_at: Instant::now(),
            diagnostic_log_path,
            diagnostic_log_failed: false,
            #[cfg(feature = "mpv-runtime")]
            video_host: None,
        }
    }
```

Add method inside `impl DesktopRuntime`:

```rust
    fn record_diagnostic(&mut self, level: &str, message: impl AsRef<str>) {
        if self.diagnostic_log_failed {
            return;
        }
        if crate::platform::append_diagnostic_line(
            &self.diagnostic_log_path,
            &crate::platform::diagnostic_timestamp_now(),
            level,
            message.as_ref(),
        )
        .is_err()
        {
            self.diagnostic_log_failed = true;
        }
    }
```

Modify runtime construction in `run()`:

```rust
    let diagnostic_log_path = crate::platform::default_log_file(paths.as_ref());
    let runtime = Rc::new(RefCell::new(DesktopRuntime::new(
        config,
        history,
        subtitle_prefs,
        sidebar,
        diagnostic_log_path,
    )));
```

- [ ] **Step 6: Log non-fatal controller and screenshot errors**

Modify `with_runtime_controller` error paths in `apps/yoyovideo-desktop/src/app.rs`.

When controller is missing, before status update:

```rust
        let message = runtime.status_message();
        runtime.record_diagnostic("WARN", &message);
```

When `outcome` is `Err((error, pending_restore))`, before setting status:

```rust
            runtime.record_diagnostic("ERROR", error.to_string());
```

Modify `dispatch_screenshot` path error branch:

```rust
            runtime
                .borrow_mut()
                .record_diagnostic("ERROR", format!("Screenshot path failed: {error}"));
```

Modify poll timer `Err((error, pending_restore))` branch:

```rust
                    runtime.record_diagnostic("ERROR", error.to_string());
```

- [ ] **Step 7: Log fatal startup errors in main**

Modify `apps/yoyovideo-desktop/src/main.rs`:

```rust
fn main() -> std::process::ExitCode {
    match yoyovideo_desktop::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("Fatal startup error: {error}");
            eprintln!("{message}");
            let _ = yoyovideo_desktop::platform::append_diagnostic(None, "ERROR", &message);
            std::process::ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 8: Run logging tests and checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test logging_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 9: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/platform/logging.rs apps/yoyovideo-desktop/src/platform/mod.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/main.rs apps/yoyovideo-desktop/tests/logging_contract.rs
git commit -m "feat: add local diagnostic logging"
```

Expected: Commit succeeds.

---

### Task 4: Actionable Runtime Startup Diagnostics

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Create: `apps/yoyovideo-desktop/tests/runtime_diagnostics_contract.rs`

**Interfaces:**
- Produces: `pub fn format_runtime_startup_error(error: &str) -> String`
- Consumes: `DesktopRuntime::record_diagnostic(level, message)`

- [ ] **Step 1: Write failing runtime diagnostic tests**

Create `apps/yoyovideo-desktop/tests/runtime_diagnostics_contract.rs`:

```rust
use yoyovideo_desktop::format_runtime_startup_error;

#[test]
fn windows_runtime_startup_error_mentions_mpv_dll_and_bootstrap_command() {
    let message = format_runtime_startup_error("backend init failed");

    if cfg!(target_os = "windows") {
        assert!(message.contains("mpv-2.dll"));
        assert!(message.contains("scripts/bootstrap-runtime.ps1"));
        assert!(message.contains("backend init failed"));
    } else {
        assert!(message.contains("backend init failed"));
        assert!(message.contains("libmpv"));
    }
}
```

- [ ] **Step 2: Run failing test**

Run:

```powershell
cargo test -p yoyovideo-desktop --test runtime_diagnostics_contract
```

Expected: FAIL because `format_runtime_startup_error` does not exist.

- [ ] **Step 3: Add runtime startup formatter**

Modify `apps/yoyovideo-desktop/src/app.rs` near `initial_runtime_error`:

```rust
pub fn format_runtime_startup_error(error: &str) -> String {
    if cfg!(target_os = "windows") {
        format!(
            "Playback runtime failed: {error}. Check that libmpv is staged at third_party/mpv/windows-x64/bin/mpv-2.dll for development or beside the packaged executable for release. Recovery: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force"
        )
    } else if cfg!(target_os = "macos") {
        format!(
            "Playback runtime failed: {error}. Check that libmpv.dylib is staged for macos-universal packaging or available to the app runtime."
        )
    } else {
        format!(
            "Playback runtime failed: {error}. Check that libmpv.so is staged for linux-x64 packaging or available through LD_LIBRARY_PATH."
        )
    }
}
```

Modify `apps/yoyovideo-desktop/src/lib.rs` app exports:

```rust
pub use app::{
    DesktopController, MainWindow, SettingsWindow, ShortcutDispatch, TrackPopupRowData,
    build_desktop_backend, build_desktop_backend_with_video_window, dispatch_shortcut,
    dropped_media_status, format_runtime_startup_error, refresh_window, resolve_shortcut, run,
};
```

- [ ] **Step 4: Use actionable formatter during runtime initialization**

Modify `DesktopWinitHandler::initialize_runtime` error branch in `apps/yoyovideo-desktop/src/app.rs`:

```rust
            Err(error) => {
                let message = format_runtime_startup_error(&error);
                runtime.record_diagnostic("ERROR", &message);
                runtime.mark_error(message.clone());
                if let Some(app_handle) = runtime.app_handle.clone() {
                    if let Some(app) = app_handle.upgrade() {
                        app.set_status_label(message.into());
                        refresh_sidebar(&app, &runtime);
                        refresh_tracks_popup(&app, &runtime);
                    }
                }
            }
```

- [ ] **Step 5: Run runtime diagnostic tests and feature check**

Run:

```powershell
cargo test -p yoyovideo-desktop --test runtime_diagnostics_contract
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/runtime_diagnostics_contract.rs
git commit -m "feat: improve runtime startup diagnostics"
```

Expected: Commit succeeds.

---

### Task 5: Package Smoke Verification

**Files:**
- Modify: `scripts/smoke-runtime.ps1`
- Create: `scripts/smoke-package.ps1`
- Create: `scripts/test-package-smoke.ps1`
- Modify: `README.md`
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes: `scripts/verify-package.ps1 -Platform <platform> -PackageDir <dir> [-RequireRuntime]`
- Produces: `scripts/smoke-runtime.ps1 -RuntimeBin <path> -RuntimeLib <path>` optional overrides
- Produces: `scripts/smoke-package.ps1 -Platform <platform> -PackageDir <dir> [-RequireRuntime] [-TimeoutSeconds <n>] [-SkipLaunch] [-SkipRuntimePlayback]`
- Produces: smoke log at `<PackageDir>/smoke/package-smoke.log`

- [ ] **Step 1: Write failing package smoke fixture test**

Create `scripts/test-package-smoke.ps1`:

```powershell
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Invoke-ExpectSuccess([scriptblock]$Command, [string]$Message) {
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw $Message
    }
}

function Invoke-ExpectFailure([scriptblock]$Command, [string]$ExpectedText) {
    $output = & $Command 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "Expected command to fail: $ExpectedText"
    }
    if (($output | Out-String) -notmatch [regex]::Escape($ExpectedText)) {
        throw "Expected failure containing '$ExpectedText'. Actual output: $output"
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-package-smoke-test-" + [Guid]::NewGuid())
$packageDir = Join-Path $tempRoot "YoYoVideo-windows-x64"

New-Item -ItemType Directory -Force `
    (Join-Path $packageDir "bin") `
    (Join-Path $packageDir "docs") `
    (Join-Path $packageDir "LICENSES") | Out-Null

Set-Content -LiteralPath (Join-Path $packageDir "README.md") -Value "readme"
Set-Content -LiteralPath (Join-Path $packageDir "RELEASE-NOTES.md") -Value "release"
Set-Content -LiteralPath (Join-Path $packageDir "LICENSES/README.md") -Value "licenses"
Set-Content -LiteralPath (Join-Path $packageDir "LICENSES/runtime-provenance.md") -Value "runtime"
Set-Content -LiteralPath (Join-Path $packageDir "docs/runtime-dependencies.md") -Value "runtime docs"
Set-Content -LiteralPath (Join-Path $packageDir "docs/manual-smoke-checklist.md") -Value "smoke"

Invoke-ExpectFailure {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/smoke-package.ps1") -Platform windows-x64 -PackageDir $packageDir -RequireRuntime -SkipLaunch -SkipRuntimePlayback
} "Missing desktop binary"

Set-Content -LiteralPath (Join-Path $packageDir "bin/yoyovideo-desktop.exe") -Value "fixture exe"
Invoke-ExpectFailure {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/smoke-package.ps1") -Platform windows-x64 -PackageDir $packageDir -RequireRuntime -SkipLaunch -SkipRuntimePlayback
} "Missing Windows libmpv runtime DLL"

Set-Content -LiteralPath (Join-Path $packageDir "bin/mpv-2.dll") -Value "fixture dll"
Invoke-ExpectSuccess {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/smoke-package.ps1") -Platform windows-x64 -PackageDir $packageDir -RequireRuntime -SkipLaunch -SkipRuntimePlayback
} "package smoke fixture should succeed when required files exist"

$logPath = Join-Path $packageDir "smoke/package-smoke.log"
Assert-True (Test-Path -LiteralPath $logPath -PathType Leaf) "smoke log was not created"
Assert-True ((Get-Content -Raw -LiteralPath $logPath) -match "package_smoke=ok") "smoke log did not record success"

Remove-Item -LiteralPath $tempRoot -Recurse -Force
Write-Host "package smoke fixture tests passed"
```

- [ ] **Step 2: Run failing package smoke fixture test**

Run:

```powershell
pwsh -NoProfile -File scripts/test-package-smoke.ps1
```

Expected: FAIL because `scripts/smoke-package.ps1` does not exist.

- [ ] **Step 3: Add runtime directory overrides to runtime smoke**

Modify `scripts/smoke-runtime.ps1` parameters:

```powershell
param(
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform = "windows-x64",

    [int]$TimeoutSeconds = 5,

    [string]$RuntimeBin,

    [string]$RuntimeLib
)
```

Replace the existing `$runtimeBin` and `$runtimeLib` assignments:

```powershell
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($RuntimeBin)) {
    $runtimeBin = Join-Path $repoRoot "third_party/mpv/$Platform/bin"
} else {
    $runtimeBin = [System.IO.Path]::GetFullPath($RuntimeBin)
}
if ([string]::IsNullOrWhiteSpace($RuntimeLib)) {
    $runtimeLib = Join-Path $repoRoot "third_party/mpv/$Platform/lib"
} else {
    $runtimeLib = [System.IO.Path]::GetFullPath($RuntimeLib)
}
```

Keep the rest of the script behavior unchanged.

- [ ] **Step 4: Add package smoke script**

Create `scripts/smoke-package.ps1`:

```powershell
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform,

    [Parameter(Mandatory = $true)]
    [string]$PackageDir,

    [switch]$RequireRuntime,

    [int]$TimeoutSeconds = 5,

    [switch]$SkipLaunch,

    [switch]$SkipRuntimePlayback
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-SmokeLog "ERROR $Message"
    Write-Error $Message
    exit 1
}

function Write-SmokeLog([string]$Message) {
    if ([string]::IsNullOrWhiteSpace($script:SmokeLog)) {
        return
    }
    $parent = Split-Path -Parent $script:SmokeLog
    New-Item -ItemType Directory -Force $parent | Out-Null
    Add-Content -LiteralPath $script:SmokeLog -Value "$([DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")) $Message"
}

function Require-File([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Missing $Description at $Path"
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$PackageDir = [System.IO.Path]::GetFullPath($PackageDir)
$script:SmokeLog = Join-Path $PackageDir "smoke/package-smoke.log"
$binaryName = if ($Platform -eq "windows-x64") { "yoyovideo-desktop.exe" } else { "yoyovideo-desktop" }
$binaryPath = Join-Path $PackageDir "bin/$binaryName"

Write-SmokeLog "package_smoke=start platform=$Platform package=$PackageDir"

$verifyArgs = @("-NoProfile", "-File", (Join-Path $repoRoot "scripts/verify-package.ps1"), "-Platform", $Platform, "-PackageDir", $PackageDir)
if ($RequireRuntime) {
    $verifyArgs += "-RequireRuntime"
}
& pwsh @verifyArgs
if ($LASTEXITCODE -ne 0) {
    Fail "Package layout verification failed"
}

Require-File $binaryPath "desktop binary"

if (-not $SkipLaunch) {
    Write-SmokeLog "launch=start binary=$binaryPath"
    if ($IsWindows) {
        $process = Start-Process -FilePath $binaryPath -PassThru -WindowStyle Hidden
    } else {
        $process = Start-Process -FilePath $binaryPath -PassThru
    }
    Start-Sleep -Seconds ([Math]::Min($TimeoutSeconds, 3))
    if ($process.HasExited -and $process.ExitCode -ne 0) {
        Fail "Desktop binary exited during launch smoke with code $($process.ExitCode)"
    }
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    Write-SmokeLog "launch=ok"
}

if ($RequireRuntime -and -not $SkipRuntimePlayback) {
    $runtimeBin = Join-Path $PackageDir "bin"
    $smokeArgs = @(
        "-NoProfile",
        "-File",
        (Join-Path $repoRoot "scripts/smoke-runtime.ps1"),
        "-Platform",
        $Platform,
        "-TimeoutSeconds",
        $TimeoutSeconds,
        "-RuntimeBin",
        $runtimeBin,
        "-RuntimeLib",
        $runtimeBin
    )
    $oldPath = $env:PATH
    $oldDyld = $env:DYLD_LIBRARY_PATH
    $oldLd = $env:LD_LIBRARY_PATH
    try {
        if ($Platform -eq "windows-x64") {
            $env:PATH = "$runtimeBin;$env:PATH"
        }
        if ($Platform -eq "macos-universal") {
            $env:DYLD_LIBRARY_PATH = "$runtimeBin;$env:DYLD_LIBRARY_PATH"
        }
        if ($Platform -eq "linux-x64") {
            $env:LD_LIBRARY_PATH = "$runtimeBin;$env:LD_LIBRARY_PATH"
        }
        Write-SmokeLog "runtime_playback=start"
        & pwsh @smokeArgs
        if ($LASTEXITCODE -ne 0) {
            Fail "Runtime playback smoke failed"
        }
        Write-SmokeLog "runtime_playback=ok"
    } finally {
        $env:PATH = $oldPath
        $env:DYLD_LIBRARY_PATH = $oldDyld
        $env:LD_LIBRARY_PATH = $oldLd
    }
}

Write-SmokeLog "package_smoke=ok"
Write-Host "Package smoke passed: $PackageDir"
Write-Host "Smoke log: $script:SmokeLog"
```

- [ ] **Step 5: Run package smoke fixture test**

Run:

```powershell
pwsh -NoProfile -File scripts/test-package-smoke.ps1
```

Expected: PASS.

- [ ] **Step 6: Document package smoke command**

Modify `README.md` after the existing `verify-package.ps1` example:

Run package smoke after verification to launch the packaged binary briefly and, when runtime files are required, run temporary-media playback against the package runtime:

```powershell
pwsh -NoProfile -File scripts/smoke-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

Modify `docs/testing/manual-smoke-checklist.md` under `## Package Artifacts`:

```markdown
- Run `pwsh -NoProfile -File scripts/smoke-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime` and confirm it writes `smoke/package-smoke.log`.
```

- [ ] **Step 7: Run script and docs checks**

Run:

```powershell
pwsh -NoProfile -File scripts/test-package-smoke.ps1
$content = Get-Content -Raw docs/testing/manual-smoke-checklist.md
if ($content -notmatch [regex]::Escape("scripts/smoke-package.ps1")) {
  Write-Error "manual smoke checklist does not mention package smoke"
  exit 1
}
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add scripts/smoke-runtime.ps1 scripts/smoke-package.ps1 scripts/test-package-smoke.ps1 README.md docs/testing/manual-smoke-checklist.md
git commit -m "test: add package smoke verification"
```

Expected: Commit succeeds.

---

### Task 6: Final Verification And Manual Smoke Coverage

**Files:**
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes: all prior tasks.
- Produces: final manual coverage for drag-drop, context menu, diagnostics, and package smoke.

- [ ] **Step 1: Ensure manual smoke checklist includes daily-use checks**

Confirm `docs/testing/manual-smoke-checklist.md` includes these lines under `## UX`:

```markdown
- Drag a single local video file onto the window and confirm playback starts.
- Drag multiple supported media files onto the window and confirm the playlist contains all dropped media in drop order.
- Drag a folder containing supported and unsupported files and confirm only supported media appears in the playlist.
- Drag only unsupported files and confirm current playback continues while the status label reports no playable media.
- Right-click or open `Menu`, then use `Open File`, `Open Folder`, `Playlist`, `History`, `Screenshot`, `Video Tools`, `Fullscreen`, and `Settings`.
- Trigger a playback/runtime error and confirm it appears in the status label and in the local diagnostic log.
```

If any line is missing, add it exactly.

- [ ] **Step 2: Run full automated verification**

Run:

```powershell
cargo fmt --check
cargo test
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
pwsh -NoProfile -File scripts/test-package-smoke.ps1
git status --short
```

Expected:

- `cargo fmt --check`: PASS
- `cargo test`: PASS
- `cargo check -p yoyo-mpv --features mpv-runtime`: PASS
- `cargo check -p yoyovideo-desktop --features mpv-runtime`: PASS
- `scripts/test-package-smoke.ps1`: PASS
- `git status --short`: only `docs/testing/manual-smoke-checklist.md` is modified if Step 1 added lines.

- [ ] **Step 3: Optional runtime smoke when Windows runtime is staged**

Run this if `third_party/mpv/windows-x64/bin/mpv-2.dll` exists:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64 -TimeoutSeconds 8
```

Expected: PASS with `runtime_smoke=ok`.

- [ ] **Step 4: Commit final checklist if changed**

If `git status --short` shows `docs/testing/manual-smoke-checklist.md`, run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add daily use release smoke checks"
```

Expected: Commit succeeds.

If no files are modified, skip the commit.

---

## Self-Review

**Spec coverage:** The plan covers drag-and-drop file/folder opening, multiple-file playlist replacement, unsupported drag safety, context menu daily actions, local diagnostics, actionable runtime startup errors, package executable/runtime verification, basic launch smoke, temporary-media playback smoke, automated tests, manual smoke coverage, and final verification commands.

**Placeholder scan:** No placeholder markers remain. Each task names exact files, interfaces, test code, implementation snippets, commands, expected outcomes, and commit messages.

**Type consistency:** `DroppedMediaAction`, `classify_dropped_paths`, `dropped_media_status`, `open_playlist_entries`, `default_log_file`, `append_diagnostic_line`, `append_diagnostic`, and `format_runtime_startup_error` are introduced before later tasks consume them. Desktop-only behavior remains in `apps/yoyovideo-desktop`; core and mpv boundaries are preserved.
