# YoYoVideo Rust + Slint + libmpv Design

## Summary

YoYoVideo is a cross-platform local and network video player for Windows, macOS, and Linux. The first release targets a near-production MVP rather than a throwaway prototype. It uses Rust for the application shell and playback orchestration, Slint for native desktop UI, and libmpv as the embedded playback engine.

The product direction is a simple player UI with a PotPlayer-like control depth available through shortcuts and context menus. Qt is excluded from the chosen architecture because the project prioritizes a Rust-native UI stack and avoids default desktop widget aesthetics.

## Goals

- Support Windows, macOS, and Linux desktop platforms.
- Keep memory usage and runtime overhead low by using native UI and an embedded native playback engine.
- Support local media files, local folders, playlists, recent history, and network stream URLs.
- Support playback, pause, seeking, speed control, volume control, video zoom, audio channel switching, video rotation, A-B repeat, full screen, and keyboard shortcuts.
- Provide a stable near-production MVP with settings, error handling, packaging-ready structure, and a cross-platform validation checklist.

## Non-Goals For First Release

- Advanced subtitle styling and subtitle editor features.
- Video filters, color grading, recording, screen capture, or GIF export.
- Plugin scripting, online media aggregation, or resource crawling.
- Reimplementing decoding, demuxing, or rendering logic that libmpv already provides.
- Fully cloning every PotPlayer menu item or keyboard shortcut in the first release.

## Technical References

- Slint desktop platform support: https://docs.slint.dev/latest/docs/slint/guide/platforms/desktop/
- Slint winit backend: https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit/
- Slint winit window accessor: https://docs.slint.dev/latest/docs/rust/slint/winit_030/trait.WinitWindowAccessor
- libmpv embedding API: https://github.com/mpv-player/mpv/blob/master/DOCS/man/libmpv.rst
- mpv manual: https://mpv.io/manual/stable/

## Architecture

The application is split into four layers.

### app-shell

`app-shell` is the Rust process entry and coordinator. It starts logging, loads configuration, initializes the Slint UI, creates the player service, wires command routing, and owns the top-level window lifecycle.

It does not directly implement playback operations. It translates UI actions, menu actions, and keyboard shortcuts into commands for `player-core`.

### ui-slint

`ui-slint` owns the visual interface:

- Main video area.
- Bottom playback control bar.
- Collapsible playlist panel.
- Context menu.
- URL open dialog.
- Settings view.
- Shortcut hint display.

The UI emits intent events such as `PlayPause`, `OpenFile`, `OpenUrl`, `SetSpeed`, or `SetRotation`. It does not call libmpv directly.

### player-core

`player-core` is the playback service. It wraps libmpv, exposes a stable Rust command API, and converts mpv state changes into application-level `PlayerState` updates.

Its public commands include:

- `open_file(path)`
- `open_folder(path)`
- `open_url(url)`
- `toggle_pause()`
- `seek_relative(seconds)`
- `seek_absolute(seconds)`
- `set_speed(rate)`
- `set_volume(percent)`
- `set_audio_channel(mode)`
- `set_rotation(degrees)`
- `set_video_zoom(scale)`
- `set_ab_loop_a()`
- `set_ab_loop_b()`
- `clear_ab_loop()`
- `set_fullscreen(enabled)`

The service keeps all libmpv calls behind this API so UI and application code do not depend on mpv-specific property names.

### platform-bridge

`platform-bridge` contains thin platform-specific adapters:

- Native window handle access for video rendering.
- File and folder dialogs.
- System configuration and cache directories.
- Window-level shortcut registration.
- Packaging-specific runtime library discovery.

Business logic must not live in this layer.

## Playback Engine Design

Playback is implemented as a single command entry with a background mpv event pump.

The command side receives application-level commands:

- `OpenFile(PathBuf)`
- `OpenFolder(PathBuf)`
- `OpenUrl(String)`
- `TogglePause`
- `SeekRelative(f64)`
- `SeekAbsolute(f64)`
- `SetSpeed(f64)`
- `SetVolume(f64)`
- `SetAudioChannel(AudioChannelMode)`
- `SetRotation(i32)`
- `SetVideoZoom(f32)`
- `SetABLoopPointA`
- `SetABLoopPointB`
- `ClearABLoop`
- `ToggleFullscreen`

The event pump observes mpv events and properties, then publishes `PlayerState` updates:

- pause state
- current time
- duration
- speed
- volume
- track list
- selected audio track
- audio channel mode
- video rotation
- video zoom
- buffering/loading state
- end-of-file state
- last playback error

This keeps buttons, shortcuts, context menu items, and future command palette actions on the same behavior path.

## Video Rendering Strategy

The first release should avoid decoding frames manually and avoid copying video frames through the UI layer. libmpv should own video rendering and bind to a native video surface or window region exposed by the desktop window integration.

Slint owns application chrome and controls. The video surface is treated as a dedicated embedded rendering target below or beside the Slint control layer. If direct composition with Slint proves unstable on a target platform, the fallback is a dedicated native child or sibling video window that is positioned and resized with the main Slint window.

This design prioritizes playback performance and reduces the risk of high CPU/GPU overhead from frame copying.

## Feature Mapping

Playback and pause map to mpv pause control.

Seeking supports relative seeking from shortcuts and absolute seeking from the progress bar.

Speed supports a fixed first-release set: `0.5x`, `0.75x`, `1.0x`, `1.25x`, `1.5x`, and `2.0x`.

Video zoom is view-level zoom and maps to mpv video zoom behavior. It is separate from resizing the application window.

Audio channel switching supports `stereo`, `mono-left`, and `mono-right` in the first release.

Video rotation supports `0`, `90`, `180`, and `270` degrees.

A-B repeat uses two explicit loop points. Users can set point A, set point B, and clear the loop.

Network playback uses a single `open_url` flow. The first release accepts HTTP, HTTPS, RTSP, and RTMP URLs when the bundled libmpv/FFmpeg build supports those protocols.

Hardware acceleration is best-effort. The application should prefer hardware decoding and rendering when available, but automatically fall back to stable software decoding when hardware initialization fails.

## Keyboard Shortcuts

Default shortcuts:

- `Space`: play or pause.
- `Left`: seek backward 5 seconds.
- `Right`: seek forward 5 seconds.
- `Up`: increase volume.
- `Down`: decrease volume.
- `[`: decrease speed.
- `]`: increase speed.
- `0`: reset speed to `1.0x`.
- `A`: set A-B repeat point A.
- `B`: set A-B repeat point B.
- `Ctrl+A`: clear A-B repeat.
- `R`: rotate clockwise by 90 degrees.
- `Z`: zoom out.
- `X`: zoom in.
- `C`: switch audio channel mode.
- `F`: toggle full screen.
- `O`: open file.
- `U`: open URL.

Shortcut handling must route through the same command bus as UI buttons and context menu items.

The settings screen must allow shortcuts to be changed. Shortcut conflicts must be detected before saving.

## UI Design

The UI follows a simple player layout with professional controls available on demand:

- A black video canvas occupies most of the window.
- A bottom control bar contains play/pause, progress, time, volume, speed, loop state, and full screen.
- A collapsible playlist panel contains local files, opened URLs, and recent items.
- A context menu exposes open actions, playback controls, speed, rotation, audio channel switching, A-B repeat, full screen, and settings.
- The settings view contains shortcut mapping, default playback behavior, hardware acceleration policy, recent history behavior, and default volume.

The UI should not use generic default desktop styling. It should define a small visual language with deliberate colors, spacing, icons, and hover states while staying lightweight.

## Configuration And Storage

Configuration is stored in the system configuration directory, not beside the executable.

`config.toml` stores:

- shortcut mappings
- hardware acceleration policy
- default volume
- default speed
- UI preferences
- playback behavior defaults

`history.json` stores:

- recently opened local files
- recently opened URLs
- last playback position per item
- last selected item when the application exits

Malformed configuration must not prevent the application from starting. The app should preserve the bad file for inspection, load defaults, and show a concise warning.

## Error Handling

File open failures must show the failed path and a concise playback error summary.

Network failures must distinguish invalid URL, connection failure, unsupported protocol, and unsupported media format when the error information is available.

libmpv initialization failure must report that the playback runtime failed to initialize and suggest checking bundled runtime files.

Hardware acceleration failure must fall back to software decoding and show a non-blocking status message.

End-of-file behavior depends on playlist settings. The first release supports stop-at-end and play-next-item behavior.

## Testing Strategy

`player-core` has unit tests for command mapping, state translation, and boundary conditions such as invalid speed values or invalid rotation values.

`config` has tests for TOML and JSON serialization, malformed configuration fallback, and default value loading.

`shortcut` has tests for parsing, display formatting, duplicate detection, and command mapping.

The UI layer is covered by Slint preview checks and a manual acceptance checklist rather than heavy UI automation in the first release.

Cross-platform validation must cover:

- application startup
- local file playback
- local folder playlist creation
- URL playback
- play/pause
- seek
- volume
- speed
- video zoom
- audio channel switching
- rotation
- A-B repeat
- full screen
- shortcut handling
- settings persistence
- recent history restore

## Packaging And Runtime Dependencies

The application should be structured for platform packaging from the start, but packaging polish can be finalized after core playback is stable.

The libmpv runtime and its dependent libraries must be bundled or discovered deterministically per platform. The app should not rely on users manually installing mpv unless a developer-mode flag explicitly enables system libmpv loading.

Licensing must be reviewed before distribution because libmpv and its FFmpeg build options affect redistribution obligations.

## Acceptance Criteria

- The app starts on Windows, macOS, and Linux.
- A user can open and play a local media file.
- A user can open a URL and attempt network stream playback.
- Playback controls work from both UI and keyboard shortcuts.
- Speed, zoom, audio channel switching, rotation, and A-B repeat are available from keyboard shortcuts and context menu entries.
- Playback state updates are visible in the UI without blocking playback.
- Configuration and history persist across restarts.
- Hardware acceleration failure does not crash the app.
- Playback errors are shown as actionable user-facing messages.

