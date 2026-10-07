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

- Launch the installed test build and confirm the main player reads as `Cinema Deck Pro`: OLED black shell, dominant video stage, red primary play control, cyan progress/status accents, and amber marker ticks.
- Confirm the main deck has two clear priorities: row 1 for play/progress/time/volume/fullscreen, row 2 for secondary actions such as open, speed, jump, tracks, picture, markers, playlist, and actions.
- Confirm the main control strip no longer looks like default Slint buttons arranged in a long toolbar.
- Open the action panel, tracks popup, video tools popup, menu popup, and sidebar; confirm all use the same dark glass visual language.
- Collapse and reopen the sidebar; confirm the collapsed state is a slim rail and the video stage remains visually dominant.
- Launch and confirm the main player uses the Cinema Deck layout with a video-first surface and custom-styled primary controls.
- Open a video, drag the progress rail, and confirm preview updates while playback seeks only after release.
- Use `M`, `J`, `Ctrl+M`, `P`, `Shift+Right`, and `Shift+Left`, and confirm mute, jump, marker, action panel, and chapter/marker navigation work.
- Open media with embedded chapters and confirm chapter ticks appear on the progress rail and rows appear in the action panel.
- Add and remove a user marker, restart, reopen the same media, and confirm markers persist.
- Confirm OSD messages appear for mute, seek, jump, speed, marker, screenshot, and filter actions.
- Verify context menu actions match toolbar actions.
- Verify keyboard shortcuts trigger the same commands as buttons.
- Verify settings changes persist across restarts.
- Resize and move the window, close the app, restart, and confirm size and position are restored.
- Maximize the window, close the app, restart, and confirm the maximized state is restored.
- Open several files, folders, and URLs, then confirm the `Recent` section in the menu shows newest entries first.
- Select a recent file, folder, and URL, and confirm each opens through the normal playback path.
- Remove a recent local file, select it from the menu, and confirm the app shows a non-fatal missing item message without clearing current playback.
- Change playback-end behavior in settings to `Stop`, `Loop Current`, `Loop Playlist`, and `Play Next`, then confirm EOF handling matches each mode.
- Verify recent history and last position are restored when enabled.
- Confirm the right sidebar honors the startup preference on wide windows.
- Launch the app with a window narrower than `1050px` and confirm the sidebar starts collapsed.
- Toggle the sidebar from the control surface and confirm the collapsed strip can reopen it.
- Open a folder and confirm the `Playlist` tab shows the scanned queue with the active item highlighted.
- Click a playlist item in the sidebar and confirm playback switches to that item.
- Restart the app, open the `History` tab, and confirm recent items show resume metadata.
- Click a history item and confirm playback resumes near the stored position.
- Click a history item pointing to a removed file and confirm the app shows a clear error without crashing.
- Open the dedicated settings window and confirm `Apply`, `OK`, and `Cancel` behave like a desktop dialog.
- Change the play/pause shortcut in settings and confirm the new shortcut works immediately without restarting.
- Attempt to bind the same shortcut to two different actions and confirm the save is blocked with a clear conflict message.
- Disable playback history in settings, play a new media item, and confirm no new history entry is written.
- Change default speed or default volume in settings, open new media, and confirm the new defaults apply without changing media that was already playing before the save.
- Drag a single local video file onto the window and confirm playback starts.
- Drag multiple supported media files onto the window and confirm the playlist contains all dropped media in drop order.
- Drag a folder containing supported and unsupported files and confirm only supported media appears in the playlist.
- Drag only unsupported files and confirm current playback continues while the status label reports no playable media.
- Right-click or open `Menu`, then use `Open File`, `Open Folder`, `Playlist`, `History`, `Screenshot`, `Video Tools`, `Fullscreen`, and `Settings`.
- Trigger a playback/runtime error and confirm it appears in the status label and in the local diagnostic log.
- Open a media file with multiple audio or subtitle tracks, open the `Tracks` popup, and confirm the lists reflect the available tracks.
- Switch audio, subtitle, and video tracks from the popup and confirm playback updates without leaving the main window.
- Choose the subtitle `Off` entry and confirm subtitles disappear immediately.
- Load an external subtitle file from the popup and confirm it appears and becomes usable without interrupting playback.
- Adjust subtitle delay, scale, and vertical position from the popup and confirm the changes apply during playback.
- Close and reopen the same media and confirm the last subtitle/track preferences are restored.
- Remove a previously remembered external subtitle file, reopen the same media, and confirm playback still starts while the app shows only a non-blocking warning.
- Open a local video, open `Video Tools`, click `Screenshot`, and confirm a `.png` file appears in `Pictures/YoYoVideo Screenshots` or the fallback screenshots directory shown by the app.
- Pause playback, use `Prev Frame` and `Next Frame`, and confirm the displayed frame changes one frame at a time.
- Use the default `S` shortcut and confirm it saves a screenshot through the same status path as the button.
- Use the default `,` and `.` shortcuts and confirm previous-frame and next-frame stepping work.
- Type `s,.` into the URL input and confirm screenshot and frame-step shortcuts do not fire while the input is focused.
- Move brightness, contrast, saturation, gamma, and hue sliders and confirm visible picture changes.
- Click `Reset Picture` and confirm brightness, contrast, saturation, gamma, and hue return to neutral.
- Select `Sharpen`, `Light Denoise`, `Grayscale`, and `Invert` filter presets and confirm each preset applies.
- Select `None` and confirm the YoYoVideo preset filter is removed.

## Package Artifacts

- Run `pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun` and confirm it prints the manifest entry without downloading.
- Run `pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force` on a clean machine with maintainer runtime environment variables set.
- Download each GitHub Actions artifact: `YoYoVideo-windows-x64`, `YoYoVideo-macos-aarch64`, and `YoYoVideo-linux-x64`.
- Extract the archive and confirm the top-level directory is named `dist/YoYoVideo-<platform>` locally or `YoYoVideo-<platform>` inside the uploaded archive.
- Confirm `README.md`, `LICENSES/`, `docs/runtime-dependencies.md`, and `docs/manual-smoke-checklist.md` are present.
- Confirm `RELEASE-NOTES.md`, `LICENSES/README.md`, and `LICENSES/runtime-provenance.md` are present in the package.
- Confirm `bin/yoyovideo-desktop.exe` exists on Windows and `bin/yoyovideo-desktop` exists on macOS and Linux.
- For runtime-enabled artifacts, confirm Windows includes `bin/mpv-2.dll`, macOS includes `bin/libmpv.dylib`, and Linux includes `bin/libmpv.so*`.
- Build `dist/YoYoVideo-windows-x64-setup.exe` with `scripts/build-installer.ps1` when NSIS is installed.
- Install the Windows setup package and launch YoYoVideo from the Start Menu shortcut.
- Uninstall YoYoVideo and confirm the installed directory is removed.
- Run `pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64` when runtime files are staged.
- Run `pwsh -NoProfile -File scripts/smoke-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime` and confirm it writes `smoke/package-smoke.log`.
- Launch the app from the extracted `bin/` directory and run the Playback and UX checks above.

## Visible Video Host

- Launch with `cargo run -p yoyovideo-desktop --features mpv-runtime`.
- Open a local video and confirm video is visible inside the video area.
- Confirm the video does not cover controls.
- Resize the window and confirm the video area tracks the UI.
- Toggle fullscreen and confirm the video host resizes.
- Type in the URL input and confirm player shortcuts do not fire while it is focused.
- Use keyboard shortcuts for play/pause, seek, volume, speed, zoom, rotation, audio channel, A-B loop, and fullscreen.


## Experimental Wayland GL Compositing (not yet hardware-verified)

- Run a feature-enabled Linux build in an actual Wayland session (not XWayland).
  Confirm Slint uses its femtovg OpenGL renderer and no video child window opens.
- Open one video from the command line: initialization must precede playback,
  without a second mpv window, startup deadlock, or blank first frame.
- Play for several minutes; verify continuous updates, orientation, aspect ratio,
  subtitles, seek/pause/frame-step, and A/V synchronization.
- Resize while playing and while paused. Move between monitors with fractional
  scale factors; verify sharp physical-pixel sizing and no stale cached frame.
- Open context menus/settings, show OSD, seek and change volume over the picture.
  Video must remain behind UI overlays and inputs must keep working.
- Open several files as a grid: expect the explicit unsupported-Wayland-grid
  message, no extra top-level windows, and usable single-video controls afterward.
- Minimize/restore, fullscreen/unfullscreen, then close while playing. Check for
  GL errors, callback-after-close crashes, and "Missing GL teardown" diagnostics.
- Force a missing/unsupported GL renderer or driver in a test environment. Expect
  an actionable startup error, not an endless retry/log loop.
- Recheck Windows/X11 native child-window playback and macOS render-API playback;
  neither should select the Wayland composited surface.


## Windows installed-app identity and launch

- The package builder and installer builder run `scripts/test-windows-app-artifacts.ps1`
  against the actual executable: PE subsystem must be Windows GUI (2), with
  embedded icon and icon-group resources. Missing resources or Console (3) block packaging.
- Run `pwsh -NoProfile -File scripts/test-windows-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64`
  with NSIS on PATH. This compiles and silently installs the production installer
  under a unique app identity, verifies the actual installed shortcut and uninstall
  registration, and uninstalls/cleans up. It must not replace a normal YoYoVideo installation.
- On a real installation, the Start Menu icon and Add/Remove Programs icon must
  resolve inside the installed application, never a repository or CI runner path.
- Launch from the Start Menu and by opening the installed EXE in Explorer. No
  console/terminal window should appear; fatal startup errors remain in the diagnostic log.


## Restored-window visibility

- With the app closed, test a saved geometry from a disconnected display or a
  far-offscreen position (for example x=11830, y=1001, 2910x1650). On next launch,
  the main window must be reachable on a connected monitor, not merely present
  on the taskbar. Back up the window-state file before any manual edits.
- Valid negative-coordinate secondary displays must retain the saved position;
  gaps between monitors must not count as visible desktop space.
- Oversized dimensions and a title bar above the screen must be brought inside
  the selected display. Later user moves must not be overridden by restoration.
- Run `cargo test -p yoyovideo-desktop --test window_restore_smoke -- --ignored`
  on Windows. This creates a fresh MainWindow without reading/writing user data,
  exercises the real deferred native-window restoration, checks monitor overlap,
  visibility and nonblank rendered content, then verifies a later move is retained.
- The package/release workflows run that same test for Windows. Merely keeping a
  process alive, or using `Start-Process -WindowStyle Hidden`, is not a visibility test.
