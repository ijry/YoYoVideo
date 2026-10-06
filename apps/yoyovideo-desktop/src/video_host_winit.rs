use std::sync::Arc;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{NativeVideoWindowId, VideoHost, VideoHostBounds, VideoHostError};

/// The video surface for one video area.
///
/// Two shapes, because the platforms genuinely differ:
///
/// - On X11 and Win32 mpv embeds into a native window handle (`--wid`), so all
///   this owns is a child window and the handle it hands over.
/// - On macOS mpv has no `--wid` embedding at all, so the app owns an OpenGL
///   context attached to the child window's view and renders through mpv's render
///   API instead. That state lives in `macos`.
pub struct WinitVideoHost {
    window: Arc<Window>,
    window_id: WindowId,
    #[cfg(target_os = "macos")]
    macos: Option<MacVideoSurface>,
}

/// macOS render state: the GL surface plus mpv's render context into it.
#[cfg(target_os = "macos")]
struct MacVideoSurface {
    gl: crate::macos_gl::GlSurface,
    render: Option<yoyo_mpv::MpvGlRenderContext>,
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
        let window = Arc::new(window);

        #[cfg(target_os = "macos")]
        let macos = Some(Self::create_macos_surface(&window)?);

        Ok(Self {
            window,
            window_id,
            #[cfg(target_os = "macos")]
            macos,
        })
    }

    /// Builds the GL surface for a freshly created child window.
    #[cfg(target_os = "macos")]
    fn create_macos_surface(window: &Window) -> Result<MacVideoSurface, VideoHostError> {
        use raw_window_handle::HasWindowHandle;

        let handle = window
            .window_handle()
            .map_err(|error| {
                VideoHostError::new(format!("video host handle unavailable: {error}"))
            })?
            .as_raw();
        let RawWindowHandle::AppKit(appkit) = handle else {
            return Err(VideoHostError::new("the macOS host expects an AppKit window handle"));
        };

        // SAFETY: `appkit.ns_view` is the live NSView of the window winit just
        // created, and we are on the main thread (window creation requires it).
        let gl = unsafe { crate::macos_gl::GlSurface::new(appkit.ns_view) }
            .map_err(VideoHostError::new)?;
        Ok(MacVideoSurface { gl, render: None })
    }

    /// Creates mpv's render context against this surface.
    ///
    /// macOS only, and must run after the window exists and the GL context has been
    /// made current.
    ///
    /// # Safety
    ///
    /// `backend` must outlive this host; mpv keeps a pointer into it.
    #[cfg(target_os = "macos")]
    pub unsafe fn attach_render_context(
        &mut self,
        backend: &yoyo_mpv::MpvBackend,
    ) -> Result<(), VideoHostError> {
        let surface =
            self.macos.as_mut().ok_or_else(|| VideoHostError::new("no macOS video surface"))?;

        surface.gl.make_current();
        let mut render = unsafe { backend.create_gl_render_context(surface.gl.get_proc_address()) }
            .map_err(|error| VideoHostError::new(error.to_string()))?;

        // mpv calls this from its own thread when it wants a redraw. winit's
        // `request_redraw` is the thread-safe way back into the event loop, and the
        // render itself happens on the main thread in `render_frame`.
        let needs_render = surface.gl.needs_render();
        let window = Arc::clone(&self.window);
        let callback = move || {
            needs_render.store(true, std::sync::atomic::Ordering::Release);
            window.request_redraw();
        };
        // SAFETY: the callback is Send and only touches an atomic and a window
        // handle; the render context owns the closure and outlives it.
        unsafe { render.set_update_callback(callback) }
            .map_err(|error| VideoHostError::new(error.to_string()))?;

        surface.render = Some(render);
        Ok(())
    }

    /// Draws one frame, if mpv has asked for one.
    ///
    /// macOS only.
    #[cfg(target_os = "macos")]
    pub fn render_frame(&mut self) -> Result<(), VideoHostError> {
        // Read the size before borrowing the surface: it comes from the window,
        // which self still owns.
        let (width, height) = self.physical_size();

        let Some(surface) = self.macos.as_mut() else {
            return Ok(());
        };
        let Some(render) = surface.render.as_ref() else {
            return Ok(());
        };
        surface.gl.make_current();
        // SAFETY: the surface was just made current, and the size comes from the
        // same window the context is attached to.
        unsafe { surface.gl.render(render, width as i32, height as i32) }
            .map_err(|error| VideoHostError::new(error.to_string()))
    }

    /// Whether mpv has requested a redraw since the last frame.
    #[cfg(target_os = "macos")]
    pub fn render_requested(&self) -> bool {
        self.macos.as_ref().is_some_and(|surface| {
            surface.gl.needs_render().load(std::sync::atomic::Ordering::Acquire)
        })
    }

    /// Tells the GL context its drawable changed size.
    #[cfg(target_os = "macos")]
    pub fn refresh_drawable(&self) {
        if let Some(surface) = self.macos.as_ref() {
            surface.gl.update_drawable();
        }
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
            // macOS is handled by the render API instead; see `attach_render_context`.
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
        #[cfg(target_os = "macos")]
        {
            // Availability on macOS is "the GL surface exists and mpv has been
            // wired into it", not "there is a window id".
            return self.macos.as_ref().is_some_and(|surface| surface.render.is_some());
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.raw_window_id().is_ok()
        }
    }
}
