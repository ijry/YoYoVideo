# Daily Use Release Readiness Design

## Goal

Make YoYoVideo feel reliable enough for daily local use and safer to package for testers. This phase focuses on high-frequency desktop workflows and release verification, not on adding advanced playback algorithms.

## Scope

- Drag local files or folders onto the main window to open media.
- Expand the context menu with daily actions: open file, open folder, screenshot, video tools, playlist, settings, and fullscreen.
- Improve startup and runtime error messages so missing libmpv files and playback failures explain what failed and how to recover.
- Write application diagnostics to a local log file for startup failures, runtime backend errors, and packaging smoke failures.
- Strengthen package smoke checks for executable presence, mpv runtime files, basic process startup, and temporary media playback.

## Non-Goals

- No automatic updater.
- No code signing or notarization workflow.
- No remote crash reporting service.
- No online subtitle matching.
- No LUT, HDR, shader, or color-management work.
- No large UI redesign beyond menu and status-message improvements.

## User Experience

Drag-and-drop should be forgiving. Dropping a supported media file opens it immediately. Dropping multiple supported files creates a playlist and starts the first item. Dropping a folder scans it with the existing media scanner, replaces the playlist, and starts the first supported item. Unsupported files are ignored if at least one supported item exists; if none exist, the status label explains that no playable media was found.

The context menu should expose the same core actions as the toolbar so mouse users do not need to hunt for controls. Menu actions dispatch through the existing typed command path instead of mutating Slint state directly. Screenshot keeps the current automatic path policy and reports the saved path through the status label.

Startup errors should use actionable wording. For example, missing Windows runtime should mention `third_party/mpv/windows-x64/bin/mpv-2.dll` and the bootstrap command. Playback errors should remain non-fatal when possible and should be visible in the status label.

## Architecture

Keep the current separation:

- `yoyo-core` remains the typed command, state, playlist, and session boundary.
- `yoyo-mpv` remains the libmpv adapter and playback event translator.
- `apps/yoyovideo-desktop` owns desktop-only concerns: drag-and-drop, Slint callbacks, context menu wiring, runtime diagnostics, screenshot path creation, and user-facing error wording.
- `scripts/` owns package and runtime smoke verification.

Drag-and-drop should reuse existing `OpenFile`, `OpenFolder`, and playlist/session paths. If existing APIs do not support multi-file playlist replacement cleanly, add a small typed command or helper in `yoyo-core` rather than special-casing playlist mutation in the desktop layer.

Diagnostics should be a small desktop module, for example `platform/logging.rs`, that resolves an app-owned log directory and appends timestamped plain-text lines. It must not require a logging daemon or network access.

## Data Flow

1. Slint/winit receives a dropped path list.
2. Desktop layer classifies paths into supported files and folders using existing media scanning rules.
3. Desktop layer dispatches typed core commands or a dedicated playlist-opening helper.
4. Core updates playlist/session state and forwards playback commands to the backend.
5. Backend errors are shown in the status label and appended to the diagnostic log.

For package smoke:

1. Script verifies package layout and required runtime files.
2. Script launches a minimal runtime probe or executable smoke path.
3. Probe generates temporary media, opens it through the runtime backend, waits for expected events, and exits with a clear failure message if anything is missing.
4. Failures are printed to the terminal and written to a smoke log artifact.

## Error Handling

- Dragging only unsupported files should not crash or clear the current playlist.
- Folder scans that produce no supported media should leave current playback untouched and show a status message.
- Screenshot path failures should remain non-fatal and should be logged.
- Missing runtime files should fail early with a focused diagnostic instead of a generic backend initialization failure.
- Logging failures should never block playback; if a log write fails, show at most one concise status warning and continue.

## Testing

Automated tests should cover:

- Path classification for dropped files and folders.
- Drag-and-drop dispatch for single file, multiple files, folder, unsupported-only input, and mixed input.
- Context menu callback compile contract and command forwarding.
- Runtime diagnostic message formatting for missing Windows mpv runtime.
- Log path creation and append behavior using temporary directories.
- Package smoke script checks for required executable and runtime files.

Manual smoke should cover:

- Drag a single video file onto the window and confirm playback starts.
- Drag a folder and confirm playlist replacement starts at the first media item.
- Drag unsupported files and confirm the current media is not disrupted.
- Use right-click menu actions for open, screenshot, video tools, settings, and fullscreen.
- Launch without staged runtime files and confirm the error names the missing file and bootstrap command.
- Run package smoke against a built Windows package with staged runtime files.

## Implementation Order

1. Add desktop path classification tests and drag-and-drop dispatch design seams.
2. Implement drag-and-drop behavior through existing command/session paths.
3. Expand context menu callbacks and tests.
4. Add diagnostic log path resolution and append-only logging.
5. Improve runtime startup error wording and logging.
6. Strengthen package smoke scripts and documentation.
7. Run full Rust tests, runtime feature checks, and Windows runtime smoke.

## Acceptance Criteria

- Dragging supported files or folders into the player starts playback through the same backend path as toolbar/menu open actions.
- Unsupported drag input is reported without crashing or clearing current playback.
- Context menu exposes the agreed daily actions and dispatches typed commands.
- Missing runtime diagnostics name the missing file and a concrete recovery command.
- Runtime and playback errors are written to a local log file.
- Package smoke catches missing executable or runtime files before release.
- `cargo fmt --check`, `cargo test`, `cargo check -p yoyo-mpv --features mpv-runtime`, and `cargo check -p yoyovideo-desktop --features mpv-runtime` pass.
