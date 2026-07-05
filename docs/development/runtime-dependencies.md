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
