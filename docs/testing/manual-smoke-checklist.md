# Manual Smoke Checklist

## Startup

- Launch the app on Windows, macOS, and Linux.
- Confirm the window opens without crashing when libmpv runtime files are present.
- Confirm the app shows an actionable error when libmpv runtime files are missing.

## Playback

- Build and run `cargo run -p yoyovideo-desktop --features mpv-runtime`.
- Confirm startup fails clearly if libmpv link/runtime files are unavailable.
- Open a local video file and confirm play/pause works.
- Open a URL and confirm network playback attempts begin.
- Verify speed, volume, rotation, zoom, audio channel switching, and A-B repeat.
- Open a folder and confirm EOF advances to the next playlist item.
- Confirm hardware acceleration falls back to software decoding without exiting the app.

## UX

- Verify context menu actions match toolbar actions.
- Verify keyboard shortcuts trigger the same commands as buttons.
- Verify settings changes persist across restarts.
- Verify recent history and last position are restored when enabled.

## Package Artifacts

- Download each GitHub Actions artifact: `YoYoVideo-windows-x64`, `YoYoVideo-macos-universal`, and `YoYoVideo-linux-x64`.
- Extract the archive and confirm the top-level directory is named `dist/YoYoVideo-<platform>` locally or `YoYoVideo-<platform>` inside the uploaded archive.
- Confirm `README.md`, `LICENSES/`, `docs/runtime-dependencies.md`, and `docs/manual-smoke-checklist.md` are present.
- Confirm `bin/yoyovideo-desktop.exe` exists on Windows and `bin/yoyovideo-desktop` exists on macOS and Linux.
- For runtime-enabled artifacts, confirm Windows includes `bin/mpv-2.dll`, macOS includes `bin/libmpv.dylib`, and Linux includes `bin/libmpv.so*`.
- Launch the app from the extracted `bin/` directory and run the Playback and UX checks above.
