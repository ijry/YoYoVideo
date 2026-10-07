<p align="center">
  <img src="docs-site/docs/public/logo.svg" width="72" alt="YoYoVideo">
</p>

<h1 align="center">YoYoVideo</h1>

<p align="center">
  A full-format local video player built with Rust, Slint and libmpv.<br>
  Multi-tile batch playback · subtitle and track switching · picture filters · A-B loop · offline playback
</p>

<p align="center">
  <a href="https://github.com/ijry/YoYoVideo/releases/latest"><img alt="Download" src="https://img.shields.io/github/v/release/ijry/YoYoVideo?label=download&style=flat-square"></a>
  <a href="https://ijry.github.io/YoYoVideo/"><img alt="Docs" src="https://img.shields.io/badge/docs-ijry.github.io%2FYoYoVideo-38bdf8?style=flat-square"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue?style=flat-square"></a>
</p>

---

## Interface

The window on startup. The video area is empty, and the chrome fades out a few seconds after the
pointer rests.

![The YoYoVideo window on startup](docs/assets/player-default.png)

## What it is

A **local** video player: the interface is drawn with [Slint](https://slint.dev/) and playback is
handled by [libmpv](https://mpv.io/). It mainly exists to do one thing most players do not:
**let you watch several videos at once.**

- **Multi-tile batch playback.** Drop in several files or a whole folder; each tile is an independent
  playback session with its own transport, position and volume, and tile sizes you can drag.
- **Full transport control.** Exact time jumps, speed, zoom, rotation, picture panning, A-B loop,
  chapter jumps, and droppable markers on the timeline.
- **Subtitles and audio.** External subtitle files, adjustable delay, scale and vertical position,
  audio track switching and cycling channel modes.
- **Picture tools.** Common filter presets plus brightness, contrast and saturation.
- **A quiet frameless shell.** Self-drawn title bar and control bar, wheel for volume, hover menus.
- **Offline playback, no telemetry.** Local playback needs no network. Optional update checks contact GitHub for versions and packages, never upload media/history, and can be disabled.

See the [documentation site](https://ijry.github.io/YoYoVideo/) for the full feature and shortcut
list.

## Download and updates

The new 0.0.1 release pipeline uses **Velopack 1.2.161**. Availability is determined by the files actually published on [Releases](https://github.com/ijry/YoYoVideo/releases/latest).

| Platform | Format | In-app updates |
| --- | --- | --- |
| Windows x64 | `YoYoVideo-stable-windows-x64-Setup.exe` | Supported after installation |
| macOS Apple Silicon / Intel | Architecture-specific `Portable.zip` containing .app | Supported for the Velopack app bundle |
| Linux x64 | `YoYoVideo.AppImage` | Supported at a writable location |
| Linux x64 | `YoYoVideo-linux-x64.deb` | Package manager/manual updates only |

Open **Check for updates** to control automatic checks, download, postpone, or confirm exit and install. Signatures and package hashes are checked before installation. Pending downloads are not silently installed on startup. Settings/history are preserved; playback does not automatically resume after an update.

- Development builds, old NSIS installs and ordinary portable ZIPs require manual installation of the new format.
- macOS uses **ad-hoc signing, without Apple notarization**. This is not an Apple Developer signature; Gatekeeper may block first launch. Download only from the trusted release page and use normal system approval flows rather than disabling system security.
- AppImages target Ubuntu 22.04 and bundle libmpv/distributable libraries, not the host graphics drivers, desktop or audio services. Existing Wayland/grid-playback limitations still apply.

See [updater development notes](docs/development/updater.md) for verification status, key management and publishing.

## Building from source

You need Rust stable (edition 2024). On Windows you also need the Visual Studio Build Tools C++
workload — `libmpv-sys` needs the MSVC toolchain at link time.

```powershell
# Tests: default features do not need libmpv
cargo test --workspace
cargo fmt --check

# Fetch the playback core (public upstream build, pinned by sha256)
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64

# Run with real playback
pwsh -NoProfile -File scripts/dev-run.ps1

# Package
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
```

> Use `dev-run.ps1` rather than a bare `cargo run`: `mpv-runtime` cannot be a default feature, or
> `cargo test` would fail on machines without libmpv. `dev-run.ps1` uses its own `--target-dir`,
> because a plain `cargo build` rebuilds the same binary **without** `mpv-runtime` and silently
> replaces it.

## Workspace layout

| Path | Role |
| --- | --- |
| `crates/yoyo-core` | Playback session and command logic, with **no playback core dependency** |
| `crates/yoyo-mpv` | libmpv adapter, translating core events into domain events |
| `apps/yoyovideo-desktop` | Slint desktop app, platform integration, shipped binary |
| `scripts/` | Runtime fetch, packaging, verification, smoke tests |
| `runtime/manifest.toml` | Playback core source, version and checksum |
| `docs-site/` | Documentation site (VitePress) |

There is not a single line of libmpv code in `yoyo-core`, so the whole playback domain is testable
on a machine that has never heard of libmpv — which is what makes the default `cargo test` possible.

## Contributing

Before submitting:

```powershell
cargo fmt --check
cargo test --workspace
```

CI checks that the `mpv-runtime` feature compiles on Windows, macOS and Linux. The release process is
documented [here](https://ijry.github.io/YoYoVideo/en/dev/release).

## License

YoYoVideo is released under the **GPL-3.0-or-later**; the full text is in [LICENSE](LICENSE).

The playback core bundled in release packages comes from
[`shinchiro/mpv-winbuild-cmake`](https://github.com/shinchiro/mpv-winbuild-cmake), where both mpv and
FFmpeg are **GPL-2.0-or-later**. Redistributing the binaries requires providing the corresponding
source — see [LICENSES.md](LICENSES.md) and the `LICENSES/` directory inside each package.

[中文](README.md) · [Documentation](https://ijry.github.io/YoYoVideo/en/)
