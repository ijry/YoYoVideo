#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    pub framebuffer_object: u32,
    pub width: u32,
    pub height: u32,
    pub flipped: bool,
}

#[derive(Default)]
pub struct MpvRenderBridge {
    redraw_requested: bool,
}

impl MpvRenderBridge {
    pub fn needs_redraw(&self) -> bool {
        self.redraw_requested
    }

    pub fn mark_dirty(&mut self) {
        self.redraw_requested = true;
    }

    pub fn render(&mut self, _target: RenderTarget) -> Result<(), crate::MpvError> {
        self.redraw_requested = false;
        Ok(())
    }
}
