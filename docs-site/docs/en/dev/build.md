---
title: Building locally
description: Compile, run and package YoYoVideo on your own machine.
---

# Building locally

## Requirements

- Rust stable (edition 2024)
- On Windows, the Visual Studio Build Tools C++ workload — `libmpv-sys` needs the MSVC toolchain at
  link time
- PowerShell 7 (the scripts invoke `pwsh`)
- 7-Zip, to expand the pinned runtime archive on Windows

## Run the tests

With default features playback uses a dry-run seam, so **libmpv is not required**:

```powershell
cargo test --workspace
cargo fmt --check
```

That is the premise of the whole test strategy: there is no libmpv code in `crates/yoyo-core`, so
the domain logic is fully testable anywhere.

## Run with real playback

Stage the playback core first:

```powershell
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64
```

then:

```powershell
pwsh -NoProfile -File scripts/dev-run.ps1
pwsh -NoProfile -File scripts/dev-run.ps1 D:\videos\a.mp4 D:\videos\b.mp4
```

::: warning Use dev-run.ps1, not a bare cargo run
`mpv-runtime` cannot be a default feature — that would break `cargo test` on machines without
libmpv. `dev-run.ps1` uses its own `--target-dir`, because a plain `cargo test` / `cargo build`
rebuilds the same binary **without** `mpv-runtime` and silently replaces it. The next launch then
reports "Playback runtime is disabled in this build", which looks exactly like a regression.
:::

## Package

```powershell
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
```

Artifacts land in `dist/YoYoVideo-windows-x64/` and `dist/YoYoVideo-windows-x64.zip`.

Verification and smoke test:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -RequireRuntime
pwsh -NoProfile -File scripts/smoke-package.ps1 -Platform windows-x64 -RequireRuntime
```

`smoke-package.ps1` generates a WAV, actually decodes it with the packaged playback core, and waits
for duration, position and track events — **an artifact that cannot decode never reaches the
release page**.

Optional NSIS installer:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe
```

## Updating the playback core

The core's version and checksum live in `runtime/manifest.toml`. To upgrade, change `source_url`,
`sha256` and `version`, then re-run fetch, package and smoke. `version` is part of the cache
filename, so changing it forces a fresh download instead of silently reusing the old archive.


## Icons

The app icon is **generated**, not a hand-exported bitmap:

```powershell
node scripts/generate-icons.mjs
```

It renders the same geometry (a rounded square with a play triangle, shared with
`docs-site/docs/public/logo.svg`) into a Windows `.ico` with seven sizes, the PNGs the Linux package
installs, and the PNG the Slint window uses. The outputs are committed; re-run the script after
changing the shape. It needs no image tooling -- the PNG and ICO encoding is in the script.