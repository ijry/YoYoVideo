# YoYoVideo

Rust + Slint + libmpv cross-platform desktop media player.

## Workspace

- `crates/yoyo-core`: playback/session domain logic
- `crates/yoyo-mpv`: libmpv adapter and render bridge
- `apps/yoyovideo-desktop`: Slint desktop application

## MVP Scope

- Local files and folders
- Network URLs
- Playback, seeking, speed, zoom, rotation, A-B repeat
- Playlist, history, context menu, and keyboard shortcuts

## Development

- Default tests use dry-run playback seams and do not require libmpv: `cargo test`
- Real playback alpha: `cargo run -p yoyovideo-desktop --features mpv-runtime`
- On Windows, the runtime feature requires `mpv.lib` at link time and the matching mpv DLLs at run time.

## Visible Video Runtime

The desktop app uses a native video host for visible video when built with `mpv-runtime` and when the current windowing backend can provide an mpv-compatible window id. If visible video embedding is unavailable, the app stays open and reports the limitation in the status label.

Run:

```powershell
cargo run -p yoyovideo-desktop --features mpv-runtime
```

## Packaging

Runtime-enabled packages are built from repository-local staging files under `third_party/mpv/<platform>/`. Prepare those files with the runtime bootstrap script before packaging.

Local package commands:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
pwsh -NoProfile -File scripts/package.ps1 -Platform macos-universal -Configuration release -RequireRuntime
pwsh -NoProfile -File scripts/package.ps1 -Platform linux-x64 -Configuration release -RequireRuntime
```

Use `scripts/verify-package.ps1` to validate generated package directories without launching the GUI:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

Run package smoke after verification to launch the packaged binary briefly and, when runtime files are required, run temporary-media playback against the package runtime:

```powershell
pwsh -NoProfile -File scripts/smoke-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

On Windows, build an optional NSIS installer after the portable package is verified:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version dev
```

Actual libmpv and FFmpeg runtime binaries are intentionally not committed. Stage them locally or provide maintainer runtime archive URLs to GitHub Actions after licensing review.
