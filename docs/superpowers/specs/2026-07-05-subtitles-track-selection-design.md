# Subtitles And Track Selection Design

## Goal

Add a first usable subtitles and media-track control surface to YoYoVideo so users can inspect and switch embedded tracks, load external subtitles, and adjust core subtitle playback parameters without leaving the main player window.

This phase is intentionally limited to in-window popup controls for audio, subtitle, and video tracks; external subtitle loading; subtitle visibility and timing/placement controls; and per-media subtitle preference persistence.

## Why This Phase Exists

The player already has working playback, playlist/history navigation, persistent config storage, and a dedicated settings surface. What it still lacks is a practical way to control real-world multi-track media such as dual-audio files, multiple subtitle streams, or external subtitle files.

The codebase does not yet have a formal track model in `yoyo-core`, real subtitle command/event plumbing in `yoyo-mpv`, or a desktop flow for per-media subtitle preferences. The next highest-value step is to add those boundaries cleanly before attempting richer subtitle styling or more advanced playback features.

## In Scope

- Add a main-window popup panel for `Tracks / Subtitles`.
- Show and switch available audio tracks.
- Show and switch available subtitle tracks.
- Show and switch available video tracks when multiple video tracks exist.
- Support a subtitle `Off` state.
- Load external subtitle files from local disk.
- Expose subtitle controls for:
  - visibility
  - delay
  - scale
  - vertical position
- Persist subtitle and track preferences per media item using `MediaLocator` as the key.
- Restore saved subtitle and track preferences when the same media is opened again.
- Add automated tests for core state updates, mpv translation, desktop preference persistence, and popup-facing view-model mapping.

## Out Of Scope

- Full subtitle style editing such as font family, color, outline, shadow, or background.
- Online subtitle search, download, or auto-match.
- Per-folder or per-series subtitle inheritance rules.
- Track renaming, reordering, or batch rules.
- A full subtitle shortcut customization surface beyond the existing generic shortcut system.
- Advanced filters, screenshots, scripts, or plugin features.

## Locked Decisions For This Phase

- The primary control surface is a main-window popup panel, not a sidebar tab or dedicated settings window.
- Audio, subtitle, and video track selection are all modeled in `yoyo-core` instead of being desktop-only mpv passthrough actions.
- External subtitles join the same subtitle-track model instead of living in a separate parallel selection system.
- Per-media subtitle preferences are stored in a dedicated persistence file, not inside `history.json`.
- Preference restoration is best-effort. Failed restore steps are skipped individually and do not block playback.
- Runtime truth comes from observed backend state. Saved preferences are restore hints, not authoritative state.

## UI Direction

The main player window keeps its existing layout. A new popup panel is added for playback-time subtitle and track control, keeping the interaction closer to a desktop media player than to a heavy settings dialog.

Popup entry points:

- A dedicated `Tracks` button is added near the existing playback controls or menu access.
- The existing menu may also expose the same action, but the popup remains the primary surface.

Popup layout:

- `Tracks`
  - `Audio Tracks`
  - `Subtitle Tracks`
  - `Video Tracks`
- `External Subtitle`
  - `Load Subtitle...`
- `Subtitle Controls`
  - `Visible`
  - `Delay`
  - `Scale`
  - `Vertical Position`
- `Status`
  - short inline status text for restore or load failures

Track rows should show compact, human-readable labels assembled from the best available metadata:

- explicit track title when present
- language code when present
- fallback numeric track id when metadata is sparse

The subtitle track section always includes an `Off` row so users can disable subtitles explicitly from the same list used for subtitle selection.

## Interaction Model

- Opening the popup shows the currently observed track lists and current selections.
- Clicking an audio, subtitle, or video track row immediately sends the matching selection command.
- Clicking subtitle `Off` disables active subtitles immediately.
- Clicking `Load Subtitle...` opens a file picker for local subtitle files such as `.srt`, `.ass`, or `.ssa`.
- After a successful external subtitle load:
  - the subtitle track list refreshes
  - the new subtitle track may become the active subtitle track
  - the current media preference remembers the chosen external subtitle path
- Subtitle visibility, delay, scale, and vertical position changes apply immediately and update the current media preference.
- Popup controls do not use `Apply` or `OK`; they are live playback controls.

Restore flow for a media item:

1. Desktop opens the media through the existing controller/session path.
2. mpv reports track lists and current selections through backend events.
3. Desktop looks up per-media subtitle preferences for the active `MediaLocator`.
4. Desktop attempts to restore saved track choices, subtitle visibility, external subtitle path, and subtitle controls.
5. The backend continues to report actual resulting state, and the popup refreshes from observed truth.

This deferred restore ordering avoids firing track-selection commands before media metadata and track enumeration are ready.

## Data Model And Flow

`yoyo-core` becomes the typed source of truth for subtitle and track state.

Core additions:

- Add a reusable track descriptor type that can represent audio, subtitle, and video tracks.
- Extend `PlayerState` with:
  - available audio tracks
  - available subtitle tracks
  - available video tracks
  - selected audio track id
  - selected subtitle track id
  - selected video track id
  - subtitle visible flag
  - external subtitle source path
  - subtitle delay
  - subtitle scale
  - subtitle vertical position
  - a flag marking whether per-media subtitle preferences were already restored for the active media
- Extend `BackendCommand` with typed track-selection and subtitle-control commands.
- Extend `BackendEvent` with track-list updates, selected-track updates, subtitle-control updates, and external subtitle load/reporting events.
- Extend `AppCommand` with user-facing commands for selecting tracks, toggling subtitles, loading external subtitles, and adjusting subtitle controls.

Desktop additions:

- Add a popup-facing Rust view-model mapper that turns typed track state into simple Slint rows.
- Add a dedicated subtitle-preference runtime responsible for:
  - loading a per-media subtitle preference store at startup
  - updating in-memory preferences when subtitle/track changes are observed
  - restoring preferences for the newly active `MediaLocator`
  - throttled persistence and shutdown flush
- Keep the existing history runtime separate from this new subtitle-preference runtime.

Persistence model:

- Add a new persisted store such as `subtitle_prefs.json`.
- Use `MediaLocator` as the key.
- Persist:
  - selected audio track identity
  - selected subtitle track identity
  - selected video track identity
  - subtitle visible flag
  - external subtitle path
  - subtitle delay
  - subtitle scale
  - subtitle vertical position

Track identity should prefer a stable backend-facing track id within the media session. Preference restore logic should tolerate missing tracks by skipping that specific restore step.

## Runtime Application Strategy

Track and subtitle controls remain live playback operations handled through the existing desktop controller and backend boundary.

Rules:

- The popup never mutates `PlayerState` directly.
- The desktop layer sends commands and waits for backend events to confirm actual state.
- Preference persistence happens from observed state changes, not optimistic UI guesses.
- Preference restoration runs once per active media item after track enumeration is available.
- If a later media switch happens, the restore-ready flag resets for the new media item.

Specific persistence semantics:

- Embedded track selections are remembered per media item.
- External subtitle paths are remembered per media item.
- Subtitle controls are remembered per media item.
- Subtitle disable state is remembered per media item through the persisted subtitle-visible flag.
- URL media can also use per-media preferences, keyed by the exact `MediaLocator::Url` string.
- Missing external subtitle files are ignored during restore with a non-blocking status message.

## Error Handling

Errors are handled without interrupting active playback unless the backend itself already fails the media.

Restore errors:

- If a remembered track id no longer exists, skip only that track restore.
- If a remembered external subtitle file is missing, skip only that load and report a concise status message.
- If a subtitle control value cannot be applied by the backend, keep playback running and rely on the next observed backend state.

Runtime interaction errors:

- Track selection failures surface in the status label and popup status area.
- External subtitle load failures surface in the same non-blocking status area.
- Temporary empty track lists render as empty/disabled popup sections rather than crashing or showing stale forced selection.

Persistence errors:

- If the subtitle preference file is unreadable or malformed at startup, fall back to an empty in-memory store and continue playback.
- If a save fails during runtime or shutdown flush, keep the in-memory state and expose a concise status message.

## Testing Strategy

Core tests:

- Track-list and selected-track state updates in `PlayerState`.
- Subtitle visible/delay/scale/position state updates.
- Restore-ready flag behavior across media changes.
- Session handling for the new `AppCommand` to `BackendCommand` mapping.

mpv tests:

- Command translation for audio/subtitle/video track selection.
- Command translation for subtitle visibility, delay, scale, and position.
- Command translation for external subtitle loading.
- Property and event decoding for track lists, selected track ids, and subtitle parameter changes.

Desktop tests:

- Popup row mapping for audio, subtitle, and video tracks.
- Correct inclusion of subtitle `Off`.
- Per-media preference store save/load behavior.
- Restore flow that waits for track enumeration before applying preferences.
- Graceful handling of missing external subtitle paths.
- Runtime refresh behavior that follows backend-confirmed state instead of optimistic UI-only state.

Manual smoke additions:

- Open media with multiple audio tracks and confirm switching works.
- Open media with multiple embedded subtitle tracks and confirm switching works.
- Disable subtitles and confirm the `Off` state persists for that media.
- Load an external subtitle file and confirm it appears as a selectable subtitle track.
- Adjust subtitle delay, scale, and vertical position and confirm the changes take effect.
- Reopen the same media and confirm the last subtitle and track preferences are restored.
- Delete the remembered external subtitle file and confirm reopen still plays while showing only a non-blocking warning.

## Architecture Boundaries

- `yoyo-core` owns typed track/subtitle state, commands, and backend event semantics.
- `yoyo-mpv` owns mpv-specific property observation, event decoding, and command translation.
- `yoyovideo-desktop` owns popup wiring, row mapping, file-picker integration for external subtitles, and per-media subtitle preference persistence policy.
- Slint stays declarative and receives simple lists, scalar properties, and callbacks rather than raw mpv concepts or persistence logic.

## Acceptance Criteria

- The main player exposes a popup-based `Tracks / Subtitles` control surface.
- The popup reflects current audio, subtitle, and video tracks from real backend state.
- Users can switch embedded audio, subtitle, and video tracks from that popup.
- Users can disable subtitles with an explicit `Off` action.
- Users can load a local external subtitle file and use it like a normal subtitle track.
- Subtitle visibility, delay, scale, and vertical position controls apply during playback.
- Subtitle and track preferences are remembered per media item and restored when that media is opened again.
- Missing tracks or missing external subtitle files fail gracefully without interrupting playback.
- The added behavior is covered by automated Rust tests plus manual smoke checklist updates.

## Follow-On Phases

This design intentionally creates boundaries that later phases can reuse for:

- richer subtitle styling and preset management
- dedicated subtitle shortcuts and faster playback-time controls
- smarter subtitle matching and source discovery
- broader per-media playback preference restoration
