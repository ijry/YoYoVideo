//! A video texture created, rendered and released in Slint's current GL context.
//! Native child-window paths keep using `video_host_winit`; Wayland uses this
//! surface because mpv cannot embed into a Wayland `--wid`.

use std::ffi::{CStr, c_void};
use std::num::NonZeroU32;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use raw_window_handle::RawWindowHandle;
use slint::{BorrowedOpenGLTextureBuilder, Image};
use yoyo_mpv::{GlFunctions, MpvBackend, MpvGlRenderContext, TextureTarget};

pub fn requires_compositing(handle: RawWindowHandle) -> bool {
    matches!(handle, RawWindowHandle::Wayland(_))
}

/// Empty/uninitialized layouts wait for the next draw instead of allocating a
/// bogus texture. Round up so fractional scale factors never undersize it.
pub fn physical_video_size(width: f32, height: f32, scale: f32) -> Option<(i32, i32)> {
    if [width, height, scale].iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return None;
    }
    let (w, h) = ((width * scale).ceil(), (height * scale).ceil());
    if !w.is_finite() || !h.is_finite() || w >= i32::MAX as f32 || h >= i32::MAX as f32 {
        return None;
    }
    Some((w.max(1.0) as i32, h.max(1.0) as i32))
}

/// Coalesce mpv's decoder-thread notifications. A callback must never touch UI,
/// GL, or mpv; it only queues a redraw on the UI event loop.
#[derive(Clone, Default)]
pub struct FrameWakeup(Arc<AtomicBool>);

impl FrameWakeup {
    pub fn request(&self) -> bool {
        !self.0.swap(true, Ordering::AcqRel)
    }
    pub fn take(&self) -> bool {
        self.0.swap(false, Ordering::AcqRel)
    }
}

#[derive(Default)]
pub struct CompositedVideo {
    functions: Option<GlFunctions>,
    target: Option<TextureTarget>,
    context: Option<MpvGlRenderContext>,
    wakeup: FrameWakeup,
    failed: bool,
    rendered: bool,
    visibility: Option<crate::VisibilityPermit>,
}

impl CompositedVideo {
    fn permit(&mut self) -> &crate::VisibilityPermit {
        self.visibility.get_or_insert_with(|| {
            let value = crate::VisibilityPermit::default();
            value.request_visible(true);
            value
        })
    }
    pub fn set_privacy_blocked(&mut self, blocked: bool) {
        self.permit().set_privacy_blocked(blocked);
        if blocked {
            self.rendered = false;
            self.wakeup.take();
        }
    }
    pub fn set_media_access(
        &mut self,
        access: Option<Arc<dyn yoyo_core::PlaybackAccess>>,
        media: Option<yoyo_core::privacy::MediaKey>,
    ) {
        self.permit().set_media_access(access, media);
    }
    pub fn output_allowed(&self) -> bool {
        self.visibility.as_ref().is_none_or(|permit| permit.visible())
    }

    pub fn is_ready(&self) -> bool {
        self.context.is_some() && !self.failed
    }
    pub fn has_context(&self) -> bool {
        self.context.is_some()
    }
    pub fn has_failed(&self) -> bool {
        self.failed
    }
    pub fn mark_failed(&mut self) {
        self.failed = true;
    }

    /// Initialize lazily from BeforeRendering: the playback backend might not
    /// exist at RenderingSetup. The loader is borrowed only for this call.
    ///
    /// # Safety
    /// Slint's GL context must be current. `backend` must outlive this surface's
    /// render context, which must be torn down under the same current GL context.
    pub unsafe fn setup(
        &mut self,
        backend: &MpvBackend,
        loader: &dyn Fn(&CStr) -> *const c_void,
        request_redraw: impl Fn() + Send + 'static,
    ) -> Result<(), String> {
        let functions = unsafe { GlFunctions::load(loader) }.map_err(|e| e.to_string())?;
        let mut context = unsafe {
            functions.with_saved_bindings(|| backend.create_gl_render_context_with_loader(loader))
        }
        .map_err(|e| e.to_string())?;
        let wakeup = self.wakeup.clone();
        unsafe {
            context.set_update_callback(move || {
                if wakeup.request() {
                    request_redraw();
                }
            })
        }
        .map_err(|e| e.to_string())?;
        self.functions = Some(functions);
        self.context = Some(context);
        Ok(())
    }

    /// Render queued frames and redraw the current frame when resized.
    ///
    /// # Safety
    /// The same context as setup must be current. Returned images borrow this
    /// surface's texture and must be cleared before teardown.
    pub unsafe fn render(&mut self, width: i32, height: i32) -> Result<Option<Image>, String> {
        if !self.output_allowed() {
            self.rendered = false;
            self.wakeup.take();
            return Ok(None);
        }

        let (Some(functions), Some(context)) = (self.functions, self.context.as_ref()) else {
            return Ok(None);
        };
        self.wakeup.take();
        let resized = self.target.as_ref().is_none_or(|t| t.size() != (width, height));
        unsafe {
            functions.with_saved_bindings(|| {
                let update = context.update();
                if !resized && !update.frame() {
                    return Ok(None);
                }
                if let Some(target) = self.target.as_mut() {
                    target.resize(&functions, width, height).map_err(|e| e.to_string())?;
                } else {
                    self.target = Some(
                        TextureTarget::new(&functions, width, height).map_err(|e| e.to_string())?,
                    );
                }
                let target = self.target.as_ref().expect("target just created");
                // mpv binds the FBO itself. Flip to Slint's top-left texture origin.
                context
                    .render(target.framebuffer(), width, height, true)
                    .map_err(|e| e.to_string())?;
                self.rendered = true;
                let texture = NonZeroU32::new(target.texture()).expect("allocated texture");
                Ok(Some(
                    BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                        texture,
                        (width as u32, height as u32).into(),
                    )
                    .build(),
                ))
            })
        }
    }

    /// Called at AfterRendering, immediately before Slint presents the frame.
    pub fn report_swap(&mut self) {
        if !self.output_allowed() {
            self.rendered = false;
            return;
        }
        if std::mem::take(&mut self.rendered) {
            if let Some(context) = &self.context {
                context.report_swap();
            }
        }
    }

    /// # Safety
    /// Only call at RenderingTeardown, with the creating context current, after
    /// clearing every Image that borrows this surface's texture.
    pub unsafe fn teardown(&mut self) {
        if let Some(functions) = self.functions.take() {
            unsafe {
                functions.with_saved_bindings(|| {
                    self.context = None;
                    if let Some(mut target) = self.target.take() {
                        target.destroy(&functions);
                    }
                })
            };
        }
        self.wakeup.take();
        self.failed = false;
        self.rendered = false;
    }
}

impl Drop for CompositedVideo {
    fn drop(&mut self) {
        // No current-context guarantee here. The notifier normally tears down
        // first; DesktopRuntime retains the backend in this exceptional case.
        if let Some(context) = self.context.take() {
            tracing::error!(
                "Missing GL teardown; retaining render context to avoid invalid GL access"
            );
            std::mem::forget(context);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raw_window_handle::{RawWindowHandle, WaylandWindowHandle, XlibWindowHandle};

    #[test]
    fn only_wayland_uses_ui_compositing() {
        let wayland =
            RawWindowHandle::Wayland(WaylandWindowHandle::new(std::ptr::NonNull::dangling()));
        assert!(requires_compositing(wayland));
        assert!(!requires_compositing(RawWindowHandle::Xlib(XlibWindowHandle::new(42))));
    }

    #[test]
    fn dimensions_use_physical_pixels_and_defer_empty_or_invalid_layout() {
        assert_eq!(physical_video_size(640.0, 360.0, 1.5), Some((960, 540)));
        assert_eq!(physical_video_size(100.25, 50.25, 1.0), Some((101, 51)));
        for (width, height, scale) in [
            (0.0, 10.0, 1.0),
            (10.0, 0.0, 1.0),
            (10.0, 10.0, 0.0),
            (f32::NAN, 1.0, 1.0),
            (f32::INFINITY, 1.0, 1.0),
        ] {
            assert_eq!(physical_video_size(width, height, scale), None);
        }
    }

    #[test]
    fn redraw_notifications_coalesce_until_render_consumes_them() {
        let wakeup = FrameWakeup::default();
        assert!(wakeup.request());
        assert!(!wakeup.request());
        assert!(wakeup.take());
        assert!(!wakeup.take());
        assert!(wakeup.request());
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "video_surface_gl_smoke.rs"]
mod native_smoke;
