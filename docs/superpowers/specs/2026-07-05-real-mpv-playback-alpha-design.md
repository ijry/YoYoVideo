# Real mpv Playback Alpha Design

## Goal

Build the first truly playable YoYoVideo alpha by replacing the placeholder mpv backend with a real `libmpv` client behind the existing `PlayerBackend` trait. This phase must prove local files and supported URLs can be opened, controlled, and observed through mpv without requiring Slint video-surface embedding yet.

## Scope

This phase includes:

- Creating and initializing a real mpv handle when the `mpv-runtime` feature is enabled.
- Sending open, pause, seek, speed, volume, audio-channel, rotation, zoom, and A-B loop commands to mpv.
- Observing mpv playback state and converting it into existing `BackendEvent` values.
- Surfacing deterministic startup and runtime errors when libmpv is unavailable or command execution fails.
- Keeping testable command translation and event mapping separate from unsafe FFI calls.

This phase excludes:

- Slint texture or native-window video embedding.
- Cross-platform packaging of bundled libmpv binaries.
- Advanced subtitles, filters, capture, scripting, or plugin features.
- Full PotPlayer parity.

## Architecture

`yoyo-core` remains the stable domain boundary. `AppSession` continues to own player state, playlist behavior, and command dispatch through `PlayerBackend`.

`yoyo-mpv` gains two backend implementations:

- `MpvBackend`: the production backend compiled with `mpv-runtime`, backed by `libmpv-sys`.
- `DryRunMpvBackend`: a non-runtime backend used by tests and developer builds that records translated actions but never pretends playback is real.

The current `MpvBackend::default()` placeholder behavior will be removed or renamed so production naming does not hide the absence of libmpv. Desktop startup must explicitly choose the real backend only when runtime support is enabled.

## Components

### `MpvClient`

`MpvClient` owns the raw mpv handle and is responsible for all unsafe FFI. It exposes safe methods:

- `new() -> Result<Self, MpvError>`
- `command(&mut self, args: &[&str]) -> Result<(), MpvError>`
- `set_flag(&mut self, name: &str, value: bool) -> Result<(), MpvError>`
- `set_string(&mut self, name: &str, value: &str) -> Result<(), MpvError>`
- `set_i64(&mut self, name: &str, value: i64) -> Result<(), MpvError>`
- `set_f64(&mut self, name: &str, value: f64) -> Result<(), MpvError>`
- `observe_properties(&mut self) -> Result<(), MpvError>`
- `drain_events(&mut self) -> Vec<Result<MpvEvent, MpvError>>`

`Drop` must terminate and destroy the mpv handle exactly once.

### `MpvAction` Execution

`translate_open()` and `translate_command()` remain pure. A new executor maps `MpvAction` into `MpvClient` calls. This keeps all command behavior unit-testable without loading libmpv.

### Event Mapping

`MpvEvent` represents decoded mpv events and observed property changes inside `yoyo-mpv`. A mapper converts `MpvEvent` to `BackendEvent`.

The alpha observes:

- `pause` -> `BackendEvent::PauseChanged`
- `time-pos` -> `BackendEvent::PositionChanged`
- `duration` -> `BackendEvent::DurationChanged`
- `speed` -> `BackendEvent::SpeedChanged`
- `volume` -> `BackendEvent::VolumeChanged`
- `eof-reached` or end-file event -> `BackendEvent::EndOfFile`
- mpv warnings/errors -> `BackendEvent::Warning` or `BackendEvent::Error`

## Desktop Behavior

The Slint UI can keep its current placeholder video rectangle. This phase is still valuable because real playback can be heard and controlled once mpv is initialized, and all player commands will exercise the real engine.

Desktop startup behavior:

- If built with `mpv-runtime`, construct the real `MpvBackend`.
- If libmpv initialization fails, show the error in the UI status label and return an actionable error from `run()`.
- If built without `mpv-runtime`, do not silently run fake playback in release-oriented paths. Developer tests can use `DryRunMpvBackend` directly.

UI callbacks should be wired for the existing controls that already exist in `main-window.slint`: open file, open folder, open URL, play/pause, speed up/down, rotation, audio channel, A-B loop, full screen, and settings.

## Error Handling

`MpvError` should include:

- Runtime feature disabled.
- Handle creation failure.
- Initialization failure.
- Command failure with the command name or property name.
- Invalid string conversion before FFI.
- Unsupported or unknown mpv event data.

Errors crossing into `yoyo-core` continue to use `AppError::Message` until a broader error model is justified.

## Testing

Unit tests must not require a user-installed libmpv. They should cover:

- `MpvAction` execution against a fake sink.
- Mapping mpv property changes into `BackendEvent`.
- Startup selection behavior when runtime feature is disabled.
- Desktop controller wiring for open URL/file and playback controls.

Manual smoke testing covers real playback:

- Build with `--features yoyo-mpv/mpv-runtime`.
- Launch with libmpv discoverable on the platform.
- Open a local video file.
- Toggle pause, seek, speed, volume, audio channel, rotation, zoom, and A-B loop.
- Open a supported network URL.
- Confirm missing libmpv reports a clear startup error.

## Success Criteria

- `cargo test` passes without requiring libmpv.
- `cargo test --features yoyo-mpv/mpv-runtime` compiles in an environment with libmpv development files.
- A developer can run the desktop app with runtime support and play a local media file through real mpv.
- Placeholder playback is not exposed as the default production backend.
- The design keeps video-surface embedding as a separate later phase.
