#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoTexture {
    pub texture_id: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for VideoTexture {
    fn default() -> Self {
        Self {
            texture_id: 0,
            width: 1280,
            height: 720,
        }
    }
}
