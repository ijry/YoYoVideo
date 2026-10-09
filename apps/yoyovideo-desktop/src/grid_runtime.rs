//! Batch ("grid") playback: several independent videos in one window.
//!
//! Each tile owns its own mpv instance and its own native child window, so play/pause
//! and volume are genuinely independent. The single-video path in `app.rs` is untouched;
//! the two modes are mutually exclusive.
//!
//! Tiles deliberately do **not** participate in history, marker, or subtitle-preference
//! persistence: those stores are keyed only by media locator, so N tiles would race on
//! the same entries.

use slint::winit_030::winit::window::WindowId;
use std::sync::Arc;
use yoyo_core::{
    AppCommand, AppSession, MediaLocator, PlaybackAccess, PlayerState, privacy::MediaKey,
};
use yoyo_mpv::MpvBackend;

use crate::video_host_winit::WinitVideoHost;
use crate::{
    LogicalVideoRect, VideoAreaPointer, VideoHost, accepted_tile_count, active_after_removal,
    aspect_from_size, plan_grid,
};

/// One video in the grid.
///
/// On macOS, `host` is declared before `session` on purpose. Rust drops fields in
/// declaration order, and on macOS the host owns mpv's render context, which keeps
/// a pointer into the mpv handle that `session`'s backend owns. mpv requires the
/// render context to be freed before the handle, so swapping these two would make
/// teardown a use-after-free. Win32 instead keeps the embedding HWND alive until
/// the mpv session has terminated.
pub struct GridTile {
    // Win32 embedding: terminate mpv before destroying the host HWND.
    #[cfg(windows)]
    session: AppSession<MpvBackend>,
    host: WinitVideoHost,
    // macOS: release its render context before the mpv handle.
    #[cfg(not(windows))]
    session: AppSession<MpvBackend>,
    /// Per-tile gesture tracking; the picture is a native window Slint never sees.
    pointer: VideoAreaPointer,
    title: String,
    /// User-chosen size within its grid cell.
    scale: f32,
    /// Scale when the current resize drag started, so the drag is absolute.
    scale_at_drag_start: f32,
}

impl GridTile {
    pub fn state(&self) -> &PlayerState {
        self.session.state()
    }

    pub fn window_id(&self) -> WindowId {
        self.host.window_id()
    }

    pub fn pointer_mut(&mut self) -> &mut VideoAreaPointer {
        &mut self.pointer
    }

    pub fn host_physical_size(&self) -> (u32, u32) {
        self.host.physical_size()
    }

    /// Draws one frame of this tile's video.
    ///
    /// macOS only: every tile owns its own mpv render context, because there is no
    /// `--wid` embedding there. See `build_host_and_backend`.
    #[cfg(target_os = "macos")]
    pub fn render_frame(&mut self) -> Result<(), String> {
        let _ = self.host.set_privacy_blocked(self.session.privacy_blocked());
        let _ = self.session.enforce_privacy();
        self.host.render_frame().map_err(|error| error.to_string())
    }

    /// Tells this tile's GL context that its drawable was resized.
    #[cfg(target_os = "macos")]
    pub fn refresh_drawable(&self) {
        self.host.refresh_drawable();
    }
}

/// A tile's state, flattened for the UI. Keeps Slint types out of this module.
#[derive(Debug, Clone, PartialEq)]
pub struct GridTileView {
    pub title: String,
    pub paused: bool,
    pub muted: bool,
    pub volume: i32,
    pub selected: bool,
    pub privacy_blocked: bool,
}

#[derive(Default)]
pub struct GridRuntime {
    tiles: Vec<GridTile>,
    access: Option<Arc<dyn PlaybackAccess>>,
    /// `ActiveEventLoop` is only available inside a winit event callback, so opening
    /// files parks the locators here and the next event tick creates the windows.
    pending_open: Vec<MediaLocator>,
    active: Option<usize>,
    /// Tiles are hidden while a popup is open, exactly as the single-video surface is.
    suppressed: bool,
    /// Files that did not fit the cap, awaiting a status message.
    dropped: usize,
}

impl GridRuntime {
    pub fn set_playback_access(&mut self, access: Arc<dyn PlaybackAccess>) {
        self.access = Some(access.clone());
        for tile in &mut self.tiles {
            tile.session.set_playback_access(access.clone());
            tile.host
                .set_media_access(Some(access.clone()), tile.session.current_media_key().cloned());
        }
        self.enforce_privacy();
    }

    pub fn playback_access(&self) -> Option<Arc<dyn PlaybackAccess>> {
        self.access.clone()
    }

    pub fn can_open(&self, locator: &MediaLocator) -> bool {
        self.access.as_ref().is_none_or(|access| {
            MediaKey::from_locator(locator).is_ok_and(|key| !access.restricted(&key))
        })
    }

    pub fn enforce_privacy(&mut self) {
        for tile in &mut self.tiles {
            tile.host
                .set_media_access(self.access.clone(), tile.session.current_media_key().cloned());
            let _ = tile.host.set_privacy_blocked(tile.session.privacy_blocked());
        }
        for tile in &mut self.tiles {
            let _ = tile.session.enforce_privacy();
        }
    }

    /// Whether grid mode is showing. True as soon as files are queued, so the UI can
    /// switch over before the windows actually exist.
    pub fn is_active(&self) -> bool {
        !self.tiles.is_empty() || !self.pending_open.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    #[cfg(feature = "privacy-qa")]
    pub(crate) fn qa_snapshot(&self, media_root: &std::path::Path) -> serde_json::Value {
        serde_json::Value::Array(self.tiles.iter().map(|tile| {
            let state=tile.state();
            let fixture=state.current.as_ref().is_some_and(|locator|matches!(locator,MediaLocator::File(path) if path.starts_with(media_root)));
            if !fixture {return serde_json::json!({"external_media":true});}
            let flags=tile.session.backend().qa_output_flags().ok();
            serde_json::json!({"position":state.position_seconds,"paused":state.paused,"user_muted":state.muted,"volume":state.volume_percent,
                "blocked":tile.session.privacy_blocked(),"native_visible":tile.host.native_visible(),"hwnd":tile.host.mpv_window_id().ok().map(|id|id.0),
                "backend_paused":flags.map(|v|v.0),"backend_muted":flags.map(|v|v.1),"backend_idle":flags.map(|v|v.2)})
        }).collect())
    }

    pub fn active_locator(&self) -> Option<MediaLocator> {
        self.active
            .and_then(|index| self.tiles.get(index))
            .and_then(|tile| tile.state().current.clone())
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.tiles.len() {
            self.active = Some(index);
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_open.is_empty()
    }

    /// Queues media for the next event tick, honouring the tile cap.
    ///
    /// Returns how many were dropped, for the caller to surface.
    pub fn queue_open(&mut self, locators: Vec<MediaLocator>) -> usize {
        let count = locators.len();
        let locators: Vec<_> =
            locators.into_iter().filter(|locator| self.can_open(locator)).collect();
        let denied = count - locators.len();
        let existing = self.tiles.len() + self.pending_open.len();
        let (accepted, dropped) = accepted_tile_count(existing, locators.len());
        self.pending_open.extend(locators.into_iter().take(accepted));
        self.dropped += dropped + denied;
        dropped + denied
    }

    pub fn take_pending(&mut self) -> Vec<MediaLocator> {
        std::mem::take(&mut self.pending_open)
    }

    pub fn take_dropped(&mut self) -> usize {
        std::mem::take(&mut self.dropped)
    }

    /// Adds a live tile. The session must already have been told to open its media.
    pub fn push_tile(
        &mut self,
        mut session: AppSession<MpvBackend>,
        mut host: WinitVideoHost,
        title: String,
    ) {
        if let Some(access) = &self.access {
            session.set_playback_access(access.clone());
        }
        host.set_media_access(self.access.clone(), session.current_media_key().cloned());
        let _ = host.set_privacy_blocked(session.privacy_blocked());
        let _ = session.enforce_privacy();
        self.tiles.push(GridTile {
            session,
            host,
            pointer: VideoAreaPointer::default(),
            title,
            scale: crate::MAX_TILE_SCALE,
            scale_at_drag_start: crate::MAX_TILE_SCALE,
        });
        if self.active.is_none() {
            self.active = Some(0);
        }
    }

    pub fn tile_index_for_window(&self, window_id: WindowId) -> Option<usize> {
        self.tiles.iter().position(|tile| tile.window_id() == window_id)
    }

    pub fn tile_mut(&mut self, index: usize) -> Option<&mut GridTile> {
        self.tiles.get_mut(index)
    }

    /// Sends a command to one tile.
    pub fn dispatch(&mut self, index: usize, command: AppCommand) -> Result<(), String> {
        self.enforce_privacy();
        let tile = self.tiles.get_mut(index).ok_or_else(|| "no such tile".to_string())?;
        let result = tile.session.handle_command(command).map_err(|error| error.to_string());
        self.enforce_privacy();
        result
    }

    /// Brings every tile to the same paused state.
    ///
    /// Use an absolute pause command so a protected/already-paused tile never toggles on.
    pub fn set_all_paused(&mut self, paused: bool) {
        self.enforce_privacy();
        for tile in &mut self.tiles {
            let _ = tile.session.handle_command(AppCommand::SetPaused(paused));
        }
    }

    /// True when at least one tile is playing, used to label the play-all button.
    pub fn any_playing(&self) -> bool {
        self.tiles.iter().any(|tile| !tile.state().paused && !tile.session.privacy_blocked())
    }

    /// Remembers the current size so a resize drag can be applied absolutely from where
    /// it started, instead of accumulating every intermediate move.
    pub fn begin_resize(&mut self, index: usize) {
        if let Some(tile) = self.tiles.get_mut(index) {
            tile.scale_at_drag_start = tile.scale;
        }
    }

    /// Applies a resize drag. `fraction` is the drag distance as a fraction of the cell,
    /// measured from where the drag began.
    pub fn resize_by(&mut self, index: usize, fraction: f32) {
        if let Some(tile) = self.tiles.get_mut(index) {
            tile.scale = crate::clamp_tile_scale(tile.scale_at_drag_start + fraction);
        }
    }

    pub fn scale(&self, index: usize) -> Option<f32> {
        self.tiles.get(index).map(|tile| tile.scale)
    }

    /// Drains every tile's mpv event queue.
    pub fn poll_all(&mut self) {
        self.enforce_privacy();
        for tile in &mut self.tiles {
            let _ = tile.session.poll_backend();
        }
        self.enforce_privacy();
    }

    pub fn close(&mut self, index: usize) {
        if index >= self.tiles.len() {
            return;
        }
        let len_before = self.tiles.len();
        // Dropping the tile terminates its mpv handle and destroys its child window.
        self.tiles.remove(index);
        self.active = active_after_removal(self.active, index, len_before);
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.pending_open.clear();
        self.active = None;
        self.dropped = 0;
    }

    /// Hides or reveals every tile's surface. Mirrors the single-video suppression so a
    /// popup is not occluded by the native windows.
    pub fn set_suppressed(&mut self, suppressed: bool) {
        if self.suppressed == suppressed {
            return;
        }
        self.suppressed = suppressed;
        if suppressed {
            for tile in &mut self.tiles {
                let _ = tile.host.hide();
            }
        }
    }

    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }

    /// Lays the tiles out and moves each native window onto its cell.
    ///
    /// Returns the strip rectangles in tile order, **relative to `container`**, because
    /// Slint draws them inside the video area while the native windows are positioned in
    /// window coordinates.
    pub fn sync_layout(
        &mut self,
        container: LogicalVideoRect,
        strip_height: f32,
        gutter: f32,
        scale_factor: f64,
    ) -> Vec<crate::TileRect> {
        self.enforce_privacy();
        let aspects: Vec<f32> = self
            .tiles
            .iter()
            .map(|tile| aspect_from_size(tile.state().video_width, tile.state().video_height))
            .collect();
        let scales: Vec<f32> = self.tiles.iter().map(|tile| tile.scale).collect();
        let cells = plan_grid(container, &aspects, &scales, strip_height, gutter);

        if !self.suppressed {
            for (tile, cell) in self.tiles.iter_mut().zip(&cells) {
                let bounds = LogicalVideoRect {
                    x: cell.video.x,
                    y: cell.video.y,
                    width: cell.video.width,
                    height: cell.video.height,
                }
                .to_physical(scale_factor);
                if tile.host.set_bounds(bounds).is_ok() {
                    let _ = tile.host.show();
                }
            }
        }

        cells
            .iter()
            .map(|cell| crate::TileRect {
                x: cell.strip.x - container.x,
                y: cell.strip.y - container.y,
                width: cell.strip.width,
                height: cell.strip.height,
            })
            .collect()
    }

    pub fn views(&self) -> Vec<GridTileView> {
        self.tiles
            .iter()
            .enumerate()
            .map(|(index, tile)| {
                let state = tile.state();
                let privacy_blocked = tile.session.privacy_blocked();
                GridTileView {
                    title: if privacy_blocked {
                        "Protected content".into()
                    } else {
                        tile.title.clone()
                    },
                    paused: state.paused || privacy_blocked,
                    muted: state.muted || privacy_blocked,
                    privacy_blocked,
                    volume: i32::from(state.volume_percent),
                    selected: self.active == Some(index),
                }
            })
            .collect()
    }
}
