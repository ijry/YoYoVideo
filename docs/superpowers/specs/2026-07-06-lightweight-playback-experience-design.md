# Lightweight Playback Experience Design

## Goal

Improve daily playback ergonomics without expanding YoYoVideo into a full media library. This phase adds window state restore, a recent-open menu, and configurable playback-end behavior. The work should preserve the current Rust + Slint + libmpv architecture and keep default behavior compatible with the current player.

## Scope

- Remember and restore the main window size, position, and maximized state when supported by the platform.
- Add recent-open entries for successfully opened local files, folders, and URLs.
- Add a playback-end setting with `Play next`, `Stop`, `Loop current`, and `Loop playlist`.
- Surface these controls through the existing settings window and context menu.
- Add contract tests and manual smoke coverage.

## Non-Goals

- No full media library, tagging, thumbnails, indexing, or database.
- No playlist save/load format in this phase.
- No global hotkeys or OS media key integration.
- No UI redesign beyond small settings/menu additions.
- No cloud sync or cross-device recent list.

## Approach

Use the existing boundaries:

- `yoyo-core` owns durable playback semantics, including playback-end behavior.
- `apps/yoyovideo-desktop` owns desktop persistence for window placement and recent-open entries.
- Slint UI only exposes properties/callbacks and delegates behavior to Rust.
- Settings save keeps existing strict validation and does not rebuild the mpv runtime.

This is intentionally incremental. It improves the startup and reopen loop that users hit every day while avoiding large playlist/media-library commitments.

## Window State Restore

Add a desktop-only window state store under the existing app data directory, separate from playback history and settings. The store records:

- `width`
- `height`
- `x`
- `y`
- `maximized`

The app restores the saved placement during startup after `MainWindow` creation and before normal user interaction. It updates the saved state from resize, move, maximize, and close paths using winit/Slint window events where available.

Safety rules:

- Ignore missing or corrupt window state files and continue with defaults.
- Clamp restored width and height to a usable minimum.
- If a saved position is clearly off-screen or cannot be applied, restore size only.
- If platform APIs do not expose a value reliably, skip that field instead of failing startup.

## Recent Open Menu

Add a desktop recent-open store under app data, independent from playback history. It records up to 10 most-recent successful opens:

```text
RecentOpenItem {
    kind: File | Folder | Url,
    target: string,
    title: string,
    opened_at: timestamp
}
```

Recent entries are updated only after a successful open dispatch:

- Open File adds a file item.
- Open Folder adds a folder item.
- Open URL adds a URL item.
- Dropping a single supported file adds that file.
- Dropping multiple files or a folder records the user-facing source when available. Mixed arbitrary drops do not create a synthetic recent item.

The context menu gains a compact `Recent` section below `Open Folder`. Each row re-dispatches through the same controller path used by normal open actions. Missing local files/folders show a non-fatal status message and do not clear the current playlist.

This store must not be controlled by `remember_history`. History is about playback resume metadata; recent-open is a convenience launch list.

## Playback-End Behavior

Add a `PlaybackEndBehavior` enum in `yoyo-core` config:

```text
PlaybackEndBehavior = PlayNext | Stop | LoopCurrent | LoopPlaylist
```

Default: `PlayNext`, preserving current behavior.

EOF behavior in `AppSession::poll_backend`:

- `PlayNext`: open the next playlist item if one exists; otherwise leave playback ended.
- `Stop`: do not advance; mark the state as paused/finished and show a concise status.
- `LoopCurrent`: restart the current media from the beginning.
- `LoopPlaylist`: open the next item; when the current item is the last playlist entry, wrap to index `0`.

The setting belongs in `AppConfig.playback`, is editable from the Settings window, and is applied immediately after save for future EOF handling. It does not interrupt currently playing media at save time.

Config compatibility must be explicit. New config fields use serde defaults so existing user config files continue to load.

## UI Changes

Main window:

- Context menu adds a `Recent` section with up to 10 entries.
- Empty recent list shows a disabled or status-only row such as `No Recent Items`.
- Selecting a recent row calls a new `recent_open_item_requested(index)` callback.

Settings window:

- Playback section adds a playback-end behavior control.
- Existing playback defaults remain unchanged.
- Save/apply behavior remains the same: validate, persist, update runtime config, refresh UI.

No visual companion is needed for this phase because the UI changes are small additions to existing menus and settings sections.

## Data Flow

Startup:

1. Load `AppConfig`.
2. Load playback history if enabled.
3. Load recent-open store.
4. Create `MainWindow`.
5. Apply saved window state best-effort.
6. Refresh recent menu rows and existing sidebar state.

Open actions:

1. Dialog/drop/menu callback resolves the target.
2. Existing controller opens file/folder/URL.
3. On success, desktop runtime records a recent-open item.
4. UI refreshes recent rows and status label.

EOF:

1. mpv emits `EndOfFile`.
2. `yoyo-mpv` maps it to `BackendEvent::EndOfFile`.
3. `AppSession::poll_backend` applies `PlaybackEndBehavior`.
4. Desktop refresh/persistence paths run as they do today.

Shutdown:

1. Flush playback history and subtitle preferences as today.
2. Save latest window state best-effort.
3. Recent-open store has already been updated after successful opens.

## Error Handling

- Recent store read failure: log warning after diagnostic logging is available and continue with an empty recent list.
- Recent store write failure: show a non-blocking status message only if it happens during an explicit user open; otherwise log and continue.
- Recent local path missing: show `Recent item is missing: <path>` and keep current playback.
- Window state restore failure: log diagnostic and continue with default placement.
- EOF loop command failure: surface through existing backend error path.

## Testing

Automated coverage:

- Config serialization loads older config files without new fields.
- `PlaybackEndBehavior` default is `PlayNext`.
- EOF contract tests for stop, next, loop current, and loop playlist.
- Settings controller snapshot/save preserves playback-end behavior.
- Recent-open store orders newest first, deduplicates by kind+target, caps at 10, and survives corrupt/missing files.
- Desktop controller/recent dispatch does not clear playback when a recent local path is missing.
- Slint compile contract for recent menu callback/property surface.
- Window state model tests for clamping and invalid position handling.

Manual smoke:

- Resize/move the window, close, restart, and confirm placement is restored.
- Maximize, close, restart, and confirm maximized state is restored.
- Open several files/folders/URLs and confirm recent menu ordering.
- Click a recent file/folder/URL and confirm it opens through the normal playback path.
- Remove a recent local file and confirm selecting it shows a non-fatal status message.
- Set each playback-end behavior and confirm EOF handling matches the selected mode.

## Acceptance Criteria

- Existing default behavior remains unchanged for users with no saved config.
- Existing config files continue to load.
- Recent-open convenience works even when playback history is disabled.
- Missing recent items and window-state persistence failures never crash or clear playback.
- Full `cargo test`, `cargo fmt --check`, and existing package smoke tests pass.
