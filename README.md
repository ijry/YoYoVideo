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
