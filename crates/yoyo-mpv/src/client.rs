use yoyo_core::{BackendCommand, BackendEvent, MediaLocator, PlayerBackend};

use crate::{translate_command, translate_open, MpvError};

#[derive(Default)]
pub struct MpvBackend {
    pending_events: Vec<BackendEvent>,
    #[allow(dead_code)]
    last_actions: Vec<String>,
}

impl PlayerBackend for MpvBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.last_actions = translate_open(locator)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.last_actions = translate_command(&command)
            .into_iter()
            .map(|action| format!("{action:?}"))
            .collect();
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl MpvBackend {
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
