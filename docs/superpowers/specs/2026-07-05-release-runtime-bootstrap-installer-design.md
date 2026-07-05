# Release Runtime Bootstrap And Installer Design

## Goal

Finish the practical release path for YoYoVideo by making libmpv runtime preparation reproducible, preserving the existing portable package flow, and adding a Windows installer path suitable for early user distribution.

This phase is about delivery infrastructure. It does not add new playback features and does not change the runtime behavior of the player.

## Current State

The project already has:

- A Rust + Slint + libmpv desktop player with playback, controls, playlists, history, settings, subtitle and track controls, and visible video host support behind the `mpv-runtime` feature.
- A documented runtime staging layout under `third_party/mpv/<platform>/`.
- `scripts/package.ps1`, which builds package directories and archives for `windows-x64`, `macos-universal`, and `linux-x64`.
- `scripts/verify-package.ps1`, which validates package structure and required runtime files.
- `.github/workflows/package.yml`, which can download maintainer-provided runtime archives from repository secrets before packaging.
- Manual smoke-test documentation for runtime-enabled packages.

The project still needs:

- A repository-owned runtime manifest that records runtime source, version, checksum, and required files.
- A reusable bootstrap script that prepares `third_party/mpv/<platform>/` locally and in CI.
- Better failure messages that point maintainers to the bootstrap path instead of raw linker or missing-DLL symptoms.
- Windows installer generation in addition to the existing portable zip.
- Package license and release-note files that describe runtime provenance and redistribution obligations.
- Automated validation of the manifest, bootstrap dry-run, package contents, and optional runtime smoke path.

## Scope

This phase includes:

- Adding a runtime manifest for supported platforms.
- Adding a bootstrap script that downloads, verifies, and extracts runtime archives into `third_party/mpv/<platform>/`.
- Updating GitHub Actions packaging to call the same bootstrap path used locally.
- Keeping the existing portable archives:
  - `YoYoVideo-windows-x64.zip`
  - `YoYoVideo-macos-universal.tar.gz`
  - `YoYoVideo-linux-x64.tar.gz`
- Adding a Windows installer output:
  - `YoYoVideo-windows-x64-setup.exe`
- Adding release notes and license notice templates to package output.
- Adding dry-run and validation paths that do not require network access.
- Adding an optional runtime smoke script that opens generated media and verifies mpv events.

This phase excludes:

- macOS `.app` bundling, `.dmg` creation, notarization, or codesigning.
- Linux AppImage, Flatpak, Snap, or distro packaging.
- Windows code signing.
- Automatic GitHub Release publishing.
- Bundling runtime binaries directly in git.
- Replacing legal review for libmpv, FFmpeg, or their dependencies.

## Chosen Approach

Use a manifest-driven bootstrap flow shared by local development and GitHub Actions.

The fixed data flow is:

```text
runtime manifest
  -> scripts/bootstrap-runtime.ps1
  -> third_party/mpv/<platform>/
  -> scripts/package.ps1
  -> scripts/verify-package.ps1
  -> portable archive and optional installer
```

This avoids three separate runtime definitions in documentation, workflow YAML, and local developer notes. The manifest becomes the source of truth. The bootstrap script prepares runtime files only; packaging remains responsible for building YoYoVideo and assembling distributable outputs.

Windows gets the first installer path because it is the primary verified runtime target in the current workspace. macOS and Linux stay on artifact-level tarballs until platform-specific bundle loading and signing paths are designed separately.

## Rejected Approaches

### Keep Secrets-Only Runtime Downloads

The current GitHub Actions workflow can download runtime archives from secrets, but local development still requires manual staging. Keeping that model makes runtime problems hard to reproduce and encourages divergent CI/local layouts.

### Commit Runtime Binaries To Git

Committing libmpv and FFmpeg binaries would simplify bootstrap, but it creates repository bloat and licensing risk. Runtime binaries remain untracked unless redistribution is explicitly reviewed and approved.

### Build Full Native Installers For Every Platform Now

Building Windows installer, macOS `.dmg`, and Linux AppImage in one phase is too broad. It would mix runtime provenance, platform loader behavior, signing, and installer UX into one change. This phase only adds a Windows installer and leaves macOS/Linux as portable artifacts.

## Runtime Manifest

The manifest lives at `runtime/manifest.toml`.

It declares one entry per platform:

- `platform`: one of `windows-x64`, `macos-universal`, or `linux-x64`.
- `version`: the runtime build identifier.
- `source_url`: archive URL used by bootstrap.
- `sha256`: checksum of the downloaded archive.
- `archive_format`: `zip`, `7z`, `tar.gz`, or `tar.xz`.
- `strip_components`: number of leading archive path components to remove during extraction.
- `destination`: the relative staging directory, such as `third_party/mpv/windows-x64`.
- `required_files`: files that must exist after extraction.
- `license_files`: files or glob patterns copied into package license notices when available.
- `notes`: concise provenance text shown in generated release notes.

Windows required files are:

- `lib/mpv.lib`
- `bin/mpv-2.dll`

macOS required files are:

- `lib/libmpv.dylib`

Linux required files are:

- `lib/libmpv.so` or a versioned `lib/libmpv.so*`

The manifest should support maintainer override URLs through environment variables. For example, `YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL` can override the manifest URL while still requiring the manifest checksum unless an explicit `-AllowUnverifiedOverride` flag is passed.

## Bootstrap Script

The primary script is `scripts/bootstrap-runtime.ps1`.

It accepts:

- `-Platform windows-x64|macos-universal|linux-x64`
- `-Manifest runtime/manifest.toml`
- `-DestinationRoot third_party/mpv`
- `-DryRun`
- `-Force`
- `-AllowUnverifiedOverride`

Responsibilities:

- Parse the manifest and select the requested platform.
- Resolve the effective runtime URL, including supported environment overrides.
- Download to a deterministic cache under `.cache/runtime/`.
- Verify the archive checksum.
- Extract into a temporary directory.
- Normalize the extracted layout into `third_party/mpv/<platform>/`.
- Validate every required file.
- Print a short summary with platform, version, source URL, checksum, and destination.
- Fail with actionable errors when URL, checksum, extraction, or required files are invalid.

`-DryRun` must not download or modify files. It prints the platform entry, effective URL, destination, expected checksum, and required files. This path can run in CI without network dependency.

The script should be PowerShell-first because the repository already uses PowerShell for cross-platform package scripts. A shell wrapper can be added later if needed, but it is not required for this phase.

## Package Pipeline

`scripts/package.ps1` remains the main package entry point.

Changes:

- Add an optional `-BootstrapRuntime` switch.
- When `-RequireRuntime -BootstrapRuntime` is used, call `scripts/bootstrap-runtime.ps1` before building.
- When runtime files are missing and `-BootstrapRuntime` is not used, print a message that includes the exact bootstrap command.
- Copy generated release notes and license notices into the package directory.
- Continue to call `scripts/verify-package.ps1` before archive creation.

The package layout remains inspectable:

```text
dist/
  YoYoVideo-windows-x64/
    README.md
    RELEASE-NOTES.md
    LICENSES/
      README.md
      runtime-provenance.md
      third-party-notices/
    docs/
      runtime-dependencies.md
      manual-smoke-checklist.md
    bin/
      yoyovideo-desktop.exe
      mpv-2.dll
      other runtime DLLs
```

macOS and Linux package directories use the same docs and license layout, with platform runtime files in `bin/`.

## Windows Installer

Windows installer generation is added after portable package verification.

Use NSIS for this phase because it is lightweight, scriptable, and maps cleanly to the current portable package layout. WiX/MSI remains a future option if enterprise deployment requirements appear.

New files:

- `installer/windows/yoyovideo.nsi`
- `scripts/build-installer.ps1`

`scripts/build-installer.ps1` accepts:

- `-PackageDir dist/YoYoVideo-windows-x64`
- `-OutputPath dist/YoYoVideo-windows-x64-setup.exe`
- `-Version <semver-or-build-id>`

Installer behavior:

- Install the verified package directory into `%LocalAppData%\Programs\YoYoVideo` by default.
- Create Start Menu shortcut.
- Provide an uninstall entry.
- Preserve the same bundled runtime DLLs as the portable package.
- Include README, release notes, docs, and license notices.

If NSIS is missing, the installer step fails with a concise message that explains how to install NSIS. Portable package generation remains usable when installer generation is not requested.

## GitHub Actions

`.github/workflows/package.yml` should stop duplicating archive download logic.

Each package job should:

- Checkout repository.
- Install Rust.
- Run bootstrap dry-run to validate manifest.
- Run runtime bootstrap for the target platform.
- Run `scripts/package.ps1 -Platform <platform> -Configuration release -RequireRuntime`.
- On Windows, run `scripts/build-installer.ps1` after portable package verification.
- Upload portable archive.
- Upload Windows installer when generated.

Maintainer override URLs remain supported through repository secrets:

- `YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL`
- `YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL`
- `YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL`

The workflow must fail before packaging if the runtime manifest is invalid, the checksum mismatches, or required files are missing.

## Release Notes And License Notices

Package output should include generated or templated release documentation:

- `RELEASE-NOTES.md`: version, build date, platform, included runtime version, and known limitations.
- `LICENSES/README.md`: short explanation of YoYoVideo and bundled runtime notices.
- `LICENSES/runtime-provenance.md`: runtime source URL, checksum, version, and packaging date.
- `LICENSES/third-party-notices/`: copied license files when present in the runtime archive.

If license files declared by the manifest are missing:

- Development packaging prints a warning.
- Release packaging fails unless an explicit `-AllowMissingRuntimeLicenseFiles` switch is used.

The generated notices must state that final public redistribution still requires review of the exact libmpv/FFmpeg build and its enabled codecs. This keeps the package honest without blocking internal smoke artifacts.

## Runtime Smoke Script

Add `scripts/smoke-runtime.ps1` as an optional verification path.

Responsibilities:

- Generate a small WAV file in a temporary directory.
- Build or run a small Rust probe against `yoyovideo-desktop` with `mpv-runtime`.
- Prefix `PATH` or platform-equivalent loader path with the staged runtime directory.
- Construct the real desktop backend.
- Open the WAV file.
- Drain events for a short bounded interval.
- Pass only if `DurationChanged`, `TracksChanged`, and `PositionChanged` are observed with no backend error events.

This smoke test is not part of default `cargo test`. It is intended for release validation, Windows-first local checks, and optional CI runs where runtime files are available.

## Error Handling

Runtime and release failures should point to the next action:

- Missing manifest entry: `No runtime manifest entry for <platform>`.
- Missing runtime files: `Run scripts/bootstrap-runtime.ps1 -Platform <platform>, then retry packaging`.
- Checksum mismatch: print expected checksum, actual checksum, and downloaded file path.
- Extraction failure: print archive path, format, and extraction command.
- Required file missing after extraction: print the missing relative file and destination root.
- Installer tool missing: print the expected `makensis` command and continue to leave portable package artifacts intact when installer generation is optional.
- Missing license files in release mode: print the declared missing license paths and the manifest entry that declared them.

Errors should avoid raw tool-only messages where the repository can provide a clearer instruction.

## Testing Strategy

Automated validation:

- `cargo fmt --check`
- `cargo test`
- Manifest validation through `scripts/bootstrap-runtime.ps1 -Platform <platform> -DryRun` for every platform.
- Bootstrap missing-file validation with a small local fixture archive.
- Checksum mismatch validation with a fixture archive and wrong checksum.
- `scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -BootstrapRuntime` when runtime network access is available.
- `scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime`
- `scripts/build-installer.ps1` against a verified Windows package when NSIS is available.
- Optional `scripts/smoke-runtime.ps1 -Platform windows-x64` after bootstrap.

Manual validation:

- Run bootstrap on a clean Windows machine.
- Build the Windows portable package.
- Launch from the extracted portable package.
- Build the Windows installer and install it.
- Launch from Start Menu shortcut.
- Confirm the uninstall entry removes the installed files.
- Run the existing manual smoke checklist against the portable and installed app.

## Success Criteria

- A clean Windows development machine can prepare runtime files using one bootstrap command.
- The same bootstrap command path is used by GitHub Actions packaging jobs.
- Missing runtime files produce an actionable bootstrap instruction.
- The manifest records runtime version, URL, checksum, required files, and runtime provenance.
- Portable package generation still works for Windows, macOS, and Linux.
- Windows packaging can also produce `YoYoVideo-windows-x64-setup.exe`.
- Package output includes release notes, runtime provenance, docs, and license notice scaffolding.
- Runtime checksum mismatches fail before building the player.
- Installer generation does not weaken portable package verification.
- Default tests do not require libmpv or network access.

## Relationship To Later Work

After this phase, YoYoVideo should be easier to build, verify, and distribute for early testers, especially on Windows.

Later release work should handle:

- Windows code signing.
- GitHub Release publishing with checksums.
- macOS `.app` bundle layout, loader paths, `.dmg`, signing, and notarization.
- Linux AppImage, Flatpak, or distribution-specific packages.
- Release-channel update checks.
- Final legal review of runtime redistribution obligations.
