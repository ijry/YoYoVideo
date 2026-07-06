# Cinema Deck Playback Experience Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Cinema Deck player experience with mute, jump-to-time, preview-seeking, OSD feedback, embedded chapters, user markers, and a polished video-first Slint layout.

**Architecture:** Keep playback state and commands in `yoyo-core`, mpv property translation/decoding in `yoyo-mpv`, and desktop persistence/UI orchestration in `apps/yoyovideo-desktop`. Implement as vertical slices so each commit has failing tests, working behavior, and no unconnected UI.

**Tech Stack:** Rust 2024, Slint 1.17.0, libmpv through `yoyo-mpv`, serde/TOML desktop stores, chrono timestamps, existing PowerShell smoke scripts.

## Global Constraints

- Do not replace Slint, add a webview, or move playback logic into `.slint` files.
- Do not add media-library indexing, thumbnails, tagging, or search.
- Do not mutate media files; user markers are YoYoVideo-local metadata.
- Do not add global hotkeys or OS media keys.
- Existing playback, playlist, history, recent, tracks, subtitles, screenshot, frame-step, filters, settings, and shortcuts must keep working.
- Progress drag must commit one seek on release, not repeated seeks during preview.
- Embedded chapters are read-only and come from mpv `chapter-list`.
- User markers are independent from `remember_history`.
- Missing or corrupt marker stores load as empty and never block playback.
- Full verification at the end includes `cargo fmt --check`, `cargo test`, runtime feature checks, package smoke, and runtime smoke when staged mpv files exist.

---

## File Structure

- Modify `crates/yoyo-core/src/player_state.rs`: add `MediaChapter`, `MediaMarker`, mute/chapter/marker state, and marker helpers.
- Modify `crates/yoyo-core/src/backend.rs`: add mute and chapter backend command/event variants.
- Modify `crates/yoyo-core/src/app_command.rs`: add mute, jump, chapter, and marker commands.
- Modify `crates/yoyo-core/src/session.rs`: implement mute, jump clamping, chapter updates, marker add/remove/dedupe, and chapter/marker seeking.
- Modify `crates/yoyo-core/src/shortcut.rs`: add shortcut actions and defaults.
- Modify `crates/yoyo-core/src/lib.rs`: export new chapter/marker types.
- Modify `crates/yoyo-core/tests/session_contract.rs`: cover core behavior.
- Modify `crates/yoyo-core/tests/config_shortcut_contract.rs`: cover shortcut defaults.
- Create `crates/yoyo-mpv/src/chapter_list.rs`: decode mpv `chapter-list` nodes.
- Modify `crates/yoyo-mpv/src/client.rs`: observe `mute` and `chapter-list`, decode new properties.
- Modify `crates/yoyo-mpv/src/event.rs`: map mute/chapter events.
- Modify `crates/yoyo-mpv/src/translate.rs`: translate mute command.
- Modify `crates/yoyo-mpv/src/lib.rs`: include `chapter_list`.
- Modify `crates/yoyo-mpv/tests/event_contract.rs`: cover mute/chapter mapping.
- Modify `crates/yoyo-mpv/tests/translate_contract.rs`: cover mute translation.
- Create `apps/yoyovideo-desktop/src/platform/markers.rs`: marker store persistence.
- Modify `apps/yoyovideo-desktop/src/platform/mod.rs`: export marker store helpers.
- Create `apps/yoyovideo-desktop/tests/marker_store_contract.rs`: marker store tests.
- Create `apps/yoyovideo-desktop/src/progress.rs`: time parsing, preview labels, ticks, chapter/marker rows.
- Create `apps/yoyovideo-desktop/src/osd.rs`: OSD presenter model and labels.
- Modify `apps/yoyovideo-desktop/src/lib.rs`: export progress and OSD helpers.
- Modify `apps/yoyovideo-desktop/tests/presenter_contract.rs`: add progress, jump, and OSD presenter tests.
- Modify `apps/yoyovideo-desktop/src/keyboard.rs`: support `Shift+Left`, `Shift+Right`, and `P/J/M` mappings through existing character paths.
- Modify `apps/yoyovideo-desktop/src/app.rs`: load marker store, wire callbacks, refresh rows/ticks, set OSD, and show action/jump surfaces.
- Modify `apps/yoyovideo-desktop/ui/main-window.slint`: add Cinema Deck properties, callbacks, custom progress rail, OSD overlay, action panel, jump overlay, chapter/marker rows, and custom visual controls.
- Modify `apps/yoyovideo-desktop/tests/context_menu_contract.rs`: cover new main-window surface.
- Modify `apps/yoyovideo-desktop/tests/shortcut_contract.rs`: cover new shortcut dispatch paths.
- Modify `apps/yoyovideo-desktop/tests/keyboard_contract.rs`: cover shifted arrow normalization.
- Modify `docs/testing/manual-smoke-checklist.md`: add Cinema Deck smoke checks.

---

### Task 1: Core Playback Model, Mute, Chapters, Markers, And Shortcuts

**Files:**
- Modify: `crates/yoyo-core/src/player_state.rs`
- Modify: `crates/yoyo-core/src/backend.rs`
- Modify: `crates/yoyo-core/src/app_command.rs`
- Modify: `crates/yoyo-core/src/session.rs`
- Modify: `crates/yoyo-core/src/shortcut.rs`
- Modify: `crates/yoyo-core/src/lib.rs`
- Modify: `crates/yoyo-core/tests/session_contract.rs`
- Modify: `crates/yoyo-core/tests/config_shortcut_contract.rs`

**Interfaces:**
- Produces: `pub struct MediaChapter { pub title: Option<String>, pub time_seconds: f64 }`
- Produces: `pub struct MediaMarker { pub id: String, pub title: String, pub time_seconds: f64, pub created_at: String }`
- Produces: `BackendCommand::SetMuted(bool)`
- Produces: `BackendEvent::MutedChanged(bool)`
- Produces: `BackendEvent::ChaptersChanged(Vec<MediaChapter>)`
- Produces: `AppCommand::SetMuted(bool)`, `ToggleMute`, `JumpToTime(f64)`, `AddMarkerAtCurrentPosition { created_at: String }`, `RemoveMarker(String)`, `SeekToChapter(usize)`, `SeekToMarker(String)`, `SeekToNextChapterOrMarker`, `SeekToPreviousChapterOrMarker`
- Produces: `AppSession<B>::set_markers(Vec<MediaMarker>)`
- Produces: shortcut actions `ToggleMute`, `JumpToTime`, `AddMarker`, `OpenActionPanel`, `NextChapterOrMarker`, `PreviousChapterOrMarker`

- [ ] **Step 1: Add failing core session tests**

Append to `crates/yoyo-core/tests/session_contract.rs`:

```rust
#[test]
fn toggle_mute_updates_state_and_backend() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);

    session.handle_command(AppCommand::ToggleMute).unwrap();

    assert!(session.state().muted);
    assert_eq!(session.backend().commands, vec![BackendCommand::SetMuted(true)]);

    session.backend_mut().events.push(BackendEvent::MutedChanged(false));
    session.poll_backend().unwrap();

    assert!(!session.state().muted);
}

#[test]
fn jump_to_time_clamps_when_duration_is_known() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session.backend_mut().events.push(BackendEvent::DurationChanged(Some(90.0)));
    session.poll_backend().unwrap();

    session.handle_command(AppCommand::JumpToTime(120.0)).unwrap();

    assert_eq!(session.backend().commands, vec![BackendCommand::SeekAbsolute(90.0)]);
}

#[test]
fn chapters_event_replaces_chapter_state_and_opening_media_clears_it() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    let chapters = vec![
        yoyo_core::MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 },
        yoyo_core::MediaChapter { title: Some("Scene".into()), time_seconds: 42.0 },
    ];

    session.backend_mut().events.push(BackendEvent::ChaptersChanged(chapters.clone()));
    session.poll_backend().unwrap();
    assert_eq!(session.state().chapters, chapters);

    session.handle_command(AppCommand::OpenFile(PathBuf::from("movie.mp4"))).unwrap();
    assert!(session.state().chapters.is_empty());
}

#[test]
fn markers_add_dedupe_remove_and_seek() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session.backend_mut().events.push(BackendEvent::PositionChanged(12.25));
    session.poll_backend().unwrap();

    session
        .handle_command(AppCommand::AddMarkerAtCurrentPosition {
            created_at: "2026-07-06T10:00:00+08:00".into(),
        })
        .unwrap();
    session.backend_mut().events.push(BackendEvent::PositionChanged(12.80));
    session.poll_backend().unwrap();
    session
        .handle_command(AppCommand::AddMarkerAtCurrentPosition {
            created_at: "2026-07-06T10:00:01+08:00".into(),
        })
        .unwrap();

    assert_eq!(session.state().markers.len(), 1);
    let id = session.state().markers[0].id.clone();
    session.handle_command(AppCommand::SeekToMarker(id.clone())).unwrap();
    assert_eq!(session.backend().commands.last(), Some(&BackendCommand::SeekAbsolute(12.25)));

    session.handle_command(AppCommand::RemoveMarker(id)).unwrap();
    assert!(session.state().markers.is_empty());
}

#[test]
fn seek_to_next_and_previous_chapter_or_marker_uses_sorted_points() {
    let backend = MockBackend::default();
    let mut session = AppSession::new(AppConfig::default(), backend);
    session.backend_mut().events.push(BackendEvent::PositionChanged(20.0));
    session
        .backend_mut()
        .events
        .push(BackendEvent::ChaptersChanged(vec![
            yoyo_core::MediaChapter { title: Some("A".into()), time_seconds: 10.0 },
            yoyo_core::MediaChapter { title: Some("B".into()), time_seconds: 50.0 },
        ]));
    session.poll_backend().unwrap();
    session.set_markers(vec![yoyo_core::MediaMarker {
        id: "marker-30000".into(),
        title: "Marker 00:30".into(),
        time_seconds: 30.0,
        created_at: "2026-07-06T10:00:00+08:00".into(),
    }]);

    session.handle_command(AppCommand::SeekToNextChapterOrMarker).unwrap();
    session.handle_command(AppCommand::SeekToPreviousChapterOrMarker).unwrap();

    assert_eq!(
        session.backend().commands,
        vec![BackendCommand::SeekAbsolute(30.0), BackendCommand::SeekAbsolute(10.0)]
    );
}
```

- [ ] **Step 2: Add failing shortcut tests**

Append to `crates/yoyo-core/tests/config_shortcut_contract.rs`:

```rust
#[test]
fn cinema_deck_shortcuts_are_registered() {
    let map = yoyo_core::ShortcutMap::default();

    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("M").unwrap()),
        Some(yoyo_core::ShortcutAction::ToggleMute)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("J").unwrap()),
        Some(yoyo_core::ShortcutAction::JumpToTime)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("Ctrl+M").unwrap()),
        Some(yoyo_core::ShortcutAction::AddMarker)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("P").unwrap()),
        Some(yoyo_core::ShortcutAction::OpenActionPanel)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("Shift+Right").unwrap()),
        Some(yoyo_core::ShortcutAction::NextChapterOrMarker)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("Shift+Left").unwrap()),
        Some(yoyo_core::ShortcutAction::PreviousChapterOrMarker)
    );
}
```

- [ ] **Step 3: Run failing core tests**

Run:

```powershell
cargo test -p yoyo-core --test session_contract toggle_mute_updates_state_and_backend
cargo test -p yoyo-core --test session_contract markers_add_dedupe_remove_and_seek
cargo test -p yoyo-core --test config_shortcut_contract cinema_deck_shortcuts_are_registered
```

Expected: FAIL because new types, commands, events, and shortcut actions do not exist.

- [ ] **Step 4: Implement player state types**

Modify `crates/yoyo-core/src/player_state.rs` after `MediaTrack`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaChapter {
    pub title: Option<String>,
    pub time_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaMarker {
    pub id: String,
    pub title: String,
    pub time_seconds: f64,
    pub created_at: String,
}

pub const MARKER_DEDUPE_TOLERANCE_SECONDS: f64 = 0.75;
```

Add fields to `PlayerState`:

```rust
pub muted: bool,
pub chapters: Vec<MediaChapter>,
pub markers: Vec<MediaMarker>,
```

Add default values:

```rust
muted: false,
chapters: Vec::new(),
markers: Vec::new(),
```

- [ ] **Step 5: Export new core types**

Modify `crates/yoyo-core/src/lib.rs` player state exports:

```rust
pub use player_state::{
    AudioChannelMode, FrameStepDirection, LoopState, MARKER_DEDUPE_TOLERANCE_SECONDS,
    MAX_VIDEO_ADJUSTMENT, MIN_VIDEO_ADJUSTMENT, MediaChapter, MediaMarker, MediaTrack,
    MediaTrackKind, PlayerState, Rotation, SubtitlePlaybackState, VideoAdjustmentKind,
    VideoAdjustments, VideoFilterPreset,
};
```

- [ ] **Step 6: Add command and event variants**

Modify `crates/yoyo-core/src/backend.rs` imports:

```rust
use crate::{
    AudioChannelMode, FrameStepDirection, MediaChapter, MediaLocator, MediaTrack, Rotation,
    VideoAdjustmentKind, VideoFilterPreset,
};
```

Add to `BackendCommand`:

```rust
SetMuted(bool),
```

Add to `BackendEvent`:

```rust
MutedChanged(bool),
ChaptersChanged(Vec<MediaChapter>),
```

Modify `crates/yoyo-core/src/app_command.rs` imports:

```rust
use crate::{FrameStepDirection, VideoAdjustmentKind, VideoFilterPreset};
```

Add to `AppCommand`:

```rust
SetMuted(bool),
ToggleMute,
JumpToTime(f64),
AddMarkerAtCurrentPosition { created_at: String },
RemoveMarker(String),
SeekToChapter(usize),
SeekToMarker(String),
SeekToNextChapterOrMarker,
SeekToPreviousChapterOrMarker,
```

- [ ] **Step 7: Implement session behavior**

Modify `crates/yoyo-core/src/session.rs` imports:

```rust
use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent, MediaChapter,
    MediaLocator, MediaMarker, MediaTrack, PlaybackEndBehavior, PlayerBackend, PlayerState,
    Playlist, PlaylistEntry, PlaylistSnapshot, Rotation, SubtitlePlaybackState,
    MARKER_DEDUPE_TOLERANCE_SECONDS,
};
```

Add helpers inside `impl<B: PlayerBackend> AppSession<B>`:

```rust
fn reset_navigation_state_for_new_media(&mut self) {
    self.state.chapters.clear();
    self.state.markers.clear();
}

fn clamp_seek_target(&self, seconds: f64) -> Result<f64, AppError> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(AppError::Message("Invalid seek target".into()));
    }
    Ok(match self.state.duration_seconds {
        Some(duration) if duration.is_finite() && duration >= 0.0 => seconds.min(duration),
        _ => seconds,
    })
}

fn marker_id_for_position(seconds: f64) -> String {
    format!("marker-{}", (seconds * 1000.0).round() as u64)
}

fn marker_title_for_position(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("Marker {:02}:{:02}", total / 60, total % 60)
}

fn add_marker_at_current_position(&mut self, created_at: String) {
    let position = self.state.position_seconds.max(0.0);
    if self
        .state
        .markers
        .iter()
        .any(|marker| (marker.time_seconds - position).abs() <= MARKER_DEDUPE_TOLERANCE_SECONDS)
    {
        self.state.status_message = Some("Marker already exists near this position".into());
        return;
    }

    self.state.markers.push(MediaMarker {
        id: Self::marker_id_for_position(position),
        title: Self::marker_title_for_position(position),
        time_seconds: position,
        created_at,
    });
    self.state
        .markers
        .sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
    self.state.status_message = Some("Marker added".into());
}

fn chapter_marker_points(&self) -> Vec<f64> {
    let mut points = self
        .state
        .chapters
        .iter()
        .map(|chapter| chapter.time_seconds)
        .chain(self.state.markers.iter().map(|marker| marker.time_seconds))
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .collect::<Vec<_>>();
    points.sort_by(|left, right| left.total_cmp(right));
    points.dedup_by(|left, right| (*left - *right).abs() <= 0.001);
    points
}

pub fn set_markers(&mut self, markers: Vec<MediaMarker>) {
    self.state.markers = markers
        .into_iter()
        .filter(|marker| marker.time_seconds.is_finite() && marker.time_seconds >= 0.0)
        .collect();
    self.state
        .markers
        .sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
}
```

Call `self.reset_navigation_state_for_new_media();` in `replace_playlist`, `open_playlist_index`, and `open_single_locator` after `reset_track_state_for_new_media()`.

Add command branches in `handle_command`:

```rust
AppCommand::SetMuted(muted) => {
    self.state.muted = muted;
    self.backend.send(BackendCommand::SetMuted(muted)).map_err(AppError::Message)?;
}
AppCommand::ToggleMute => {
    let muted = !self.state.muted;
    self.state.muted = muted;
    self.backend.send(BackendCommand::SetMuted(muted)).map_err(AppError::Message)?;
}
AppCommand::JumpToTime(seconds) => {
    let target = self.clamp_seek_target(seconds)?;
    self.backend.send(BackendCommand::SeekAbsolute(target)).map_err(AppError::Message)?;
    self.state.status_message = Some(format!("Jumped to {:.1}s", target));
}
AppCommand::AddMarkerAtCurrentPosition { created_at } => {
    self.add_marker_at_current_position(created_at);
}
AppCommand::RemoveMarker(id) => {
    self.state.markers.retain(|marker| marker.id != id);
    self.state.status_message = Some("Marker removed".into());
}
AppCommand::SeekToChapter(index) => {
    if let Some(chapter) = self.state.chapters.get(index) {
        let target = self.clamp_seek_target(chapter.time_seconds)?;
        self.backend.send(BackendCommand::SeekAbsolute(target)).map_err(AppError::Message)?;
    }
}
AppCommand::SeekToMarker(id) => {
    if let Some(marker) = self.state.markers.iter().find(|marker| marker.id == id) {
        let target = self.clamp_seek_target(marker.time_seconds)?;
        self.backend.send(BackendCommand::SeekAbsolute(target)).map_err(AppError::Message)?;
    }
}
AppCommand::SeekToNextChapterOrMarker => {
    if let Some(target) = self
        .chapter_marker_points()
        .into_iter()
        .find(|point| *point > self.state.position_seconds + 0.5)
    {
        self.backend.send(BackendCommand::SeekAbsolute(target)).map_err(AppError::Message)?;
    }
}
AppCommand::SeekToPreviousChapterOrMarker => {
    if let Some(target) = self
        .chapter_marker_points()
        .into_iter()
        .rev()
        .find(|point| *point < self.state.position_seconds - 0.5)
    {
        self.backend.send(BackendCommand::SeekAbsolute(target)).map_err(AppError::Message)?;
    }
}
```

Add event branches in `poll_backend`:

```rust
BackendEvent::MutedChanged(muted) => self.state.muted = muted,
BackendEvent::ChaptersChanged(chapters) => {
    self.state.chapters = chapters
        .into_iter()
        .filter(|chapter| chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0)
        .collect();
    self.state
        .chapters
        .sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
}
```

- [ ] **Step 8: Implement shortcut actions and defaults**

Modify `crates/yoyo-core/src/shortcut.rs`.

Add enum variants:

```rust
ToggleMute,
JumpToTime,
AddMarker,
OpenActionPanel,
NextChapterOrMarker,
PreviousChapterOrMarker,
```

Replace `ShortcutAction::all()` array with 27 entries including the new variants at the end.

Add labels:

```rust
ShortcutAction::ToggleMute => "Toggle Mute",
ShortcutAction::JumpToTime => "Jump To Time",
ShortcutAction::AddMarker => "Add Marker",
ShortcutAction::OpenActionPanel => "Open Action Panel",
ShortcutAction::NextChapterOrMarker => "Next Chapter / Marker",
ShortcutAction::PreviousChapterOrMarker => "Previous Chapter / Marker",
```

Add default bindings:

```rust
bindings.insert(Shortcut("M".into()), ShortcutAction::ToggleMute);
bindings.insert(Shortcut("J".into()), ShortcutAction::JumpToTime);
bindings.insert(Shortcut("Ctrl+M".into()), ShortcutAction::AddMarker);
bindings.insert(Shortcut("P".into()), ShortcutAction::OpenActionPanel);
bindings.insert(Shortcut("Shift+Right".into()), ShortcutAction::NextChapterOrMarker);
bindings.insert(Shortcut("Shift+Left".into()), ShortcutAction::PreviousChapterOrMarker);
```

- [ ] **Step 9: Run core tests**

Run:

```powershell
cargo test -p yoyo-core --test session_contract
cargo test -p yoyo-core --test config_shortcut_contract
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```powershell
git add crates/yoyo-core/src/player_state.rs crates/yoyo-core/src/backend.rs crates/yoyo-core/src/app_command.rs crates/yoyo-core/src/session.rs crates/yoyo-core/src/shortcut.rs crates/yoyo-core/src/lib.rs crates/yoyo-core/tests/session_contract.rs crates/yoyo-core/tests/config_shortcut_contract.rs
git commit -m "feat: add cinema deck playback state"
```

Expected: Commit succeeds.

---

### Task 2: mpv Mute And Embedded Chapter Support

**Files:**
- Create: `crates/yoyo-mpv/src/chapter_list.rs`
- Modify: `crates/yoyo-mpv/src/client.rs`
- Modify: `crates/yoyo-mpv/src/event.rs`
- Modify: `crates/yoyo-mpv/src/translate.rs`
- Modify: `crates/yoyo-mpv/src/lib.rs`
- Modify: `crates/yoyo-mpv/tests/event_contract.rs`
- Modify: `crates/yoyo-mpv/tests/translate_contract.rs`

**Interfaces:**
- Consumes: `BackendCommand::SetMuted(bool)`
- Consumes: `BackendEvent::MutedChanged(bool)`
- Consumes: `BackendEvent::ChaptersChanged(Vec<MediaChapter>)`
- Produces: `MpvEvent::Muted(bool)`
- Produces: `MpvEvent::Chapters(Vec<MediaChapter>)`
- Produces: `chapter_list::normalize_chapters(Vec<MediaChapter>) -> Vec<MediaChapter>`

- [ ] **Step 1: Add failing translate and event tests**

Append to `crates/yoyo-mpv/tests/translate_contract.rs`:

```rust
#[test]
fn mute_translates_to_mpv_mute_property() {
    assert_eq!(
        translate_command(&BackendCommand::SetMuted(true)),
        vec![MpvAction::SetFlag { name: "mute".into(), value: true }]
    );
}
```

Append to `crates/yoyo-mpv/tests/event_contract.rs`:

```rust
#[test]
fn mute_event_maps_to_backend_muted_changed() {
    assert_eq!(map_event(MpvEvent::Muted(true)), Some(BackendEvent::MutedChanged(true)));
}

#[test]
fn chapter_event_maps_to_backend_chapters_changed() {
    let chapters = vec![yoyo_core::MediaChapter {
        title: Some("Intro".into()),
        time_seconds: 0.0,
    }];

    assert_eq!(
        map_event(MpvEvent::Chapters(chapters.clone())),
        Some(BackendEvent::ChaptersChanged(chapters))
    );
}
```

- [ ] **Step 2: Add chapter normalization unit tests**

Create `crates/yoyo-mpv/src/chapter_list.rs` with this test module and empty public function stub:

```rust
use yoyo_core::MediaChapter;

pub(crate) fn normalize_chapters(chapters: Vec<MediaChapter>) -> Vec<MediaChapter> {
    chapters
}

#[cfg(test)]
mod tests {
    use super::normalize_chapters;
    use yoyo_core::MediaChapter;

    #[test]
    fn normalize_chapters_sorts_skips_negative_and_generates_titles() {
        let chapters = normalize_chapters(vec![
            MediaChapter { title: None, time_seconds: 20.0 },
            MediaChapter { title: Some("Bad".into()), time_seconds: -1.0 },
            MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 },
        ]);

        assert_eq!(
            chapters,
            vec![
                MediaChapter { title: Some("Intro".into()), time_seconds: 0.0 },
                MediaChapter { title: Some("Chapter 2".into()), time_seconds: 20.0 },
            ]
        );
    }
}
```

- [ ] **Step 3: Run failing mpv tests**

Run:

```powershell
cargo test -p yoyo-mpv --test translate_contract mute_translates_to_mpv_mute_property
cargo test -p yoyo-mpv --test event_contract mute_event_maps_to_backend_muted_changed
cargo test -p yoyo-mpv chapter_list::tests::normalize_chapters_sorts_skips_negative_and_generates_titles
```

Expected: FAIL because event variants and translation support do not exist, and chapter normalization is only a stub.

- [ ] **Step 4: Implement mute translation and event mapping**

Modify `crates/yoyo-mpv/src/translate.rs`:

```rust
BackendCommand::SetMuted(muted) => {
    vec![MpvAction::SetFlag { name: "mute".into(), value: *muted }]
}
```

Modify `crates/yoyo-mpv/src/event.rs` imports:

```rust
use yoyo_core::{BackendEvent, MediaChapter, MediaTrack};
```

Add variants:

```rust
Muted(bool),
Chapters(Vec<MediaChapter>),
```

Add mapping:

```rust
MpvEvent::Muted(value) => Some(BackendEvent::MutedChanged(value)),
MpvEvent::Chapters(chapters) => Some(BackendEvent::ChaptersChanged(chapters)),
```

- [ ] **Step 5: Implement chapter normalization and runtime decoder**

Replace `crates/yoyo-mpv/src/chapter_list.rs` with:

```rust
use yoyo_core::MediaChapter;

pub(crate) fn normalize_chapters(chapters: Vec<MediaChapter>) -> Vec<MediaChapter> {
    let mut chapters = chapters
        .into_iter()
        .filter(|chapter| chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0)
        .collect::<Vec<_>>();
    chapters.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
    for (index, chapter) in chapters.iter_mut().enumerate() {
        if chapter.title.as_deref().unwrap_or("").trim().is_empty() {
            chapter.title = Some(format!("Chapter {}", index + 1));
        }
    }
    chapters
}

#[cfg(feature = "mpv-runtime")]
pub(crate) fn decode_chapter_list_property(
    property: &libmpv_sys::mpv_event_property,
) -> Option<crate::MpvEvent> {
    if property.data.is_null() {
        return None;
    }
    let node = unsafe { &*(property.data as *const libmpv_sys::mpv_node) };
    Some(crate::MpvEvent::Chapters(normalize_chapters(decode_chapters(node)?)))
}

#[cfg(feature = "mpv-runtime")]
fn decode_chapters(node: &libmpv_sys::mpv_node) -> Option<Vec<MediaChapter>> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_NODE_ARRAY {
        return None;
    }
    let list = unsafe { node.u.list.as_ref()? };
    let mut chapters = Vec::new();
    for index in 0..list.num {
        let entry = unsafe { list.values.add(index as usize).as_ref()? };
        if let Some(chapter) = decode_chapter(entry) {
            chapters.push(chapter);
        }
    }
    Some(chapters)
}

#[cfg(feature = "mpv-runtime")]
fn decode_chapter(node: &libmpv_sys::mpv_node) -> Option<MediaChapter> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_NODE_MAP {
        return None;
    }
    let map = unsafe { node.u.list.as_ref()? };
    let mut title = None;
    let mut time_seconds = None;
    for index in 0..map.num {
        let key_ptr = unsafe { map.keys.add(index as usize).as_ref()? };
        let key = unsafe { std::ffi::CStr::from_ptr(*key_ptr) }.to_string_lossy();
        let value = unsafe { map.values.add(index as usize).as_ref()? };
        match key.as_ref() {
            "title" => title = decode_string(value),
            "time" => time_seconds = decode_f64(value),
            _ => {}
        }
    }
    Some(MediaChapter { title, time_seconds: time_seconds? })
}

#[cfg(feature = "mpv-runtime")]
fn decode_string(node: &libmpv_sys::mpv_node) -> Option<String> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_STRING {
        return None;
    }
    Some(unsafe { std::ffi::CStr::from_ptr(node.u.string) }.to_string_lossy().into_owned())
}

#[cfg(feature = "mpv-runtime")]
fn decode_f64(node: &libmpv_sys::mpv_node) -> Option<f64> {
    match node.format {
        libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE => Some(unsafe { node.u.double_ }),
        libmpv_sys::mpv_format_MPV_FORMAT_INT64 => Some(unsafe { node.u.int64 as f64 }),
        _ => None,
    }
}
```

Keep the test module from Step 2 at the bottom of the file.

- [ ] **Step 6: Wire chapter module and observed properties**

Modify `crates/yoyo-mpv/src/lib.rs`:

```rust
mod chapter_list;
```

Modify `MpvClient::observe_default_properties` in `crates/yoyo-mpv/src/client.rs`:

```rust
self.observe_property(12, "mute", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
self.observe_property(13, "chapter-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE)?;
```

Modify `decode_property_event`:

```rust
("mute", libmpv_sys::mpv_format_MPV_FORMAT_FLAG) => {
    let value = unsafe { *(property.data as *const std::os::raw::c_int) };
    Some(MpvEvent::Muted(value != 0))
}
("chapter-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE) => {
    crate::chapter_list::decode_chapter_list_property(property)
}
```

- [ ] **Step 7: Run mpv tests**

Run:

```powershell
cargo test -p yoyo-mpv --test translate_contract
cargo test -p yoyo-mpv --test event_contract
cargo test -p yoyo-mpv chapter_list
cargo check -p yoyo-mpv --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add crates/yoyo-mpv/src/chapter_list.rs crates/yoyo-mpv/src/client.rs crates/yoyo-mpv/src/event.rs crates/yoyo-mpv/src/translate.rs crates/yoyo-mpv/src/lib.rs crates/yoyo-mpv/tests/event_contract.rs crates/yoyo-mpv/tests/translate_contract.rs
git commit -m "feat: read mpv mute and chapters"
```

Expected: Commit succeeds.

---

### Task 3: Desktop Marker Store, Progress Presenter, And OSD Presenter

**Files:**
- Create: `apps/yoyovideo-desktop/src/platform/markers.rs`
- Modify: `apps/yoyovideo-desktop/src/platform/mod.rs`
- Create: `apps/yoyovideo-desktop/src/progress.rs`
- Create: `apps/yoyovideo-desktop/src/osd.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Create: `apps/yoyovideo-desktop/tests/marker_store_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/presenter_contract.rs`

**Interfaces:**
- Produces: `MarkerStore`, `MediaMarkerSet`, `marker_store_path`
- Produces: `parse_jump_time(input: &str) -> Result<f64, String>`
- Produces: `build_navigation_rows(chapters: &[MediaChapter], markers: &[MediaMarker]) -> Vec<NavigationRow>`
- Produces: `build_progress_ticks(chapters: &[MediaChapter], markers: &[MediaMarker], duration: Option<f64>) -> Vec<ProgressTick>`
- Produces: `format_preview_label(seconds: f64, nearest_label: Option<&str>) -> String`
- Produces: `OsdState`, `OsdKind`, `format_osd_message`

- [ ] **Step 1: Add failing marker store tests**

Create `apps/yoyovideo-desktop/tests/marker_store_contract.rs`:

```rust
use tempfile::tempdir;
use yoyo_core::MediaMarker;
use yoyovideo_desktop::platform::{MarkerStore, marker_store_path};

fn marker(id: &str, seconds: f64) -> MediaMarker {
    MediaMarker {
        id: id.into(),
        title: format!("Marker {seconds}"),
        time_seconds: seconds,
        created_at: "2026-07-06T10:00:00+08:00".into(),
    }
}

#[test]
fn marker_store_round_trips_sorted_markers() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("markers.toml");
    let mut store = MarkerStore::with_path(Some(path.clone()));
    store.set_markers("file:movie.mp4".into(), vec![marker("b", 20.0), marker("a", 5.0)]);

    store.save().unwrap();
    let loaded = MarkerStore::load(Some(path)).unwrap();

    assert_eq!(
        loaded.markers_for("file:movie.mp4"),
        vec![marker("a", 5.0), marker("b", 20.0)]
    );
}

#[test]
fn marker_store_missing_and_corrupt_files_load_empty() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing.toml");
    assert!(MarkerStore::load(Some(missing)).unwrap().items.is_empty());

    let corrupt = dir.path().join("corrupt.toml");
    std::fs::write(&corrupt, "not valid toml").unwrap();
    assert!(MarkerStore::load(Some(corrupt)).unwrap().items.is_empty());
}

#[test]
fn marker_store_path_uses_app_data_when_paths_exist() {
    assert!(marker_store_path(None).is_none());
}
```

- [ ] **Step 2: Add failing presenter tests**

Append to `apps/yoyovideo-desktop/tests/presenter_contract.rs`:

```rust
#[test]
fn parse_jump_time_accepts_seconds_minutes_and_hours() {
    assert_eq!(yoyovideo_desktop::parse_jump_time("75").unwrap(), 75.0);
    assert_eq!(yoyovideo_desktop::parse_jump_time("01:15").unwrap(), 75.0);
    assert_eq!(yoyovideo_desktop::parse_jump_time("1:02:03.5").unwrap(), 3723.5);
    assert!(yoyovideo_desktop::parse_jump_time("1:2:3:4").is_err());
    assert!(yoyovideo_desktop::parse_jump_time("-1").is_err());
}

#[test]
fn progress_ticks_and_preview_labels_are_stable() {
    let chapters = vec![yoyo_core::MediaChapter { title: Some("Intro".into()), time_seconds: 10.0 }];
    let markers = vec![yoyo_core::MediaMarker {
        id: "m1".into(),
        title: "Marker 00:20".into(),
        time_seconds: 20.0,
        created_at: "2026-07-06T10:00:00+08:00".into(),
    }];

    let ticks = yoyovideo_desktop::build_progress_ticks(&chapters, &markers, Some(100.0));
    assert_eq!(ticks.len(), 2);
    assert_eq!(ticks[0].kind, yoyovideo_desktop::ProgressTickKind::Chapter);
    assert_eq!(ticks[0].percent, 0.1);
    assert_eq!(
        yoyovideo_desktop::format_preview_label(20.0, Some("Marker 00:20")),
        "00:20 - Marker 00:20"
    );
}

#[test]
fn osd_message_formats_common_actions() {
    assert_eq!(
        yoyovideo_desktop::format_osd_message(yoyovideo_desktop::OsdKind::Muted(true)),
        "Muted"
    );
    assert_eq!(
        yoyovideo_desktop::format_osd_message(yoyovideo_desktop::OsdKind::JumpedTo(75.0)),
        "Jumped to 01:15"
    );
}
```

- [ ] **Step 3: Run failing desktop presenter tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test marker_store_contract
cargo test -p yoyovideo-desktop --test presenter_contract parse_jump_time_accepts_seconds_minutes_and_hours
```

Expected: FAIL because marker store and presenter helpers do not exist.

- [ ] **Step 4: Implement marker store**

Create `apps/yoyovideo-desktop/src/platform/markers.rs`:

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::{MediaMarker, StorageError};

use super::AppPaths;

pub const MAX_MARKER_SETS: usize = 200;
pub const MAX_MARKERS_PER_MEDIA: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaMarkerSet {
    pub locator_key: String,
    pub markers: Vec<MediaMarker>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MarkerStore {
    #[serde(skip)]
    path: Option<PathBuf>,
    pub items: Vec<MediaMarkerSet>,
}

impl MarkerStore {
    pub fn with_path(path: Option<PathBuf>) -> Self {
        Self { path, items: Vec::new() }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let Some(path_value) = path.clone() else {
            return Ok(Self::with_path(None));
        };
        if !path_value.exists() {
            return Ok(Self::with_path(Some(path_value)));
        }
        let raw = fs::read_to_string(&path_value)?;
        let mut store = toml::from_str::<MarkerStore>(&raw).unwrap_or_default();
        store.path = Some(path_value);
        store.normalize();
        Ok(store)
    }

    pub fn markers_for(&self, locator_key: &str) -> Vec<MediaMarker> {
        self.items
            .iter()
            .find(|set| set.locator_key == locator_key)
            .map(|set| set.markers.clone())
            .unwrap_or_default()
    }

    pub fn set_markers(&mut self, locator_key: String, markers: Vec<MediaMarker>) {
        let mut markers = markers
            .into_iter()
            .filter(|marker| marker.time_seconds.is_finite() && marker.time_seconds >= 0.0)
            .collect::<Vec<_>>();
        markers.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
        markers.truncate(MAX_MARKERS_PER_MEDIA);
        self.items.retain(|set| set.locator_key != locator_key);
        self.items.insert(0, MediaMarkerSet { locator_key, markers });
        self.items.truncate(MAX_MARKER_SETS);
    }

    pub fn save(&self) -> Result<(), StorageError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }

    fn normalize(&mut self) {
        let mut normalized = Vec::new();
        for mut item in self.items.drain(..) {
            item.markers.retain(|marker| marker.time_seconds.is_finite() && marker.time_seconds >= 0.0);
            item.markers.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
            item.markers.truncate(MAX_MARKERS_PER_MEDIA);
            if !item.locator_key.trim().is_empty() {
                normalized.push(item);
            }
        }
        normalized.truncate(MAX_MARKER_SETS);
        self.items = normalized;
    }
}

pub fn marker_store_path(paths: Option<&AppPaths>) -> Option<PathBuf> {
    paths.map(|paths| paths.data_dir.join("markers.toml"))
}
```

Modify `apps/yoyovideo-desktop/src/platform/mod.rs`:

```rust
mod markers;
pub use markers::{
    MAX_MARKER_SETS, MAX_MARKERS_PER_MEDIA, MarkerStore, MediaMarkerSet, marker_store_path,
};
```

- [ ] **Step 5: Implement progress helpers**

Create `apps/yoyovideo-desktop/src/progress.rs`:

```rust
use yoyo_core::{MediaChapter, MediaMarker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressTickKind {
    Chapter,
    Marker,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressTick {
    pub percent: f32,
    pub label: String,
    pub kind: ProgressTickKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationRow {
    pub title: String,
    pub subtitle: String,
    pub seconds: f64,
    pub is_marker: bool,
    pub id: String,
}

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

pub fn parse_jump_time(input: &str) -> Result<f64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a time".into());
    }
    let parts = trimmed.split(':').collect::<Vec<_>>();
    if parts.len() > 3 {
        return Err("Use ss, mm:ss, or hh:mm:ss".into());
    }
    let mut total = 0.0;
    for part in parts {
        if part.trim().is_empty() {
            return Err("Invalid time".into());
        }
        let value = part.parse::<f64>().map_err(|_| "Invalid time".to_string())?;
        if !value.is_finite() || value < 0.0 {
            return Err("Invalid time".into());
        }
        total = total * 60.0 + value;
    }
    Ok(total)
}

pub fn build_progress_ticks(
    chapters: &[MediaChapter],
    markers: &[MediaMarker],
    duration: Option<f64>,
) -> Vec<ProgressTick> {
    let Some(duration) = duration.filter(|duration| duration.is_finite() && *duration > 0.0) else {
        return Vec::new();
    };
    let mut ticks = Vec::new();
    for chapter in chapters {
        if chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0 {
            ticks.push(ProgressTick {
                percent: (chapter.time_seconds / duration).clamp(0.0, 1.0) as f32,
                label: chapter.title.clone().unwrap_or_else(|| "Chapter".into()),
                kind: ProgressTickKind::Chapter,
            });
        }
    }
    for marker in markers {
        if marker.time_seconds.is_finite() && marker.time_seconds >= 0.0 {
            ticks.push(ProgressTick {
                percent: (marker.time_seconds / duration).clamp(0.0, 1.0) as f32,
                label: marker.title.clone(),
                kind: ProgressTickKind::Marker,
            });
        }
    }
    ticks.sort_by(|left, right| left.percent.total_cmp(&right.percent));
    ticks
}

pub fn build_navigation_rows(
    chapters: &[MediaChapter],
    markers: &[MediaMarker],
) -> Vec<NavigationRow> {
    let mut rows = Vec::new();
    for (index, chapter) in chapters.iter().enumerate() {
        rows.push(NavigationRow {
            title: chapter.title.clone().unwrap_or_else(|| format!("Chapter {}", index + 1)),
            subtitle: fmt_clock(chapter.time_seconds),
            seconds: chapter.time_seconds,
            is_marker: false,
            id: index.to_string(),
        });
    }
    for marker in markers {
        rows.push(NavigationRow {
            title: marker.title.clone(),
            subtitle: fmt_clock(marker.time_seconds),
            seconds: marker.time_seconds,
            is_marker: true,
            id: marker.id.clone(),
        });
    }
    rows.sort_by(|left, right| left.seconds.total_cmp(&right.seconds));
    rows
}

pub fn format_preview_label(seconds: f64, nearest_label: Option<&str>) -> String {
    match nearest_label {
        Some(label) if !label.trim().is_empty() => format!("{} - {}", fmt_clock(seconds), label),
        _ => fmt_clock(seconds),
    }
}
```

- [ ] **Step 6: Implement OSD presenter**

Create `apps/yoyovideo-desktop/src/osd.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum OsdKind {
    Muted(bool),
    JumpedTo(f64),
    SeekedTo(f64),
    Volume(u8),
    Speed(f32),
    MarkerAdded,
    MarkerRemoved,
    Chapter(String),
    Screenshot(String),
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OsdState {
    pub visible: bool,
    pub message: String,
    pub generation: u64,
}

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn format_osd_message(kind: OsdKind) -> String {
    match kind {
        OsdKind::Muted(true) => "Muted".into(),
        OsdKind::Muted(false) => "Sound On".into(),
        OsdKind::JumpedTo(seconds) => format!("Jumped to {}", fmt_clock(seconds)),
        OsdKind::SeekedTo(seconds) => format!("Seek {}", fmt_clock(seconds)),
        OsdKind::Volume(volume) => format!("Volume {volume}%"),
        OsdKind::Speed(speed) => format!("{speed:.2}x"),
        OsdKind::MarkerAdded => "Marker added".into(),
        OsdKind::MarkerRemoved => "Marker removed".into(),
        OsdKind::Chapter(title) => title,
        OsdKind::Screenshot(path) => format!("Screenshot saved: {path}"),
        OsdKind::Message(message) => message,
    }
}
```

- [ ] **Step 7: Export helpers**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
mod osd;
mod progress;
```

Add exports:

```rust
pub use osd::{OsdKind, OsdState, format_osd_message};
pub use progress::{
    NavigationRow, ProgressTick, ProgressTickKind, build_navigation_rows, build_progress_ticks,
    format_preview_label, parse_jump_time,
};
```

- [ ] **Step 8: Run desktop presenter and marker tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test marker_store_contract
cargo test -p yoyovideo-desktop --test presenter_contract
```

Expected: PASS.

- [ ] **Step 9: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/platform/markers.rs apps/yoyovideo-desktop/src/platform/mod.rs apps/yoyovideo-desktop/src/progress.rs apps/yoyovideo-desktop/src/osd.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/marker_store_contract.rs apps/yoyovideo-desktop/tests/presenter_contract.rs
git commit -m "feat: add marker and progress presenters"
```

Expected: Commit succeeds.

---

### Task 4: Slint Surface Contracts For Cinema Deck

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/video_tools_window_contract.rs`

**Interfaces:**
- Consumes: `NavigationRow`, `ProgressTick`, `ProgressTickKind`, `OsdState`
- Produces Slint structs: `ProgressTickRowData`, `NavigationRowData`
- Produces main-window properties: `muted`, `mute_label`, `osd_visible`, `osd_message`, `progress_preview_visible`, `progress_preview_label`, `progress_preview_value`, `progress_tick_rows`, `navigation_rows`, `action_panel_visible`, `jump_panel_visible`, `jump_input_text`
- Produces callbacks: `toggle_mute_requested`, `progress_preview_requested(float)`, `progress_commit_requested(float)`, `progress_preview_cleared`, `jump_panel_requested`, `jump_input_changed(string)`, `jump_commit_requested(string)`, `action_panel_requested`, `action_panel_close_requested`, `add_marker_requested`, `remove_marker_requested(string)`, `navigation_row_requested(int)`, `previous_chapter_marker_requested`, `next_chapter_marker_requested`

- [ ] **Step 1: Add failing Slint surface tests**

Append to `apps/yoyovideo-desktop/tests/context_menu_contract.rs`:

```rust
#[test]
fn main_window_cinema_deck_surface_compiles() {
    let window = MainWindow::new().unwrap();

    window.set_muted(true);
    assert!(window.get_muted());
    window.set_mute_label("Muted".into());
    window.set_osd_visible(true);
    window.set_osd_message("Muted".into());
    window.set_progress_preview_visible(true);
    window.set_progress_preview_label("01:15".into());
    window.set_progress_preview_value(0.5);
    window.set_action_panel_visible(true);
    window.set_jump_panel_visible(true);
    window.set_jump_input_text("01:15".into());

    window.on_toggle_mute_requested(|| {});
    window.on_progress_preview_requested(|_| {});
    window.on_progress_commit_requested(|_| {});
    window.on_progress_preview_cleared(|| {});
    window.on_jump_panel_requested(|| {});
    window.on_jump_input_changed(|_| {});
    window.on_jump_commit_requested(|_| {});
    window.on_action_panel_requested(|| {});
    window.on_action_panel_close_requested(|| {});
    window.on_add_marker_requested(|| {});
    window.on_remove_marker_requested(|_| {});
    window.on_navigation_row_requested(|_| {});
    window.on_previous_chapter_marker_requested(|| {});
    window.on_next_chapter_marker_requested(|| {});
}
```

- [ ] **Step 2: Run failing Slint contract**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract main_window_cinema_deck_surface_compiles
```

Expected: FAIL because the properties and callbacks do not exist.

- [ ] **Step 3: Add Slint structs, properties, and callbacks**

Modify `apps/yoyovideo-desktop/ui/main-window.slint` near exported structs:

```slint
export struct ProgressTickRowData {
    percent: float,
    label: string,
    is_marker: bool,
}

export struct NavigationRowData {
    title: string,
    subtitle: string,
    id: string,
    is_marker: bool,
}
```

Add properties to `MainWindow`:

```slint
in-out property <bool> muted: false;
in-out property <string> mute_label: "Sound";
in-out property <bool> osd_visible: false;
in-out property <string> osd_message: "";
in-out property <bool> progress_preview_visible: false;
in-out property <string> progress_preview_label: "";
in-out property <float> progress_preview_value: 0;
in-out property <[ProgressTickRowData]> progress_tick_rows: [];
in-out property <[NavigationRowData]> navigation_rows: [];
in-out property <bool> action_panel_visible: false;
in-out property <bool> jump_panel_visible: false;
in-out property <string> jump_input_text: "";
```

Add callbacks to `MainWindow`:

```slint
callback toggle_mute_requested();
callback progress_preview_requested(float);
callback progress_commit_requested(float);
callback progress_preview_cleared();
callback jump_panel_requested();
callback jump_input_changed(string);
callback jump_commit_requested(string);
callback action_panel_requested();
callback action_panel_close_requested();
callback add_marker_requested();
callback remove_marker_requested(string);
callback navigation_row_requested(int);
callback previous_chapter_marker_requested();
callback next_chapter_marker_requested();
```

Add a temporary button block near existing controls so callbacks are reachable before the visual refactor:

```slint
Button { text: root.mute_label; clicked => { root.toggle_mute_requested(); } }
Button { text: "Jump"; clicked => { root.jump_panel_requested(); } }
Button { text: "Actions"; clicked => { root.action_panel_requested(); } }
Button { text: "Marker"; clicked => { root.add_marker_requested(); } }
Button { text: "Prev Nav"; clicked => { root.previous_chapter_marker_requested(); } }
Button { text: "Next Nav"; clicked => { root.next_chapter_marker_requested(); } }
```

- [ ] **Step 4: Add basic action and jump overlays**

Add inside `MainWindow` after `video_tools_popup`:

```slint
action_panel := PopupWindow {
    close-policy: close-on-click-outside;
    width: 420px;
    height: 620px;

    ScrollView {
        VerticalBox {
            padding: 14px;
            spacing: 8px;
            Text { text: "Quick Actions"; color: #f2f5f7; }
            Button { text: "Screenshot"; clicked => { root.screenshot_requested(); } }
            Button { text: "Previous Frame"; clicked => { root.frame_step_previous_requested(); } }
            Button { text: "Next Frame"; clicked => { root.frame_step_next_requested(); } }
            Button { text: "Jump To Time"; clicked => { root.jump_panel_requested(); } }
            Button { text: "Add Marker"; clicked => { root.add_marker_requested(); } }
            Text { text: "Chapters & Markers"; color: #f2f5f7; }
            if root.navigation_rows.length == 0: Text { text: "No chapters or markers"; color: #7d8790; }
            for row[idx] in root.navigation_rows: Button {
                text: row.title + "  " + row.subtitle;
                clicked => { root.navigation_row_requested(idx); }
            }
        }
    }
}

jump_panel := PopupWindow {
    close-policy: close-on-click-outside;
    width: 280px;
    height: 120px;

    VerticalBox {
        padding: 12px;
        spacing: 8px;
        Text { text: "Jump To Time"; color: #f2f5f7; }
        jump_input := LineEdit {
            text: root.jump_input_text;
            placeholder-text: "ss, mm:ss, hh:mm:ss";
            edited => { root.jump_input_changed(self.text); }
            accepted => { root.jump_commit_requested(self.text); jump_panel.close(); }
        }
        Button { text: "Go"; clicked => { root.jump_commit_requested(jump_input.text); jump_panel.close(); } }
    }
}
```

Temporarily show popups from new buttons:

```slint
Button { text: "Jump"; clicked => { jump_panel.show(); root.jump_panel_requested(); } }
Button { text: "Actions"; clicked => { action_panel.show(); root.action_panel_requested(); } }
```

- [ ] **Step 5: Run Slint contracts**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo test -p yoyovideo-desktop --test video_tools_window_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/context_menu_contract.rs apps/yoyovideo-desktop/tests/video_tools_window_contract.rs
git commit -m "feat: expose cinema deck ui surface"
```

Expected: Commit succeeds.

---

### Task 5: Desktop Runtime Wiring For Markers, Jump, Mute, Navigation, And OSD

**Files:**
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/src/keyboard.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Modify: `apps/yoyovideo-desktop/tests/shortcut_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/keyboard_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`

**Interfaces:**
- Consumes: Task 1 core commands and shortcuts.
- Consumes: Task 3 marker store, progress helpers, OSD helpers.
- Consumes: Task 4 Slint surface.
- Produces: runtime marker persistence and OSD refresh.
- Produces: shifted arrow keyboard gestures.

- [ ] **Step 1: Add failing keyboard and shortcut tests**

Append to `apps/yoyovideo-desktop/tests/keyboard_contract.rs`:

```rust
#[test]
fn keyboard_input_normalizes_shifted_chapter_marker_shortcuts() {
    assert_eq!(
        shortcut_gesture(KeyboardInput::named(NamedDesktopKey::Right).with_shift()),
        Some("Shift+Right".to_string())
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput::named(NamedDesktopKey::Left).with_shift()),
        Some("Shift+Left".to_string())
    );
}
```

Append to `apps/yoyovideo-desktop/tests/shortcut_contract.rs`:

```rust
#[test]
fn cinema_deck_shortcuts_resolve_to_dispatches() {
    let map = yoyo_core::ShortcutMap::default();

    assert_eq!(
        yoyovideo_desktop::dispatch_shortcut(&map, "M"),
        Some(yoyo_core::AppCommand::ToggleMute)
    );
    assert_eq!(
        yoyovideo_desktop::dispatch_shortcut(&map, "Ctrl+M"),
        Some(yoyo_core::AppCommand::AddMarkerAtCurrentPosition {
            created_at: "shortcut".into()
        })
    );
    assert_eq!(
        yoyovideo_desktop::dispatch_shortcut(&map, "Shift+Right"),
        Some(yoyo_core::AppCommand::SeekToNextChapterOrMarker)
    );
}
```

The `created_at` value for shortcut tests is deterministic. Runtime dispatch replaces it with the current local timestamp when handling real shortcut input.

- [ ] **Step 2: Run failing tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test keyboard_contract keyboard_input_normalizes_shifted_chapter_marker_shortcuts
cargo test -p yoyovideo-desktop --test shortcut_contract cinema_deck_shortcuts_resolve_to_dispatches
```

Expected: FAIL until dispatch support exists.

- [ ] **Step 3: Update shortcut dispatch**

Modify `apps/yoyovideo-desktop/src/app.rs` `ShortcutDispatch`:

```rust
pub enum ShortcutDispatch {
    Command(AppCommand),
    TakeScreenshot,
    OpenJumpPanel,
    OpenActionPanel,
    AddMarker,
}
```

Modify `resolve_shortcut`:

```rust
ShortcutAction::ToggleMute => Some(ShortcutDispatch::Command(AppCommand::ToggleMute)),
ShortcutAction::JumpToTime => Some(ShortcutDispatch::OpenJumpPanel),
ShortcutAction::AddMarker => Some(ShortcutDispatch::AddMarker),
ShortcutAction::OpenActionPanel => Some(ShortcutDispatch::OpenActionPanel),
ShortcutAction::NextChapterOrMarker => {
    Some(ShortcutDispatch::Command(AppCommand::SeekToNextChapterOrMarker))
}
ShortcutAction::PreviousChapterOrMarker => {
    Some(ShortcutDispatch::Command(AppCommand::SeekToPreviousChapterOrMarker))
}
```

Modify `dispatch_shortcut`:

```rust
ShortcutDispatch::AddMarker => Some(AppCommand::AddMarkerAtCurrentPosition {
    created_at: "shortcut".into(),
}),
ShortcutDispatch::OpenJumpPanel | ShortcutDispatch::OpenActionPanel => None,
```

- [ ] **Step 4: Add runtime marker store fields**

Modify `DesktopRuntime` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
marker_store: crate::platform::MarkerStore,
osd: crate::OsdState,
```

Update `DesktopRuntime::new` to accept and store `marker_store`.

Load marker store in `run()`:

```rust
let marker_store =
    crate::platform::MarkerStore::load(crate::platform::marker_store_path(paths.as_ref()))
        .unwrap_or_else(|_| crate::platform::MarkerStore::with_path(crate::platform::marker_store_path(paths.as_ref())));
```

- [ ] **Step 5: Add refresh helpers**

Add to `apps/yoyovideo-desktop/src/app.rs`:

```rust
fn current_locator_key(state: &PlayerState) -> Option<String> {
    state.current.as_ref().map(|locator| locator.as_label())
}

fn refresh_navigation_surfaces(window: &MainWindow, state: &PlayerState) {
    let rows = crate::build_navigation_rows(&state.chapters, &state.markers)
        .into_iter()
        .map(|row| NavigationRowData {
            title: row.title.into(),
            subtitle: row.subtitle.into(),
            id: row.id.into(),
            is_marker: row.is_marker,
        })
        .collect::<Vec<_>>();
    window.set_navigation_rows(model_from_vec(rows));

    let ticks = crate::build_progress_ticks(&state.chapters, &state.markers, state.duration_seconds)
        .into_iter()
        .map(|tick| ProgressTickRowData {
            percent: tick.percent,
            label: tick.label.into(),
            is_marker: tick.kind == crate::ProgressTickKind::Marker,
        })
        .collect::<Vec<_>>();
    window.set_progress_tick_rows(model_from_vec(ticks));
}

fn set_osd(window: &MainWindow, runtime: &mut DesktopRuntime, kind: crate::OsdKind) {
    runtime.osd.visible = true;
    runtime.osd.message = crate::format_osd_message(kind);
    runtime.osd.generation = runtime.osd.generation.saturating_add(1);
    window.set_osd_visible(true);
    window.set_osd_message(runtime.osd.message.clone().into());
}
```

Call `refresh_navigation_surfaces(&app, &state);` in the success path of `with_runtime_controller` after `refresh_tracks_popup`.

- [ ] **Step 6: Restore and persist markers around media changes**

In the success path after controller state is refreshed, add:

```rust
if let Some(locator_key) = current_locator_key(&state) {
    let markers = runtime.marker_store.markers_for(&locator_key);
    if markers != state.markers {
        if let Some(controller) = runtime.controller_mut() {
            controller.session_mut().set_markers(markers);
        }
    }
}
```

After marker add/remove commands, persist the current marker snapshot:

```rust
fn persist_current_markers(runtime: &mut DesktopRuntime, state: &PlayerState) {
    let Some(locator_key) = current_locator_key(state) else {
        return;
    };
    runtime.marker_store.set_markers(locator_key, state.markers.clone());
    if let Err(error) = runtime.marker_store.save() {
        runtime.record_diagnostic("WARN", format!("Marker store save failed: {error}"));
    }
}
```

- [ ] **Step 7: Wire Slint callbacks**

In `run()`, add callbacks:

```rust
app.on_toggle_mute_requested(command_callback(&app, &runtime, AppCommand::ToggleMute));

app.on_progress_preview_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |percent| {
        let Some(app) = app_handle.upgrade() else { return; };
        let runtime = runtime.borrow();
        let Some(controller) = runtime.controller() else { return; };
        let state = controller.session().state();
        let Some(duration) = state.duration_seconds else { return; };
        let seconds = duration * f64::from(percent.clamp(0.0, 1.0));
        app.set_progress_preview_visible(true);
        app.set_progress_preview_value(percent.clamp(0.0, 1.0));
        app.set_progress_preview_label(crate::format_preview_label(seconds, None).into());
    }
});

app.on_progress_commit_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |percent| {
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            let Some(duration) = controller.session().state().duration_seconds else {
                return Ok(());
            };
            controller.dispatch(AppCommand::SeekAbsolute(duration * f64::from(percent.clamp(0.0, 1.0))))
        });
    }
});

app.on_progress_preview_cleared({
    let app_handle = app.as_weak();
    move || {
        if let Some(app) = app_handle.upgrade() {
            app.set_progress_preview_visible(false);
        }
    }
});

app.on_jump_commit_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |input| match crate::parse_jump_time(input.as_str()) {
        Ok(seconds) => {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::JumpToTime(seconds))
            });
        }
        Err(message) => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(message.into());
            }
        }
    }
});

app.on_add_marker_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move || {
        let created_at = chrono::Local::now().to_rfc3339();
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::AddMarkerAtCurrentPosition { created_at })
        });
    }
});

app.on_remove_marker_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |id| {
        let id = id.to_string();
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::RemoveMarker(id))
        });
    }
});

app.on_previous_chapter_marker_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SeekToPreviousChapterOrMarker,
));
app.on_next_chapter_marker_requested(command_callback(
    &app,
    &runtime,
    AppCommand::SeekToNextChapterOrMarker,
));
```

Wire `navigation_row_requested` by looking up `navigation_rows` index from current state and dispatching `SeekToMarker(id)` or `SeekToChapter(index)`.

- [ ] **Step 8: Update keyboard shifted named keys**

`apps/yoyovideo-desktop/src/keyboard.rs` already includes `shift` in gestures. No key enum changes are required for shifted arrows because `NamedDesktopKey::Left` and `Right` already exist. Ensure tests from Step 1 pass.

- [ ] **Step 9: Run wiring tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
cargo test -p yoyovideo-desktop --test controller_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/src/keyboard.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/shortcut_contract.rs apps/yoyovideo-desktop/tests/keyboard_contract.rs apps/yoyovideo-desktop/tests/controller_contract.rs
git commit -m "feat: wire cinema deck interactions"
```

Expected: Commit succeeds.

---

### Task 6: Cinema Deck Visual Refactor

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes: Slint surface from Task 4.
- Produces: custom-styled main playback surface and action panel.

- [ ] **Step 1: Add visual smoke checklist entries**

Add under `## UX` in `docs/testing/manual-smoke-checklist.md`:

```markdown
- Launch and confirm the main player uses the Cinema Deck layout with a video-first surface and custom-styled primary controls.
- Open a video, drag the progress rail, and confirm preview updates while playback seeks only after release.
- Use `M`, `J`, `Ctrl+M`, `P`, `Shift+Right`, and `Shift+Left`, and confirm mute, jump, marker, action panel, and chapter/marker navigation work.
- Open media with embedded chapters and confirm chapter ticks appear on the progress rail and rows appear in the action panel.
- Add and remove a user marker, restart, reopen the same media, and confirm markers persist.
- Confirm OSD messages appear for mute, seek, jump, speed, marker, screenshot, and filter actions.
```

- [ ] **Step 2: Replace default main controls with custom components**

In `apps/yoyovideo-desktop/ui/main-window.slint`, keep `std-widgets` imports for `LineEdit`, `ScrollView`, and settings, but introduce custom components above `MainWindow`:

```slint
component DeckButton inherits Rectangle {
    in property <string> text;
    callback clicked();
    min-width: 42px;
    height: 34px;
    border-radius: 17px;
    background: touch.has-hover ? #243241 : #15202a;
    border-width: 1px;
    border-color: touch.has-hover ? #3ba9c9 : #2a3845;

    Text {
        text: root.text;
        color: #f2f7fa;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 12px;
        font-weight: 600;
    }

    touch := TouchArea {
        clicked => { root.clicked(); }
    }
}

component DeckLabel inherits Rectangle {
    in property <string> text;
    height: 28px;
    border-radius: 14px;
    background: #0d151d;
    Text {
        text: root.text;
        color: #b8c7d4;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 12px;
    }
}
```

Replace main `Button` controls in the playback area with `DeckButton` and `DeckLabel`. Keep Settings window `Button` usage unchanged in this task.

- [ ] **Step 3: Build custom progress rail**

Replace the main progress `Slider` with a `Rectangle` rail:

```slint
progress_rail := Rectangle {
    height: 28px;
    background: transparent;

    Rectangle {
        x: 0;
        y: 11px;
        width: parent.width;
        height: 6px;
        border-radius: 3px;
        background: #1d2a34;
    }

    Rectangle {
        x: 0;
        y: 11px;
        width: root.progress_value * parent.width;
        height: 6px;
        border-radius: 3px;
        background: #38bdf8;
    }

    for tick in root.progress_tick_rows: Rectangle {
        x: tick.percent * parent.width - 1px;
        y: 7px;
        width: tick.is_marker ? 4px : 2px;
        height: 14px;
        border-radius: 2px;
        background: tick.is_marker ? #f59e0b : #7dd3fc;
    }

    if root.progress_preview_visible: Rectangle {
        x: root.progress_preview_value * parent.width - 42px;
        y: -24px;
        width: 84px;
        height: 22px;
        border-radius: 11px;
        background: #08111acc;
        Text {
            text: root.progress_preview_label;
            color: #f8fafc;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-size: 11px;
        }
    }

    touch := TouchArea {
        clicked => {
            root.progress_commit_requested(self.mouse-x / progress_rail.width);
        }
    }
}
```

If `mouse-x` does not compile with Slint 1.17, replace preview/commit coordinate calculation with a fallback using `clicked` to commit `root.progress_value`, keep `progress_commit_requested(float)` for Rust-driven tests, and document hover preview as runtime best-effort in manual smoke. The implementation must pass `cargo check`.

- [ ] **Step 4: Add video overlay OSD**

Inside `video_area`, add:

```slint
if root.osd_visible: Rectangle {
    width: 220px;
    height: 54px;
    border-radius: 18px;
    background: #020817dd;
    border-width: 1px;
    border-color: #3ba9c966;
    x: (parent.width - self.width) / 2;
    y: (parent.height - self.height) / 2;

    Text {
        text: root.osd_message;
        color: #f8fafc;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 16px;
        font-weight: 700;
    }
}
```

- [ ] **Step 5: Restyle action panel and popups**

Restyle `action_panel`, `tracks_popup`, `video_tools_popup`, and `menu_popup` backgrounds to:

```slint
background: #08111f;
```

Use `DeckButton` inside the action panel for quick actions. Keep `LineEdit` for jump input because text input should use the native Slint widget.

- [ ] **Step 6: Run visual compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo check -p yoyovideo-desktop
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/tests/context_menu_contract.rs docs/testing/manual-smoke-checklist.md
git commit -m "feat: add cinema deck interface"
```

Expected: Commit succeeds.

---

### Task 7: Full Verification And Runtime Smoke

**Files:**
- Modify only if needed: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes all prior tasks.
- Produces release-ready verification signal.

- [ ] **Step 1: Run full Rust formatting and tests**

Run:

```powershell
cargo fmt --check
cargo test
```

Expected: PASS.

- [ ] **Step 2: Run runtime feature checks**

Run:

```powershell
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 3: Run package smoke**

Run:

```powershell
pwsh -NoProfile -File scripts/test-package-smoke.ps1
```

Expected: PASS.

- [ ] **Step 4: Run optional runtime smoke when Windows mpv is staged**

Run this only when `third_party/mpv/windows-x64/bin/mpv-2.dll` exists:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64 -TimeoutSeconds 8
```

Expected: PASS and output includes `runtime_smoke=ok`.

- [ ] **Step 5: Confirm git status**

Run:

```powershell
git status --short
```

Expected: no uncommitted files.

- [ ] **Step 6: Commit verification docs if changed**

If `docs/testing/manual-smoke-checklist.md` changed after Task 6, run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add cinema deck smoke checks"
```

Expected: Commit succeeds or no commit is needed because Task 6 already committed the manual smoke entries.

---

## Self-Review

**Spec coverage:** Task 1 covers core mute, jump, chapters, markers, marker dedupe, chapter/marker seeking, and shortcuts. Task 2 covers mpv mute and `chapter-list`. Task 3 covers marker persistence, jump parsing, progress ticks, navigation rows, and OSD presenters. Task 4 covers the Slint API surface. Task 5 wires runtime behavior, marker persistence, progress commit, jump, mute, navigation, and OSD. Task 6 applies the Cinema Deck visual refactor and manual smoke entries. Task 7 covers final verification.

**Placeholder scan:** The plan avoids red-flag placeholder wording and vague untestable steps. Each task names exact files, commands, expected outcomes, and commit messages.

**Type consistency:** `MediaChapter`, `MediaMarker`, `BackendCommand::SetMuted`, `BackendEvent::MutedChanged`, `BackendEvent::ChaptersChanged`, `AppCommand` variants, `MarkerStore`, `ProgressTickRowData`, `NavigationRowData`, `OsdState`, `OsdKind`, and Slint callback/property names are introduced before later tasks consume them.
