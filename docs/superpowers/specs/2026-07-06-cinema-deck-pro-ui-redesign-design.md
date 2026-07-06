# Cinema Deck Pro UI Redesign Design

## Goal

Bring YoYoVideo's shipped desktop interface up to the visual direction previously selected for the player. The current build has the playback features, but the main window still reads like a dense prototype: many equal-weight buttons, visible native widgets in the primary control strip, and only a shallow dark theme.

This phase is a visual and interaction refactor of the existing Rust + Slint + libmpv app. It must keep the playback engine, command wiring, shortcuts, playlist/history, tracks, subtitles, screenshot, frame stepping, filters, markers, chapters, and packaging behavior intact.

## Confirmed Direction

The confirmed direction is `Cinema Deck Pro`.

The player should feel like a modern native desktop video player rather than a settings-heavy utility. The video surface is the product center. Controls sit in a deliberate cinematic deck that appears lightweight, layered, and fast.

Visual language:

- OLED black and near-black background instead of flat gray panels.
- Deep midnight blue surfaces for hierarchy.
- Wine red for primary playback emphasis.
- Cyan/blue for status, progress, chapters, and active feedback.
- Amber for user-created markers.
- Thin luminous borders, soft glass-like panels, and restrained glow.
- No emoji icons and no default-looking primary buttons.

## Current Gap

The current `apps/yoyovideo-desktop/ui/main-window.slint` implementation exposes the needed callbacks and state, but the presentation is not yet at product quality:

- Main actions are arranged as a long toolbar of equal-weight text buttons.
- The progress rail is improved but still not visually dominant enough.
- Primary and advanced controls compete for attention.
- Popups use a dark background but still contain many default widgets.
- The sidebar is visually heavy relative to the video area.
- The player lacks a strong title/chrome layer and clear visual rhythm.

This redesign should correct those issues without changing the backend architecture.

## Visual System

Use a compact design token layer at the top of the Slint file or through local reusable components.

Palette:

- App background: `#000000`
- Video well: `#05070b`
- Deck base: `#080d14`
- Raised glass: `#101827`
- Hairline border: `#263241`
- Text strong: `#f8fafc`
- Text muted: `#94a3b8`
- Primary red: `#e11d48`
- Primary red hover: `#fb315f`
- Cyan accent: `#38bdf8`
- Cyan dim: `#164e63`
- Marker amber: `#f59e0b`

Typography:

- Prefer a bundled, modern UI font if one is added in this phase.
- If no bundled font is added, keep native text rendering but improve hierarchy through size, weight, spacing, and casing.
- Primary deck labels should use compact, high-contrast text.
- Avoid oversized labels inside dense controls.

Shape and effects:

- Main video surface uses large rounded corners and a subtle inner border.
- Bottom deck uses a floating rounded rectangle with translucent dark layering.
- Primary play button is circular or pill-shaped and visually stronger than secondary controls.
- Focus and hover states use border and color changes, not layout-shifting scale.
- OSD uses centered translucent glass with one clear message.

## Main Layout

The main window becomes a three-layer video-first composition.

Layer 1: app shell

- Full dark background with a subtle radial or linear depth effect using Slint rectangles where practical.
- A thin top chrome row with app name, current media title/status, and window-level actions.
- No heavy menu bar.

Layer 2: video stage

- The video host takes most of the width and height.
- Empty state is clean and intentional: app name, "Open a video", drag/drop hint, and a small shortcut hint.
- OSD and status messages render inside the video stage.
- The video stage should remain visually dominant even when the sidebar is open.

Layer 3: control deck

- The deck floats at the bottom of the video stage or directly below it with the same visual treatment.
- Row 1 is playback only: play/pause, current time, progress rail, duration, mute/volume, fullscreen.
- Row 2 is compact and secondary: open, speed, jump, tracks, picture, markers, playlist/action panel.
- Advanced functions must not expand the main strip into a wall of buttons.

## Component Model

Introduce or refactor local Slint components:

- `DeckIconButton`: compact circular or square icon/text button for secondary actions.
- `DeckPrimaryButton`: stronger play/pause button using red accent.
- `DeckPill`: read-only compact label such as speed, zoom, loop, audio mode.
- `DeckRail`: custom progress rail with played fill, preview bubble, chapter ticks, and marker ticks.
- `DeckPanel`: shared glass surface for action panel, tracks, video tools, and menu popups.
- `DeckSectionTitle`: consistent section header for panels.
- `SidebarCard`: playlist/history rows with better selection states.

The implementation can use text glyphs or short labels at first where Slint SVG icon support would add too much scope, but the visual treatment must still be custom and consistent.

## Popups And Sidebar

Popups should feel like part of the same deck system:

- Dark glass panel background.
- Clear section titles.
- Equal spacing.
- Red only for primary or destructive emphasis.
- Cyan for active selections.
- Default widgets are allowed for actual text entry and sliders, but they should be wrapped in styled panels and not dominate the primary player surface.

Sidebar behavior:

- Sidebar remains available for playlist and history.
- It should use a quieter dark surface and not compete with the video.
- Collapsed state should become a slim rail, not a visually awkward block.
- Playlist current item should have a clear cyan or red-accented selection state.

## Interaction Rules

- Primary controls must remain one click away.
- Advanced controls move to action panel or focused popups.
- Existing keyboard shortcuts stay unchanged.
- Hover, pressed, disabled, and focused states must be visually distinct.
- Progress interaction keeps the existing command model: preview during movement, commit seek on click/release path supported by current Slint APIs.
- OSD appears for common actions and must not block playback.

## Architecture Boundaries

This phase should mostly touch:

- `apps/yoyovideo-desktop/ui/main-window.slint`
- Slint compile-contract tests where component/property coverage needs updates.
- Manual smoke checklist if visual acceptance steps need more precision.
- Packaging/install only after rebuilding the test package.

Avoid changing:

- `yoyo-core` playback model.
- `yoyo-mpv` command/event translation.
- Marker store format.
- Shortcut defaults.
- Runtime packaging scripts, unless reinstall automation needs the new package.

If Rust changes are needed, they should be limited to view-model text labels or UI-only presenter helpers.

## Error Handling And Performance

- The UI must remain usable if no media is loaded.
- The video host rectangle must still provide stable coordinates for native video embedding.
- Styled components should not introduce expensive animation loops.
- Hover and OSD effects should be static or event-driven.
- If a visual effect is difficult in Slint without extra cost, prefer a simpler native-looking custom implementation over adding a heavy dependency.

## Testing

Automated checks:

- `cargo fmt --check`
- `cargo test -p yoyovideo-desktop --test context_menu_contract`
- `cargo check -p yoyovideo-desktop`
- Full workspace tests after implementation.
- Runtime/package smoke after rebuilding the test install.

Manual visual checks:

- Launch app and confirm it no longer looks like default Slint controls arranged in rows.
- Confirm the video area is visually dominant.
- Confirm the bottom deck has clear primary and secondary hierarchy.
- Confirm the progress rail, OSD, sidebar, action panel, tracks popup, and video tools popup share one visual language.
- Confirm the UI remains usable at the default 1200x760 window size.
- Open a video and verify controls do not cover or break the native video host.

## Acceptance Criteria

- The installed test build visually matches `Cinema Deck Pro`: video-first, cinematic, dark, glass-like, and polished.
- Primary player controls no longer look like default widgets.
- Advanced functionality remains available but does not clutter the main control deck.
- Existing playback, shortcuts, sidebar, tracks, subtitles, screenshot, frame step, filters, markers, chapters, and packaging smoke checks continue to pass.
- The user can test an updated local install from the desktop shortcut after the implementation package is rebuilt.

## Self-Review

- All requirements are explicit and complete.
- The scope is limited to UI visual quality and layout, not playback engine changes.
- The design resolves the reported mismatch between promised `Cinema Deck` direction and current installed UI.
- Acceptance criteria include both automated compile checks and manual visual confirmation.
