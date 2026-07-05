# Video Host And Control Surface Design

## Goal

Turn the current control-only playback alpha into a visible player alpha by embedding a native video host for libmpv and completing the primary playback controls and keyboard shortcut routing in the Slint desktop UI.

This phase combines the user's selected priorities:

- `1`: real video picture display.
- `2`: practical player controls and shortcuts.

## Current State

The project already has:

- A Rust workspace with `yoyo-core`, `yoyo-mpv`, and `yoyovideo-desktop`.
- A real `MpvClient` and `MpvBackend` behind the `mpv-runtime` feature.
- Pure command translation for open, pause, seek, speed, volume, audio channel, rotation, zoom, and A-B loop.
- mpv event mapping for pause, position, duration, speed, volume, rotation, warnings, errors, and EOF.
- A Slint main window with a black video rectangle and basic callbacks.
- Pure shortcut mapping tests through `dispatch_shortcut`.
- Packaging scripts and GitHub Actions foundation.

The project is still missing:

- A real native rendering target for mpv video.
- Binding the mpv video output to a window handle.
- Resizing and positioning the video target with the Slint video area.
- Real window keyboard event routing.
- Progress seeking, volume control, richer labels, and visible state for zoom, rotation, audio channel, and A-B repeat.
- Manual smoke evidence with runtime files and visible video.

## Scope

This phase includes:

- Adding a `VideoHost` abstraction in `yoyovideo-desktop`.
- Creating a native child video window through Slint's winit integration where the platform supports it.
- Exporting video-area geometry from `main-window.slint` so Rust can keep the native video host aligned with the UI.
- Adding mpv runtime options so a video window id can be passed before mpv initialization.
- Binding mpv to the video host with the mpv `wid`/window-id option.
- Reworking desktop startup so the UI can initialize first, then create the video host and runtime backend once winit has a parent window.
- Adding actual winit keyboard event routing to the existing shortcut command path.
- Expanding the main UI with progress seeking, volume, speed, zoom, rotation, audio channel, A-B loop, fullscreen, and status controls.
- Updating docs and smoke checklist for visible video and shortcut validation.

This phase excludes:

- Deep Slint renderer composition with `mpv_render_context`.
- CPU frame-copy rendering into Slint images.
- Advanced subtitle UI, filters, capture, plugin scripting, or full PotPlayer menu parity.
- Signed installers or public runtime redistribution.
- Claiming macOS and Wayland video-host support without a verified native host path.

## Chosen Approach

Use a native video host window and keep Slint as the control surface.

Slint continues to draw the player chrome, controls, playlist/status labels, and dialogs. A platform video host owns the video rectangle. mpv renders directly into that host, avoiding high CPU frame copying and avoiding fragile deep integration with Slint's renderer internals.

The primary implementation path is:

1. Select the Slint winit backend.
2. Install a winit custom application handler.
3. Let Slint create the main window.
4. Create a child video host window using the main window's raw parent handle.
5. Pass the child host id into `MpvClientOptions` before calling `mpv_initialize`.
6. Keep the child host bounds synchronized with the Slint video area.
7. Route Slint buttons and winit keyboard events through the same `DesktopController` command path.

## Rejected Approaches

### mpv OpenGL Render API Inside Slint

`libmpv-sys` exposes `mpv_render_context`, but Slint 1.17.0 does not provide a stable, simple public hook for an application-owned mpv OpenGL renderer to draw into an item inside the generated Slint scene. This path risks long renderer-specific work before the app becomes usable.

### CPU Frame Copy To Slint Image

Copying decoded frames into Slint images would be easier to compose visually, but it conflicts with the project's low memory and high-performance requirement. It would also duplicate work libmpv already performs efficiently.

## Architecture

### `yoyo-mpv`

`yoyo-mpv` remains the playback engine adapter. It gains initialization options, but it does not create windows and does not depend on Slint or winit.

New concepts:

- `MpvClientOptions`: safe configuration passed before mpv initialization.
- `MpvVideoWindow`: a simple value containing the mpv-compatible native window id.
- `MpvBackend::new_runtime_with_options(options)`: creates a runtime backend with optional video binding.

`MpvClient::new()` remains available and delegates to `MpvClient::new_with_options(MpvClientOptions::default())`.

### `yoyovideo-desktop`

`yoyovideo-desktop` owns platform integration:

- `video_host.rs`: common `VideoHost` trait, `VideoHostBounds`, `VideoHostError`, and `NativeVideoWindowId`.
- `video_host_winit.rs`: winit-backed child video host creation and bounds synchronization.
- `keyboard.rs`: maps winit keyboard events to the existing shortcut gesture strings.
- `app.rs`: coordinates deferred runtime backend creation, UI refresh, video host synchronization, and shortcut dispatch.

The existing `DesktopController<B>` remains responsible for command dispatch and session state. It should not know how a platform window is created.

## Startup Flow

The current startup creates `MpvBackend` before the Slint/winit window exists. That is too early for video embedding because mpv should receive the video window id before initialization.

The new flow:

1. Configure the Slint winit backend with `with_winit_custom_application_handler`.
2. Create `MainWindow`.
3. Initialize a `DesktopRuntime` with no player session yet.
4. Show/run the window.
5. When the winit parent window is available, create the video host.
6. Create `MpvBackend::new_runtime_with_options(MpvClientOptions { video_window: Some(host.mpv_window()) })`.
7. Create `AppSession` and `DesktopController`.
8. Enable playback controls and update the status label to `Ready`.

If video host creation fails on a platform, the app remains open in control-only mode and shows an actionable status message. It must not silently claim video embedding is active.

## Video Host

`VideoHost` exposes a small interface:

- `mpv_window_id() -> NativeVideoWindowId`
- `set_bounds(VideoHostBounds) -> Result<(), VideoHostError>`
- `show() -> Result<(), VideoHostError>`
- `hide() -> Result<(), VideoHostError>`
- `is_available() -> bool`

`VideoHostBounds` is expressed in physical pixels:

- `x`
- `y`
- `width`
- `height`

Slint exports the logical video rectangle:

- `video_area_x`
- `video_area_y`
- `video_area_width`
- `video_area_height`

Rust converts logical coordinates with the current winit scale factor. The host is resized on:

- main window creation,
- main window resize,
- scale factor change,
- fullscreen toggles,
- periodic fallback polling when geometry changes are missed.

The video host must never overlap the bottom controls. This is a hard requirement because native child windows may paint above Slint content on some platforms.

## Platform Behavior

### Windows

Windows is a required target for the first visible-video implementation. The video host uses a child native window handle compatible with mpv's `wid` option. The host is clipped to the Slint video area and resizes with the main window.

### Linux X11

Linux X11 is a required design target and uses an X11 window id compatible with mpv's `wid` option. If the runtime session is Wayland and child-window embedding is unavailable, the app reports that visible embedding is not supported in the current session and keeps controls available.

### macOS

macOS keeps the same `VideoHost` interface, but implementation must be guarded by capability detection. If the adapter cannot produce a verified mpv-compatible host id, visible embedding is disabled with a clear status message. This avoids shipping fake cross-platform support.

## mpv Binding

`MpvClientOptions` includes:

- `video_window: Option<MpvVideoWindow>`
- `force_window: bool`
- `profile: Option<String>`

When `video_window` is present, `MpvClient` sets mpv options before `mpv_initialize`:

- `wid` to the platform window id.
- `force-window` to `yes`.

The existing command translation remains unchanged. Playback commands continue to flow through `PlayerBackend`.

Runtime errors from window binding are converted into `MpvError::VideoOutput(String)` or another explicit mpv runtime error variant. They must surface in the UI status label and through `AppError::Message` at the desktop boundary.

## Control UI

The main window evolves from the current basic layout into a usable player surface:

- Video area with native host geometry exports.
- Play/pause button.
- Open file, open folder, and URL entry.
- Progress slider with time label.
- Volume slider.
- Speed display and speed down/up/reset controls.
- Zoom in/out controls.
- Rotation control.
- Audio channel cycle control.
- A-B loop set A, set B, and clear controls.
- Fullscreen control.
- Status/error label.

The UI should remain lightweight and avoid heavy widget styling. The visual direction stays close to a media player: dark video canvas, compact control bar, clear status labels, and no large decorative surfaces.

## Keyboard Routing

Slint's winit integration exposes `on_winit_window_event`. The app uses it to translate physical keyboard events into existing shortcut gestures.

The keyboard mapper handles:

- `Space`
- `Left`
- `Right`
- `Up`
- `Down`
- `[`
- `]`
- `0`
- `A`
- `B`
- `Ctrl+A`
- `R`
- `Z`
- `X`
- `C`
- `F`
- `O`
- `U`

The mapper ignores key release events and repeated events unless the command is safe to repeat, such as seeking and volume adjustment.

Keyboard shortcuts must not fire while the URL input is focused. `main-window.slint` exposes URL focus state to Rust so typed URL text is not interpreted as player commands.

## Fullscreen

Fullscreen remains an application-level command. In this phase it must affect both:

- `PlayerState.fullscreen`.
- The actual winit main window fullscreen state.

The video host bounds are recomputed after fullscreen changes.

## Error Handling

User-facing errors should be concise:

- Missing libmpv runtime: show the existing runtime initialization error.
- Unsupported video host: show `Video embedding is not supported on this windowing backend yet`.
- Failed video host creation: show the platform error message.
- Failed mpv window binding: show `mpv video output could not attach to the video host`.
- Runtime command errors: show the existing backend command error.

The app should stay open when video embedding fails. Opening files without video embedding may still exercise audio/control paths, but the status label must make the limitation clear.

## Testing Strategy

Automated tests must still pass without libmpv.

Unit and integration tests cover:

- `MpvClientOptions` default values and video-window option formatting without loading libmpv.
- `VideoHostBounds` logical-to-physical conversion.
- Keyboard event mapping into gesture strings.
- Shortcut dispatch from mapped gestures into `AppCommand`.
- URL input focus suppresses shortcut dispatch.
- `DesktopRuntime` reports unsupported video host without panicking.
- UI presenter labels for progress, volume, speed, rotation, audio channel, zoom, and A-B loop.

Runtime feature checks cover:

- `cargo check -p yoyo-mpv --features mpv-runtime`
- `cargo check -p yoyovideo-desktop --features mpv-runtime`

Manual smoke tests cover:

- Launch with staged libmpv runtime.
- Confirm video is visible inside the video area and does not cover controls.
- Resize the window and confirm video bounds track the UI.
- Toggle fullscreen and confirm the video host resizes.
- Open a local file and URL.
- Use buttons and shortcuts for play/pause, seek, volume, speed, zoom, rotation, audio channel, A-B loop, and fullscreen.
- Confirm typing into the URL box does not trigger playback shortcuts.

## Success Criteria

- The desktop UI can create or clearly reject a native video host.
- On supported platforms, mpv receives a native video window id before playback starts.
- A local video file can display visible video inside the Slint video area when `mpv-runtime` and runtime files are available.
- The video host stays aligned with the Slint video rectangle during resize and fullscreen transitions.
- Primary playback controls are visible and routed through `DesktopController`.
- Keyboard shortcuts are routed from real window events and share the same command path as buttons.
- URL text input does not trigger player shortcuts while focused.
- Default `cargo test` still passes without libmpv.
- Runtime feature `cargo check` still passes.
- Smoke checklist and runtime docs describe visible-video validation and platform limitations.

## Relationship To Remaining Work

After this phase, the player should be visibly playable on supported windowing backends, but it still will not be a complete PotPlayer-class application.

Remaining work after this phase includes:

- Full playlist panel UI.
- Recent history UI and resume workflows.
- Complete settings UI for shortcut editing and playback defaults.
- Subtitle controls.
- Hardware acceleration fallback messaging backed by observed mpv state.
- Verified macOS and Wayland video-host implementations if they are not completed in this phase.
- Signed installers and public release automation after runtime licensing review.
