use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{render::MpvRenderBridge, translate_command, translate_open, MpvError};

pub struct MpvBackend {
    pending_events: Vec<BackendEvent>,
    render_bridge: MpvRenderBridge,
    #[allow(dead_code)]
    last_actions: Vec<String>,
}

impl Default for MpvBackend {
    fn default() -> Self {
        Self {
            pending_events: Vec::new(),
            render_bridge: MpvRenderBridge::default(),
            last_actions: Vec::new(),
        }
    }
}

impl PlayerBackend for MpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.last_actions = translate_open(locator)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        self.render_bridge.mark_dirty();
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.last_actions = translate_command(&command)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        self.render_bridge.mark_dirty();
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl MpvBackend {
    pub fn render_bridge(&mut self) -> &mut MpvRenderBridge {
        &mut self.render_bridge
    }

    pub fn ensure_runtime_feature() -> Result<(), MpvError> {
        #[cfg(feature = "mpv-runtime")]
        {
            Ok(())
        }
        #[cfg(not(feature = "mpv-runtime"))]
        {
            Err(MpvError::RuntimeDisabled)
        }
    }
}
