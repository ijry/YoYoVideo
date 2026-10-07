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
    /// Select `vo=libmpv` for an application-owned render context. Creating a
    /// render context alone does not change mpv's automatic video-output choice.
    pub render_api: bool,
    pub profile: Option<String>,
    /// mpv `ao` setting. `None` lets mpv choose.
    ///
    /// libmpv is a library, not the `mpv` binary: it never reads `mpv.conf`, so
    /// every option has to be passed in explicitly. That makes this the only way
    /// to run against a machine with no audio output device.
    pub audio_output: Option<String>,
}

impl MpvClientOptions {
    pub fn mpv_option_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if self.render_api {
            pairs.push(("vo", "libmpv".to_string()));
        }
        if let Some(window) = self.video_window {
            pairs.push(("wid", window.id().to_string()));
        }
        if self.force_window {
            pairs.push(("force-window", "yes".to_string()));
        }
        if let Some(profile) = &self.profile {
            pairs.push(("profile", profile.clone()));
        }
        if let Some(audio_output) = &self.audio_output {
            pairs.push(("ao", audio_output.clone()));
        }
        pairs
    }
}
