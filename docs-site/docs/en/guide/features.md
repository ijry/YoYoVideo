---
title: Features
description: Playback, batch, subtitle, picture and interface features, plus the default shortcuts.
---

# Features

## Batch playback

Drop several files, or a whole folder, onto the window to enter batch mode. Each tile is an
independent playback session:

- its own play / pause / stop
- its own position and volume
- tile sizes can be dragged to resize, and neighbours reflow on release
- closing one tile leaves the others untouched

The command line does the same thing:

```powershell
yoyovideo-desktop.exe a.mp4 b.mkv c.mov
```

## Playback control

| Capability | Notes |
| --- | --- |
| Seeking | Exact time entry, plus drag and preview on the timeline |
| Chapters | Container chapters; `Shift+←/→` cycles chapters and markers |
| Markers | `Ctrl+M` drops a marker at the current time, jumpable from the timeline |
| A-B loop | `A` / `B` set the two points, `Ctrl+A` clears them |
| Speed | `[` / `]` to change, `0` to reset |
| Frame stepping | `,` / `.` |
| Screenshots | `S` saves the current frame |

## Subtitles and audio

- Load external subtitle files; preferences persist alongside the playlist
- Subtitle delay, scale and vertical position are adjustable
- Audio track switching and cycling channel modes
- Subtitle defaults are saved with your settings

## Picture

- Zoom (`Z` / `X`), rotation (`R`) and picture panning
- Common video filter presets
- Brightness, contrast and saturation

## Interface

- Self-drawn frameless window with auto-hiding title bar and control bar
- Playlist and history side panel
- Right-click menus and hover submenus
- Mouse wheel for volume
- Chinese-first UI text, switchable to English in settings
- Every shortcut is rebindable in the settings window

## Default shortcuts

These ship as the factory bindings and can be changed in settings.

| Shortcut | Action |
| --- | --- |
| `Space` | Play / pause |
| `←` / `→` | Seek back / forward 5s |
| `↑` / `↓` | Volume up / down |
| `M` | Toggle mute |
| `[` / `]` / `0` | Slower / faster / reset speed |
| `A` / `B` / `Ctrl+A` | Set point A / point B / clear loop |
| `Ctrl+M` | Add marker |
| `Shift+←` / `Shift+→` | Previous / next chapter or marker |
| `J` | Open the jump panel |
| `P` | Open the action panel |
| `,` / `.` | Frame step back / forward |
| `R` | Rotate clockwise |
| `Z` / `X` | Zoom out / in |
| `C` | Cycle channel mode |
| `F` | Toggle fullscreen |
| `S` | Screenshot |
| `O` | Open file |
| `U` | Open URL |
