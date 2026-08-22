use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeVideoWindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoHostBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalVideoRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LogicalVideoRect {
    pub fn to_physical(self, scale_factor: f64) -> VideoHostBounds {
        VideoHostBounds {
            x: (f64::from(self.x) * scale_factor).round() as i32,
            y: (f64::from(self.y) * scale_factor).round() as i32,
            width: (f64::from(self.width) * scale_factor).round().max(1.0) as u32,
            height: (f64::from(self.height) * scale_factor).round().max(1.0) as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoHostError {
    message: String,
}

impl VideoHostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for VideoHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for VideoHostError {}

pub trait VideoHost {
    fn mpv_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError>;
    fn set_bounds(&mut self, bounds: VideoHostBounds) -> Result<(), VideoHostError>;
    fn show(&mut self) -> Result<(), VideoHostError>;
    fn hide(&mut self) -> Result<(), VideoHostError>;
    fn is_available(&self) -> bool;
}

/// Whether the native video surface is currently hidden to keep it from occluding a
/// Slint popup, and what the caller should do about a requested change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VideoHostSuppression {
    suppressed: bool,
}

/// What [`VideoHostSuppression::request`] wants the caller to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressionAction {
    /// Hide the surface: a popup is opening.
    Hide,
    /// Show the surface and resync its bounds: the last popup closed.
    Reveal,
}

impl VideoHostSuppression {
    /// True while the surface must stay hidden. Bounds syncing runs on a repeating
    /// timer and always shows the surface, so it has to consult this first.
    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }

    /// Applies a requested suppression state, returning the action to perform. Repeating
    /// the current state yields `None`, so switching directly between two popups does
    /// not flash the video surface back on.
    pub fn request(&mut self, suppressed: bool) -> Option<SuppressionAction> {
        if self.suppressed == suppressed {
            return None;
        }
        self.suppressed = suppressed;
        Some(if suppressed { SuppressionAction::Hide } else { SuppressionAction::Reveal })
    }
}

pub struct UnsupportedVideoHost {
    message: String,
}

impl UnsupportedVideoHost {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    fn error(&self) -> VideoHostError {
        VideoHostError::new(self.message.clone())
    }
}

impl VideoHost for UnsupportedVideoHost {
    fn mpv_window_id(&self) -> Result<NativeVideoWindowId, VideoHostError> {
        Err(self.error())
    }

    fn set_bounds(&mut self, _bounds: VideoHostBounds) -> Result<(), VideoHostError> {
        Err(self.error())
    }

    fn show(&mut self) -> Result<(), VideoHostError> {
        Err(self.error())
    }

    fn hide(&mut self) -> Result<(), VideoHostError> {
        Err(self.error())
    }

    fn is_available(&self) -> bool {
        false
    }
}
