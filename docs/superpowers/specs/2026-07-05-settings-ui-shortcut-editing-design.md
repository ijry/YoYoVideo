# Settings UI And Shortcut Editing Design

## Goal

Add a first usable settings surface to YoYoVideo so playback defaults, interface preferences, and keyboard shortcuts can be edited from the desktop app and persisted safely.

This phase is intentionally limited to a dedicated settings window, single-binding shortcut editing, validation, persistence, and runtime application rules for settings that can safely take effect immediately.

## Why This Phase Exists

The player already has working playback controls, real keyboard routing, persistent config storage, and a sidebar for playlist and history. What it still lacks is a usable way to inspect or change those preferences without editing configuration files manually.

The codebase already contains `AppConfig`, default shortcuts, and a minimal `SettingsController`, so the next highest-value step is to expose those existing settings through a stable desktop flow before moving on to subtitles and track selection.

## In Scope

- Add a dedicated settings window opened from the main player.
- Expose every current `AppConfig` field for editing:
  - `playback.default_speed`
  - `playback.default_volume_percent`
  - `playback.prefer_hardware_decode`
  - `ui.remember_history`
  - `ui.show_playlist_on_startup`
  - `shortcuts`
- Add single-binding shortcut editing with key-capture input.
- Support row-level shortcut clear and restore-default actions.
- Support window-level `Restore Defaults`, `Cancel`, `Apply`, and `OK`.
- Detect invalid values and shortcut conflicts before saving.
- Persist changes to `config.toml`.
- Apply safe runtime changes immediately after a successful save.
- Add pure Rust tests for settings draft behavior, validation, and runtime application boundaries.

## Out Of Scope

- Multi-binding shortcuts for a single action.
- Global hotkeys outside the player window.
- Importing or exporting shortcut presets.
- Editing actions that do not already exist in `ShortcutAction`.
- Subtitle, audio-track, or video-track selection UI.
- Theme customization or major visual redesign of the main player window.
- Live rebuilding of the mpv backend when hardware decode preference changes.

## Locked Decisions For This Phase

- The settings surface is a dedicated window, not a sidebar tab or full-window overlay.
- Settings use explicit save semantics: `Apply`, `OK`, and `Cancel`.
- Each action has at most one shortcut, and each shortcut can belong to at most one action.
- Shortcut editing uses direct key capture instead of free-form text as the primary input.
- Shortcut conflicts block saving instead of silently reassigning bindings.
- Settings that are safe to apply immediately do so after save; playback defaults only affect future playback state and future app launches.

## UI Direction

The settings surface is a compact desktop-style window that feels like a media-player configuration dialog rather than a generic web form.

Window layout:

- Left navigation rail with three sections: `Playback`, `Interface`, and `Shortcuts`.
- Right content panel showing the controls for the selected section.
- Bottom action bar with `Restore Defaults`, `Cancel`, `Apply`, and `OK`.
- Bottom status area for save errors and global validation messages.

Section contents:

- `Playback`
  - default speed control
  - default volume control
  - hardware decode preference toggle
- `Interface`
  - remember history toggle
  - show playlist on startup toggle
- `Shortcuts`
  - action label
  - current binding display
  - `Edit`
  - `Clear`
  - `Restore Default`

This phase does not add search, grouping by category beyond the three sections above, or complex per-row descriptions unless needed for clarity.

## Interaction Model

- Clicking `Settings` from the main window opens the dedicated settings window.
- Opening the window clones the current runtime config into a local draft model.
- All in-window edits change only the draft, not the live runtime config.
- When the draft differs from the current config, the window becomes dirty and enables `Apply`.
- `Apply` validates, saves, and applies runtime-safe changes but keeps the window open.
- `OK` runs the same save path as `Apply`, then closes the window on success.
- `Cancel` closes the window and discards the draft.
- `Restore Defaults` resets the whole draft to `AppConfig::default()`.
- Shortcut rows also support row-level restore to the default binding for just that action.

Shortcut capture flow:

- Clicking `Edit` on a shortcut row puts that row into a temporary capture state.
- The next accepted key combination is normalized into the stored gesture format.
- Pure modifier presses such as only `Ctrl` or only `Shift` are ignored.
- The captured binding updates the draft immediately.
- If the captured binding conflicts with another action, the row shows a conflict state and save remains blocked until fixed.

## Data Model And Flow

`yoyo-core::AppConfig` remains the only persisted configuration model.

Desktop additions:

- Add a draft-focused settings view model in `yoyovideo-desktop`.
- Keep the draft model responsible for:
  - editable field values
  - dirty tracking
  - validation errors
  - active shortcut capture state
- Keep persistence and final config application in Rust, not in Slint expressions.

Suggested responsibility split:

- `yoyo-core`
  - config serialization
  - typed config structures
  - shortcut parsing and default bindings
- `yoyovideo-desktop`
  - draft creation and mutation
  - shortcut capture state
  - conflict detection
  - settings window callbacks
  - runtime config replacement after successful save

Save flow:

1. Read the current runtime config and create a draft when the settings window opens.
2. Mutate the draft through section controls and shortcut-capture actions.
3. Convert the draft into a candidate `AppConfig`.
4. Run field validation and full shortcut conflict validation.
5. Persist the candidate config to `config.toml`.
6. If persistence succeeds, replace the live desktop runtime config and any immediately-applicable settings.
7. Refresh any affected UI state and status messaging.

## Validation Rules

- `default_speed` must stay within `0.25x..=4.0x`.
- `default_volume_percent` must stay within `0..=100`.
- Shortcut strings must be accepted through the existing shortcut parser, not trusted as raw UI strings.
- A shortcut may be unbound for an action if the user clears it.
- A shortcut may not be assigned to more than one action.
- Validation must run both during editing where possible and again at save time before writing the file.

Conflict behavior:

- Conflicts are surfaced on the affected shortcut rows.
- The bottom status area also shows a concise global error summary.
- `Apply` and `OK` fail while conflicts remain.

## Runtime Application Strategy

The settings window does not mutate playback state directly. It submits a validated config through a single desktop runtime application path.

Apply immediately after successful save:

- shortcut bindings used by keyboard dispatch
- `ui.remember_history`
- `ui.show_playlist_on_startup` as the stored startup preference for future launches

Do not force-apply to the active playback session:

- `playback.default_speed`
- `playback.default_volume_percent`
- `playback.prefer_hardware_decode`

These values become the new defaults for future playback behavior and future launches, but they do not interrupt or rewrite the currently playing session.

Specific runtime semantics:

- Turning off `remember_history` stops future history writes but does not delete the existing history file.
- Changing `show_playlist_on_startup` does not override the sidebar visibility the user already has in the current window.
- Changing default speed or volume affects newly created playback defaults, not the currently loaded media item.
- Changing hardware decode preference is stored for later runtime initialization rather than rebuilding the active backend in place.

## Error Handling

Errors are handled at three levels.

Field-level errors:

- Invalid speed value
- Invalid volume value
- Invalid captured shortcut

Shortcut-level errors:

- duplicate binding conflict
- unsupported or ignored capture input

Persistence/runtime errors:

- config file write failure
- config directory creation failure if the target path is not ready

Save ordering is strict:

1. Validate the draft.
2. Write the config file.
3. Apply runtime config updates.

This avoids split-brain state where the live session changes even though persistence failed.

Failure behavior:

- The window stays open.
- The draft remains intact.
- The last good live runtime config stays unchanged.
- The user can adjust values and retry saving.

## Testing Strategy

Pure Rust tests should cover the main behavior because the risk is in config semantics, not visual rendering.

Core logic coverage:

- draft dirty-state detection
- converting a draft into `AppConfig`
- restore-default behavior
- clearing a single shortcut
- shortcut conflict detection
- save rejection when conflicts exist
- runtime application split between immediate and future-only settings

Desktop contract coverage:

- opening settings clones the current config instead of mutating it in place
- `Cancel` discards unsaved changes
- `Apply` updates runtime config only after successful persistence
- shortcut capture ignores pure modifiers
- row-level restore resets to the product default binding

Manual smoke additions:

- Change a shortcut, restart the app, and confirm the new shortcut still works.
- Attempt to save conflicting shortcuts and confirm the save is blocked with a clear error.
- Disable history recording, play new media, and confirm no new history entry is written.
- Change default speed or volume, open new media, and confirm the new default is used without disturbing media already playing before the save.

## Architecture Boundaries

- `yoyo-core` continues to own persistent config types and shortcut parsing primitives.
- `yoyovideo-desktop` owns the settings draft, settings window wiring, shortcut capture, and runtime application policy.
- Slint stays declarative and receives simple view-model properties plus callbacks.
- The active `DesktopController` remains the only route for playback commands; settings do not bypass that boundary to mutate session internals ad hoc.

## Acceptance Criteria

- The player exposes a dedicated settings window from the main UI.
- Every current `AppConfig` field is editable from that window.
- Shortcut rows support edit, clear, and restore-default flows.
- Invalid values and duplicate shortcut bindings are clearly surfaced and block saving.
- `Apply`, `OK`, and `Cancel` have stable desktop-dialog semantics.
- Successful saves persist to `config.toml`.
- Shortcut changes and other safe runtime settings take effect without restarting the app.
- Playback defaults update future behavior without unexpectedly changing the media already playing.
- The added behavior is covered by automated Rust tests plus manual smoke checklist updates.

## Follow-On Phases

This design intentionally sets up boundaries that later phases can reuse for:

- subtitle and media-track settings
- richer shortcut customization such as multi-binding or import/export
- advanced playback defaults and end-of-file behavior
