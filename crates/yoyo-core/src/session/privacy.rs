use super::AppSession;
use crate::{
    AppCommand, AppError, BackendCommand, MediaLocator, PlaybackAccess, PlayerBackend,
    privacy::MediaKey,
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct SessionPrivacy {
    forced_mute: bool,
    managed_mute: bool,
    unloaded: bool,
    stop_pending: bool,
    resume_required: bool,
    resume_at: Option<f64>,
}

impl<B: PlayerBackend> AppSession<B> {
    pub fn set_playback_access(&mut self, access: Arc<dyn PlaybackAccess>) {
        if self.media_key.is_none() {
            self.media_key = self
                .state
                .current
                .as_ref()
                .and_then(|locator| MediaKey::from_locator(locator).ok());
        }
        self.access = Some(access);
    }

    pub fn playback_access(&self) -> Option<Arc<dyn PlaybackAccess>> {
        self.access.clone()
    }

    pub fn current_media_key(&self) -> Option<&MediaKey> {
        self.media_key.as_ref()
    }

    pub fn privacy_blocked(&self) -> bool {
        match (&self.access, &self.media_key) {
            (Some(access), Some(key)) => access.restricted(key),
            (Some(_), None) => self.state.current.is_some(),
            _ => false,
        }
    }

    /// Call after concealing native/composited surfaces. This gate is also used
    /// before commands and when draining events, independently of any UI timer.
    pub fn enforce_privacy(&mut self) -> Result<(), AppError> {
        if self.privacy_blocked() {
            if !self.privacy.forced_mute || self.privacy.stop_pending {
                self.force_protection()?;
            }
        } else if self.privacy.forced_mute {
            self.backend
                .send(BackendCommand::SetMuted(self.state.muted))
                .map_err(|_| AppError::Message("Could not restore audio preference".into()))?;
            self.privacy.forced_mute = false;
            if self.state.status_message.as_deref() == Some("Protected content") {
                self.state.status_message = None;
            }
            if self.state.last_error.as_deref() == Some("Protected content") {
                self.state.last_error = None;
            }
            // No SetPaused(false): unlocking is not a playback action.
            self.state.paused = true;
        }
        Ok(())
    }

    pub(super) fn force_protection(&mut self) -> Result<(), AppError> {
        self.privacy.forced_mute = true;
        self.privacy.managed_mute = true;
        self.privacy.resume_required = true;
        self.state.paused = true;
        self.state.status_message = Some("Protected content".into());
        let mute = self.backend.send(BackendCommand::SetMuted(true));
        let pause = self.backend.send(BackendCommand::SetPaused(true));
        if mute.is_err() || pause.is_err() || self.privacy.stop_pending {
            self.privacy.unloaded = true;
            self.stopped = true; // Discard late progress events, retaining the saved locator/position.
            self.privacy.stop_pending = self.backend.send(BackendCommand::Stop).is_err();
            return Err(AppError::Message(
                "Protected playback could not pause; stopping it instead".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn key_for_open(
        &self,
        locator: &MediaLocator,
    ) -> Result<Option<MediaKey>, AppError> {
        let key = MediaKey::from_locator(locator).ok();
        if let Some(access) = &self.access {
            if key.as_ref().is_none_or(|key| access.restricted(key)) {
                return Err(AppError::Message("Protected content".into()));
            }
        }
        Ok(key)
    }

    pub(super) fn open_checked(
        &mut self,
        locator: &MediaLocator,
    ) -> Result<Option<MediaKey>, AppError> {
        let key = self.key_for_open(locator)?;
        if self.privacy.resume_required && !self.privacy.unloaded {
            // A permitted replacement must not reveal the old protected frame
            // while the next file is loading in the same native/render surface.
            self.backend
                .send(BackendCommand::Stop)
                .map_err(|_| AppError::Message("Could not clear protected video output".into()))?;
            self.privacy.unloaded = true;
            self.stopped = true;
        }
        if let Err(error) = self.backend.open(locator) {
            if self
                .access
                .as_ref()
                .is_some_and(|access| key.as_ref().is_none_or(|key| access.restricted(key)))
            {
                return Err(AppError::Message("Protected content".into()));
            }
            return Err(AppError::Message(error));
        }
        Ok(key)
    }

    pub(super) fn commit_open(
        &mut self,
        locator: MediaLocator,
        key: Option<MediaKey>,
    ) -> Result<(), AppError> {
        let resume_after_privacy = self.privacy.resume_required;
        self.reset_track_state_for_new_media();
        self.reset_navigation_state_for_new_media();
        self.state.current = Some(locator);
        self.state.status_message = None;
        self.state.last_error = None;
        self.media_key = key;
        self.stopped = false;
        self.privacy.unloaded = false;
        self.privacy.stop_pending = false;
        self.privacy.resume_at = None;
        self.privacy.resume_required = false;
        if self.privacy.forced_mute && !self.privacy_blocked() {
            self.backend
                .send(BackendCommand::SetMuted(self.state.muted))
                .map_err(AppError::Message)?;
            self.privacy.forced_mute = false;
        }
        self.state.paused = false;
        // A new load needs its own pause even when the old load was concealed.
        if self.privacy_blocked() {
            self.force_protection()
        } else {
            self.enforce_privacy()?;
            // Loading a new allowed file is an explicit play request. mpv keeps
            // the previous file's pause property unless we reset it deliberately.
            if resume_after_privacy {
                self.set_paused(false)?;
            }
            Ok(())
        }
    }

    pub(super) fn reset_privacy_after_stop(&mut self) -> Result<(), AppError> {
        self.media_key = None;
        if self.privacy.forced_mute {
            self.backend
                .send(BackendCommand::SetMuted(self.state.muted))
                .map_err(AppError::Message)?;
        }
        let managed_mute = self.privacy.managed_mute;
        self.privacy = SessionPrivacy { managed_mute, ..SessionPrivacy::default() };
        Ok(())
    }

    pub(super) fn guard_app_command(&mut self, command: &AppCommand) -> Result<(), AppError> {
        if !matches!(command, AppCommand::Stop) {
            self.enforce_privacy()?;
        }
        if self.privacy_blocked()
            && !matches!(
                command,
                AppCommand::Stop
                    | AppCommand::SetPaused(true)
                    | AppCommand::OpenFile(_)
                    | AppCommand::OpenFolder(_)
                    | AppCommand::OpenUrl(_)
                    | AppCommand::NextItem
                    | AppCommand::PreviousItem
                    | AppCommand::SetMuted(_)
                    | AppCommand::ToggleMute
                    | AppCommand::SetVolume(_)
                    | AppCommand::AdjustVolume(_)
                    | AppCommand::ToggleFullscreen
            )
        {
            return Err(AppError::Message("Protected content".into()));
        }
        Ok(())
    }

    pub(super) fn send_backend(&mut self, command: BackendCommand) -> Result<(), String> {
        if command != BackendCommand::Stop {
            self.enforce_privacy().map_err(|error| error.to_string())?;
        }
        if self.privacy_blocked()
            && !matches!(
                command,
                BackendCommand::Stop
                    | BackendCommand::SetPaused(true)
                    | BackendCommand::SetMuted(true)
                    | BackendCommand::SetVolume(_)
            )
        {
            return Err("Protected content".into());
        }
        self.backend.send(command)
    }

    pub(super) fn set_paused(&mut self, paused: bool) -> Result<(), AppError> {
        if !paused && self.privacy.unloaded {
            if let Some(locator) = self.state.current.clone() {
                self.open_checked(&locator)?;
                self.privacy.resume_at =
                    (self.state.position_seconds > 0.0).then_some(self.state.position_seconds);
                self.privacy.unloaded = false;
                self.stopped = false;
            }
        }
        self.send_backend(BackendCommand::SetPaused(paused)).map_err(AppError::Message)?;
        self.state.paused = paused;
        if !paused {
            self.privacy.resume_required = false;
        }
        Ok(())
    }

    pub(super) fn effective_mute(&self) -> bool {
        self.state.muted || self.privacy.forced_mute || self.privacy_blocked()
    }

    pub(super) fn on_pause_event(&mut self, paused: bool) -> Result<(), AppError> {
        if !paused && self.privacy_blocked() {
            self.force_protection()?;
        } else if !paused && self.privacy.resume_required {
            self.backend
                .send(BackendCommand::SetPaused(true))
                .map_err(|_| AppError::Message("Could not keep playback paused".into()))?;
            self.state.paused = true;
        } else {
            self.state.paused = paused;
        }
        Ok(())
    }

    pub(super) fn on_mute_event(&mut self, muted: bool) -> Result<(), AppError> {
        if !muted && self.privacy_blocked() {
            self.force_protection()?;
        }
        // Observer events are the effective OR of user/privacy mute. Once privacy
        // has controlled it, only explicit user commands can change user intent.
        if !self.privacy.managed_mute {
            self.state.muted = muted;
        }
        Ok(())
    }

    pub(super) fn eof_allowed(&self) -> bool {
        !self.privacy_blocked() && !self.privacy.resume_required && !self.privacy.unloaded
    }

    pub(super) fn restore_privacy_position(&mut self) -> Result<(), AppError> {
        if let Some(position) = self.privacy.resume_at {
            self.send_backend(BackendCommand::SeekAbsolute(position)).map_err(AppError::Message)?;
            self.privacy.resume_at = None;
        }
        Ok(())
    }
}
