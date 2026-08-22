use std::sync::Arc;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{NativeVideoWindowId, VideoHost, VideoHostBounds, VideoHostError};

pub struct WinitVideoHost {
    window: Arc<Window>,
    window_id: WindowId,
}

impl WinitVideoHost {
    pub fn new_child(
        event_loop: &ActiveEventLoop,
        parent: &Window,
    ) -> Result<Self, VideoHostError> {
        let parent_handle = parent
            .window_handle()
            .map_err(|error| {
                VideoHostError::new(format!("parent window handle unavailable: {error}"))
            })?
            .as_raw();
        let attributes = unsafe {
            WindowAttributes::default()
                .with_title("YoYoVideo Video Host")
                .with_visible(false)
                .with_decorations(false)
                .with_parent_window(Some(parent_handle))
        };
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| VideoHostError::new(format!("create video host window: {error}")))?;
        let window_id = window.id();
        Ok(Self { window: Arc::new(window), window_id })
    }

    /// Identifies this window in the winit event loop. Slint does not own it, so its
    /// events only reach the custom application handler.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Physical inner size, clamped to at least 1x1 so callers can divide by it.
    pub fn physical_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width.max(1), size.height.max(1))
    }

    fn raw_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError> {
        let handle = self
            .window
            .window_handle()
            .map_err(|error| {
                VideoHostError::new(format!("video host handle unavailable: {error}"))
            })?
            .as_raw();
        match handle {
            RawWindowHandle::Win32(handle) => Ok(NativeVideoWindowId(handle.hwnd.get() as u64)),
            RawWindowHandle::Xlib(handle) => Ok(NativeVideoWindowId(u64::from(handle.window))),
            RawWindowHandle::Xcb(handle) => Ok(NativeVideoWindowId(u64::from(handle.window.get()))),
            _ => Err(VideoHostError::new(
                "Video embedding is not supported on this windowing backend yet",
            )),
        }
    }
}

impl VideoHost for WinitVideoHost {
    fn mpv_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError> {
        self.raw_window_id()
    }

    fn set_bounds(&mut self, bounds: VideoHostBounds) -> Result<(), VideoHostError> {
        self.window.set_outer_position(PhysicalPosition::new(bounds.x, bounds.y));
        let _ = self.window.request_inner_size(PhysicalSize::new(bounds.width, bounds.height));
        Ok(())
    }

    fn show(&mut self) -> Result<(), VideoHostError> {
        self.window.set_visible(true);
        Ok(())
    }

    fn hide(&mut self) -> Result<(), VideoHostError> {
        self.window.set_visible(false);
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.raw_window_id().is_ok()
    }
}
