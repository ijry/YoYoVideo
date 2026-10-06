---
title: Installation
description: Download YoYoVideo v0.0.1 and run it on Windows.
---

# Installation

## Download

Grab v0.0.1 from the [Releases page](https://github.com/ijry/YoYoVideo/releases/latest).

| File | What it is |
| --- | --- |
| `YoYoVideo-windows-x64-setup.exe` | NSIS installer, adds a Start menu entry |
| `YoYoVideo-windows-x64.zip` | Portable package, unpack and run |

Both contain the same payload: `bin/` (the app and the playback core), `LICENSES/` (license and
runtime provenance) and the documentation.

::: tip No decoders to install first
libmpv and the FFmpeg build it depends on are already in `bin/`. Whether or not mpv or a third-party
codec pack is installed on your system makes no difference to YoYoVideo.
:::

## Verify

Every asset on the release page carries a SHA-256:

```powershell
Get-FileHash -Algorithm SHA256 .\YoYoVideo-windows-x64.zip
```

The value should match the one listed under "校验和 / SHA-256" for the same file.

## Run it

**Portable:** unpack, then launch `bin\yoyovideo-desktop.exe`.

**Installer:** run it, then start YoYoVideo from the Start menu.

On first launch the window may start small or partly offscreen; the player corrects this itself.
Window state lives under `%APPDATA%\xyito\YoYoVideo\`.

## Hand it some media

- Launch `bin\yoyovideo-desktop.exe`, then drag videos or a whole folder onto the window.
- Right-click a file in Explorer and pick "Open with" → `yoyovideo-desktop.exe`.
- Or pass files on the command line; several files start batch playback:

```powershell
.\bin\yoyovideo-desktop.exe D:\videos\a.mp4 D:\videos\b.mkv D:\videos\c.mp4
```

## Other platforms

Releases cover Windows x64, macOS (Apple Silicon and Intel) and Linux x64. How video reaches the screen
differs per platform, and so does how much of it is proven:

| Platform | Video | Runtime |
| --- | --- | --- |
| Windows x64 | mpv `--wid` | bundled DLL from a pinned upstream build |
| Linux x64 | mpv `--wid` under X11 | declared as `.deb` dependencies |
| macOS | mpv render API over OpenGL | Homebrew dylibs, bundled and rewritten to `@rpath` |

**Two caveats, stated plainly:**

- **Wayland is not supported.** There is no verified host path, so the app reports the limitation rather
  than pretending.
- **macOS video is compile-verified only.** The render-API path builds for both architectures, but it has
  not been run on a real Mac yet — CI runners are headless and never create a window. Treat it as
  unproven until someone confirms it on hardware.

If you want to help close either gap, the `notes` on the matching entry in
[`runtime/manifest.toml`](https://github.com/ijry/YoYoVideo/blob/main/runtime/manifest.toml) spell out
what is missing.
