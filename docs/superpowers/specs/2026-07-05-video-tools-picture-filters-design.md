# Video Tools, Screenshot, Frame Step, And Picture Filters Design

## Goal

Add a practical video-tools surface to YoYoVideo for screenshot capture, frame-by-frame navigation, preset video filters, and picture-parameter enhancement while preserving the existing Rust + Slint + libmpv architecture.

This phase targets high-value playback-time tools that users expect from desktop players such as PotPlayer, without introducing a full filter editor or plugin system.

## Current State

The player already has:

- `yoyo-core` command, backend, session, and player-state boundaries.
- `yoyo-mpv` translation from typed backend commands into mpv commands and properties.
- A Slint main window with transport, seek, speed, volume, zoom, rotation, audio-channel, A-B loop, playlist/history, and tracks/subtitle controls.
- Configurable keyboard shortcuts routed through the same command path as UI buttons.
- Runtime packaging and smoke checks for staged libmpv.

The player is still missing:

- Screenshot capture from the current video frame.
- Single-frame forward and backward stepping.
- User-facing picture parameters such as brightness, contrast, saturation, gamma, and hue.
- A controlled set of video filter presets.
- UI state and tests for these tools.

## In Scope

- Add a `Video Tools` popup to the main window.
- Capture screenshots to a default screenshots folder under the user's system pictures directory.
- Generate screenshot filenames automatically.
- Add next-frame and previous-frame controls.
- Add picture sliders for:
  - brightness
  - contrast
  - saturation
  - gamma
  - hue
- Add a reset action for all picture parameters.
- Add safe preset filters:
  - none
  - sharpen
  - light denoise
  - grayscale
  - invert
- Add keyboard shortcuts for:
  - screenshot
  - next frame
  - previous frame
- Surface screenshot and filter failures through existing status messaging.
- Add automated tests for core commands, mpv translation, shortcut dispatch, controller behavior, and presenter/view-model formatting.
- Update manual smoke coverage for the new tools.

## Out Of Scope

- Arbitrary user-authored mpv `vf` filter-chain editing.
- Filter profile persistence.
- Per-media picture-parameter persistence.
- Image-gallery browsing or screenshot management UI.
- Screenshot format selection.
- Region capture or GIF/video clip export.
- GPU shader management, LUT import, SVP interpolation, HDR tone-mapping UI, or plugin scripting.

## Locked Decisions For This Phase

- Filters are preset-based for the first version.
- Screenshots are saved automatically; there is no per-shot save dialog in this phase.
- The default screenshot directory is `Pictures/YoYoVideo Screenshots` when the OS exposes a pictures directory.
- If the system pictures directory cannot be resolved, the desktop app falls back to an app-owned screenshots directory and reports the actual path in status text.
- Picture parameter changes are session-level playback controls, not saved settings.
- The UI is a popup, not a new settings page or sidebar tab.
- The popup sends typed app commands; Slint does not call mpv or mutate core state directly.

## UI Direction

The main control surface gains a `Video Tools` entry near the existing `Menu` and `Tracks` buttons. This opens a popup with compact live controls.

Popup layout:

- `Capture`
  - `Screenshot`
  - status text for the latest saved path or error
- `Frame Step`
  - `Prev Frame`
  - `Next Frame`
- `Picture`
  - brightness slider
  - contrast slider
  - saturation slider
  - gamma slider
  - hue slider
  - `Reset Picture`
- `Filter Preset`
  - one button per supported preset

Controls apply immediately. There is no `Apply` button because these are playback-time tools.

The main transport bar should stay compact. Only the `Video Tools` entry is added there; the detailed controls live inside the popup.

## Interaction Model

Screenshot flow:

1. User presses the screenshot button or shortcut.
2. Desktop resolves and creates the screenshot directory if needed.
3. Desktop generates a filename using local date/time and a collision suffix when required.
4. The command is sent through `AppCommand` and `BackendCommand`.
5. mpv writes the current video frame to that file.
6. The UI status label reports the saved path or a concise failure message.

Frame-step flow:

1. User presses previous-frame or next-frame from the popup or shortcut.
2. The command is routed through the session/backend path.
3. mpv performs `frame-step` or `frame-back-step`.
4. Position updates continue to come from backend events.

Picture-parameter flow:

1. User moves a slider.
2. The desktop layer clamps the value to the supported range.
3. `AppSession` updates `PlayerState.video_adjustments`.
4. The backend receives a typed parameter command.
5. mpv applies the matching property.
6. Backend-observed values may later refresh state when property observation is added.

Filter-preset flow:

1. User selects a preset.
2. `AppSession` updates the active preset in player state.
3. The backend receives a typed preset command.
4. mpv replaces the YoYoVideo-owned video filter slot.
5. The UI shows the selected preset from core state.

## Data Model And Flow

`yoyo-core` owns the public model for these tools.

Core additions:

- `VideoAdjustmentKind` for brightness, contrast, saturation, gamma, and hue.
- `VideoAdjustments` struct storing the current parameter values.
- `VideoFilterPreset` enum for none, sharpen, light denoise, grayscale, and invert.
- `FrameStepDirection` enum for previous and next.
- Screenshot commands carry a direct `PathBuf` target generated by the desktop layer.
- `PlayerState.video_adjustments`.
- `PlayerState.video_filter_preset`.
- Screenshot success and failure messages use the existing player status-message path instead of adding a dedicated screenshot state field.

Command additions:

- `AppCommand::TakeScreenshot(PathBuf)`.
- `AppCommand::StepFrame(FrameStepDirection)`.
- `AppCommand::SetVideoAdjustment(VideoAdjustmentKind, i16)`.
- `AppCommand::ResetVideoAdjustments`.
- `AppCommand::SetVideoFilterPreset(VideoFilterPreset)`.
- Matching `BackendCommand` variants.

Desktop additions:

- Screenshot path resolver in the desktop platform layer.
- Popup-facing label/row helpers for adjustment values and filter presets.
- Callback wiring from Slint controls into `DesktopController`.
- Shortcut dispatch for screenshot and frame stepping.

The existing command path remains authoritative:

`Slint callback or shortcut -> DesktopController -> AppSession -> BackendCommand -> yoyo-mpv -> libmpv`.

## Parameter Ranges

Picture parameters use mpv-compatible integer ranges and default to `0`:

- brightness: `-100..=100`
- contrast: `-100..=100`
- saturation: `-100..=100`
- gamma: `-100..=100`
- hue: `-100..=100`

The desktop UI clamps values before dispatch. Core session handling also clamps values so keyboard or future callers cannot bypass validation.

Reset sets every parameter to `0` and sends backend commands for each property.

## mpv Mapping

Screenshot capture maps to a screenshot-to-file command with the generated path.

Frame stepping maps to mpv commands:

- next frame: `frame-step`
- previous frame: `frame-back-step`

Picture parameters map to mpv properties:

- `brightness`
- `contrast`
- `saturation`
- `gamma`
- `hue`

Filter presets are implemented as a YoYoVideo-owned video-filter chain update. The first version replaces the app-owned filter preset instead of appending unbounded filters. This prevents repeated clicks from stacking duplicate filters.

Preset intent:

- `none`: remove the YoYoVideo-owned preset filter.
- `sharpen`: apply a mild sharpening filter.
- `light denoise`: apply conservative denoise suitable for noisy sources without heavy CPU cost.
- `grayscale`: apply grayscale conversion.
- `invert`: invert luma/color for accessibility/testing use.

Exact mpv filter expressions are implementation details of `yoyo-mpv` and are covered by translation tests.

## Keyboard Shortcuts

Add shortcut actions:

- `TakeScreenshot`
- `FrameStepBackward`
- `FrameStepForward`

Default bindings:

- screenshot: `S`
- previous frame: `,`
- next frame: `.`

The existing URL-focus suppression remains in effect so typing into the URL field does not trigger these actions.

The settings shortcut editor automatically includes these actions through `ShortcutAction::all()`.

## Error Handling

Screenshot errors:

- Directory creation failures surface as a status message and no backend screenshot command is sent.
- Backend screenshot failures surface through existing backend error handling.
- Filename collisions are handled by suffixing the generated filename before dispatch.

Frame-step errors:

- Backend command failures surface in the status label.
- Frame-step actions are no-ops from the UI perspective when no media is loaded, except for any backend error already returned.

Picture and filter errors:

- Invalid adjustment values are clamped.
- Filter-preset application failures surface in the status label.
- The UI does not pretend a filter was applied if the backend returns an immediate command error.

General rule:

- Playback must continue unless mpv itself reports a fatal playback error.

## Testing Strategy

Core tests:

- Default player state includes neutral video adjustments and `none` filter preset.
- Session maps screenshot, frame-step, adjustment, reset, and filter app commands to backend commands.
- Adjustment values clamp to `-100..=100`.
- Reset emits neutral adjustment commands and updates state.

mpv tests:

- Screenshot command translates to screenshot-to-file with the requested path.
- Frame-step commands translate to `frame-step` and `frame-back-step`.
- Picture parameters translate to the expected mpv properties.
- Filter presets translate to deterministic video-filter commands.
- `none` clears the YoYoVideo-owned filter preset.

Desktop tests:

- Shortcut dispatch maps `S`, `,`, and `.` to the new commands.
- Custom shortcut binding still overrides defaults.
- Screenshot path generation creates stable names and collision suffixes.
- Presenter helpers format adjustment and preset labels.
- Controller forwards the new commands through the existing dispatch path.

Manual smoke additions:

- Open a local video and take a screenshot.
- Confirm the screenshot file appears under the selected screenshots directory.
- Step forward and backward while paused.
- Move every picture slider and confirm visible changes.
- Reset picture parameters and confirm the image returns to neutral.
- Select each filter preset and confirm it applies.
- Select `none` and confirm the preset filter is removed.
- Confirm screenshot and frame-step shortcuts work.
- Confirm the shortcuts do not fire while typing in the URL box.

## Architecture Boundaries

- `yoyo-core` owns typed state, commands, clamping, and default values.
- `yoyo-mpv` owns mpv command/property/filter translation.
- `yoyovideo-desktop` owns screenshot file-path policy, Slint popup wiring, shortcut dispatch, and UI formatting.
- Slint owns only declarative controls and callbacks.

This keeps advanced video tools testable without libmpv and avoids coupling the UI directly to mpv-specific strings.

## Acceptance Criteria

- The main window exposes a `Video Tools` popup.
- Users can take screenshots saved automatically to the default screenshot directory.
- Users can step one frame forward and one frame backward.
- Users can adjust brightness, contrast, saturation, gamma, and hue.
- Users can reset all picture parameters to neutral.
- Users can select the supported video filter presets and return to `none`.
- Screenshot, previous-frame, and next-frame shortcuts are available and configurable.
- New commands use the existing controller/session/backend architecture.
- Errors are non-blocking and visible through status text.
- Default `cargo test` passes without requiring libmpv.
- Runtime feature checks still compile.
- Manual smoke checklist covers the new tools.

## Follow-On Phases

Later phases can build on these boundaries for:

- Persistent picture profiles.
- Per-media picture/filter restoration.
- Arbitrary advanced filter-chain editing with validation.
- LUT and shader preset management.
- Screenshot format and destination preferences.
- Clip export and thumbnail gallery workflows.
