# Playlist History Sidebar Design

## Goal

Add a first-class in-window sidebar for playlist and playback history so YoYoVideo can expose queue navigation and resume-entry selection without leaving the main player window.

This phase is intentionally limited to a right-side collapsible `Playlist / History` sidebar and the data flow needed to populate and activate those entries.

## Why This Phase Exists

The player now has visible video, practical playback controls, and real keyboard routing, but it still lacks a usable navigation surface for the current queue and recent playback items. The codebase already contains core playlist and history models, so the next highest-value step is exposing them in the desktop UI before adding larger settings or subtitle surfaces.

## In Scope

- Add a collapsible right sidebar to the desktop app.
- Add `Playlist` and `History` tabs inside that sidebar.
- Show the current playback queue with current-item highlighting.
- Show persisted recent playback items with resume-position metadata.
- Allow clicking a playlist item to switch playback to that item.
- Allow clicking a history item to reopen that media and resume from saved position.
- Keep all sidebar actions on the existing desktop controller and session command path.
- Add tests for sidebar-facing view models and command generation.

## Out Of Scope

- Subtitle UI, subtitle track switching, or subtitle styling.
- Settings page UI or shortcut editing UI.
- Playlist editing features such as drag reordering, delete, rename, or multi-select.
- History search, filtering, bulk clear, or session-level restore.
- Reconstructing the full historical playlist that existed when a history entry was created.

## UI Direction

The main window keeps the current `video area + bottom control surface` layout. A new collapsible sidebar is added on the right edge of the main window.

Sidebar behavior:

- Startup visibility follows `ui.show_playlist_on_startup`, except windows narrower than `1050px`, which start collapsed.
- Collapsed state keeps a `36px` edge strip with a visible affordance to reopen.
- Expanded state uses a `320px` target width with two tabs at the top, `Playlist` and `History`.
- Narrow windows keep the same layout model, but use a reduced `260px` sidebar width instead of introducing an overlay mode in this phase.

Sidebar contents:

- `Playlist`: list rows with display label, current-item highlight, and active-state styling.
- `History`: list rows with media label plus resume position summary such as last known time.
- A dedicated control-surface button toggles sidebar visibility instead of depending on startup settings alone.

## Interaction Model

- Opening a single file or URL replaces the current playlist with one item.
- Opening a folder replaces the current playlist with the scanned media entries and starts from the first playable item.
- Clicking a playlist row switches playback to that item and updates current-row highlight.
- Clicking a history row reopens that media and resumes from the stored progress position.
- History activation restores a single media item only. It does not restore the original historical queue.
- The sidebar is a navigation surface, not a playlist editor, in this phase.

## Data Model And Flow

`yoyo-core` remains the source of truth for playback state, playlist content, and history persistence semantics.

Core additions:

- Add a read-only playlist snapshot surface that exposes playlist entries and current index to the desktop layer.
- Add a small history-restore command path that turns a history selection into media reopen plus resume position.

Desktop additions:

- Add lightweight UI state for:
  - sidebar visibility
  - active sidebar tab
  - mapped playlist rows
  - mapped history rows
- Extend `refresh_window()` so it updates sidebar rows and selected state in addition to transport labels.
- Keep all row activation flowing through `DesktopController`, which continues to own the session boundary.

Persistence behavior:

- History is loaded once during startup.
- History writes are throttled to at most once every 2 seconds during active playback, with an immediate flush on pause, media switch, and app shutdown.
- After a successful history write, the desktop history view is refreshed from the current in-memory model.

## Error Handling

- If a history entry points to a missing local file, activation fails gracefully and reports a clear status-bar error.
- If reopening a historical URL fails, the existing session/backend error path is reused.
- If a stored resume position exceeds the playable duration, the restored seek position is clamped to the playable range.
- If playlist snapshot data is temporarily inconsistent, the sidebar falls back to no active highlight rather than panicking.

## Testing Strategy

Core tests:

- Playlist snapshot contract coverage.
- History restore command coverage.
- Resume-position clamp behavior.

Desktop tests:

- Sidebar view-model mapping for playlist rows and history rows.
- Current playlist highlight mapping.
- Command generation for playlist row activation.
- Command generation for history row activation.
- Sidebar visibility and active-tab state helpers where they can stay pure.

Manual smoke additions:

- Open a folder and confirm the playlist sidebar populates.
- Click playlist entries and confirm playback switches to the selected item.
- Restart the app and confirm recent history is visible.
- Click a history item and confirm playback resumes near the stored position.

## Architecture Boundaries

- `yoyo-core` owns typed playlist/history snapshots and restore semantics.
- `yoyovideo-desktop` owns sidebar UI state, row presentation, and click wiring.
- Slint stays declarative and mostly dumb. Row mapping and activation rules live in Rust, not in complex Slint logic.

## Acceptance Criteria

- The desktop app shows a right-side collapsible `Playlist / History` sidebar.
- The playlist tab reflects the current queue and highlights the active item.
- The history tab reflects persisted recent items with resume metadata.
- Clicking a playlist entry switches playback to that entry.
- Clicking a history entry reopens that media and resumes playback from saved position.
- Missing or invalid history entries fail clearly without crashing the app.
- The added logic is covered by pure Rust tests plus manual smoke checklist updates.

## Follow-On Phases

This design intentionally creates the shell that later phases can reuse for:

- settings and shortcut editing UI
- subtitle and track selection surfaces
- richer playlist management behavior
