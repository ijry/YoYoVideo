---
title: Installation
description: YoYoVideo platform packages and automatic updates.
---

# Installation

The new 0.0.1 pipeline uses Velopack. Download availability is determined by the assets actually published on [Releases](https://github.com/ijry/YoYoVideo/releases/latest).

| Platform | File | Updates |
| --- | --- | --- |
| Windows x64 | `YoYoVideo-stable-windows-x64-Setup.exe` | In-app after installation |
| macOS ARM / Intel | Architecture-specific `Portable.zip` containing .app | In-app |
| Linux x64 | `YoYoVideo.AppImage` | In-app at a writable location |
| Linux x64 | `YoYoVideo-linux-x64.deb` | Package manager/manual |

Compare downloaded assets against the release page SHA-256, for example:

```powershell
Get-FileHash -Algorithm SHA256 .\YoYoVideo-stable-windows-x64-Setup.exe
```

On Windows, launch from the Start menu after installation. On macOS, extract the .app to a writable applications directory. On Linux, give the AppImage execute permission.
Windows/macOS/AppImage packages bundle libmpv; .deb uses distribution dependencies.
Click **Open file** in the empty window or drop media/folders onto it.

## Automatic updates

Open **Check for updates** to disable automatic checking, download, postpone, or confirm exit and install.
Signatures and package hashes are checked first. Unconfirmed downloads are not applied on startup. Settings/history are retained; playback does not automatically resume.
Old NSIS installs, ordinary ZIPs and development builds require manual installation of the new format.

macOS uses **ad-hoc signing without Apple notarization**. Gatekeeper may block first launch. Download only from the trusted release page and use normal system approval flows; do not disable system security.
Windows does not currently have a trusted Authenticode certificate either, so SmartScreen may warn. Update authentication is not OS code signing.

AppImages are built on Ubuntu 22.04 and still require host graphics drivers, a desktop and audio services.
Packaging success is not proof of successful installation/upgrading on every platform; see the [verification status](https://github.com/ijry/YoYoVideo/blob/main/docs/development/updater.md).

## Other platforms

Releases cover Windows x64, macOS (Apple Silicon and Intel) and Linux x64. How video reaches the screen
differs per platform, and so does how much of it is proven:

| Platform | Video | Runtime |
| --- | --- | --- |
| Windows x64 | mpv `--wid` | bundled DLL from a pinned upstream build |
| Linux x64 | mpv `--wid` under X11 | bundled non-host libraries in AppImage; system dependencies for .deb |
| macOS | mpv render API over OpenGL | Homebrew dylibs, bundled and rewritten to `@rpath` |

**Two caveats, stated plainly:**

- **Wayland remains experimental.** Single-video playback uses Slint/OpenGL compositing,
  not yet verified on a Wayland desktop. Grid playback is unsupported.
- **macOS video is compile-verified only.** The render-API path builds for both architectures, but it has
  not been run on a real Mac yet — CI runners are headless and never create a window. Treat it as
  unproven until someone confirms it on hardware.

If you want to help close either gap, the `notes` on the matching entry in
[`runtime/manifest.toml`](https://github.com/ijry/YoYoVideo/blob/main/runtime/manifest.toml) spell out
what is missing.
