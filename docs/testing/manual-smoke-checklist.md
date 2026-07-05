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
