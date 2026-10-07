//! An OpenGL texture plus framebuffer that mpv renders into.
//!
//! This exists so video can be composited by the UI toolkit instead of living in a
//! native child window. Two reasons that matters:
//!
//! - **Wayland.** mpv reads `--wid` only in its X11 and Win32 backends
//!   (`video/out/x11_common.c`, `video/out/w32_common.c`); its Wayland backend
//!   never looks at `opts->WinID`. There is no window handle to hand over, and no
//!   child-window concept to use instead.
//! - **The UI already has a GL context.** Slint's renderer owns the window's
//!   context, and its rendering notifier calls back with that context current and
//!   hands over a `get_proc_address`. A texture made there can be handed straight
//!   back to Slint as a composited image.
//!
//! Platform-neutral on purpose: the only input is a GL entry-point loader, so this
//! compiles and is unit-testable anywhere, and the platform glue stays in the
//! desktop crate.

use std::ffi::{CStr, c_void};

/// Where to find a GL entry point. Mirrors what Slint's
/// `GraphicsAPI::NativeOpenGL` provides.
pub type GlProcAddress<'a> = dyn Fn(&CStr) -> *const c_void + 'a;

/// A GL problem worth reporting rather than panicking on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlError {
    message: String,
}

impl GlError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for GlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for GlError {}

/// The GL entry points this module calls, resolved once.
///
/// Every one is looked up through the caller's loader rather than linked, because
/// the loader belongs to whichever context is current -- on Linux that is EGL, on
/// macOS it is NSOpenGL, and neither is a link-time dependency of this crate.
#[derive(Clone, Copy)]
pub struct GlFunctions {
    get_integer: unsafe extern "C" fn(u32, *mut i32),
    active_texture: unsafe extern "C" fn(u32),
    gen_textures: unsafe extern "C" fn(i32, *mut u32),
    delete_textures: unsafe extern "C" fn(i32, *const u32),
    bind_texture: unsafe extern "C" fn(u32, u32),
    tex_image_2d: unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void),
    tex_parameter_i: unsafe extern "C" fn(u32, u32, i32),
    gen_framebuffers: unsafe extern "C" fn(i32, *mut u32),
    delete_framebuffers: unsafe extern "C" fn(i32, *const u32),
    bind_framebuffer: unsafe extern "C" fn(u32, u32),
    framebuffer_texture_2d: unsafe extern "C" fn(u32, u32, u32, u32, i32),
    check_framebuffer_status: unsafe extern "C" fn(u32) -> u32,
    viewport: unsafe extern "C" fn(i32, i32, i32, i32),
}

// GL constants, spelled out so this does not depend on a GL binding crate.
const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: i32 = 0x2601;
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
const GL_RGBA8: i32 = 0x8058;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;

impl GlFunctions {
    /// Resolves every entry point. Fails if a required one is missing.
    ///
    /// # Safety
    ///
    /// The loader must belong to a context that is current on the calling thread,
    /// and that context must stay current whenever these pointers are used.
    pub unsafe fn load(loader: &GlProcAddress<'_>) -> Result<Self, GlError> {
        fn name(symbol: &str) -> Result<std::ffi::CString, GlError> {
            std::ffi::CString::new(symbol)
                .map_err(|_| GlError::new(format!("GL symbol name has an interior NUL: {symbol}")))
        }

        // SAFETY: `lookup` only calls the caller-provided loader, which the caller
        // guarantees belongs to a current context, and each pointer is transmuted
        // to the signature OpenGL defines for that symbol.
        unsafe {
            let lookup = |symbol: &str| -> Result<*const c_void, GlError> {
                let name = name(symbol)?;
                let pointer = loader(&name);
                if pointer.is_null() {
                    return Err(GlError::new(format!(
                        "the OpenGL driver does not provide {symbol}"
                    )));
                }
                Ok(pointer)
            };

            macro_rules! fetch {
                ($symbol:literal, $signature:ty) => {{
                    let pointer = lookup($symbol)?;
                    std::mem::transmute_copy::<*const c_void, $signature>(&pointer)
                }};
            }

            Ok(Self {
                gen_textures: fetch!("glGenTextures", unsafe extern "C" fn(i32, *mut u32)),
                delete_textures: fetch!("glDeleteTextures", unsafe extern "C" fn(i32, *const u32)),
                bind_texture: fetch!("glBindTexture", unsafe extern "C" fn(u32, u32)),
                tex_image_2d: fetch!(
                    "glTexImage2D",
                    unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void)
                ),
                tex_parameter_i: fetch!("glTexParameteri", unsafe extern "C" fn(u32, u32, i32)),
                gen_framebuffers: fetch!("glGenFramebuffers", unsafe extern "C" fn(i32, *mut u32)),
                delete_framebuffers: fetch!(
                    "glDeleteFramebuffers",
                    unsafe extern "C" fn(i32, *const u32)
                ),
                bind_framebuffer: fetch!("glBindFramebuffer", unsafe extern "C" fn(u32, u32)),
                framebuffer_texture_2d: fetch!(
                    "glFramebufferTexture2D",
                    unsafe extern "C" fn(u32, u32, u32, u32, i32)
                ),
                check_framebuffer_status: fetch!(
                    "glCheckFramebufferStatus",
                    unsafe extern "C" fn(u32) -> u32
                ),
                viewport: fetch!("glViewport", unsafe extern "C" fn(i32, i32, i32, i32)),
                get_integer: fetch!("glGetIntegerv", unsafe extern "C" fn(u32, *mut i32)),
                active_texture: fetch!("glActiveTexture", unsafe extern "C" fn(u32)),
            })
        }
    }
}

/// The bindings we disturb when rendering into the texture. This is not a full
/// GL state snapshot: libmpv's remaining state follows its standard-defaults
/// contract, and the UI renderer sets its own drawing state after the notifier.
struct SavedBindings<'a> {
    gl: &'a GlFunctions,
    framebuffer: i32,
    texture: i32,
    texture0: i32,
    active_texture: i32,
    viewport: [i32; 4],
}

impl Drop for SavedBindings<'_> {
    fn drop(&mut self) {
        // SAFETY: only constructed within with_saved_bindings; the caller keeps
        // the same context current throughout the operation and restoration.
        unsafe {
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, self.framebuffer as u32);
            (self.gl.active_texture)(0x84C0); // GL_TEXTURE0
            (self.gl.bind_texture)(GL_TEXTURE_2D, self.texture0 as u32);
            (self.gl.active_texture)(self.active_texture as u32);
            (self.gl.bind_texture)(GL_TEXTURE_2D, self.texture as u32);
            let [x, y, w, h] = self.viewport;
            (self.gl.viewport)(x, y, w, h);
        }
    }
}

impl GlFunctions {
    /// Saves framebuffer, viewport, active texture unit and 2D texture bindings,
    /// runs `draw`, and restores them even if `draw` returns an error or unwinds.
    ///
    /// # Safety
    /// The owning context must stay current throughout `draw` and restoration.
    pub unsafe fn with_saved_bindings<T>(&self, draw: impl FnOnce() -> T) -> T {
        let mut saved = SavedBindings {
            gl: self,
            framebuffer: 0,
            texture: 0,
            texture0: 0,
            active_texture: 0,
            viewport: [0; 4],
        };
        unsafe {
            (self.get_integer)(0x8CA6, &mut saved.framebuffer); // GL_FRAMEBUFFER_BINDING
            (self.get_integer)(0x8069, &mut saved.texture); // GL_TEXTURE_BINDING_2D
            (self.get_integer)(0x84E0, &mut saved.active_texture); // GL_ACTIVE_TEXTURE
            (self.get_integer)(0x0BA2, saved.viewport.as_mut_ptr()); // GL_VIEWPORT
            (self.active_texture)(0x84C0);
            (self.get_integer)(0x8069, &mut saved.texture0);
        }
        let result = draw();
        drop(saved);
        result
    }
}

/// A texture and the framebuffer that renders into it.
///
/// Owns both GL objects. Call `destroy` while the creating context is current;
/// dropping the Rust value alone does not call OpenGL.
pub struct TextureTarget {
    texture: u32,
    framebuffer: u32,
    width: i32,
    height: i32,
}

impl TextureTarget {
    /// Allocates an RGBA8 texture of `width` x `height` and binds a framebuffer to it.
    ///
    /// # Safety
    ///
    /// The context `functions` was loaded from must be current.
    pub unsafe fn new(functions: &GlFunctions, width: i32, height: i32) -> Result<Self, GlError> {
        let width = width.max(1);
        let height = height.max(1);

        let mut texture = 0u32;
        // SAFETY: the caller guarantees a current context for `functions`.
        unsafe { (functions.gen_textures)(1, &mut texture) };
        if texture == 0 {
            return Err(GlError::new("glGenTextures returned texture 0"));
        }

        // SAFETY: as above; the texture is freshly generated and bound.
        unsafe {
            (functions.bind_texture)(GL_TEXTURE_2D, texture);
            // Linear filtering, clamped: the next frame may be a different size, and
            // wrapping would smear edge pixels into the picture.
            (functions.tex_parameter_i)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            (functions.tex_parameter_i)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            (functions.tex_parameter_i)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            (functions.tex_parameter_i)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            (functions.tex_image_2d)(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }

        let mut framebuffer = 0u32;
        // SAFETY: as above.
        unsafe { (functions.gen_framebuffers)(1, &mut framebuffer) };
        if framebuffer == 0 {
            // SAFETY: the texture was created above and is not used elsewhere.
            unsafe { (functions.delete_textures)(1, &texture) };
            return Err(GlError::new("glGenFramebuffers returned framebuffer 0"));
        }

        // SAFETY: as above.
        unsafe {
            (functions.bind_framebuffer)(GL_FRAMEBUFFER, framebuffer);
            (functions.framebuffer_texture_2d)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                texture,
                0,
            );
        }

        let status = unsafe { (functions.check_framebuffer_status)(GL_FRAMEBUFFER) };
        if status != GL_FRAMEBUFFER_COMPLETE {
            // SAFETY: both objects were created above and are not used elsewhere.
            unsafe {
                (functions.bind_framebuffer)(GL_FRAMEBUFFER, 0);
                (functions.delete_framebuffers)(1, &framebuffer);
                (functions.delete_textures)(1, &texture);
            }
            return Err(GlError::new(format!(
                "framebuffer is incomplete (glCheckFramebufferStatus = 0x{status:04X})"
            )));
        }
        unsafe { (functions.bind_framebuffer)(GL_FRAMEBUFFER, 0) };

        Ok(Self { texture, framebuffer, width, height })
    }

    /// The texture Slint should composite.
    pub fn texture(&self) -> u32 {
        self.texture
    }

    /// The framebuffer mpv should render into.
    pub fn framebuffer(&self) -> i32 {
        self.framebuffer as i32
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Reallocates the backing store at a new size, keeping the same objects.
    ///
    /// Reusing the handles matters: Slint may still hold an `Image` borrowed from
    /// the texture, and handing it a new id every resize would mean rebuilding that
    /// image on every frame of a window drag.
    ///
    /// # Safety
    ///
    /// The context `functions` was loaded from must be current.
    pub unsafe fn resize(
        &mut self,
        functions: &GlFunctions,
        width: i32,
        height: i32,
    ) -> Result<(), GlError> {
        let width = width.max(1);
        let height = height.max(1);
        if width == self.width && height == self.height {
            return Ok(());
        }

        // SAFETY: the caller guarantees a current context.
        unsafe {
            (functions.bind_texture)(GL_TEXTURE_2D, self.texture);
            (functions.tex_image_2d)(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }
        self.width = width;
        self.height = height;
        Ok(())
    }

    /// Binds this framebuffer and sets the viewport to match.
    ///
    /// # Safety
    ///
    /// The context must be current.
    pub unsafe fn bind(&self, functions: &GlFunctions) {
        // SAFETY: the caller guarantees a current context.
        unsafe {
            (functions.bind_framebuffer)(GL_FRAMEBUFFER, self.framebuffer);
            (functions.viewport)(0, 0, self.width, self.height);
        }
    }
}

impl TextureTarget {
    /// Deletes the GL objects.
    ///
    /// Explicit rather than a `Drop` impl on purpose: GL object names are only
    /// meaningful to the context that made them, and `Drop` cannot check whether
    /// that context is current. Deleting a name that belongs to some other context
    /// corrupts that context's state, which is far worse than leaking two objects
    /// during teardown. The caller knows when the right moment is -- Slint's
    /// notifier reports `RenderingState::RenderingTeardown` -- so it calls this
    /// there.
    ///
    /// # Safety
    ///
    /// The context `functions` was loaded from must be current.
    pub unsafe fn destroy(&mut self, functions: &GlFunctions) {
        if self.framebuffer != 0 {
            // SAFETY: the caller guarantees a current context.
            unsafe {
                (functions.bind_framebuffer)(GL_FRAMEBUFFER, 0);
                (functions.delete_framebuffers)(1, &self.framebuffer);
            }
            self.framebuffer = 0;
        }
        if self.texture != 0 {
            // SAFETY: as above.
            unsafe { (functions.delete_textures)(1, &self.texture) };
            self.texture = 0;
        }
    }
}

/// # Safety
///
/// The GL object names are plain integers owned exclusively by this struct.
unsafe impl Send for TextureTarget {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_entry_point_is_reported_rather_than_dereferenced() {
        // Every symbol resolves to null, which is what a context without the
        // function looks like.
        let loader = |_: &CStr| std::ptr::null();
        // SAFETY: `load` only calls the loader, and this loader touches no GL state.
        let result = unsafe { GlFunctions::load(&loader) };
        let error = result.err().expect("a null loader must not produce functions");
        assert!(
            error.to_string().contains("does not provide glGenTextures"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn gl_error_carries_its_message() {
        let error = GlError::new("framebuffer is incomplete");
        assert_eq!(error.to_string(), "framebuffer is incomplete");
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use std::cell::RefCell;

    // GL is an external boundary. Model only state owned by the caller, so this
    // test verifies our restoration contract rather than a real driver's code.
    thread_local! {
        static STATE: RefCell<[i32; 7]> = const { RefCell::new([41, 23, 0, 0, 800, 600, 0x84C0]) };
    }
    unsafe extern "C" fn get_integer(name: u32, out: *mut i32) {
        STATE.with(|state| {
            let state = state.borrow();
            let values: &[i32] = match name {
                0x8CA6 => &state[0..1],
                0x8069 => &state[1..2],
                0x0BA2 => &state[2..6],
                0x84E0 => &state[6..7],
                _ => panic!("unexpected GL query"),
            };
            unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), out, values.len()) };
        });
    }
    unsafe extern "C" fn bind_framebuffer(_: u32, id: u32) {
        STATE.with(|s| s.borrow_mut()[0] = id as i32);
    }
    unsafe extern "C" fn bind_texture(_: u32, id: u32) {
        STATE.with(|s| s.borrow_mut()[1] = id as i32);
    }
    unsafe extern "C" fn viewport(x: i32, y: i32, w: i32, h: i32) {
        STATE.with(|s| s.borrow_mut()[2..6].copy_from_slice(&[x, y, w, h]));
    }
    unsafe extern "C" fn active_texture(unit: u32) {
        STATE.with(|s| s.borrow_mut()[6] = unit as i32);
    }
    unsafe extern "C" fn unused() {}

    #[test]
    fn caller_gl_bindings_are_restored_even_when_drawing_fails() {
        let loader = |name: &CStr| match name.to_bytes() {
            b"glGetIntegerv" => get_integer as *const c_void,
            b"glBindFramebuffer" => bind_framebuffer as *const c_void,
            b"glBindTexture" => bind_texture as *const c_void,
            b"glViewport" => viewport as *const c_void,
            b"glActiveTexture" => active_texture as *const c_void,
            _ => unused as *const c_void, // Never invoked by this test.
        };
        let gl = unsafe { GlFunctions::load(&loader) }.unwrap();
        let result: Result<(), &str> = unsafe {
            gl.with_saved_bindings(|| {
                bind_framebuffer(GL_FRAMEBUFFER, 9);
                bind_texture(GL_TEXTURE_2D, 10);
                viewport(0, 0, 123, 456);
                Err("draw failed")
            })
        };
        assert_eq!(result, Err("draw failed"));
        STATE.with(|state| assert_eq!(*state.borrow(), [41, 23, 0, 0, 800, 600, 0x84C0]));
    }
}
