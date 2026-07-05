#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpvVideoWindow {
    id: u64,
}

impl MpvVideoWindow {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MpvClientOptions {
    pub video_window: Option<MpvVideoWindow>,
    pub force_window: bool,
    pub profile: Option<String>,
}

impl MpvClientOptions {
    pub fn mpv_option_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(window) = self.video_window {
            pairs.push(("wid", window.id().to_string()));
        }
        if self.force_window {
            pairs.push(("force-window", "yes".to_string()));
        }
        if let Some(profile) = &self.profile {
            pairs.push(("profile", profile.clone()));
        }
        pairs
    }
}
