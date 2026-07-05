# Windows x64 libmpv Runtime Files

Expected files for runtime-enabled packaging:

- `lib/mpv.lib`: MSVC import library used by Rust linking when `mpv-runtime` is enabled.
- `bin/mpv-2.dll`: libmpv runtime library loaded by the packaged desktop app.
- `bin/avcodec-*.dll`, `bin/avformat-*.dll`, `bin/avutil-*.dll`, and related FFmpeg/runtime DLLs from the same libmpv build.

Keep binaries untracked until licensing is approved. The package script copies `bin/*` into the package `bin/` directory.
