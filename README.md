# YoYoVideo

Rust + Slint + libmpv cross-platform desktop media player.

## Workspace

- `crates/yoyo-core`: playback/session domain logic
- `crates/yoyo-mpv`: libmpv adapter and render bridge
- `apps/yoyovideo-desktop`: Slint desktop application

## MVP Scope

- Local files and folders
- Network URLs
- Playback, seeking, speed, zoom, rotation, A-B repeat
- Playlist, history, context menu, and keyboard shortcuts

## Development

- Default tests use dry-run playback seams and do not require libmpv: `cargo test`
- Real playback alpha: `cargo run -p yoyovideo-desktop --features mpv-runtime`
- On Windows, the runtime feature requires `mpv.lib` at link time and the matching mpv DLLs at run time.
