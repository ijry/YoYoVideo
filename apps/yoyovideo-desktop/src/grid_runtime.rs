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
use yoyo_core::{AppCommand, AppSession, MediaLocator, PlayerState};
use yoyo_mpv::MpvBackend;

use crate::video_host_winit::WinitVideoHost;
use crate::{
    LogicalVideoRect, VideoAreaPointer, VideoHost, accepted_tile_count, active_after_removal,
    aspect_from_size, plan_grid,
};

/// One video in the grid.
pub struct GridTile {
    session: AppSession<MpvBackend>,
    host: WinitVideoHost,
    /// Per-tile gesture tracking; the picture is a native window Slint never sees.
    pointer: VideoAreaPointer,
    title: String,
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
}

/// A tile's state, flattened for the UI. Keeps Slint types out of this module.
#[derive(Debug, Clone, PartialEq)]
pub struct GridTileView {
    pub title: String,
    pub paused: bool,
    pub muted: bool,
    pub volume: i32,
    pub selected: bool,
}

#[derive(Default)]
pub struct GridRuntime {
    tiles: Vec<GridTile>,
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
        let existing = self.tiles.len() + self.pending_open.len();
        let (accepted, dropped) = accepted_tile_count(existing, locators.len());
        self.pending_open.extend(locators.into_iter().take(accepted));
        self.dropped += dropped;
        dropped
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
        session: AppSession<MpvBackend>,
        host: WinitVideoHost,
        title: String,
    ) {
        self.tiles.push(GridTile {
            session,
            host,
            pointer: VideoAreaPointer::default(),
            title,
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
        let tile = self.tiles.get_mut(index).ok_or_else(|| "no such tile".to_string())?;
        tile.session.handle_command(command).map_err(|error| error.to_string())
    }

    /// Brings every tile to the same paused state.
    ///
    /// Compares each tile first, because there is no absolute set-paused command: blindly
    /// toggling would invert a mixed grid rather than converging it.
    pub fn set_all_paused(&mut self, paused: bool) {
        for tile in &mut self.tiles {
            if tile.session.state().paused != paused {
                let _ = tile.session.handle_command(AppCommand::TogglePause);
            }
        }
    }

    /// True when at least one tile is playing, used to label the play-all button.
    pub fn any_playing(&self) -> bool {
        self.tiles.iter().any(|tile| !tile.state().paused)
    }

    /// Drains every tile's mpv event queue.
    pub fn poll_all(&mut self) {
        for tile in &mut self.tiles {
            let _ = tile.session.poll_backend();
        }
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
        let aspects: Vec<f32> = self
            .tiles
            .iter()
            .map(|tile| aspect_from_size(tile.state().video_width, tile.state().video_height))
            .collect();
        let cells = plan_grid(container, &aspects, strip_height, gutter);

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
                GridTileView {
                    title: tile.title.clone(),
                    paused: state.paused,
                    muted: state.muted,
                    volume: i32::from(state.volume_percent),
                    selected: self.active == Some(index),
                }
            })
            .collect()
    }
}
