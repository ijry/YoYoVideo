# libmpv runtime checklist

- Default `cargo test` keeps playback tests in dry-run mode and does not require libmpv.
- Build the real playback alpha with `cargo run -p yoyovideo-desktop --features mpv-runtime`.
- The desktop `mpv-runtime` feature enables `yoyo-mpv/mpv-runtime` and switches startup to the real backend.
- Without the feature, desktop backend construction fails with a clear runtime-disabled error instead of silently pretending playback works.
- Bundle `libmpv` and its FFmpeg-dependent runtime libraries inside the platform package.
- Do not rely on a user-installed system mpv by default.
- On Windows, provide `mpv.lib` for linking and matching mpv DLLs for runtime loading.
- On macOS, embed libmpv and dependent libraries inside the app bundle or configure deterministic loader paths.
- On Linux, package libmpv dependencies with the app or document the distribution package requirement for developer mode.
- Test both hardware-decoding success and software-decoding fallback paths after video-surface embedding lands.
- Verify Windows DLL search path, macOS app bundle embedding, and Linux runtime lookup strategy.
- Review redistribution obligations for the exact libmpv/FFmpeg build before publishing.

## Runtime Staging For Packages

Package creation reads runtime files from `third_party/mpv/<platform>/`.

### Windows x64

- Required link file: `third_party/mpv/windows-x64/lib/mpv.lib`
- Required runtime file: `third_party/mpv/windows-x64/bin/mpv-2.dll`
- Expected additional runtime files: FFmpeg and dependency DLLs from the same libmpv build, copied into `third_party/mpv/windows-x64/bin/`

### macOS Universal

- Required runtime/link file: `third_party/mpv/macos-universal/lib/libmpv.dylib`
- Later app-bundle work should move this into the final bundle layout and configure deterministic loader paths.

### Linux x64

- Required runtime/link file: `third_party/mpv/linux-x64/lib/libmpv.so` or versioned `libmpv.so*`
- Release packages should not silently rely on an unknown user-installed libmpv.

## GitHub Actions Runtime Archives

The packaging workflow can download maintainer-provided runtime archives before running `scripts/package.ps1`.

Supported repository secrets:

- `YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL`: zip archive whose contents expand into `third_party/mpv/windows-x64/`
- `YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL`: zip archive whose contents expand into `third_party/mpv/macos-universal/`
- `YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL`: zip archive whose contents expand into `third_party/mpv/linux-x64/`

If a secret is absent or the archive lacks required files, packaging fails with a missing-runtime message instead of uploading an incomplete artifact.

## Video Host Requirements

Visible video uses mpv's `wid` window binding. The desktop app creates a native video host and passes that id to mpv before initialization.

- Windows: required first target for native video host embedding.
- Linux X11: required design target using an X11 window id.
- Wayland: reports unsupported embedding unless a verified host path is implemented.
- macOS: reports unsupported embedding unless a verified host path is implemented.
