---
title: What is YoYoVideo
description: What YoYoVideo is, what it does well, and where its boundaries are.
---

# What is YoYoVideo

YoYoVideo is a local video player written in Rust, drawn with
[Slint](https://slint.dev/), and played back through [libmpv](https://mpv.io/).

It exists for one specific reason: **when you want to watch several videos at once, most players
will not do it.** Single-file playback is table stakes. Dropping in a folder, playing every tile
simultaneously, and giving each one its own pause, seek and volume — few players bother.

## What it does

- **Multi-tile batch playback.** Drag in several files and each tile becomes an independent
  session: its own transport, its own volume, and tile sizes you can drag to resize.
- **Full transport control.** Exact time jumps, speed, zoom, rotation, picture panning, A-B loop,
  chapter jumps, and droppable markers on the timeline.
- **Subtitles and tracks.** External subtitle files, adjustable subtitle delay, scale and vertical
  position, audio track switching and cycling channel modes.
- **Picture tools.** Common video filter presets plus brightness, contrast and saturation.
- **A quiet frameless shell.** The title bar and control bar are drawn by the app and fade out
  after a few idle seconds.
- **Local playback, no telemetry.** Local media needs no network. Optional update checks contact GitHub, without uploading media or playback history.

## What it is not

- **Not a transcoder.** YoYoVideo plays; it does not edit or export.
- **Not a streaming client.** It can open a network URL, but it is not a product with
  recommendations and accounts.
- **Platform validation varies.** A package target is not proof of native playback or upgrades; see [Installation](/en/guide/installation).

## How it is put together

| Crate | Role |
| --- | --- |
| `crates/yoyo-core` | Playback session and command logic, with no playback core dependency |
| `crates/yoyo-mpv` | libmpv adapter, translating core events into domain events |
| `apps/yoyovideo-desktop` | Slint desktop app, platform integration, and the shipped binary |

That split is deliberate. There is not a single line of libmpv code in `yoyo-core`, so the entire
playback domain can be tested on a machine that has never heard of libmpv — which is exactly how the
default `cargo test` works.
