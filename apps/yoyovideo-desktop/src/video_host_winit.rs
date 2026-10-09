use std::sync::Arc;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::{
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{NativeVideoWindowId, VideoHost, VideoHostBounds, VideoHostError};
#[cfg(not(target_os = "macos"))]
use slint::winit_030::winit::dpi::{PhysicalPosition, PhysicalSize};

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
    visibility: crate::VisibilityPermit,
    #[cfg(target_os = "macos")]
    macos: Option<MacVideoSurface>,
}

/// macOS render state: the GL surface plus mpv's render context into it.
#[cfg(target_os = "macos")]
struct MacVideoSurface {
    placement: Arc<dispatch2::MainThreadBound<crate::macos_video_window::MacVideoWindow>>,
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
        #[cfg(target_os = "macos")]
        let attributes = {
            use slint::winit_030::winit::platform::macos::WindowAttributesExtMacOS;
            attributes.with_active(false).with_has_shadow(false)
        };
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| VideoHostError::new(format!("create video host window: {error}")))?;
        let window_id = window.id();
        let window = Arc::new(window);

        #[cfg(target_os = "macos")]
        let macos = Some(Self::create_macos_surface(parent, &window)?);

        Ok(Self {
            window,
            window_id,
            visibility: crate::VisibilityPermit::default(),
            #[cfg(target_os = "macos")]
            macos,
        })
    }

    /// Builds the GL surface for a freshly created child window.
    #[cfg(target_os = "macos")]
    fn create_macos_surface(
        parent: &Window,
        window: &Window,
    ) -> Result<MacVideoSurface, VideoHostError> {
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
        let mtm = objc2::MainThreadMarker::new()
            .ok_or_else(|| VideoHostError::new("video windows require the main thread"))?;
        let placement = crate::macos_video_window::MacVideoWindow::new(parent, window, mtm)?;
        let placement = Arc::new(dispatch2::MainThreadBound::new(placement, mtm));
        Ok(MacVideoSurface { placement, gl, render: None })
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

        // mpv may invoke this while holding locks that synchronous player calls
        // on the UI thread need. winit::request_redraw is thread-safe, but on
        // macOS it synchronously dispatches to the main thread: calling it here
        // deadlocks both threads. Only enqueue work here; touch winit on the UI
        // turn, just like the composited render path. Never revive a closed host.
        let needs_render = surface.gl.needs_render();
        let window = Arc::downgrade(&self.window);
        let callback = move || {
            needs_render.store(true, std::sync::atomic::Ordering::Release);
            let window = window.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = window.upgrade() {
                    window.request_redraw();
                }
            });
        };
        // SAFETY: the callback is Send and only updates an atomic and posts a
        // nonblocking event. The render context owns and unregisters the closure.
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
        if !self.visibility.visible() {
            self.sync_visibility()?;
            return Ok(());
        }

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

    fn update_window(
        &self,
        update: impl FnOnce(&Window) + Send + 'static,
    ) -> Result<(), VideoHostError> {
        #[cfg(target_os = "macos")]
        {
            // A Slint display-link timer can run outside a winit event callback.
            // AppKit may synchronously emit Moved/Resized/Focused from these setters,
            // re-entering DesktopWinitHandler while its runtime is still borrowed.
            // Queue the native operation onto a clean event-loop turn instead; winit
            // can then deliver all events normally, without dropped events or panics
            // escaping the Objective-C callback boundary. Do not keep a closed host alive.
            let window = Arc::downgrade(&self.window);
            slint::invoke_from_event_loop(move || {
                if let Some(window) = window.upgrade() {
                    update(&window);
                }
            })
            .map_err(|error| {
                VideoHostError::new(format!("queue video host window update: {error}"))
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            update(&self.window);
            Ok(())
        }
    }

    #[cfg(target_os = "macos")]
    fn update_macos_window(
        &self,
        update: impl FnOnce(&crate::macos_video_window::MacVideoWindow) -> Result<(), VideoHostError>
        + Send
        + 'static,
    ) -> Result<(), VideoHostError> {
        let surface =
            self.macos.as_ref().ok_or_else(|| VideoHostError::new("no macOS video surface"))?;
        let placement = Arc::downgrade(&surface.placement);
        self.update_window(move |_| {
            if let (Some(placement), Some(mtm)) =
                (placement.upgrade(), objc2::MainThreadMarker::new())
            {
                if let Err(error) = update(placement.get(mtm)) {
                    tracing::warn!("Native video window update failed: {error}");
                }
            }
        })
    }

    fn sync_visibility(&self) -> Result<(), VideoHostError> {
        let permit = self.visibility.clone();
        #[cfg(target_os = "macos")]
        {
            self.update_macos_window(move |window| {
                window.set_visible(permit.visible());
                Ok(())
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.update_window(move |window| window.set_visible(permit.visible()))
        }
    }

    #[cfg(feature = "privacy-qa")]
    pub(crate) fn native_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(false)
    }

    #[cfg(feature = "privacy-qa")]
    pub(crate) fn qa_visibility_permitted(&self) -> bool {
        self.visibility.visible()
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
        #[cfg(target_os = "macos")]
        {
            self.update_macos_window(move |window| window.set_bounds(bounds))
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.update_window(move |window| {
                window.set_outer_position(PhysicalPosition::new(bounds.x, bounds.y));
                let _ = window.request_inner_size(PhysicalSize::new(bounds.width, bounds.height));
            })
        }
    }

    fn show(&mut self) -> Result<(), VideoHostError> {
        self.visibility.request_visible(true);
        self.sync_visibility()
    }

    fn hide(&mut self) -> Result<(), VideoHostError> {
        self.visibility.request_visible(false);
        self.sync_visibility()
    }

    fn set_privacy_blocked(&mut self, blocked: bool) -> Result<(), VideoHostError> {
        if self.visibility.set_privacy_blocked(blocked) {
            self.sync_visibility()?;
        }
        Ok(())
    }

    fn set_media_access(
        &mut self,
        access: Option<Arc<dyn yoyo_core::PlaybackAccess>>,
        media: Option<yoyo_core::privacy::MediaKey>,
    ) {
        self.visibility.set_media_access(access, media);
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

// Release mpv while its GL context is current, and before the winit window/view
// or the parent binding can be destroyed. Queued native work only holds weak refs.
#[cfg(target_os = "macos")]
impl Drop for WinitVideoHost {
    fn drop(&mut self) {
        if let Some(mut surface) = self.macos.take() {
            surface.gl.make_current();
            drop(surface.render.take());
        }
    }
}
