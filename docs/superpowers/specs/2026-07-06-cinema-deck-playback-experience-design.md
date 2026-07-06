# Cinema Deck Playback Experience Design

## Goal

Turn YoYoVideo from a function-complete prototype into a player that feels like a polished desktop video app. This phase combines playback interaction improvements with a focused `Cinema Deck` UI direction: video-first layout, a custom bottom control deck, transient OSD feedback, richer seek behavior, chapters, user markers, and a lightweight action panel.

The work must preserve the current Rust + Slint + libmpv architecture. It should not replace Slint, introduce a web runtime, or move playback logic into the UI file.

## Scope

- Redesign the main player surface around the selected `Cinema Deck` direction.
- Replace the default-looking main playback controls with custom Slint components where practical.
- Add mute/unmute state and shortcuts.
- Add jump-to-time entry with timestamp parsing.
- Change progress seeking so drag/hover can preview the target time while actual seek is committed explicitly.
- Add progress hover preview labels and chapter/marker tick marks.
- Read embedded mpv chapters when available.
- Add local user markers for the current media item.
- Show chapters and markers in both the progress rail and a lightweight action panel.
- Add transient OSD for common actions such as seek, volume, mute, speed, screenshot, frame step, filters, chapters, and markers.
- Keep existing playlist, history, recent, tracks, settings, screenshot, frame step, filters, and subtitles functionality available.
- Add automated contract coverage and manual smoke checks for all new behavior.

## Non-Goals

- No full media library, tagging, indexing, thumbnails, or search database.
- No waveform or video-thumbnail preview generation in this phase.
- No file mutation: user markers are stored by YoYoVideo and are not written back into MKV/MP4 chapters.
- No global hotkeys or OS media key integration.
- No new UI toolkit or webview runtime.
- No skin/theme marketplace.
- No complete settings redesign beyond new shortcut rows if needed.
- No drag-reorder playlist work.

## Chosen Direction

The visual direction is `Cinema Deck`:

- The video host remains the dominant element.
- Controls sit in a semi-transparent bottom deck over or directly below the video area.
- Primary playback actions are always one click away.
- Advanced actions move into compact popups or an action panel instead of filling the main control strip.
- The sidebar remains available, but it should feel secondary to playback.
- The design uses a dark cinematic palette with restrained blue/teal accents, glass-like surfaces, and clear focus states.

This direction was chosen over a dense `Studio Console` and a conservative `Clean Native` layout because YoYoVideo is a video player first, not a media-management dashboard.

## Architecture

Use the existing split:

- `yoyo-core` owns playback state, commands, validation, chapters, markers, and shortcut actions.
- `yoyo-mpv` owns libmpv command translation and mpv property/event decoding.
- `apps/yoyovideo-desktop` owns Slint UI wiring, local persistence, OSD timing, dialogs, and desktop-specific paths.
- Slint owns visual layout and exposes only typed properties/callbacks.

The phase should be implemented as vertical slices:

1. Core playback model extensions.
2. mpv event/command support.
3. desktop persistence and presenters.
4. Slint surface additions.
5. Cinema Deck visual refactor.

This avoids a large UI rewrite with unconnected behavior.

## Core Playback Model

Add durable state and commands to `yoyo-core`.

New or extended state:

```text
PlayerState {
    muted: bool,
    chapters: Vec<MediaChapter>,
    markers: Vec<MediaMarker>,
}

MediaChapter {
    title: Option<String>,
    time_seconds: f64,
}

MediaMarker {
    id: String,
    title: String,
    time_seconds: f64,
    created_at: String,
}
```

Commands:

```text
AppCommand::SetMuted(bool)
AppCommand::ToggleMute
AppCommand::JumpToTime(f64)
AppCommand::AddMarkerAtCurrentPosition
AppCommand::RemoveMarker(String)
AppCommand::SeekToChapter(usize)
AppCommand::SeekToMarker(String)
```

`JumpToTime` and chapter/marker seeking route through the existing absolute seek behavior. Core clamps seek targets to `0..duration` when duration is known and rejects invalid or non-finite values.

Opening new media clears embedded chapters until mpv reports them. User markers are restored by the desktop layer after the current media identity is known.

## mpv Integration

Extend `yoyo-mpv` with chapter and mute support:

- Observe mpv `mute` as a boolean property.
- Translate `BackendCommand::SetMuted(bool)` to mpv `mute`.
- Observe `chapter-list` as `MPV_FORMAT_NODE`.
- Decode `chapter-list` into `Vec<MediaChapter>`.
- Map decoded chapters to `BackendEvent::ChaptersChanged(Vec<MediaChapter>)`.

The mpv stable manual documents `chapter-list` as an array-like property whose entries include chapter title and start time. YoYoVideo should decode only these fields and ignore unknown extra fields.

Failure rules:

- If `chapter-list` observation fails, playback still works and the UI simply has no embedded chapter ticks.
- If a chapter has no title, display a generated label such as `Chapter 3`.
- If a chapter time is non-finite or negative, skip that entry.
- If mpv reports duplicate or unsorted chapter times, sort by time and keep stable labels.

## Marker Persistence

User markers are desktop-local convenience data. They are stored separately from embedded chapters.

Storage model:

```text
MarkerStore {
    items: Vec<MediaMarkerSet>
}

MediaMarkerSet {
    locator_key: String,
    markers: Vec<MediaMarker>
}
```

Rules:

- `locator_key` uses the existing media locator label so file and URL markers stay separate.
- Store under app data, for example `markers.toml`.
- Cap marker sets and markers per media item to avoid unbounded growth.
- Missing or corrupt marker files load as empty.
- Markers are sorted by time.
- Adding a marker within 0.75 seconds of an existing marker does not create a duplicate. It keeps the existing marker and shows a non-blocking OSD/status message.

Markers should not depend on `remember_history`; they are user-authored navigation aids, not resume metadata.

## Jump-To-Time

Add a lightweight jump overlay accessible from the control deck and shortcut.

Input format:

- `ss`
- `mm:ss`
- `hh:mm:ss`
- Optional fractional seconds for all forms.

Examples:

- `75` means 75 seconds.
- `01:15` means 1 minute 15 seconds.
- `1:02:03.5` means 1 hour, 2 minutes, 3.5 seconds.

Invalid input shows a non-fatal status/OSD message and does not seek. If duration is known, targets beyond duration clamp to the end.

## Progress Rail Behavior

The current slider commits seeks through `changed`, which can cause too many seeks during dragging. Replace or wrap it with a custom progress rail:

- Hovering computes a preview percent and preview timestamp.
- Pressing/dragging updates preview state only.
- Releasing commits one `SeekAbsolute`.
- Keyboard and shortcut seeks keep using the existing command path.
- If duration is unknown, hover and drag preview are disabled.

Visual elements:

- Played progress fill.
- Buffered/cache fill is reserved for a future phase with reliable backend data. This phase does not require it.
- Embedded chapter ticks.
- User marker ticks with a different accent.
- Preview bubble showing target timestamp and nearest chapter/marker title when applicable.

If Slint input APIs make hover position difficult on a platform, the implementation should still deliver drag preview and commit-on-release, then treat hover preview as best-effort.

## OSD Feedback

Add a desktop-side transient OSD presenter rather than storing OSD in `yoyo-core`.

OSD events:

- Play/pause.
- Seek relative/absolute.
- Volume and mute.
- Speed changes.
- Screenshot saved or failed.
- Frame step.
- Rotation, zoom, audio channel.
- A-B loop changes.
- Filter and picture reset.
- Jump-to-time success/failure.
- Marker added/removed.
- Chapter/marker navigation.

Behavior:

- OSD auto-hides after a short duration.
- New OSD replaces the previous one.
- Status label remains for longer-lived messages and diagnostics.
- OSD must never block playback commands.

## Cinema Deck UI

Main window layout:

- Video host in a large rounded dark surface.
- Bottom `ControlDeck` component with custom controls.
- Compact top-left or top-edge brand/title text.
- Status/OSD overlays inside the video area.
- Sidebar still supports Playlist and History, but defaults to visually secondary.
- Existing Tracks and Video Tools popups remain, but should be restyled to match the new deck.
- Menu and Recent stay available from the action panel or menu button.

Control deck first row:

- Play/pause.
- Current time.
- custom progress rail.
- total time or remaining time.
- mute/volume.
- fullscreen.

Control deck second row or compact action strip:

- Open.
- Speed.
- Jump.
- Chapters/Markers.
- Tracks.
- Video Tools.
- Playlist.
- Recent/Menu.

Settings window may keep its current structure in this phase, but it should receive minimal visual cleanup if shared custom controls are introduced.

## Action Panel

Add a lightweight overlay panel, not a full command palette.

Sections:

- `Quick`: screenshot, previous frame, next frame, jump to time, add marker.
- `Navigation`: chapters, markers, playlist next/previous.
- `Media`: tracks, subtitles, audio channel, recent.
- `Picture`: filters, reset picture, video tools.

The panel uses the same command callbacks as existing toolbar/menu actions. It should not duplicate business logic.

Empty states:

- No chapters: show `No embedded chapters`.
- No markers: show `No markers yet`.
- No current media: disable marker actions and show a status row.

## Shortcut Updates

Existing shortcuts remain valid. Add new shortcut actions:

- `ToggleMute`
- `JumpToTime`
- `AddMarker`
- `OpenActionPanel`
- `NextChapterOrMarker`
- `PreviousChapterOrMarker`

Default bindings:

- `M`: toggle mute.
- `J`: jump to time.
- `Ctrl+M`: add marker at current position.
- `P`: open action panel.
- `Shift+Right`: next chapter or marker.
- `Shift+Left`: previous chapter or marker.

These do not collide with existing defaults. Any new shortcut appears in the settings shortcut editor and participates in conflict detection.

## Data Flow

Startup:

1. Load config, history, recent store, subtitle prefs, and marker store.
2. Create `MainWindow`.
3. Restore window state.
4. Refresh deck, sidebar, recent, tracks, and marker rows.

Open media:

1. Normal open path loads file/folder/URL.
2. Core clears transient chapter state.
3. Desktop resolves marker set for current media and applies user markers.
4. mpv reports tracks, duration, position, mute, and chapters through backend events.
5. UI refreshes progress ticks and action panel rows.

Progress preview:

1. Pointer hover or drag computes percent and timestamp in Slint/Rust presenter.
2. UI displays preview without dispatching seek.
3. Release dispatches `SeekAbsolute`.
4. OSD reports the committed seek target.

Marker add:

1. User triggers add marker.
2. Core creates or dedupes a marker at the current position.
3. Marker store saves best-effort.
4. UI refreshes marker ticks and action panel rows.

Chapter update:

1. mpv observes `chapter-list`.
2. `yoyo-mpv` decodes chapters.
3. Core stores chapters.
4. UI renders chapter ticks and rows.

## Error Handling

- Invalid jump input: show OSD/status, no seek.
- Unknown duration: progress rail preview and release-to-seek are disabled. Jump-to-time accepts only finite non-negative explicit timestamps and dispatches them without duration clamping.
- mpv chapter decode failure: log warning and keep chapters empty.
- Marker store read failure: continue with no markers and log warning.
- Marker store write failure: show non-blocking status and keep in-memory markers for the session.
- Mute command failure: restore state from backend event when available, otherwise surface existing backend error.
- UI preview math never panics on `NaN`, infinity, zero duration, or negative duration.

## Testing

Automated coverage:

- `yoyo-core` tests for mute state, jump target validation, chapter state replacement, marker add/remove/order, marker seek, and new shortcut defaults.
- `yoyo-mpv` translate tests for mute property.
- `yoyo-mpv` event tests for mute and chapter-list mapping.
- Desktop presenter tests for time parsing, progress preview labels, OSD labels, chapter/marker row labels, and tick percent calculations.
- Marker store tests for missing/corrupt files, round trip, cap, sort, and dedupe rule.
- Slint compile contracts for new properties/callbacks: progress preview, commit seek, jump overlay, action panel, chapters, markers, OSD, and mute.
- Existing context menu, settings, shortcuts, and playback tests remain passing.

Manual smoke:

- Launch and confirm the player uses the Cinema Deck layout without default-looking main controls.
- Open a video, drag progress, and confirm preview updates without repeated seek jumps until release.
- Hover progress and confirm preview timestamp appears when supported.
- Use jump-to-time with valid and invalid values.
- Toggle mute from button and shortcut.
- Open media with chapters and confirm chapter ticks/rows appear.
- Add and remove user markers, restart, and confirm markers persist.
- Use chapter/marker navigation.
- Open action panel and trigger screenshot, frame step, tracks, filters, and recent paths.
- Confirm OSD appears for common actions and does not block playback.

## Acceptance Criteria

- YoYoVideo retains all existing playback features.
- Main playback UI follows the `Cinema Deck` direction and no longer depends on default-looking main controls for primary playback.
- Progress drag commits only one seek on release.
- Jump-to-time works with documented timestamp formats.
- Mute state is represented in core, backend, UI, and shortcuts.
- Embedded chapters display when mpv provides them.
- User markers can be added, removed, rendered, persisted, and used for seeking.
- OSD feedback appears for common actions and auto-hides.
- Full `cargo fmt --check`, relevant package tests, and runtime/package smoke checks pass before implementation completion.

## References

- mpv stable manual: `https://mpv.io/manual/stable/`
- `chapter-list` exposes chapter title and start time through `MPV_FORMAT_NODE`.
- mpv's built-in OSC exposes chapter navigation commands, which validates chapter-aware playback UI as a native mpv concept.
