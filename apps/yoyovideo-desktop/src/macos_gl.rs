//! macOS video rendering: an OpenGL context the application owns.
//!
//! mpv embeds via `--wid` on X11 and Win32 only. Its macOS backend creates and
//! owns its own `NSWindow`/`NSView` (`initWindow` in `video/out/mac/common.swift`)
//! and never reads `opts->WinID`, so there is no host view to hand it -- the
//! render API is the only supported path, and that needs a GL context the
//! application owns.
//!
//! The pieces:
//!   1. an `NSOpenGLContext` attached to the child window's `NSView`
//!   2. mpv's render context, resolving GL entry points through that context
//!   3. a redraw driven by mpv's update callback, hopping back to the main thread
//!
//! Frames go to framebuffer 0 -- the context's own drawable. mpv's render API
//! documents `fbo: 0` as the default framebuffer, so no FBO or texture
//! bookkeeping is needed.

// Every NSOpenGL type is deprecated in favour of Metal, so this module opts out
// of the deprecation warnings wholesale. Metal is not an alternative here:
// libmpv 3.1.0 exports only MPV_RENDER_API_TYPE_OPENGL, and the render API is
// the only way to embed video on macOS at all.
#![allow(deprecated)]

use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSOpenGLContext, NSOpenGLPFAColorSize, NSOpenGLPFADepthSize, NSOpenGLPFADoubleBuffer,
    NSOpenGLPFAOpenGLProfile, NSOpenGLPixelFormat, NSOpenGLPixelFormatAttribute,
    NSOpenGLProfileVersion3_2Core,
};

use yoyo_mpv::{GetProcAddress, MpvError, MpvGlRenderContext};

// GL constants and the handful of entry points this module calls directly.
// Resolved through the context rather than linked, so no GL crate is needed.
const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
const GL_DEPTH_BUFFER_BIT: u32 = 0x0000_0100;

type GlViewport = unsafe extern "C" fn(i32, i32, i32, i32);
type GlClearColor = unsafe extern "C" fn(f32, f32, f32, f32);
type GlClear = unsafe extern "C" fn(u32);

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// `RTLD_DEFAULT` from `<dlfcn.h>`, spelled as the integer it is.
const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

/// Resolves a GL symbol the way every macOS GL loader does.
///
/// Not `NSOpenGLContext`'s own lookup: that only covers the legacy entry points,
/// and the core-profile functions this needs (`glViewport`, `glClear`) are
/// exported by the OpenGL framework once a context exists.
fn macos_get_proc_address(symbol: &str) -> *mut c_void {
    let Ok(name) = CString::new(symbol) else {
        return std::ptr::null_mut();
    };
    // SAFETY: `dlsym` with RTLD_DEFAULT walks the loaded images; `name` is a
    // valid NUL-terminated string for the duration of the call.
    unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) }
}

/// The GL context attached to one video area.
pub struct GlSurface {
    context: Retained<NSOpenGLContext>,
    needs_render: Arc<AtomicBool>,
}

impl GlSurface {
    /// Attaches an `NSOpenGLContext` to `ns_view`.
    ///
    /// # Safety
    ///
    /// `ns_view` must be a live `NSView` that outlives the returned surface, and
    /// this must be called on the main thread.
    pub unsafe fn new(ns_view: NonNull<c_void>) -> Result<Self, String> {
        // A 3.2 core profile: mpv's OpenGL renderer needs it, and it is the
        // oldest core profile macOS offers.
        let attributes: [NSOpenGLPixelFormatAttribute; 8] = [
            NSOpenGLPFAOpenGLProfile,
            NSOpenGLProfileVersion3_2Core,
            NSOpenGLPFADoubleBuffer,
            NSOpenGLPFAColorSize,
            24,
            NSOpenGLPFADepthSize,
            24,
            0,
        ];
        let attributes_ptr = NonNull::new(attributes.as_ptr() as *mut _)
            .ok_or_else(|| "attribute array pointer was null".to_string())?;

        // SAFETY: `attributes_ptr` points at a NUL-terminated attribute list that
        // lives until the call returns.
        let format = unsafe {
            NSOpenGLPixelFormat::initWithAttributes(NSOpenGLPixelFormat::alloc(), attributes_ptr)
        }
        .ok_or_else(|| "no OpenGL 3.2 core pixel format is available".to_string())?;

        // SAFETY: `format` is a valid pixel format for the duration of the call.
        let context = unsafe {
            NSOpenGLContext::initWithFormat_shareContext(NSOpenGLContext::alloc(), &format, None)
        }
        .ok_or_else(|| "could not create an NSOpenGLContext".to_string())?;

        // The view comes from winit as a raw pointer. Borrowing it rather than
        // retaining it: winit owns it, and the host window outlives this surface.
        let view: &objc2_app_kit::NSView = unsafe { &*(ns_view.as_ptr() as *const _) };
        let mtm = objc2_foundation::MainThreadMarker::new().ok_or_else(|| {
            "an OpenGL context can only be attached on the main thread".to_string()
        })?;
        context.setView(Some(view), mtm);
        context.makeCurrentContext();
        context.update(mtm);

        Ok(Self { context, needs_render: Arc::new(AtomicBool::new(false)) })
    }

    /// Flag mpv sets when it wants a redraw, and the main thread clears.
    pub fn needs_render(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.needs_render)
    }

    /// The GL entry-point resolver mpv needs at render-context creation.
    pub fn get_proc_address(&self) -> GetProcAddress {
        Arc::new(macos_get_proc_address)
    }

    /// Makes this surface current on the calling thread.
    pub fn make_current(&self) {
        self.context.makeCurrentContext();
    }

    /// Tells the context its drawable changed, after a resize.
    pub fn update_drawable(&self) {
        // A marker is only obtainable on the main thread; on any other thread
        // `update` is not required for the drawable to be re-read.
        if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
            self.context.update(mtm);
        }
    }

    /// Clears and presents one frame.
    ///
    /// # Safety
    ///
    /// Must be called with this surface current on the calling thread.
    pub unsafe fn render(
        &self,
        mpv: &MpvGlRenderContext,
        width: i32,
        height: i32,
    ) -> Result<(), MpvError> {
        let (viewport, clear_color, clear) = unsafe {
            (
                resolve::<GlViewport>("glViewport"),
                resolve::<GlClearColor>("glClearColor"),
                resolve::<GlClear>("glClear"),
            )
        };

        // SAFETY: all three are core GL entry points, resolved through the
        // context this surface just made current.
        unsafe {
            if let Some(viewport) = viewport {
                viewport(0, 0, width.max(1), height.max(1));
            }
            if let Some(clear_color) = clear_color {
                // Opaque black, matching the video area's background, so a
                // resize never flashes stale pixels.
                clear_color(0.0, 0.0, 0.0, 1.0);
            }
            if let Some(clear) = clear {
                clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
            }
        }

        // Framebuffer 0 is the context's own drawable, which mpv's render API
        // documents as supported and which avoids owning an FBO ourselves.
        unsafe { mpv.render(0, width, height, true)? };

        self.context.flushBuffer();
        mpv.report_swap();
        self.needs_render.store(false, Ordering::Release);
        Ok(())
    }
}

/// Resolves a GL entry point, or `None` when the driver does not export it.
///
/// # Safety
///
/// `T` must be the correct signature for `symbol`.
unsafe fn resolve<T: Copy>(symbol: &str) -> Option<T> {
    let pointer = macos_get_proc_address(symbol);
    if pointer.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees the signature matches the symbol.
    Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&pointer) })
}

/// The `CStr`-free variant of `dlsym` used when checking a symbol is present.
pub fn gl_symbol_available(symbol: &str) -> bool {
    !macos_get_proc_address(symbol).is_null()
}

/// Reads a NUL-terminated symbol name, for error messages.
pub fn symbol_name(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::new();
    }
    // SAFETY: mpv hands a NUL-terminated C string.
    unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gl_symbol_lookup_returns_null_for_a_name_that_cannot_exist() {
        // Runs off the main thread with no GL context, which is fine: the point
        // is that a bogus symbol does not resolve, rather than that it does.
        assert!(!gl_symbol_available("glThisSymbolDoesNotExist12345"));
    }
}
