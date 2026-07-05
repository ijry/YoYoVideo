# libmpv Runtime Staging

This directory is the repository-owned staging layout used by `scripts/package.ps1`.

The repository tracks README files and `.gitkeep` placeholders only. Do not commit libmpv, FFmpeg, or platform runtime binaries until redistribution licensing has been reviewed and explicitly approved.

## Platforms

- `windows-x64`: place `mpv.lib` in `lib/` and runtime DLLs such as `mpv-2.dll` in `bin/`.
- `macos-universal`: place `libmpv.dylib` in `lib/`.
- `linux-x64`: place `libmpv.so` or versioned `libmpv.so*` files in `lib/`.

## Licensing

Before publishing packages externally, record the exact source of the runtime binaries and include license notices for libmpv, FFmpeg, and every bundled dependency in the generated package `LICENSES/` directory.
