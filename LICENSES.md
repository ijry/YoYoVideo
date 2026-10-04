# Licenses and source availability

YoYoVideo is distributed under **GPL-3.0-or-later** and ships release packages that bundle a
playback core built from **GPL-2.0-or-later** mpv and FFmpeg. GPL-2.0-or-later code can be combined
into a GPL-3.0-or-later work, which is why this project is GPL-3.0-or-later.

Because the shipped binaries are covered by the GPL, everyone receiving them must be able to get the
complete corresponding source. This page is how.

## YoYoVideo itself

| | |
| --- | --- |
| License | GPL-3.0-or-later (full text in [`LICENSE`](LICENSE)) |
| Source | <https://github.com/ijry/YoYoVideo> |

Release packages are built from the commit their git tag points at. To get the source for a
specific release:

```powershell
git clone https://github.com/ijry/YoYoVideo
cd YoYoVideo
git checkout v0.0.1
```

## Bundled playback core

| | |
| --- | --- |
| Upstream build | [`shinchiro/mpv-winbuild-cmake`](https://github.com/shinchiro/mpv-winbuild-cmake) |
| Asset | `mpv-dev-x86_64-20261004-git-413ff0b1cd.7z` |
| SHA-256 | `445fc72a8aa10980504355a5d99c9a0a128c2fa177e029996fb312d9d3193d45` |
| mpv source | <https://github.com/mpv-player/mpv> |
| FFmpeg source | <https://git.ffmpeg.org/ffmpeg.git> |
| mpv license | GPL-2.0-or-later |

The exact `source_url`, `version` and `sha256` for every platform live in
[`runtime/manifest.toml`](runtime/manifest.toml). `scripts/package.ps1` copies the platform's
runtime summary into each package's `LICENSES/runtime-provenance.md`, so a given binary can always
be traced back to the archive it was built from.

The upstream project publishes its build scripts alongside each release, and mpv and FFmpeg are
built from their own upstream git repositories. No YoYoVideo patches are applied to the runtime.

## What YoYoVideo changes

`scripts/fetch-runtime.ps1` renames `libmpv-2.dll` to `mpv-2.dll` and regenerates an MSVC import
library from that DLL's own export table. Both are build-time packaging steps:

- the **DLL bytes are not modified** — only the file name changes
- the import library is *derived from* the shipped DLL's exports; it contains no YoYoVideo code

Nothing in the runtime binary itself is patched.

## Other dependencies

The Rust and Slint dependencies keep their own licenses, each stated in the corresponding crate's
`Cargo.toml` / `LICENSE` file in its published source. Notable ones:

| Dependency | License |
| --- | --- |
| [Slint](https://github.com/slint-ui/slint) | GPL-3.0-or-later (or a commercial license) |
| [libmpv-sys](https://crates.io/crates/libmpv-sys) | LGPL-2.1-or-later (build bindings) |

YoYoVideo is GPL-3.0-or-later, which is compatible with Slint's GPL-3.0 option. If you need
different terms for your own product, Slint offers a commercial license, and a GPL-3.0-or-later
player built on top of it is what you get here.

## Reporting a licensing problem

If something in this repository or a release package is mislicensed, please open an issue at
<https://github.com/ijry/YoYoVideo/issues>.
