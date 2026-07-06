# Compact Frameless Player UI Design

## Goal

Correct the installed YoYoVideo interface so it behaves and reads like a compact native desktop video player instead of a dense control prototype. The target is a PotPlayer-like black shell: video centered and dominant, only core playback controls visible at the bottom, advanced controls grouped behind a top-left menu, and custom integrated window controls.

This phase keeps the current Rust + Slint + libmpv architecture. It changes the desktop UI, view-model labels, window control wiring, and video pan command path only where needed to satisfy the reported issues.

## Reported Problems

- The embedded video picture is not visually centered in the stage.
- The interface is English-only; default language should be Chinese and English should remain available.
- Several layouts are abnormal, especially the volume slider stretching too far.
- Window maximize and double-click fullscreen are missing from the visible UI.
- The bottom control strip is cluttered with secondary and advanced functions.
- The native OS title bar should be removed and replaced with a black integrated custom title bar.
- Pressing and dragging on the video should move the picture position.
- Common controls should use icon-style affordances instead of text labels where practical.

## Confirmed Approach

Use a full UI correction rather than small patches:

- `MainWindow` becomes frameless with `no-frame: true` and Slint resize border support.
- A custom black title bar owns app drag, minimize, maximize/restore, close, and the top-left menu button.
- The bottom deck is reduced to playback essentials: previous/next, play/pause, time, progress, mute/volume, fullscreen.
- Advanced functions move to categorized popups opened from the top-left menu.
- UI text defaults to Chinese, with a language switch entry for English.
- The video surface gets direct pointer handling for double-click fullscreen and drag-to-pan.
- Buttons use consistent icon-like glyphs or simple geometric Slint components instead of long text labels in primary controls.

## Technical Feasibility

The current dependencies support the needed behavior:

- Slint `Window` supports `no-frame` and `resize-border-width`.
- Slint `TouchArea` supports `double-clicked`, `pressed_x`, `pressed_y`, and `moved`.
- winit 0.30 supports `set_minimized`, `set_maximized`, `set_fullscreen`, and `drag_window`.
- libmpv supports picture panning through `video-pan-x` and `video-pan-y`, which can be added to the existing command translation path.

## Layout Design

The main window has three persistent regions:

1. Top chrome: 36-42 px high, flat black, integrated with the window background.
2. Video stage: fills all remaining space above the deck and owns the native video host bounds.
3. Bottom compact deck: 58-72 px high, fixed-density controls, no advanced tool sprawl.

The video stage should use the real available content area, not an over-decorated rounded card that offsets the native child window. Empty-state text is centered only when no media is active. When media is active, overlays such as OSD and status hints remain visually centered over the video stage.

The sidebar remains available but is not part of the default primary surface. Playlist and history are opened from the menu or a compact side toggle; they should not consume space unless explicitly shown.

## Top Chrome

Left side:

- Menu icon button opens the categorized main menu.
- App title or current media title appears next to it with muted text.

Center:

- Optional status text, kept short and non-dominant.

Right side:

- Minimize button.
- Maximize/restore button.
- Close button.

Dragging:

- A drag region in the top chrome calls a Rust callback that invokes `winit_window.drag_window()`.
- Buttons and menu hit areas do not trigger window dragging.

## Main Menu

The top-left menu replaces the current scattered toolbar and organizes features into sections:

- File: open file, open folder, open URL, recent items.
- Playback: speed down/up/reset, jump to time, previous/next chapter or marker, AB repeat.
- Tracks: audio tracks, subtitle tracks, video tracks, external subtitle, subtitle controls.
- Picture: screenshot, previous/next frame, zoom, rotate, picture adjustments, filters.
- View: playlist, history, fullscreen, reset picture position.
- Settings: preferences, language switch.

The first implementation can use one menu popup with clear section headers and compact rows. Nested menus are optional only if the single menu becomes too tall at 1200x760.

## Bottom Deck

Only core controls remain visible:

- Previous item or previous chapter/marker.
- Play/pause primary control.
- Next item or next chapter/marker.
- Current time and duration.
- Progress rail with chapter and marker ticks.
- Mute button.
- Fixed-width volume slider, target width 96-120 px.
- Fullscreen button.

Speed, zoom, rotation, audio channel, screenshot, filters, track selection, markers, URL entry, and jump controls move out of the bottom deck and into the main menu or focused popups.

The progress rail keeps existing preview and commit behavior. The volume slider must have an explicit width so it cannot stretch and push other controls out of alignment.

## Icon Style

Use compact icon-like controls:

- Play/pause: simple triangle and pause bars.
- Previous/next: bar plus triangle.
- Volume/mute: speaker-like text glyph or simple Slint geometry.
- Fullscreen: corner-frame glyph or simple Slint geometry.
- Menu: three-line hamburger glyph.
- Window controls: minus, square/restore, x.

Avoid emoji icons. If pure vector path work becomes too slow in Slint, short glyph labels are acceptable for this phase, but core controls must no longer be long English words like `Play`, `Full`, or `Settings`.

## Internationalization

Default language is Chinese.

Scope for this phase:

- Add a small UI language model for Chinese and English labels used by `MainWindow`, popups, OSD, and presenter labels.
- Use Chinese defaults for static Slint text and Rust-generated labels.
- Add a menu action to switch between Chinese and English during the current session.
- Persisting language can be added to config if the implementation is straightforward; otherwise default Chinese on startup is the required behavior for this phase.

Representative Chinese labels:

- Open File: `打开文件`
- Open Folder: `打开文件夹`
- Open URL: `打开链接`
- Playlist: `播放列表`
- History: `历史记录`
- Tracks: `音轨/字幕`
- Picture: `画面`
- Settings: `设置`
- Fullscreen: `全屏`
- Volume: `音量`
- Speed: `速度`
- Zoom: `缩放`
- Brightness: `亮度`
- Contrast: `对比度`
- Saturation: `饱和度`
- Gamma: `伽马`
- Hue: `色调`

## Video Centering And Bounds

The native video host is currently synced to `video_area_x/y/width/height`. The UI should make that rectangle the true centered video stage:

- Remove inner decorative padding from the host bounds.
- Avoid rounded-card offsets that make the child video appear shifted.
- Keep the video host bounds updated after resize, maximize, fullscreen, sidebar changes, and layout changes.
- Center empty-state and OSD overlays relative to the same stage rectangle.

If mpv letterboxes or pillarboxes content internally, that is acceptable. The app requirement is that the host stage itself is centered and not visually offset by UI chrome.

## Double-Click Fullscreen

Add a `video_double_clicked` Slint callback from a full-size `TouchArea` over the video stage. It dispatches the existing `AppCommand::ToggleFullscreen` path so fullscreen state stays consistent with keyboard shortcut `F` and existing winit fullscreen code.

The bottom fullscreen button uses the same command.

## Video Drag To Pan

Add pan state to the playback model:

- `PlayerState` stores `video_pan_x` and `video_pan_y` as normalized `f64` values.
- `AppCommand::AdjustVideoPan { delta_x, delta_y }` applies small normalized changes from pointer movement.
- `AppCommand::ResetVideoPan` returns both axes to zero.
- `BackendCommand::SetVideoPan { x, y }` translates to libmpv `video-pan-x` and `video-pan-y`.

Pointer movement:

- A pressed drag over the video stage calls a Slint callback with pixel delta.
- Rust converts pixel delta to normalized pan deltas based on current video stage width and height.
- The sign should match direct manipulation: dragging right moves the picture right, dragging down moves it down.
- Drag panning works best when zoomed in, but the command remains available at any zoom.

## Error Handling

- If window control APIs fail, record diagnostics and keep the app running.
- If video pan command fails, show a non-blocking status/OSD message and keep playback running.
- If language switching hits an unknown key, fall back to Chinese.
- Missing recent items and existing playback errors continue through current error paths.

## Testing

Automated checks:

- Slint compile-contract test for the new callbacks and properties: language, window controls, video double-click, video drag, reset pan.
- Presenter tests for Chinese default labels and English alternate labels.
- Core command tests for video pan state and reset behavior.
- mpv translation tests for `video-pan-x` and `video-pan-y`.
- Existing `cargo fmt --check` and full `cargo test`.
- Runtime/package smoke after rebuilding the installed test package.

Manual checks:

- Launch at 1200x760 and verify video stage is centered.
- Verify bottom controls remain compact and volume does not stretch.
- Verify default UI text is Chinese.
- Switch to English from the menu and verify visible labels update.
- Open a video and verify play/pause, progress, mute/volume, fullscreen still work.
- Double-click the video to enter and exit fullscreen.
- Drag the top chrome to move the frameless window.
- Minimize, maximize/restore, and close from custom window buttons.
- Zoom in, drag the video, and verify the picture position changes.
- Open each menu section and verify advanced features are still reachable.

## Acceptance Criteria

- The installed test build no longer shows the native OS title bar.
- The custom black chrome includes working minimize, maximize/restore, and close controls.
- The video stage is visually centered and dominant.
- Double-clicking the video toggles fullscreen.
- Dragging on the video changes mpv picture pan.
- The bottom deck contains only core playback controls and stays aligned at default size.
- Volume has a fixed compact width.
- Advanced features remain available through categorized menus.
- Default visible UI language is Chinese, with English available.
- Existing playback, shortcuts, playlist/history, tracks, subtitles, screenshot, frame step, filters, markers, chapters, packaging, and runtime smoke checks continue to pass.

## Self-Review

- The scope is focused on the reported player UI and interaction issues.
- Requirements are explicit enough to implement and test in one follow-up plan.
- The design avoids changing playback architecture except for the small pan command path required by drag-to-pan.
- No placeholders or unresolved decisions remain.
