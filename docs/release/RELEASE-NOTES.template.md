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
- Wayland single-video playback has an experimental Slint/OpenGL-composited path.
  It has not been verified on a Wayland desktop; grid playback remains unsupported.
- macOS video goes through mpv's render API. It is compile-verified for both architectures but has
  not been run on real hardware, so treat it as unproven.

## Known Limitations

- Only the platforms whose `runtime/manifest.toml` entry is marked `available = true` are built.
- Platform code signing and store distribution are outside this release phase.
