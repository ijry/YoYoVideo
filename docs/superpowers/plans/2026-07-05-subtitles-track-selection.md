# Subtitles And Track Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a popup-based `Tracks / Subtitles` control surface to YoYoVideo that supports embedded track selection, external subtitle loading, subtitle playback controls, and per-media subtitle preference restore.

**Architecture:** Add typed media-track and subtitle-playback state in `yoyo-core`, translate and observe the matching mpv properties in `yoyo-mpv`, and keep popup presentation plus per-media subtitle preference persistence in `yoyovideo-desktop`. The desktop runtime remains the orchestrator: it refreshes the Slint popup from observed backend state, remembers per-media subtitle preferences, and applies a best-effort restore once track enumeration is available.

**Tech Stack:** Rust 2024, Slint 1.17.0, `libmpv`, `rfd` 0.17, `tempfile`, existing `yoyo-core` / `yoyo-mpv` / `yoyovideo-desktop` crates, PowerShell verification commands.

## Global Constraints

- The primary control surface is a main-window popup panel, not a sidebar tab or dedicated settings window.
- Audio, subtitle, and video track selection are all modeled in `yoyo-core` instead of being desktop-only mpv passthrough actions.
- External subtitles join the same subtitle-track model instead of living in a separate parallel selection system.
- Per-media subtitle preferences are stored in a dedicated persistence file, not inside `history.json`.
- Preference restoration is best-effort. Failed restore steps are skipped individually and do not block playback.
- Runtime truth comes from observed backend state. Saved preferences are restore hints, not authoritative state.
- Support a subtitle `Off` state.
- Load external subtitle files from local disk.
- Expose subtitle controls for visibility, delay, scale, and vertical position.
- Persist subtitle and track preferences per media item using `MediaLocator` as the key.
- Popup controls do not use `Apply` or `OK`; they are live playback controls.
- Missing tracks or missing external subtitle files must fail gracefully without interrupting playback.
- Slint stays declarative and receives simple lists, scalar properties, and callbacks rather than raw mpv concepts or persistence logic.

---

## File Structure

- `crates/yoyo-core/src/player_state.rs`: add typed media-track and subtitle-playback state plus helper accessors used by desktop persistence and popup mapping.
- `crates/yoyo-core/src/backend.rs`: add typed backend commands and events for track selection, subtitle visibility, subtitle controls, and external subtitle loading.
- `crates/yoyo-core/src/app_command.rs`: add user-facing commands for popup interactions and restore execution.
- `crates/yoyo-core/src/session.rs`: reset track state on media switches, forward the new commands to the backend, and apply new events back onto `PlayerState`.
- `crates/yoyo-core/src/lib.rs`: export the new core types.
- `crates/yoyo-core/tests/track_state_contract.rs`: cover command forwarding, event application, selected-track helpers, and media-switch reset semantics.
- `crates/yoyo-mpv/src/track_list.rs`: keep mpv track-list decoding and grouping logic isolated from the rest of the client.
- `crates/yoyo-mpv/src/translate.rs`: translate the new backend commands into `libmpv` property writes and commands.
- `crates/yoyo-mpv/src/event.rs`: map new typed mpv events into `yoyo-core::BackendEvent`.
- `crates/yoyo-mpv/src/client.rs`: observe subtitle and track properties, decode track-list updates, and enqueue the new typed events.
- `crates/yoyo-mpv/src/lib.rs`: register the new helper module.
- `crates/yoyo-mpv/tests/translate_contract.rs`: verify new command translation.
- `crates/yoyo-mpv/tests/event_contract.rs`: verify new typed event mapping.
- `apps/yoyovideo-desktop/src/subtitle_prefs.rs`: own per-media subtitle preference persistence, restore-plan generation, and throttled flush behavior.
- `apps/yoyovideo-desktop/src/track_popup.rs`: own row mapping, label formatting, and popup-control derived values.
- `apps/yoyovideo-desktop/src/platform/dialogs.rs`: extend the dialog abstraction with a subtitle-file picker.
- `apps/yoyovideo-desktop/src/app.rs`: add popup refresh helpers, runtime subtitle-preference lifecycle, file-picker integration, and restore orchestration.
- `apps/yoyovideo-desktop/src/lib.rs`: export new popup/persistence helper types and generated Slint types needed by tests.
- `apps/yoyovideo-desktop/ui/main-window.slint`: add popup properties, row structs, callbacks, the `Tracks` button, and the popup surface itself.
- `apps/yoyovideo-desktop/tests/subtitle_prefs_contract.rs`: verify per-media persistence and restore-plan generation.
- `apps/yoyovideo-desktop/tests/track_popup_contract.rs`: verify row mapping, `Off` handling, and label formatting.
- `apps/yoyovideo-desktop/tests/main_window_tracks_contract.rs`: compile-level contract for the new Slint popup surface.
- `apps/yoyovideo-desktop/tests/controller_contract.rs`: verify the desktop controller forwards the new popup commands.
- `apps/yoyovideo-desktop/tests/subtitle_runtime_contract.rs`: verify the controller can mark subtitle-restore completion without issuing backend commands.
- `docs/testing/manual-smoke-checklist.md`: add popup and restore smoke coverage.

---

### Task 1: Core Track And Subtitle State

**Files:**
- Create: `crates/yoyo-core/tests/track_state_contract.rs`
- Modify: `crates/yoyo-core/src/player_state.rs`
- Modify: `crates/yoyo-core/src/backend.rs`
- Modify: `crates/yoyo-core/src/app_command.rs`
- Modify: `crates/yoyo-core/src/session.rs`
- Modify: `crates/yoyo-core/src/lib.rs`

**Interfaces:**
- Produces: `pub enum MediaTrackKind { Audio, Subtitle, Video }`
- Produces: `pub struct MediaTrack { pub id: i64, pub kind: MediaTrackKind, pub title: Option<String>, pub language: Option<String>, pub codec: Option<String>, pub source_path: Option<PathBuf>, pub external: bool, pub selected: bool }`
- Produces: `pub struct SubtitlePlaybackState { pub visible: bool, pub delay_seconds: f64, pub scale: f32, pub vertical_position_percent: u8, pub external_path: Option<PathBuf> }`
- Produces: `PlayerState::selected_audio_track_id(&self) -> Option<i64>`
- Produces: `PlayerState::selected_subtitle_track_id(&self) -> Option<i64>`
- Produces: `PlayerState::selected_video_track_id(&self) -> Option<i64>`
- Produces: `AppSession::set_subtitle_preferences_restored(&mut self, restored: bool)`
- Produces: `BackendCommand::{SelectAudioTrack(i64), SelectSubtitleTrack(i64), SelectVideoTrack(i64), SetSubtitleVisible(bool), LoadExternalSubtitle(PathBuf), SetSubtitleDelay(f64), SetSubtitleScale(f32), SetSubtitleVerticalPosition(u8)}`
- Produces: `BackendEvent::TracksChanged { audio: Vec<MediaTrack>, subtitles: Vec<MediaTrack>, video: Vec<MediaTrack> }`
- Produces: `BackendEvent::{SubtitleVisibilityChanged(bool), SubtitleDelayChanged(f64), SubtitleScaleChanged(f32), SubtitleVerticalPositionChanged(u8)}`
- Produces: `AppCommand::{SelectAudioTrack(i64), SelectSubtitleTrack(i64), SelectVideoTrack(i64), SetSubtitleVisible(bool), LoadExternalSubtitle(PathBuf), SetSubtitleDelay(f64), SetSubtitleScale(f32), SetSubtitleVerticalPosition(u8)}`

- [ ] **Step 1: Write the failing core state tests**

Create `crates/yoyo-core/tests/track_state_contract.rs`:

```rust
use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, BackendEvent, MediaLocator, MediaTrack,
    MediaTrackKind, PlayerBackend,
};

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
    pending_events: Vec<BackendEvent>,
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
        std::mem::take(&mut self.pending_events)
    }
}

fn track(
    id: i64,
    kind: MediaTrackKind,
    title: &str,
    selected: bool,
) -> MediaTrack {
    MediaTrack {
        id,
        kind,
        title: Some(title.into()),
        language: None,
        codec: None,
        source_path: None,
        external: false,
        selected,
    }
}

#[test]
fn selecting_subtitle_track_enables_subtitles_and_sends_backend_command() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.backend_mut().pending_events.push(BackendEvent::TracksChanged {
        audio: vec![],
        subtitles: vec![
            track(3, MediaTrackKind::Subtitle, "English", false),
            track(4, MediaTrackKind::Subtitle, "Commentary", false),
        ],
        video: vec![],
    });
    session.poll_backend().unwrap();

    session.handle_command(AppCommand::SelectSubtitleTrack(4)).unwrap();

    assert_eq!(
        session.backend().commands,
        vec![BackendCommand::SelectSubtitleTrack(4)]
    );
    assert!(session.state().subtitle.visible);
    assert_eq!(session.state().selected_subtitle_track_id(), Some(4));
}

#[test]
fn tracks_event_updates_selected_track_helpers_and_external_subtitle_path() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut external = track(8, MediaTrackKind::Subtitle, "external.ass", true);
    external.external = true;
    external.source_path = Some(PathBuf::from("D:/subs/external.ass"));

    session.backend_mut().pending_events.push(BackendEvent::TracksChanged {
        audio: vec![track(2, MediaTrackKind::Audio, "Japanese", true)],
        subtitles: vec![external.clone()],
        video: vec![track(1, MediaTrackKind::Video, "Main", true)],
    });
    session
        .backend_mut()
        .pending_events
        .push(BackendEvent::SubtitleVisibilityChanged(false));
    session
        .backend_mut()
        .pending_events
        .push(BackendEvent::SubtitleDelayChanged(1.25));

    session.poll_backend().unwrap();

    assert_eq!(session.state().selected_audio_track_id(), Some(2));
    assert_eq!(session.state().selected_subtitle_track_id(), Some(8));
    assert_eq!(session.state().selected_video_track_id(), Some(1));
    assert_eq!(
        session.state().subtitle.external_path,
        Some(PathBuf::from("D:/subs/external.ass"))
    );
    assert!(!session.state().subtitle.visible);
    assert_eq!(session.state().subtitle.delay_seconds, 1.25);
}

#[test]
fn opening_new_media_clears_track_cache_and_restore_flag() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session.backend_mut().pending_events.push(BackendEvent::TracksChanged {
        audio: vec![track(2, MediaTrackKind::Audio, "Japanese", true)],
        subtitles: vec![track(3, MediaTrackKind::Subtitle, "English", true)],
        video: vec![track(1, MediaTrackKind::Video, "Main", true)],
    });
    session.poll_backend().unwrap();
    session.set_subtitle_preferences_restored(true);

    session
        .handle_command(AppCommand::OpenFile(PathBuf::from("movie-2.mkv")))
        .unwrap();

    assert!(session.state().audio_tracks.is_empty());
    assert!(session.state().subtitle_tracks.is_empty());
    assert!(session.state().video_tracks.is_empty());
    assert!(!session.state().subtitle_preferences_restored);
    assert_eq!(
        session.backend().opened,
        vec![MediaLocator::File(PathBuf::from("movie-2.mkv"))]
    );
}
```

- [ ] **Step 2: Run the failing core state tests**

Run:

```powershell
cargo test -p yoyo-core --test track_state_contract
```

Expected: FAIL because `MediaTrack`, `SubtitlePlaybackState`, the new commands/events, and the restore helpers do not exist yet.

- [ ] **Step 3: Add typed media-track and subtitle-playback state**

Modify `crates/yoyo-core/src/player_state.rs`:

```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::MediaLocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaTrackKind {
    Audio,
    Subtitle,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaTrack {
    pub id: i64,
    pub kind: MediaTrackKind,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub source_path: Option<PathBuf>,
    pub external: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePlaybackState {
    pub visible: bool,
    pub delay_seconds: f64,
    pub scale: f32,
    pub vertical_position_percent: u8,
    pub external_path: Option<PathBuf>,
}

impl Default for SubtitlePlaybackState {
    fn default() -> Self {
        Self {
            visible: true,
            delay_seconds: 0.0,
            scale: 1.0,
            vertical_position_percent: 100,
            external_path: None,
        }
    }
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
    pub audio_tracks: Vec<MediaTrack>,
    pub subtitle_tracks: Vec<MediaTrack>,
    pub video_tracks: Vec<MediaTrack>,
    pub subtitle: SubtitlePlaybackState,
    pub subtitle_preferences_restored: bool,
}

impl PlayerState {
    pub fn selected_audio_track_id(&self) -> Option<i64> {
        self.audio_tracks.iter().find(|track| track.selected).map(|track| track.id)
    }

    pub fn selected_subtitle_track_id(&self) -> Option<i64> {
        self.subtitle_tracks.iter().find(|track| track.selected).map(|track| track.id)
    }

    pub fn selected_video_track_id(&self) -> Option<i64> {
        self.video_tracks.iter().find(|track| track.selected).map(|track| track.id)
    }
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
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            video_tracks: Vec::new(),
            subtitle: SubtitlePlaybackState::default(),
            subtitle_preferences_restored: false,
        }
    }
}
```

- [ ] **Step 4: Add the new commands, events, and session behavior**

Modify `crates/yoyo-core/src/backend.rs`, `crates/yoyo-core/src/app_command.rs`, `crates/yoyo-core/src/session.rs`, and `crates/yoyo-core/src/lib.rs`:

```rust
// crates/yoyo-core/src/backend.rs
use std::path::PathBuf;

use crate::{AudioChannelMode, MediaLocator, MediaTrack, Rotation};

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
    TracksChanged { audio: Vec<MediaTrack>, subtitles: Vec<MediaTrack>, video: Vec<MediaTrack> },
    SubtitleVisibilityChanged(bool),
    SubtitleDelayChanged(f64),
    SubtitleScaleChanged(f32),
    SubtitleVerticalPositionChanged(u8),
    Warning(String),
    Error(String),
    EndOfFile,
}

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
    SelectAudioTrack(i64),
    SelectSubtitleTrack(i64),
    SelectVideoTrack(i64),
    SetSubtitleVisible(bool),
    LoadExternalSubtitle(PathBuf),
    SetSubtitleDelay(f64),
    SetSubtitleScale(f32),
    SetSubtitleVerticalPosition(u8),
}

// crates/yoyo-core/src/session.rs
use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent, MediaLocator,
    MediaTrack, PlayerBackend, PlayerState, Playlist, PlaylistEntry, PlaylistSnapshot, Rotation,
    SubtitlePlaybackState,
};

impl<B: PlayerBackend> AppSession<B> {
    pub fn set_subtitle_preferences_restored(&mut self, restored: bool) {
        self.state.subtitle_preferences_restored = restored;
    }

    fn reset_track_state_for_new_media(&mut self) {
        self.state.audio_tracks.clear();
        self.state.subtitle_tracks.clear();
        self.state.video_tracks.clear();
        self.state.subtitle = SubtitlePlaybackState::default();
        self.state.subtitle_preferences_restored = false;
    }
}

fn mark_selected(tracks: &mut [MediaTrack], id: i64) {
    for track in tracks {
        track.selected = track.id == id;
    }
}

fn selected_external_subtitle_path(tracks: &[MediaTrack]) -> Option<std::path::PathBuf> {
    tracks
        .iter()
        .find(|track| track.selected && track.external)
        .and_then(|track| track.source_path.clone())
}

// In open_playlist_index() and open_single_locator():
// self.reset_track_state_for_new_media();

// In handle_command():
AppCommand::SelectAudioTrack(id) => {
    mark_selected(&mut self.state.audio_tracks, id);
    self.backend.send(BackendCommand::SelectAudioTrack(id)).map_err(AppError::Message)?;
}
AppCommand::SelectSubtitleTrack(id) => {
    mark_selected(&mut self.state.subtitle_tracks, id);
    self.state.subtitle.visible = true;
    self.state.subtitle.external_path = selected_external_subtitle_path(&self.state.subtitle_tracks);
    self.backend.send(BackendCommand::SelectSubtitleTrack(id)).map_err(AppError::Message)?;
}
AppCommand::SelectVideoTrack(id) => {
    mark_selected(&mut self.state.video_tracks, id);
    self.backend.send(BackendCommand::SelectVideoTrack(id)).map_err(AppError::Message)?;
}
AppCommand::SetSubtitleVisible(visible) => {
    self.state.subtitle.visible = visible;
    self.backend.send(BackendCommand::SetSubtitleVisible(visible)).map_err(AppError::Message)?;
}
AppCommand::LoadExternalSubtitle(path) => {
    self.backend.send(BackendCommand::LoadExternalSubtitle(path)).map_err(AppError::Message)?;
}
AppCommand::SetSubtitleDelay(delay) => {
    self.state.subtitle.delay_seconds = delay;
    self.backend.send(BackendCommand::SetSubtitleDelay(delay)).map_err(AppError::Message)?;
}
AppCommand::SetSubtitleScale(scale) => {
    self.state.subtitle.scale = scale;
    self.backend.send(BackendCommand::SetSubtitleScale(scale)).map_err(AppError::Message)?;
}
AppCommand::SetSubtitleVerticalPosition(position) => {
    self.state.subtitle.vertical_position_percent = position;
    self.backend
        .send(BackendCommand::SetSubtitleVerticalPosition(position))
        .map_err(AppError::Message)?;
}

// In poll_backend():
BackendEvent::TracksChanged { audio, subtitles, video } => {
    self.state.audio_tracks = audio;
    self.state.subtitle_tracks = subtitles;
    self.state.video_tracks = video;
    self.state.subtitle.external_path = selected_external_subtitle_path(&self.state.subtitle_tracks);
}
BackendEvent::SubtitleVisibilityChanged(visible) => self.state.subtitle.visible = visible,
BackendEvent::SubtitleDelayChanged(delay) => self.state.subtitle.delay_seconds = delay,
BackendEvent::SubtitleScaleChanged(scale) => self.state.subtitle.scale = scale,
BackendEvent::SubtitleVerticalPositionChanged(position) => {
    self.state.subtitle.vertical_position_percent = position;
}

// crates/yoyo-core/src/lib.rs
pub use player_state::{
    AudioChannelMode, LoopState, MediaTrack, MediaTrackKind, PlayerState, Rotation,
    SubtitlePlaybackState,
};
```

- [ ] **Step 5: Run the core tests again**

Run:

```powershell
cargo test -p yoyo-core --test track_state_contract
```

Expected: PASS. The core crate now owns typed track state, subtitle playback state, and the session semantics needed by the desktop popup and per-media restore flow.

- [ ] **Step 6: Commit**

Run:

```powershell
git add crates/yoyo-core/src/player_state.rs crates/yoyo-core/src/backend.rs crates/yoyo-core/src/app_command.rs crates/yoyo-core/src/session.rs crates/yoyo-core/src/lib.rs crates/yoyo-core/tests/track_state_contract.rs
git commit -m "feat: add core subtitle and track state"
```

Expected: Commit succeeds.

---

### Task 2: mpv Translation And Track-List Observation

**Files:**
- Create: `crates/yoyo-mpv/src/track_list.rs`
- Modify: `crates/yoyo-mpv/src/translate.rs`
- Modify: `crates/yoyo-mpv/src/event.rs`
- Modify: `crates/yoyo-mpv/src/client.rs`
- Modify: `crates/yoyo-mpv/src/lib.rs`
- Modify: `crates/yoyo-mpv/tests/translate_contract.rs`
- Modify: `crates/yoyo-mpv/tests/event_contract.rs`

**Interfaces:**
- Consumes: `yoyo_core::MediaTrack`
- Consumes: `BackendCommand::{SelectAudioTrack(i64), SelectSubtitleTrack(i64), SelectVideoTrack(i64), SetSubtitleVisible(bool), LoadExternalSubtitle(PathBuf), SetSubtitleDelay(f64), SetSubtitleScale(f32), SetSubtitleVerticalPosition(u8)}`
- Produces: `pub(crate) struct RawTrackEntry`
- Produces: `pub(crate) fn split_tracks(entries: &[RawTrackEntry]) -> (Vec<MediaTrack>, Vec<MediaTrack>, Vec<MediaTrack>)`
- Produces: `MpvEvent::Tracks { audio: Vec<MediaTrack>, subtitles: Vec<MediaTrack>, video: Vec<MediaTrack> }`
- Produces: `MpvEvent::{SubtitleVisible(bool), SubtitleDelay(f64), SubtitleScale(f32), SubtitlePosition(u8)}`

- [ ] **Step 1: Write the failing mpv translation and event tests**

Modify `crates/yoyo-mpv/tests/translate_contract.rs`, `crates/yoyo-mpv/tests/event_contract.rs`, and create `crates/yoyo-mpv/src/track_list.rs` with a failing unit test:

```rust
// crates/yoyo-mpv/tests/translate_contract.rs
use std::path::PathBuf;

use yoyo_core::{BackendCommand, MediaTrack, MediaTrackKind};
use yoyo_mpv::{MpvAction, translate_command};

#[test]
fn track_selection_and_subtitle_controls_translate_to_expected_properties() {
    assert_eq!(
        translate_command(&BackendCommand::SelectAudioTrack(2)),
        vec![MpvAction::SetInt { name: "aid".into(), value: 2 }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetSubtitleVisible(false)),
        vec![MpvAction::SetFlag { name: "sub-visibility".into(), value: false }]
    );
    assert_eq!(
        translate_command(&BackendCommand::SetSubtitleVerticalPosition(88)),
        vec![MpvAction::SetInt { name: "sub-pos".into(), value: 88 }]
    );
}

#[test]
fn external_subtitle_loading_uses_sub_add_select() {
    assert_eq!(
        translate_command(&BackendCommand::LoadExternalSubtitle(PathBuf::from("movie.ass"))),
        vec![MpvAction::Command(vec![
            "sub-add".into(),
            "movie.ass".into(),
            "select".into(),
        ])]
    );
}

// crates/yoyo-mpv/tests/event_contract.rs
use std::path::PathBuf;

use yoyo_core::{BackendEvent, MediaTrack, MediaTrackKind};
use yoyo_mpv::{MpvEvent, map_event};

#[test]
fn track_list_event_maps_to_backend_tracks_changed() {
    let audio = vec![MediaTrack {
        id: 2,
        kind: MediaTrackKind::Audio,
        title: Some("Japanese".into()),
        language: Some("jpn".into()),
        codec: Some("aac".into()),
        source_path: None,
        external: false,
        selected: true,
    }];

    assert_eq!(
        map_event(MpvEvent::Tracks { audio: audio.clone(), subtitles: vec![], video: vec![] }),
        Some(BackendEvent::TracksChanged { audio, subtitles: vec![], video: vec![] })
    );
}

#[test]
fn subtitle_scale_and_position_events_are_preserved() {
    assert_eq!(
        map_event(MpvEvent::SubtitleScale(1.25)),
        Some(BackendEvent::SubtitleScaleChanged(1.25))
    );
    assert_eq!(
        map_event(MpvEvent::SubtitlePosition(90)),
        Some(BackendEvent::SubtitleVerticalPositionChanged(90))
    );
}

// crates/yoyo-mpv/src/track_list.rs
use std::path::PathBuf;

use yoyo_core::{MediaTrack, MediaTrackKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTrackEntry {
    pub id: i64,
    pub kind: MediaTrackKind,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub source_path: Option<PathBuf>,
    pub external: bool,
    pub selected: bool,
}

pub(crate) fn split_tracks(
    _entries: &[RawTrackEntry],
) -> (Vec<MediaTrack>, Vec<MediaTrack>, Vec<MediaTrack>) {
    (Vec::new(), Vec::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::{RawTrackEntry, split_tracks};
    use yoyo_core::MediaTrackKind;

    #[test]
    fn split_tracks_groups_kinds_and_preserves_external_path() {
        let (audio, subtitles, video) = split_tracks(&[
            RawTrackEntry {
                id: 1,
                kind: MediaTrackKind::Video,
                title: Some("Main".into()),
                language: None,
                codec: Some("h264".into()),
                source_path: None,
                external: false,
                selected: true,
            },
            RawTrackEntry {
                id: 2,
                kind: MediaTrackKind::Audio,
                title: Some("Japanese".into()),
                language: Some("jpn".into()),
                codec: Some("aac".into()),
                source_path: None,
                external: false,
                selected: true,
            },
            RawTrackEntry {
                id: 3,
                kind: MediaTrackKind::Subtitle,
                title: Some("external.ass".into()),
                language: Some("eng".into()),
                codec: Some("ass".into()),
                source_path: Some("D:/subs/external.ass".into()),
                external: true,
                selected: true,
            },
        ]);

        assert_eq!(audio.len(), 1);
        assert_eq!(subtitles.len(), 1);
        assert_eq!(video.len(), 1);
        assert_eq!(subtitles[0].source_path.as_deref(), Some(std::path::Path::new("D:/subs/external.ass")));
        assert!(subtitles[0].external);
        assert!(subtitles[0].selected);
    }
}
```

- [ ] **Step 2: Run the failing mpv tests**

Run:

```powershell
cargo test -p yoyo-mpv --test translate_contract
cargo test -p yoyo-mpv --test event_contract
cargo test -p yoyo-mpv split_tracks_groups_kinds_and_preserves_external_path --lib
```

Expected: FAIL because the new mpv events, translation branches, and `split_tracks()` implementation do not exist yet.

- [ ] **Step 3: Implement the new mpv command translation and event mapping**

Modify `crates/yoyo-mpv/src/translate.rs` and `crates/yoyo-mpv/src/event.rs`:

```rust
// crates/yoyo-mpv/src/translate.rs
// Add these match arms to the existing translate_command() match:
BackendCommand::SelectAudioTrack(id) => {
    vec![MpvAction::SetInt { name: "aid".into(), value: *id }]
}
BackendCommand::SelectSubtitleTrack(id) => {
    vec![MpvAction::SetInt { name: "sid".into(), value: *id }]
}
BackendCommand::SelectVideoTrack(id) => {
    vec![MpvAction::SetInt { name: "vid".into(), value: *id }]
}
BackendCommand::SetSubtitleVisible(visible) => {
    vec![MpvAction::SetFlag { name: "sub-visibility".into(), value: *visible }]
}
BackendCommand::LoadExternalSubtitle(path) => vec![MpvAction::Command(vec![
    "sub-add".into(),
    path.display().to_string(),
    "select".into(),
])],
BackendCommand::SetSubtitleDelay(delay) => {
    vec![MpvAction::SetDouble { name: "sub-delay".into(), value: *delay }]
}
BackendCommand::SetSubtitleScale(scale) => {
    vec![MpvAction::SetDouble { name: "sub-scale".into(), value: f64::from(*scale) }]
}
BackendCommand::SetSubtitleVerticalPosition(position) => {
    vec![MpvAction::SetInt { name: "sub-pos".into(), value: i64::from(*position) }]
}

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
    Tracks { audio: Vec<yoyo_core::MediaTrack>, subtitles: Vec<yoyo_core::MediaTrack>, video: Vec<yoyo_core::MediaTrack> },
    SubtitleVisible(bool),
    SubtitleDelay(f64),
    SubtitleScale(f32),
    SubtitlePosition(u8),
    Warning(String),
    Error(String),
    EndFile,
}

pub fn map_event(event: MpvEvent) -> Option<BackendEvent> {
    match event {
        MpvEvent::Tracks { audio, subtitles, video } => {
            Some(BackendEvent::TracksChanged { audio, subtitles, video })
        }
        MpvEvent::SubtitleVisible(value) => Some(BackendEvent::SubtitleVisibilityChanged(value)),
        MpvEvent::SubtitleDelay(value) => Some(BackendEvent::SubtitleDelayChanged(value)),
        MpvEvent::SubtitleScale(value) => Some(BackendEvent::SubtitleScaleChanged(value)),
        MpvEvent::SubtitlePosition(value) => {
            Some(BackendEvent::SubtitleVerticalPositionChanged(value))
        }
        // keep the existing branches for Pause/Position/Duration/Speed/Volume/Rotation/Warning/Error/EndFile below this insertion
    }
}
```

- [ ] **Step 4: Implement track-list grouping and observe the new mpv properties**

Modify `crates/yoyo-mpv/src/track_list.rs`, `crates/yoyo-mpv/src/client.rs`, and `crates/yoyo-mpv/src/lib.rs`:

```rust
// crates/yoyo-mpv/src/track_list.rs
use yoyo_core::{MediaTrack, MediaTrackKind};

pub(crate) fn split_tracks(
    entries: &[RawTrackEntry],
) -> (Vec<MediaTrack>, Vec<MediaTrack>, Vec<MediaTrack>) {
    let mut audio = Vec::new();
    let mut subtitles = Vec::new();
    let mut video = Vec::new();

    for entry in entries {
        let track = MediaTrack {
            id: entry.id,
            kind: entry.kind,
            title: entry.title.clone(),
            language: entry.language.clone(),
            codec: entry.codec.clone(),
            source_path: entry.source_path.clone(),
            external: entry.external,
            selected: entry.selected,
        };

        match entry.kind {
            MediaTrackKind::Audio => audio.push(track),
            MediaTrackKind::Subtitle => subtitles.push(track),
            MediaTrackKind::Video => video.push(track),
        }
    }

    (audio, subtitles, video)
}

#[cfg(feature = "mpv-runtime")]
pub(crate) fn decode_track_list_property(
    property: &libmpv_sys::mpv_event_property,
) -> Option<crate::MpvEvent> {
    let node = unsafe { &*(property.data as *const libmpv_sys::mpv_node) };
    let entries = decode_raw_track_entries(node)?;
    let (audio, subtitles, video) = split_tracks(&entries);
    Some(crate::MpvEvent::Tracks { audio, subtitles, video })
}

#[cfg(feature = "mpv-runtime")]
fn decode_raw_track_entries(node: &libmpv_sys::mpv_node) -> Option<Vec<RawTrackEntry>> {
    let list = unsafe { &*(node.u.list) };
    let mut tracks = Vec::new();

    for index in 0..list.num {
        let entry = unsafe { &*list.values.add(index as usize) };
        tracks.push(decode_raw_track_entry(entry)?);
    }

    Some(tracks)
}

#[cfg(feature = "mpv-runtime")]
fn decode_raw_track_entry(node: &libmpv_sys::mpv_node) -> Option<RawTrackEntry> {
    let map = unsafe { &*(node.u.list) };
    let mut id = None;
    let mut kind = None;
    let mut title = None;
    let mut language = None;
    let mut codec = None;
    let mut source_path = None;
    let mut external = false;
    let mut selected = false;

    for index in 0..map.num {
        let key = unsafe { std::ffi::CStr::from_ptr(*map.keys.add(index as usize)) }
            .to_string_lossy()
            .into_owned();
        let value = unsafe { &*map.values.add(index as usize) };

        match key.as_str() {
            "id" => id = decode_i64(value),
            "type" => kind = decode_kind(value),
            "title" => title = decode_string(value),
            "lang" => language = decode_string(value),
            "codec" => codec = decode_string(value),
            "external-filename" => source_path = decode_string(value).map(PathBuf::from),
            "external" => external = decode_bool(value).unwrap_or(false),
            "selected" => selected = decode_bool(value).unwrap_or(false),
            _ => {}
        }
    }

    Some(RawTrackEntry {
        id: id?,
        kind: kind?,
        title,
        language,
        codec,
        source_path: source_path.clone(),
        external: external || source_path.is_some(),
        selected,
    })
}

#[cfg(feature = "mpv-runtime")]
fn decode_kind(node: &libmpv_sys::mpv_node) -> Option<MediaTrackKind> {
    match decode_string(node)?.as_str() {
        "audio" => Some(MediaTrackKind::Audio),
        "sub" => Some(MediaTrackKind::Subtitle),
        "video" => Some(MediaTrackKind::Video),
        _ => None,
    }
}

#[cfg(feature = "mpv-runtime")]
fn decode_string(node: &libmpv_sys::mpv_node) -> Option<String> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_STRING {
        return None;
    }
    let value = unsafe { std::ffi::CStr::from_ptr(node.u.string) };
    Some(value.to_string_lossy().into_owned())
}

#[cfg(feature = "mpv-runtime")]
fn decode_i64(node: &libmpv_sys::mpv_node) -> Option<i64> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_INT64 {
        return None;
    }
    Some(unsafe { node.u.int64 })
}

#[cfg(feature = "mpv-runtime")]
fn decode_bool(node: &libmpv_sys::mpv_node) -> Option<bool> {
    if node.format != libmpv_sys::mpv_format_MPV_FORMAT_FLAG {
        return None;
    }
    Some(unsafe { node.u.flag != 0 })
}

// crates/yoyo-mpv/src/client.rs
pub fn observe_default_properties(&mut self) -> Result<(), MpvError> {
    self.observe_property(1, "pause", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
    self.observe_property(2, "time-pos", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
    self.observe_property(3, "duration", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
    self.observe_property(4, "speed", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
    self.observe_property(5, "volume", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
    self.observe_property(6, "video-rotate", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
    self.observe_property(7, "track-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE)?;
    self.observe_property(8, "sub-visibility", libmpv_sys::mpv_format_MPV_FORMAT_FLAG)?;
    self.observe_property(9, "sub-delay", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
    self.observe_property(10, "sub-scale", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE)?;
    self.observe_property(11, "sub-pos", libmpv_sys::mpv_format_MPV_FORMAT_INT64)?;
    Ok(())
}

#[cfg(feature = "mpv-runtime")]
fn decode_property_event(property: &libmpv_sys::mpv_event_property) -> Option<MpvEvent> {
    let name = cstr_to_string(property.name)?;
    if property.data.is_null() {
        return None;
    }

    match (name.as_str(), property.format) {
        ("track-list", libmpv_sys::mpv_format_MPV_FORMAT_NODE) => {
            crate::track_list::decode_track_list_property(property)
        }
        ("sub-visibility", libmpv_sys::mpv_format_MPV_FORMAT_FLAG) => {
            let value = unsafe { *(property.data as *const std::os::raw::c_int) };
            Some(MpvEvent::SubtitleVisible(value != 0))
        }
        ("sub-delay", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::SubtitleDelay(value))
        }
        ("sub-scale", libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE) => {
            let value = unsafe { *(property.data as *const f64) };
            Some(MpvEvent::SubtitleScale(value as f32))
        }
        ("sub-pos", libmpv_sys::mpv_format_MPV_FORMAT_INT64) => {
            let value = unsafe { *(property.data as *const i64) };
            Some(MpvEvent::SubtitlePosition(value.clamp(0, 100) as u8))
        }
        // keep the existing decode branches unchanged
        _ => None,
    }
}

// crates/yoyo-mpv/src/lib.rs
mod track_list;
```

- [ ] **Step 5: Run the mpv tests and compile checks**

Run:

```powershell
cargo test -p yoyo-mpv --test translate_contract
cargo test -p yoyo-mpv --test event_contract
cargo check -p yoyo-mpv
cargo check -p yoyo-mpv --features mpv-runtime
```

Expected: PASS. The mpv layer now observes track-list and subtitle properties, translates the new commands, and emits typed backend events for the desktop popup.

- [ ] **Step 6: Commit**

Run:

```powershell
git add crates/yoyo-mpv/src/track_list.rs crates/yoyo-mpv/src/translate.rs crates/yoyo-mpv/src/event.rs crates/yoyo-mpv/src/client.rs crates/yoyo-mpv/src/lib.rs crates/yoyo-mpv/tests/translate_contract.rs crates/yoyo-mpv/tests/event_contract.rs
git commit -m "feat: add mpv subtitle and track plumbing"
```

Expected: Commit succeeds.

---

### Task 3: Subtitle Preference Runtime And Popup Mapping

**Files:**
- Create: `apps/yoyovideo-desktop/src/subtitle_prefs.rs`
- Create: `apps/yoyovideo-desktop/src/track_popup.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Create: `apps/yoyovideo-desktop/tests/subtitle_prefs_contract.rs`
- Create: `apps/yoyovideo-desktop/tests/track_popup_contract.rs`

**Interfaces:**
- Consumes: `PlayerState::selected_audio_track_id(&self) -> Option<i64>`
- Consumes: `PlayerState::selected_subtitle_track_id(&self) -> Option<i64>`
- Consumes: `PlayerState::selected_video_track_id(&self) -> Option<i64>`
- Produces: `pub enum SubtitlePrefsFlushReason { PeriodicTick, MediaSwitch, Shutdown }`
- Produces: `pub struct SubtitlePreferenceEntry`
- Produces: `pub struct SubtitleRestorePlan { pub commands: Vec<AppCommand> }`
- Produces: `pub enum SubtitleRestoreError { MissingExternalSubtitle(PathBuf) }`
- Produces: `pub struct SubtitlePrefsRuntime`
- Produces: `SubtitlePrefsRuntime::new(path: Option<PathBuf>, store: SubtitlePreferenceStore) -> Self`
- Produces: `SubtitlePrefsRuntime::load(path: Option<PathBuf>) -> Result<Self, StorageError>`
- Produces: `SubtitlePrefsRuntime::remember_from_state(&mut self, state: &PlayerState)`
- Produces: `SubtitlePrefsRuntime::restore_plan_for(&self, locator: &MediaLocator) -> Result<Option<SubtitleRestorePlan>, SubtitleRestoreError>`
- Produces: `SubtitlePrefsRuntime::flush_if_needed(&mut self, now: Duration, reason: SubtitlePrefsFlushReason) -> Result<bool, StorageError>`
- Produces: `pub struct TrackPopupRow { pub track_id: Option<i64>, pub label: String, pub is_selected: bool }`
- Produces: `build_audio_track_rows(state: &PlayerState) -> Vec<TrackPopupRow>`
- Produces: `build_subtitle_track_rows(state: &PlayerState) -> Vec<TrackPopupRow>`
- Produces: `build_video_track_rows(state: &PlayerState) -> Vec<TrackPopupRow>`
- Produces: `format_track_label(track: &MediaTrack) -> String`
- Produces: `format_subtitle_delay_label(delay_seconds: f64) -> String`
- Produces: `format_subtitle_scale_label(scale: f32) -> String`

- [ ] **Step 1: Write the failing desktop preference and popup tests**

Create `apps/yoyovideo-desktop/tests/subtitle_prefs_contract.rs` and `apps/yoyovideo-desktop/tests/track_popup_contract.rs`:

```rust
// apps/yoyovideo-desktop/tests/subtitle_prefs_contract.rs
use std::path::PathBuf;
use std::time::Duration;

use tempfile::tempdir;
use yoyo_core::{AppCommand, MediaLocator, MediaTrack, MediaTrackKind, PlayerState, SubtitlePlaybackState};
use yoyovideo_desktop::{
    SubtitlePrefsFlushReason, SubtitlePrefsRuntime, SubtitleRestoreError,
};

fn selected_track(id: i64, kind: MediaTrackKind, title: &str) -> MediaTrack {
    MediaTrack {
        id,
        kind,
        title: Some(title.into()),
        language: None,
        codec: None,
        source_path: None,
        external: false,
        selected: true,
    }
}

#[test]
fn subtitle_prefs_runtime_persists_restore_plan_for_a_media_item() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("subtitle_prefs.json");
    let mut runtime = SubtitlePrefsRuntime::load(Some(path.clone())).unwrap();

    let mut state = PlayerState::default();
    state.current = Some(MediaLocator::File(PathBuf::from("movie.mkv")));
    state.audio_tracks = vec![selected_track(2, MediaTrackKind::Audio, "Japanese")];
    state.subtitle_tracks = vec![selected_track(8, MediaTrackKind::Subtitle, "English")];
    state.video_tracks = vec![selected_track(1, MediaTrackKind::Video, "Main")];
    state.subtitle = SubtitlePlaybackState {
        visible: true,
        delay_seconds: 1.5,
        scale: 1.25,
        vertical_position_percent: 88,
        external_path: None,
    };

    runtime.remember_from_state(&state);
    assert!(runtime
        .flush_if_needed(Duration::from_secs(0), SubtitlePrefsFlushReason::MediaSwitch)
        .unwrap());

    let reloaded = SubtitlePrefsRuntime::load(Some(path)).unwrap();
    let plan = reloaded
        .restore_plan_for(&MediaLocator::File(PathBuf::from("movie.mkv")))
        .unwrap()
        .unwrap();

    assert_eq!(
        plan.commands,
        vec![
            AppCommand::SelectAudioTrack(2),
            AppCommand::SelectVideoTrack(1),
            AppCommand::SelectSubtitleTrack(8),
            AppCommand::SetSubtitleVisible(true),
            AppCommand::SetSubtitleDelay(1.5),
            AppCommand::SetSubtitleScale(1.25),
            AppCommand::SetSubtitleVerticalPosition(88),
        ]
    );
}

#[test]
fn missing_external_subtitle_file_returns_a_restore_error() {
    let mut runtime = SubtitlePrefsRuntime::load(None).unwrap();
    let mut state = PlayerState::default();
    state.current = Some(MediaLocator::File(PathBuf::from("movie.mkv")));
    state.subtitle.visible = true;
    state.subtitle.external_path = Some(PathBuf::from("Z:/missing/external.ass"));

    runtime.remember_from_state(&state);

    let error = runtime
        .restore_plan_for(&MediaLocator::File(PathBuf::from("movie.mkv")))
        .unwrap_err();

    assert_eq!(
        error,
        SubtitleRestoreError::MissingExternalSubtitle(PathBuf::from("Z:/missing/external.ass"))
    );
}

// apps/yoyovideo-desktop/tests/track_popup_contract.rs
use yoyo_core::{MediaTrack, MediaTrackKind, PlayerState};
use yoyovideo_desktop::{
    build_subtitle_track_rows, format_subtitle_delay_label, format_track_label,
};

fn track(
    id: i64,
    kind: MediaTrackKind,
    title: Option<&str>,
    language: Option<&str>,
    selected: bool,
) -> MediaTrack {
    MediaTrack {
        id,
        kind,
        title: title.map(str::to_string),
        language: language.map(str::to_string),
        codec: None,
        source_path: None,
        external: false,
        selected,
    }
}

#[test]
fn subtitle_rows_include_off_and_current_track_selection() {
    let mut state = PlayerState::default();
    state.subtitle.visible = true;
    state.subtitle_tracks = vec![
        track(3, MediaTrackKind::Subtitle, Some("English"), Some("eng"), true),
        track(4, MediaTrackKind::Subtitle, Some("Commentary"), None, false),
    ];

    let rows = build_subtitle_track_rows(&state);

    assert_eq!(rows[0].track_id, None);
    assert!(!rows[0].is_selected);
    assert_eq!(rows[1].track_id, Some(3));
    assert!(rows[1].is_selected);
}

#[test]
fn track_label_prefers_title_then_language_then_numeric_id() {
    assert_eq!(
        format_track_label(&track(8, MediaTrackKind::Audio, Some("Japanese"), Some("jpn"), true)),
        "Japanese (jpn)"
    );
    assert_eq!(
        format_track_label(&track(5, MediaTrackKind::Subtitle, None, Some("eng"), false)),
        "eng [#5]"
    );
    assert_eq!(format_subtitle_delay_label(-1.25), "-1.25s");
}
```

- [ ] **Step 2: Run the failing desktop preference and popup tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test subtitle_prefs_contract
cargo test -p yoyovideo-desktop --test track_popup_contract
```

Expected: FAIL because the subtitle-preference runtime and popup-mapping helpers do not exist yet.

- [ ] **Step 3: Implement per-media subtitle preference persistence**

Create `apps/yoyovideo-desktop/src/subtitle_prefs.rs`:

```rust
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use yoyo_core::{AppCommand, MediaLocator, PlayerState, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitlePrefsFlushReason {
    PeriodicTick,
    MediaSwitch,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleRestoreError {
    MissingExternalSubtitle(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePreferenceEntry {
    pub locator: MediaLocator,
    pub selected_audio_track_id: Option<i64>,
    pub selected_subtitle_track_id: Option<i64>,
    pub selected_video_track_id: Option<i64>,
    pub subtitle_visible: bool,
    pub external_subtitle_path: Option<PathBuf>,
    pub subtitle_delay_seconds: f64,
    pub subtitle_scale: f32,
    pub subtitle_vertical_position: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePreferenceStore {
    pub items: Vec<SubtitlePreferenceEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleRestorePlan {
    pub commands: Vec<AppCommand>,
}

pub struct SubtitlePrefsRuntime {
    path: Option<PathBuf>,
    store: SubtitlePreferenceStore,
    dirty: bool,
    last_flush_at: Option<Duration>,
}

impl SubtitlePrefsRuntime {
    pub fn new(path: Option<PathBuf>, store: SubtitlePreferenceStore) -> Self {
        Self { path, store, dirty: false, last_flush_at: None }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let store = match path.as_ref() {
            Some(path) if path.exists() => serde_json::from_str(&fs::read_to_string(path)?)?,
            _ => SubtitlePreferenceStore::default(),
        };
        Ok(Self::new(path, store))
    }

    pub fn remember_from_state(&mut self, state: &PlayerState) {
        let Some(locator) = state.current.clone() else {
            return;
        };

        let entry = SubtitlePreferenceEntry {
            locator: locator.clone(),
            selected_audio_track_id: state.selected_audio_track_id(),
            selected_subtitle_track_id: state.selected_subtitle_track_id(),
            selected_video_track_id: state.selected_video_track_id(),
            subtitle_visible: state.subtitle.visible,
            external_subtitle_path: state.subtitle.external_path.clone(),
            subtitle_delay_seconds: state.subtitle.delay_seconds,
            subtitle_scale: state.subtitle.scale,
            subtitle_vertical_position: state.subtitle.vertical_position_percent,
        };

        self.store.items.retain(|item| item.locator != locator);
        self.store.items.insert(0, entry);
        self.dirty = true;
    }

    pub fn restore_plan_for(
        &self,
        locator: &MediaLocator,
    ) -> Result<Option<SubtitleRestorePlan>, SubtitleRestoreError> {
        let Some(entry) = self.store.items.iter().find(|item| &item.locator == locator) else {
            return Ok(None);
        };

        if let Some(path) = &entry.external_subtitle_path {
            if !path.exists() {
                return Err(SubtitleRestoreError::MissingExternalSubtitle(path.clone()));
            }
        }

        let mut commands = Vec::new();
        if let Some(path) = &entry.external_subtitle_path {
            commands.push(AppCommand::LoadExternalSubtitle(path.clone()));
        }
        if let Some(id) = entry.selected_audio_track_id {
            commands.push(AppCommand::SelectAudioTrack(id));
        }
        if let Some(id) = entry.selected_video_track_id {
            commands.push(AppCommand::SelectVideoTrack(id));
        }
        if entry.external_subtitle_path.is_none() {
            if let Some(id) = entry.selected_subtitle_track_id {
                commands.push(AppCommand::SelectSubtitleTrack(id));
            }
        }
        commands.push(AppCommand::SetSubtitleVisible(entry.subtitle_visible));
        commands.push(AppCommand::SetSubtitleDelay(entry.subtitle_delay_seconds));
        commands.push(AppCommand::SetSubtitleScale(entry.subtitle_scale));
        commands.push(AppCommand::SetSubtitleVerticalPosition(
            entry.subtitle_vertical_position,
        ));

        Ok(Some(SubtitleRestorePlan { commands }))
    }

    pub fn flush_if_needed(
        &mut self,
        now: Duration,
        reason: SubtitlePrefsFlushReason,
    ) -> Result<bool, StorageError> {
        if !self.dirty {
            return Ok(false);
        }

        if matches!(reason, SubtitlePrefsFlushReason::PeriodicTick)
            && self.last_flush_at.is_some_and(|last| now < last + Duration::from_secs(2))
        {
            return Ok(false);
        }

        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, serde_json::to_string_pretty(&self.store)?)?;
        }

        self.dirty = false;
        self.last_flush_at = Some(now);
        Ok(true)
    }
}
```

- [ ] **Step 4: Implement popup row mapping and export the new helpers**

Create `apps/yoyovideo-desktop/src/track_popup.rs` and update `apps/yoyovideo-desktop/src/lib.rs`:

```rust
// apps/yoyovideo-desktop/src/track_popup.rs
use yoyo_core::{MediaTrack, PlayerState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPopupRow {
    pub track_id: Option<i64>,
    pub label: String,
    pub is_selected: bool,
}

pub fn format_track_label(track: &MediaTrack) -> String {
    match (&track.title, &track.language) {
        (Some(title), Some(language)) => format!("{title} ({language})"),
        (Some(title), None) => title.clone(),
        (None, Some(language)) => format!("{language} [#{}]", track.id),
        (None, None) => format!("Track #{}", track.id),
    }
}

pub fn build_audio_track_rows(state: &PlayerState) -> Vec<TrackPopupRow> {
    state
        .audio_tracks
        .iter()
        .map(|track| TrackPopupRow {
            track_id: Some(track.id),
            label: format_track_label(track),
            is_selected: track.selected,
        })
        .collect()
}

pub fn build_subtitle_track_rows(state: &PlayerState) -> Vec<TrackPopupRow> {
    let mut rows = vec![TrackPopupRow {
        track_id: None,
        label: "Off".into(),
        is_selected: !state.subtitle.visible,
    }];
    rows.extend(state.subtitle_tracks.iter().map(|track| TrackPopupRow {
        track_id: Some(track.id),
        label: format_track_label(track),
        is_selected: state.subtitle.visible && track.selected,
    }));
    rows
}

pub fn build_video_track_rows(state: &PlayerState) -> Vec<TrackPopupRow> {
    state
        .video_tracks
        .iter()
        .map(|track| TrackPopupRow {
            track_id: Some(track.id),
            label: format_track_label(track),
            is_selected: track.selected,
        })
        .collect()
}

pub fn format_subtitle_delay_label(delay_seconds: f64) -> String {
    format!("{delay_seconds:+.2}s")
}

pub fn format_subtitle_scale_label(scale: f32) -> String {
    format!("{:.0}%", scale * 100.0)
}

// apps/yoyovideo-desktop/src/lib.rs
mod subtitle_prefs;
mod track_popup;

pub use subtitle_prefs::{
    SubtitlePreferenceEntry, SubtitlePrefsFlushReason, SubtitlePrefsRuntime, SubtitleRestoreError,
    SubtitleRestorePlan,
};
pub use track_popup::{
    TrackPopupRow, build_audio_track_rows, build_subtitle_track_rows, build_video_track_rows,
    format_subtitle_delay_label, format_subtitle_scale_label, format_track_label,
};
```

- [ ] **Step 5: Run the desktop preference and popup tests again**

Run:

```powershell
cargo test -p yoyovideo-desktop --test subtitle_prefs_contract
cargo test -p yoyovideo-desktop --test track_popup_contract
```

Expected: PASS. The desktop crate now owns a pure per-media subtitle-preference runtime and popup mapping helpers without touching Slint or app wiring yet.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/subtitle_prefs.rs apps/yoyovideo-desktop/src/track_popup.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/subtitle_prefs_contract.rs apps/yoyovideo-desktop/tests/track_popup_contract.rs
git commit -m "feat: add subtitle preference runtime and popup mapping"
```

Expected: Commit succeeds.

---

### Task 4: Slint Popup Surface And Subtitle File Picker

**Files:**
- Create: `apps/yoyovideo-desktop/tests/main_window_tracks_contract.rs`
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/src/platform/dialogs.rs`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`

**Interfaces:**
- Consumes: `TrackPopupRow`
- Produces: exported Slint struct `TrackPopupRowData`
- Produces: `MainWindow::set_audio_track_rows(ModelRc<TrackPopupRowData>)`
- Produces: `MainWindow::set_subtitle_track_rows(ModelRc<TrackPopupRowData>)`
- Produces: `MainWindow::set_video_track_rows(ModelRc<TrackPopupRowData>)`
- Produces: `MainWindow::set_subtitle_visible(bool)`
- Produces: `MainWindow::set_subtitle_delay_value(f32)`
- Produces: `MainWindow::set_subtitle_scale_value(f32)`
- Produces: `MainWindow::set_subtitle_position_value(f32)`
- Produces: `DialogService::pick_subtitle_file(&self) -> Option<PathBuf>`

- [ ] **Step 1: Write the failing Slint popup contract test**

Create `apps/yoyovideo-desktop/tests/main_window_tracks_contract.rs`:

```rust
use slint::ModelRc;
use yoyovideo_desktop::{MainWindow, TrackPopupRowData};

#[test]
fn main_window_exports_track_popup_properties() {
    let constructor: fn() -> Result<MainWindow, slint::PlatformError> = MainWindow::new;
    let set_audio_rows: fn(&MainWindow, ModelRc<TrackPopupRowData>) =
        MainWindow::set_audio_track_rows;
    let set_subtitle_rows: fn(&MainWindow, ModelRc<TrackPopupRowData>) =
        MainWindow::set_subtitle_track_rows;
    let set_visible: fn(&MainWindow, bool) = MainWindow::set_subtitle_visible;
    let set_delay: fn(&MainWindow, f32) = MainWindow::set_subtitle_delay_value;
    let set_scale: fn(&MainWindow, f32) = MainWindow::set_subtitle_scale_value;
    let set_position: fn(&MainWindow, f32) = MainWindow::set_subtitle_position_value;
    let set_status: fn(&MainWindow, slint::SharedString) = MainWindow::set_tracks_status_label;
    let _ = (
        constructor,
        set_audio_rows,
        set_subtitle_rows,
        set_visible,
        set_delay,
        set_scale,
        set_position,
        set_status,
    );
}
```

- [ ] **Step 2: Run the failing Slint popup contract**

Run:

```powershell
cargo test -p yoyovideo-desktop --test main_window_tracks_contract
```

Expected: FAIL because the popup row struct, popup properties, and `MainWindow` exports do not exist yet.

- [ ] **Step 3: Add the popup surface and subtitle-file picker API**

Modify `apps/yoyovideo-desktop/ui/main-window.slint`, `apps/yoyovideo-desktop/src/platform/dialogs.rs`, and `apps/yoyovideo-desktop/src/lib.rs`:

```rust
// apps/yoyovideo-desktop/src/platform/dialogs.rs
use std::path::PathBuf;

pub trait DialogService {
    fn pick_file(&self) -> Option<PathBuf>;
    fn pick_folder(&self) -> Option<PathBuf>;
    fn pick_subtitle_file(&self) -> Option<PathBuf>;
    fn prompt_url(&self) -> Option<String>;
}

impl DialogService for RfdDialogService {
    fn pick_subtitle_file(&self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Subtitle", &["srt", "ass", "ssa", "sub"])
            .pick_file()
    }
}

// apps/yoyovideo-desktop/src/lib.rs
pub use app::{MainWindow, SettingsWindow, TrackPopupRowData, build_desktop_backend, build_desktop_backend_with_video_window, dispatch_shortcut, refresh_window, run};
```

Add this to `apps/yoyovideo-desktop/ui/main-window.slint`:

```slint
export struct TrackPopupRowData {
    label: string,
    selected: bool,
}

export component MainWindow inherits Window {
    in-out property <[TrackPopupRowData]> audio_track_rows: [];
    in-out property <[TrackPopupRowData]> subtitle_track_rows: [];
    in-out property <[TrackPopupRowData]> video_track_rows: [];
    in-out property <bool> subtitle_visible: true;
    in-out property <float> subtitle_delay_value: 0.0;
    in-out property <string> subtitle_delay_label: "+0.00s";
    in-out property <float> subtitle_scale_value: 1.0;
    in-out property <string> subtitle_scale_label: "100%";
    in-out property <float> subtitle_position_value: 100.0;
    in-out property <string> tracks_status_label: "";

    callback audio_track_requested(int);
    callback subtitle_track_requested(int);
    callback video_track_requested(int);
    callback load_external_subtitle_requested();
    callback subtitle_visible_changed(bool);
    callback subtitle_delay_changed(float);
    callback subtitle_scale_changed(float);
    callback subtitle_position_changed(float);

    tracks_popup := PopupWindow {
        close-policy: close-on-click-outside;
        width: 360px;
        height: 520px;

        ScrollView {
            VerticalBox {
                padding: 12px;
                spacing: 8px;

                Text { text: "Audio Tracks"; color: #f2f5f7; }
                for row[idx] in root.audio_track_rows: Button {
                    text: row.selected ? row.label + "  *" : row.label;
                    clicked => { root.audio_track_requested(idx); }
                }

                Text { text: "Subtitle Tracks"; color: #f2f5f7; }
                for row[idx] in root.subtitle_track_rows: Button {
                    text: row.selected ? row.label + "  *" : row.label;
                    clicked => { root.subtitle_track_requested(idx); }
                }

                Button {
                    text: "Load Subtitle...";
                    clicked => { root.load_external_subtitle_requested(); }
                }

                CheckBox {
                    text: "Subtitle Visible";
                    checked: root.subtitle_visible;
                    toggled => { root.subtitle_visible_changed(self.checked); }
                }

                Text { text: root.subtitle_delay_label; color: #c7d1d8; }
                Slider {
                    minimum: -10;
                    maximum: 10;
                    value: root.subtitle_delay_value;
                    changed(value) => { root.subtitle_delay_changed(value); }
                }

                Text { text: root.subtitle_scale_label; color: #c7d1d8; }
                Slider {
                    minimum: 0.5;
                    maximum: 2.0;
                    value: root.subtitle_scale_value;
                    changed(value) => { root.subtitle_scale_changed(value); }
                }

                Text { text: "Subtitle Position"; color: #c7d1d8; }
                Slider {
                    minimum: 0;
                    maximum: 100;
                    value: root.subtitle_position_value;
                    changed(value) => { root.subtitle_position_changed(value); }
                }

                Text { text: "Video Tracks"; color: #f2f5f7; }
                for row[idx] in root.video_track_rows: Button {
                    text: row.selected ? row.label + "  *" : row.label;
                    clicked => { root.video_track_requested(idx); }
                }

                Text { text: root.tracks_status_label; color: #7d8790; }
            }
        }
    }
}
```

Also add the `Tracks` button beside the existing control buttons:

```slint
Button { text: "Tracks"; clicked => { tracks_popup.show(); } }
```

- [ ] **Step 4: Run the popup contract and desktop compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test main_window_tracks_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS. The generated Slint types and popup properties now exist, and the desktop crate compiles with the popup surface and subtitle-file picker API.

- [ ] **Step 5: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/src/platform/dialogs.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/main_window_tracks_contract.rs
git commit -m "feat: add tracks popup surface"
```

Expected: Commit succeeds.

---

### Task 5: Desktop Runtime Wiring And Best-Effort Restore

**Files:**
- Create: `apps/yoyovideo-desktop/tests/subtitle_runtime_contract.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`

**Interfaces:**
- Consumes: `SubtitlePrefsRuntime::remember_from_state(&mut self, state: &PlayerState)`
- Consumes: `SubtitlePrefsRuntime::restore_plan_for(&self, locator: &MediaLocator) -> Result<Option<SubtitleRestorePlan>, SubtitleRestoreError>`
- Consumes: `build_audio_track_rows(state: &PlayerState) -> Vec<TrackPopupRow>`
- Consumes: `build_subtitle_track_rows(state: &PlayerState) -> Vec<TrackPopupRow>`
- Consumes: `build_video_track_rows(state: &PlayerState) -> Vec<TrackPopupRow>`
- Produces: `DesktopController::set_subtitle_preferences_restored(&mut self, restored: bool)`
- Produces: `refresh_tracks_popup(window: &MainWindow, runtime: &DesktopRuntime)`
- Produces: `subtitle_prefs_file_path(paths: &AppPaths) -> PathBuf`
- Produces: `load_subtitle_prefs_runtime(paths: Option<&AppPaths>) -> SubtitlePrefsRuntime`
- Produces: `sync_subtitle_prefs_from_state(runtime: &mut DesktopRuntime, state: &PlayerState) -> Result<(), yoyo_core::StorageError>`
- Produces: `apply_subtitle_restore_if_needed(runtime: &mut DesktopRuntime, controller: &mut DesktopController<MpvBackend>, app: Option<&MainWindow>) -> Result<(), yoyo_core::AppError>`

- [ ] **Step 1: Write the failing controller/runtime contract tests**

Modify `apps/yoyovideo-desktop/tests/controller_contract.rs` and create `apps/yoyovideo-desktop/tests/subtitle_runtime_contract.rs`:

```rust
// apps/yoyovideo-desktop/tests/controller_contract.rs
use std::path::PathBuf;

use yoyo_core::{AppCommand, AppConfig, AppSession, BackendCommand, MediaLocator, PlayerBackend};
use yoyovideo_desktop::DesktopController;

#[test]
fn controller_forwards_external_subtitle_and_visibility_commands() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);

    controller
        .dispatch(AppCommand::LoadExternalSubtitle(PathBuf::from("movie.ass")))
        .unwrap();
    controller.dispatch(AppCommand::SetSubtitleVisible(false)).unwrap();

    assert_eq!(
        controller.session().backend().commands,
        vec![
            BackendCommand::LoadExternalSubtitle(PathBuf::from("movie.ass")),
            BackendCommand::SetSubtitleVisible(false),
        ]
    );
}

// apps/yoyovideo-desktop/tests/subtitle_runtime_contract.rs
use yoyo_core::{AppConfig, AppSession, PlayerBackend};
use yoyovideo_desktop::DesktopController;

#[derive(Default)]
struct QuietBackend;

impl PlayerBackend for QuietBackend {
    fn open(&mut self, _locator: &yoyo_core::MediaLocator) -> Result<(), String> {
        Ok(())
    }

    fn send(&mut self, _command: yoyo_core::BackendCommand) -> Result<(), String> {
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<yoyo_core::BackendEvent> {
        Vec::new()
    }
}

#[test]
fn controller_can_mark_subtitle_preferences_restored_without_backend_traffic() {
    let session = AppSession::new(AppConfig::default(), QuietBackend);
    let mut controller = DesktopController::new(session);

    controller.set_subtitle_preferences_restored(true);

    assert!(controller.session().state().subtitle_preferences_restored);
}
```

- [ ] **Step 2: Run the failing controller/runtime tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test subtitle_runtime_contract
```

Expected: FAIL because the controller cannot yet mark subtitle-restore completion and the new app commands are not wired through the desktop layer.

- [ ] **Step 3: Wire the runtime subtitle-preference lifecycle and popup refresh**

Modify `apps/yoyovideo-desktop/src/app.rs`:

```rust
struct DesktopRuntime {
    controller: Option<DesktopController<MpvBackend>>,
    video_host_error: Option<String>,
    app_handle: Option<slint::Weak<MainWindow>>,
    config: AppConfig,
    history: crate::HistoryRuntime,
    subtitle_prefs: crate::SubtitlePrefsRuntime,
    sidebar: crate::SidebarState,
    settings_window: Option<SettingsWindow>,
    settings_controller: Option<crate::SettingsController>,
    pending_resume: Option<crate::PendingResumeSeek>,
    last_seen_locator: Option<MediaLocator>,
    last_seen_subtitle_locator: Option<MediaLocator>,
    started_at: Instant,
    #[cfg(feature = "mpv-runtime")]
    video_host: Option<WinitVideoHost>,
}

impl DesktopRuntime {
    fn new(
        config: AppConfig,
        history: crate::HistoryRuntime,
        subtitle_prefs: crate::SubtitlePrefsRuntime,
        sidebar: crate::SidebarState,
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
            #[cfg(feature = "mpv-runtime")]
            video_host: None,
        }
    }
}

fn subtitle_prefs_file_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join("subtitle_prefs.json")
}

fn load_subtitle_prefs_runtime(paths: Option<&AppPaths>) -> crate::SubtitlePrefsRuntime {
    crate::SubtitlePrefsRuntime::load(paths.map(subtitle_prefs_file_path))
        .unwrap_or_else(|_| crate::SubtitlePrefsRuntime::new(None, Default::default()))
}

fn refresh_tracks_popup(window: &MainWindow, runtime: &DesktopRuntime) {
    let Some(state) = runtime.controller().map(|controller| controller.session().state()) else {
        window.set_audio_track_rows(model_from_vec(Vec::<TrackPopupRowData>::new()));
        window.set_subtitle_track_rows(model_from_vec(Vec::<TrackPopupRowData>::new()));
        window.set_video_track_rows(model_from_vec(Vec::<TrackPopupRowData>::new()));
        window.set_tracks_status_label(runtime.status_message().into());
        return;
    };

    window.set_audio_track_rows(model_from_vec(
        crate::build_audio_track_rows(state)
            .into_iter()
            .map(|row| TrackPopupRowData { label: row.label.into(), selected: row.is_selected })
            .collect(),
    ));
    window.set_subtitle_track_rows(model_from_vec(
        crate::build_subtitle_track_rows(state)
            .into_iter()
            .map(|row| TrackPopupRowData { label: row.label.into(), selected: row.is_selected })
            .collect(),
    ));
    window.set_video_track_rows(model_from_vec(
        crate::build_video_track_rows(state)
            .into_iter()
            .map(|row| TrackPopupRowData { label: row.label.into(), selected: row.is_selected })
            .collect(),
    ));
    window.set_subtitle_visible(state.subtitle.visible);
    window.set_subtitle_delay_value(state.subtitle.delay_seconds as f32);
    window.set_subtitle_delay_label(crate::format_subtitle_delay_label(state.subtitle.delay_seconds).into());
    window.set_subtitle_scale_value(state.subtitle.scale);
    window.set_subtitle_scale_label(crate::format_subtitle_scale_label(state.subtitle.scale).into());
    window.set_subtitle_position_value(f32::from(state.subtitle.vertical_position_percent));
    window.set_tracks_status_label(
        state
            .last_error
            .clone()
            .or_else(|| state.status_message.clone())
            .unwrap_or_default()
            .into(),
    );
}

fn sync_subtitle_prefs_from_state(
    runtime: &mut DesktopRuntime,
    state: &PlayerState,
) -> Result<(), yoyo_core::StorageError> {
    runtime.subtitle_prefs.remember_from_state(state);
    let now = history_now(runtime);
    let current = state.current.clone();
    let switched = current != runtime.last_seen_subtitle_locator;
    runtime.last_seen_subtitle_locator = current;
    runtime.subtitle_prefs.flush_if_needed(
        now,
        if switched {
            crate::SubtitlePrefsFlushReason::MediaSwitch
        } else {
            crate::SubtitlePrefsFlushReason::PeriodicTick
        },
    )?;
    Ok(())
}

fn apply_subtitle_restore_if_needed(
    runtime: &mut DesktopRuntime,
    controller: &mut DesktopController<MpvBackend>,
    app: Option<&MainWindow>,
) -> Result<(), yoyo_core::AppError> {
    let state = controller.session().state().clone();
    let Some(locator) = state.current.as_ref() else {
        controller.set_subtitle_preferences_restored(true);
        return Ok(());
    };
    if state.subtitle_preferences_restored {
        return Ok(());
    }
    if state.audio_tracks.is_empty() && state.subtitle_tracks.is_empty() && state.video_tracks.is_empty() {
        return Ok(());
    }

    match runtime.subtitle_prefs.restore_plan_for(locator) {
        Ok(Some(plan)) => {
            for command in plan.commands {
                controller.dispatch(command)?;
            }
        }
        Ok(None) => {}
        Err(crate::SubtitleRestoreError::MissingExternalSubtitle(path)) => {
            if let Some(app) = app {
                app.set_status_label(format!("Subtitle file is missing: {}", path.display()).into());
            }
        }
    }

    controller.set_subtitle_preferences_restored(true);
    Ok(())
}
```

- [ ] **Step 4: Wire popup callbacks, poll-time restore, and shutdown flush**

Continue modifying `apps/yoyovideo-desktop/src/app.rs`:

```rust
impl<B: PlayerBackend> DesktopController<B> {
    pub fn set_subtitle_preferences_restored(&mut self, restored: bool) {
        self.session.set_subtitle_preferences_restored(restored);
    }
}

// In run():
let subtitle_prefs = load_subtitle_prefs_runtime(paths.as_ref());
let runtime = Rc::new(RefCell::new(DesktopRuntime::new(config, history, subtitle_prefs, sidebar)));

refresh_runtime_window(&app, &runtime.borrow());
refresh_sidebar(&app, &runtime.borrow());
refresh_tracks_popup(&app, &runtime.borrow());

app.on_audio_track_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |index| {
        if index < 0 {
            return;
        }
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            let rows = crate::build_audio_track_rows(controller.session().state());
            let Some(id) = rows.get(index as usize).and_then(|row| row.track_id) else {
                return Ok(());
            };
            controller.dispatch(AppCommand::SelectAudioTrack(id))
        });
    }
});

app.on_subtitle_track_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |index| {
        if index < 0 {
            return;
        }
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            let rows = crate::build_subtitle_track_rows(controller.session().state());
            let Some(row) = rows.get(index as usize) else {
                return Ok(());
            };
            match row.track_id {
                Some(id) => controller.dispatch(AppCommand::SelectSubtitleTrack(id)),
                None => controller.dispatch(AppCommand::SetSubtitleVisible(false)),
            }
        });
    }
});

app.on_video_track_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |index| {
        if index < 0 {
            return;
        }
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            let rows = crate::build_video_track_rows(controller.session().state());
            let Some(id) = rows.get(index as usize).and_then(|row| row.track_id) else {
                return Ok(());
            };
            controller.dispatch(AppCommand::SelectVideoTrack(id))
        });
    }
});

app.on_load_external_subtitle_requested({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    let dialogs = Rc::clone(&dialogs);
    move || {
        if let Some(path) = dialogs.pick_subtitle_file() {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::LoadExternalSubtitle(path))
            });
        }
    }
});

app.on_subtitle_visible_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |visible| {
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::SetSubtitleVisible(visible))
        });
    }
});

app.on_subtitle_delay_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |delay| {
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::SetSubtitleDelay(f64::from(delay.clamp(-10.0, 10.0))))
        });
    }
});

app.on_subtitle_scale_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |scale| {
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::SetSubtitleScale(scale.clamp(0.5, 2.0)))
        });
    }
});

app.on_subtitle_position_changed({
    let app_handle = app.as_weak();
    let runtime = Rc::clone(&runtime);
    move |position| {
        with_runtime_controller(&app_handle, &runtime, move |controller| {
            controller.dispatch(AppCommand::SetSubtitleVerticalPosition(position.clamp(0.0, 100.0) as u8))
        });
    }
});

// After every refresh_window()/refresh_sidebar() call:
refresh_tracks_popup(&app, &runtime);

// In with_runtime_controller(), after action(controller) and apply_pending_resume():
if let Err(error) = apply_subtitle_restore_if_needed(&mut runtime, controller, app_handle.upgrade().as_ref()) {
    return Err((error, pending_before));
}
let state = controller.session().state().clone();
sync_subtitle_prefs_from_state(&mut runtime, &state).map_err(|error| (error.into(), pending_before))?;

// In the poll timer, after controller.poll_backend():
if let Err(error) = apply_subtitle_restore_if_needed(&mut runtime, controller, Some(&app)) {
    runtime.pending_resume = pending_before;
    app.set_status_label(error.to_string().into());
    return;
}
let state = controller.session().state().clone();
let _ = sync_subtitle_prefs_from_state(&mut runtime, &state);
refresh_window(&app, &state);
refresh_sidebar(&app, &runtime);
refresh_tracks_popup(&app, &runtime);

// On shutdown:
let shutdown_now = history_now(&runtime);
let _ = runtime.history.flush_if_needed(shutdown_now, crate::FlushReason::Shutdown);
let _ = runtime.subtitle_prefs.flush_if_needed(
    shutdown_now,
    crate::SubtitlePrefsFlushReason::Shutdown,
);
```

- [ ] **Step 5: Run the desktop runtime tests and compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test subtitle_runtime_contract
cargo test -p yoyovideo-desktop --test subtitle_prefs_contract
cargo test -p yoyovideo-desktop --test track_popup_contract
cargo test -p yoyovideo-desktop --test main_window_tracks_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS. The desktop runtime now refreshes the popup from observed state, persists per-media subtitle preferences, applies a best-effort restore after track enumeration, and flushes subtitle preferences at shutdown.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/tests/controller_contract.rs apps/yoyovideo-desktop/tests/subtitle_runtime_contract.rs
git commit -m "feat: wire subtitle popup runtime"
```

Expected: Commit succeeds.

---

### Task 6: Manual Smoke Coverage And Final Verification

**Files:**
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Produces: updated popup, external-subtitle, and per-media-restore smoke coverage in the shared checklist

- [ ] **Step 1: Add subtitle popup smoke coverage**

Append these lines under `## UX` in `docs/testing/manual-smoke-checklist.md`:

```markdown
- Open a media file with multiple audio or subtitle tracks, open the `Tracks` popup, and confirm the lists reflect the available tracks.
- Switch audio, subtitle, and video tracks from the popup and confirm playback updates without leaving the main window.
- Choose the subtitle `Off` entry and confirm subtitles disappear immediately.
- Load an external subtitle file from the popup and confirm it appears and becomes usable without interrupting playback.
- Adjust subtitle delay, scale, and vertical position from the popup and confirm the changes apply during playback.
- Close and reopen the same media and confirm the last subtitle/track preferences are restored.
- Remove a previously remembered external subtitle file, reopen the same media, and confirm playback still starts while the app shows only a non-blocking warning.
```

- [ ] **Step 2: Run a documentation coverage check**

Run:

```powershell
$content = Get-Content -Raw docs/testing/manual-smoke-checklist.md
$required = @(
  "Tracks` popup",
  "subtitle `Off` entry",
  "external subtitle file",
  "delay, scale, and vertical position",
  "restored",
  "non-blocking warning"
)
$missing = $required | Where-Object { $content -notmatch [regex]::Escape($_) }
if ($missing.Count -gt 0) {
  Write-Error ("Missing checklist coverage: " + ($missing -join ", "))
  exit 1
}
```

Expected: PASS.

- [ ] **Step 3: Run final formatting and verification**

Run:

```powershell
cargo fmt --check
cargo test -p yoyo-core --test track_state_contract
cargo test -p yoyo-mpv --test translate_contract
cargo test -p yoyo-mpv --test event_contract
cargo test -p yoyovideo-desktop --test subtitle_prefs_contract
cargo test -p yoyovideo-desktop --test track_popup_contract
cargo test -p yoyovideo-desktop --test main_window_tracks_contract
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test subtitle_runtime_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
git status --short
```

Expected:
- `cargo fmt --check`: PASS
- all targeted tests: PASS
- both `cargo check` commands: PASS
- `git status --short`: only the planned source/doc changes remain before the last commit

- [ ] **Step 4: Commit**

Run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add subtitle popup smoke checks"
```

Expected: Commit succeeds.

---

## Self-Review

**Spec coverage:** The plan covers the popup-based `Tracks / Subtitles` surface, embedded audio/subtitle/video track selection, subtitle `Off`, external subtitle loading, live subtitle visibility/delay/scale/position controls, dedicated per-media subtitle preference persistence, best-effort restore sequencing after track enumeration, graceful missing-file handling, runtime refresh from observed backend state, and manual smoke additions. It intentionally leaves full subtitle styling, online subtitle discovery, per-folder inheritance, and dedicated subtitle shortcut customization out of scope.

**Placeholder scan:** The plan does not use `TBD`, `TODO`, “implement later”, or “similar to Task N” placeholders. Each task names exact files, concrete types/functions, code blocks, commands, expected failures, expected passes, and commit messages.

**Type consistency:** The plan uses one consistent set of names across tasks: `MediaTrack`, `MediaTrackKind`, `SubtitlePlaybackState`, `SubtitlePrefsRuntime`, `SubtitlePrefsFlushReason`, `SubtitleRestoreError`, `SubtitleRestorePlan`, `TrackPopupRow`, `TrackPopupRowData`, `refresh_tracks_popup`, `sync_subtitle_prefs_from_state`, and `apply_subtitle_restore_if_needed`. The same command names flow from popup callbacks to `AppCommand`, then to `BackendCommand`, then to mpv translation.
