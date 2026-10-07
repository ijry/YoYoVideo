//! A thin, safe-ish wrapper over mpv's render API.
//!
//! mpv's `--wid` embedding is documented for X11, Win32 and Android only, so on
//! macOS there is no window handle to hand it. The render API is the supported
//! alternative: the application owns the GL context and draws mpv's frames into
//! it.
//!
//! Deliberately platform-neutral. Everything macOS-specific -- the NSOpenGLContext,
//! getting a proc address out of it, hopping to the main thread -- lives in the
//! desktop crate. What is here is the part that can be compiled and unit-tested
//! anywhere.

use std::ffi::{CStr, c_void};
use std::ptr;
use std::sync::Arc;

use libmpv_sys::{
    MPV_RENDER_API_TYPE_OPENGL, mpv_handle, mpv_opengl_fbo, mpv_opengl_init_params,
    mpv_render_context, mpv_render_context_create, mpv_render_context_free,
    mpv_render_context_render, mpv_render_context_report_swap,
    mpv_render_context_set_update_callback, mpv_render_context_update, mpv_render_param,
    mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE, mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y,
    mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
    mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
    mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
    mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME,
};

use crate::MpvError;

/// Resolves an OpenGL entry point for libmpv.
///
/// mpv resolves every GL function through this, and does so during
/// `mpv_render_context_create`. The owned form is retained for native hosts;
/// `new_with_loader` also accepts a loader borrowed just for initialization.
pub type GetProcAddress = Arc<dyn Fn(&str) -> *mut c_void + Send + Sync>;

/// What `mpv_render_context_update` is asking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFlags(pub u64);

impl UpdateFlags {
    /// A new frame is queued and `render` should be called.
    pub fn frame(self) -> bool {
        self.0 & u64::from(mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

struct UpdateHolder {
    callback: Box<dyn Fn() + Send + 'static>,
}

unsafe extern "C" fn update_trampoline(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` is the `&UpdateHolder` we registered below, and the holder
    // outlives the context because `MpvGlRenderContext` owns both and clears
    // the callback in `free` before dropping it.
    let holder = unsafe { &*(ctx.cast::<UpdateHolder>()) };
    (holder.callback)();
}

struct ProcAddressHolder<'a>(&'a dyn Fn(&CStr) -> *const c_void);

unsafe extern "C" fn get_proc_trampoline(
    ctx: *mut c_void,
    name: *const std::ffi::c_char,
) -> *mut c_void {
    if ctx.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    // `ctx` points to the holder, not to the data behind a trait object.
    let resolve = unsafe { &*ctx.cast::<ProcAddressHolder<'_>>() };
    (resolve.0)(unsafe { CStr::from_ptr(name) }).cast_mut()
}

/// An mpv render context that draws into a GL framebuffer we own.
///
/// Not `Sync`: mpv documents the render context as a render-thread object, and
/// the GL context behind it belongs to one thread.
pub struct MpvGlRenderContext {
    context: *mut mpv_render_context,
    holder: Option<Box<UpdateHolder>>,
}

impl MpvGlRenderContext {
    /// Creates a render context against an initialised mpv handle.
    ///
    /// # Safety
    ///
    /// `handle` must be an initialised mpv handle that outlives the returned
    /// context. The GL context the proc-address closure resolves against must be
    /// current during creation, rendering, and destruction of this object.
    pub unsafe fn new(handle: *mut mpv_handle, get_proc: GetProcAddress) -> Result<Self, MpvError> {
        // SAFETY: identical handle/context requirements; the adapter is borrowed
        // only during initialization, while `get_proc` is still alive.
        unsafe {
            Self::new_with_loader(handle, &|name| {
                name.to_str().map(|name| get_proc(name).cast_const()).unwrap_or(ptr::null())
            })
        }
    }

    /// Creates a context with the current renderer's borrowed GL loader.
    ///
    /// libmpv's OpenGL backend calls mpgl_load_functions2 synchronously in init;
    /// it retains the resulting function pointers, not this loader or its data.
    ///
    /// # Safety
    /// The initialized `handle` must outlive this context. The same GL context
    /// must be current during creation, render calls, and destruction.
    pub unsafe fn new_with_loader(
        handle: *mut mpv_handle,
        get_proc: &dyn Fn(&CStr) -> *const c_void,
    ) -> Result<Self, MpvError> {
        let holder = ProcAddressHolder(get_proc);
        let init = mpv_opengl_init_params {
            get_proc_address: Some(get_proc_trampoline),
            get_proc_address_ctx: ptr::from_ref(&holder).cast_mut().cast(),
            extra_exts: ptr::null(),
        };

        // Do not enable advanced control: callers issue synchronous player
        // commands on the UI/render thread. Advanced control permits decoder
        // requests that would deadlock those commands waiting on this thread.
        let mut params = [
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: MPV_RENDER_API_TYPE_OPENGL.as_ptr().cast_mut().cast::<c_void>(),
            },
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: ptr::addr_of!(init).cast_mut().cast::<c_void>(),
            },
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        let mut context: *mut mpv_render_context = ptr::null_mut();
        // SAFETY: caller guarantees `handle`; `params` is a valid terminated list.
        let result =
            unsafe { mpv_render_context_create(&mut context, handle, params.as_mut_ptr()) };
        if result < 0 {
            return Err(MpvError::Render(format!("mpv_render_context_create failed ({result})")));
        }
        if context.is_null() {
            return Err(MpvError::Render(
                "mpv_render_context_create returned a null context".to_string(),
            ));
        }

        Ok(Self { context, holder: None })
    }

    /// Registers the callback mpv invokes when the renderer wants attention.
    ///
    /// # Safety
    ///
    /// Called from mpv's internal thread. The callback must be safe to call from
    /// a foreign thread and must not block for long.
    pub unsafe fn set_update_callback(
        &mut self,
        callback: impl Fn() + Send + 'static,
    ) -> Result<(), MpvError> {
        if self.context.is_null() {
            return Err(MpvError::Render("render context is not alive".to_string()));
        }
        let mut holder = Box::new(UpdateHolder { callback: Box::new(callback) });
        let ctx = (&mut *holder) as *mut UpdateHolder as *mut c_void;
        // SAFETY: the holder is stored in `self` before returning, so the
        // pointer mpv keeps stays valid until `clear_update_callback` runs.
        unsafe {
            mpv_render_context_set_update_callback(self.context, Some(update_trampoline), ctx);
        }
        self.holder = Some(holder);
        Ok(())
    }

    /// Detaches the update callback before the holder it points at is dropped.
    pub fn clear_update_callback(&mut self) {
        if self.context.is_null() {
            return;
        }
        // SAFETY: clearing before dropping the holder is the ordering mpv
        // requires; a callback that fires after the free would dangle.
        unsafe {
            mpv_render_context_set_update_callback(self.context, None, ptr::null_mut());
        }
        self.holder = None;
    }

    /// Asks what needs drawing. Call on the render thread, never in the update callback.
    pub fn update(&self) -> UpdateFlags {
        if self.context.is_null() {
            return UpdateFlags(0);
        }
        // SAFETY: `self.context` is alive for the duration of the borrow.
        UpdateFlags(unsafe { mpv_render_context_update(self.context) })
    }

    /// Draws the current frame into `fbo`.
    ///
    /// # Safety
    ///
    /// The GL context that resolves this context's entry points must be current
    /// on the calling thread, and `fbo` must name a complete, colour-renderable
    /// framebuffer of exactly `width` x `height` pixels.
    pub unsafe fn render(
        &self,
        fbo: i32,
        width: i32,
        height: i32,
        flip_y: bool,
    ) -> Result<(), MpvError> {
        if self.context.is_null() {
            return Err(MpvError::Render("render context is not alive".to_string()));
        }
        let target = mpv_opengl_fbo { fbo, w: width, h: height, internal_format: 0 };
        let flip: i32 = i32::from(flip_y);
        let mut params = [
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
                data: ptr::addr_of!(target).cast_mut().cast::<c_void>(),
            },
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y,
                data: ptr::addr_of!(flip).cast_mut().cast::<c_void>(),
            },
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];
        // SAFETY: caller guarantees a current GL context and a valid framebuffer.
        let result = unsafe { mpv_render_context_render(self.context, params.as_mut_ptr()) };
        if result < 0 {
            return Err(MpvError::Render(format!("mpv_render_context_render failed ({result})")));
        }
        Ok(())
    }

    /// Tells mpv the frame has been presented, so it can pace the next one.
    pub fn report_swap(&self) {
        if self.context.is_null() {
            return;
        }
        // SAFETY: `self.context` is alive for the duration of the borrow.
        unsafe { mpv_render_context_report_swap(self.context) };
    }
}

impl Drop for MpvGlRenderContext {
    fn drop(&mut self) {
        // Order matters: stop mpv calling into the closure, release the closure,
        // and only then free the context.
        self.clear_update_callback();
        if !self.context.is_null() {
            // SAFETY: the context was created by `new` and is freed once.
            unsafe { mpv_render_context_free(self.context) };
            self.context = ptr::null_mut();
        }
    }
}

// SAFETY: mpv's render context may be moved between threads; the GL context it
// draws with is bound per-thread by the caller, and every method that touches GL
// documents that requirement.
unsafe impl Send for MpvGlRenderContext {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_flags_report_a_queued_frame() {
        let flags = UpdateFlags(u64::from(mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME));
        assert!(flags.frame());
        assert!(!UpdateFlags(0).frame());
        assert!(UpdateFlags(0).is_empty());
    }
}

#[cfg(test)]
mod loader_regression_tests {
    use super::*;

    #[test]
    fn render_context_rejects_missing_gl_entry_points() {
        let backend = crate::MpvBackend::new_runtime_with_options(crate::MpvClientOptions {
            audio_output: Some("null".into()),
            ..Default::default()
        })
        .expect("runtime available");
        // No GL pointer is returned: libmpv must report unsupported GL, not call
        // a miscast Rust closure or access GL state.
        let result = unsafe { backend.create_gl_render_context(Arc::new(|_| ptr::null_mut())) };
        assert!(result.is_err());
    }
}
