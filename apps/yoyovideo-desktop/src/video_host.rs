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
