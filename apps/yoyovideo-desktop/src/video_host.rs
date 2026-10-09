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
    fn set_privacy_blocked(&mut self, blocked: bool) -> Result<(), VideoHostError> {
        if blocked { self.hide() } else { Ok(()) }
    }
    fn set_media_access(
        &mut self,
        _access: Option<std::sync::Arc<dyn yoyo_core::PlaybackAccess>>,
        _media: Option<yoyo_core::privacy::MediaKey>,
    ) {
    }
}

/// Whether the native video surface is currently hidden to keep it from occluding a
/// Slint popup, and what the caller should do about a requested change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VideoHostSuppression {
    suppressed: bool,
    privacy_blocked: bool,
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
    pub fn is_suppressed(&self) -> bool {
        self.suppressed || self.privacy_blocked
    }

    pub fn request(&mut self, suppressed: bool) -> Option<SuppressionAction> {
        let before = self.is_suppressed();
        self.suppressed = suppressed;
        Self::transition(before, self.is_suppressed())
    }

    pub fn request_privacy(&mut self, blocked: bool) -> Option<SuppressionAction> {
        let before = self.is_suppressed();
        self.privacy_blocked = blocked;
        Self::transition(before, self.is_suppressed())
    }

    fn transition(before: bool, after: bool) -> Option<SuppressionAction> {
        (before != after).then_some(if after {
            SuppressionAction::Hide
        } else {
            SuppressionAction::Reveal
        })
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
