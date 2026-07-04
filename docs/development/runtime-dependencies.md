# libmpv runtime checklist

- Bundle `libmpv` and its FFmpeg-dependent runtime libraries inside the platform package.
- Do not rely on a user-installed system mpv by default.
- Test both hardware-decoding success and software-decoding fallback paths.
- Verify Windows DLL search path, macOS app bundle embedding, and Linux runtime lookup strategy.
- Review redistribution obligations for the exact libmpv/FFmpeg build before publishing.
