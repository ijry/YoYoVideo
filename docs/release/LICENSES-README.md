# License Notices

This package includes YoYoVideo and the platform runtime files needed for libmpv playback.

- YoYoVideo is licensed under **GPL-3.0-or-later**.
- The bundled runtime is built from mpv and FFmpeg, both **GPL-2.0-or-later**.

`runtime-provenance.md` records exactly which upstream archive this package was built from, with its
URL and SHA-256. Per the GPL, you are entitled to the complete corresponding source:

- YoYoVideo: the git tag this package was built from,
  <https://github.com/ijry/YoYoVideo>
- mpv: <https://github.com/mpv-player/mpv>
- FFmpeg: <https://git.ffmpeg.org/ffmpeg.git>
- The upstream runtime build and its scripts:
  <https://github.com/shinchiro/mpv-winbuild-cmake>

No YoYoVideo patches are applied to the runtime binary. See `LICENSES.md` in the source repository
for the full details.
