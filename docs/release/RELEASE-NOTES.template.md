# YoYoVideo Release Notes

- Version: {{VERSION}}
- Platform: {{PLATFORM}}
- Build date UTC: {{BUILD_DATE_UTC}}
- Runtime version: {{RUNTIME_VERSION}}

## Included Artifacts

- Portable package for {{PLATFORM}}
- Bundled libmpv runtime staged from `runtime/manifest.toml`
- Runtime provenance and GPL source-availability notices under `LICENSES/`

## Requirements

- No separately installed mpv or third-party codec pack is needed; the playback core is bundled.
- Native video embedding is currently implemented on Windows only. On macOS and Wayland the app
  opens and reports the limitation instead of showing a video surface.

## Known Limitations

- Only the platforms whose `runtime/manifest.toml` entry is marked `available = true` are built.
- Platform code signing and store distribution are outside this release phase.
