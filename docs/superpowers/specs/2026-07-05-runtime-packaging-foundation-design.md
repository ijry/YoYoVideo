# Runtime Packaging Foundation Design

## Goal

Create the packaging and GitHub Actions foundation required to eventually ship YoYoVideo as cross-platform software packages that contain the completed player features and the required libmpv runtime files.

This phase does not claim the player is feature-complete. It makes the release pipeline real and repeatable so each later player feature can be verified inside downloadable packages.

## Current State

The project already has:

- A Rust workspace with `yoyo-core`, `yoyo-mpv`, and `yoyovideo-desktop`.
- A dry-run playback test path that works without libmpv.
- A runtime-gated libmpv backend behind `mpv-runtime`.
- Desktop startup that explicitly requires the runtime backend when running the real app.
- Default tests and runtime feature type checks passing locally.

The project is still missing:

- `.github/workflows` CI and packaging workflows.
- A repository-owned runtime layout for libmpv link/runtime files.
- Packaging scripts used by both local development and GitHub Actions.
- Packaged artifacts containing the desktop binary, runtime files, docs, and smoke checklist.
- Real video-surface embedding, full keyboard event routing, complete settings UI, playlist/history UI, and release upload automation.

## Scope

This phase includes:

- Defining `third_party/mpv/` as the local runtime staging layout.
- Adding scripts that validate runtime files and create platform package directories.
- Adding GitHub Actions CI for formatting, tests, and runtime feature type checks.
- Adding GitHub Actions packaging jobs for Windows, macOS, and Linux artifacts.
- Uploading package artifacts from GitHub Actions.
- Documenting how maintainers provide platform libmpv files.
- Keeping the package format simple and inspectable: zip for Windows, tar.gz for Linux and macOS.

This phase excludes:

- Shipping official signed installers.
- Uploading GitHub Releases.
- Solving real Slint video-surface embedding.
- Bundling redistributable libmpv binaries directly in git.
- Claiming the produced package is feature-complete before the remaining player gaps are implemented and manually smoke-tested.

## Packaging Layout

Repository runtime staging:

```text
third_party/mpv/
  README.md
  windows-x64/
    README.md
    lib/
      mpv.lib
    bin/
      mpv-2.dll
      avcodec-*.dll
      avformat-*.dll
      avutil-*.dll
  macos-universal/
    README.md
    lib/
      libmpv.dylib
  linux-x64/
    README.md
    lib/
      libmpv.so*
```

The repository should track README files and `.gitkeep` placeholders only. Actual binary runtime files are supplied by maintainers or CI secrets/artifacts and remain untracked unless licensing is reviewed and explicitly approved.

Generated package layout:

```text
dist/
  YoYoVideo-<platform>/
    README.md
    LICENSES/
    docs/
      manual-smoke-checklist.md
      runtime-dependencies.md
    bin/
      yoyovideo-desktop[.exe]
      runtime files copied from third_party/mpv/<platform>/
```

The package script must fail clearly when runtime-enabled packaging is requested but required runtime files are missing.

## Scripts

### `scripts/package.ps1`

The primary cross-platform packaging entry point.

Responsibilities:

- Accept `-Platform windows-x64|macos-universal|linux-x64`.
- Accept `-Configuration debug|release`.
- Accept `-RequireRuntime`.
- Build `yoyovideo-desktop` with `--features mpv-runtime` when runtime files are required.
- Copy the app binary and platform runtime files into `dist/YoYoVideo-<platform>/`.
- Copy README, runtime dependency docs, smoke checklist, and license notices.
- Create `.zip` on Windows and `.tar.gz` on macOS/Linux.
- Exit non-zero with a concise message if required runtime files are missing.

### `scripts/verify-package.ps1`

Validates produced package contents without launching the GUI.

Responsibilities:

- Confirm package directory exists.
- Confirm app binary exists.
- Confirm docs are included.
- When `-RequireRuntime` is set, confirm required libmpv runtime files are present.
- Print a short summary suitable for GitHub Actions logs.

## GitHub Actions

### `.github/workflows/ci.yml`

Runs on pushes and pull requests.

Jobs:

- `fmt-test`: runs on `ubuntu-latest`, installs stable Rust, runs `cargo fmt --check` and `cargo test`.
- `runtime-check`: matrix over `windows-latest`, `macos-latest`, and `ubuntu-latest`; runs `cargo check -p yoyo-mpv --features mpv-runtime` and `cargo check -p yoyovideo-desktop --features mpv-runtime`.

Runtime check jobs may need platform package installation for libmpv development files. The first implementation should document the expected package manager commands per platform and fail clearly where unavailable.

### `.github/workflows/package.yml`

Runs on manual dispatch and tags matching `v*`.

Jobs:

- `package-windows`: runs `scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime`.
- `package-macos`: runs `scripts/package.ps1 -Platform macos-universal -Configuration release -RequireRuntime`.
- `package-linux`: runs `scripts/package.ps1 -Platform linux-x64 -Configuration release -RequireRuntime`.

Each job uploads artifacts using the current official artifact action. The workflow should be structured so release upload can be added later without rewriting package creation.

## Runtime Files And Licensing

libmpv and FFmpeg builds have redistribution obligations that depend on build options. Before official public distribution, maintainers must:

- Record the exact source of runtime binaries.
- Include license files for libmpv and its dependencies.
- Verify whether the selected FFmpeg/libmpv build is LGPL-compatible or GPL-triggering.
- Ensure GitHub Actions packages do not publish binaries with unknown redistribution status.

Until licensing is approved, package jobs can run against maintainer-provided runtime files and upload artifacts for internal validation.

## Relationship To Remaining Player Features

The final user objective remains broader than this phase. This phase only creates the release foundation.

Remaining feature phases after this foundation:

- Video embedding: replace the placeholder Slint rectangle with real mpv-rendered video.
- Complete player UI: progress bar, volume control, playlist panel, settings page, and history restore.
- Keyboard integration: route actual window keyboard events through `dispatch_shortcut`.
- Runtime polish: deterministic library loading, hardware acceleration fallback messages, and real smoke-test evidence.
- Release automation: tag-triggered GitHub Release creation with checksums after licensing approval.

Every later feature must update package verification or smoke-test docs when it changes user-visible behavior.

## Testing Strategy

Automated tests:

- `cargo fmt --check`
- `cargo test`
- `cargo check -p yoyo-mpv --features mpv-runtime`
- `cargo check -p yoyovideo-desktop --features mpv-runtime`
- `scripts/package.ps1` in dry-run or missing-runtime validation mode.
- `scripts/verify-package.ps1` against generated package directories.

Manual tests:

- Download each GitHub Actions artifact.
- Inspect package layout.
- Launch the app on the target platform with runtime files present.
- Run the manual smoke checklist.

## Success Criteria

- The repository contains documented runtime staging directories for all target platforms.
- Local package scripts can produce a package directory and archive.
- Missing runtime files fail with clear, actionable errors.
- CI runs formatting and default tests.
- Runtime feature type checks run in CI.
- Packaging workflow uploads artifacts for Windows, macOS, and Linux when required runtime files are available.
- The docs state which remaining player features still need implementation before a full-feature package is complete.
