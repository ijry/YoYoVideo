use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent, MediaLocator,
    PlayerBackend, PlayerState, Playlist, PlaylistEntry, Rotation,
};

pub struct AppSession<B: PlayerBackend> {
    config: AppConfig,
    backend: B,
    state: PlayerState,
    playlist: Playlist,
}

impl<B: PlayerBackend> AppSession<B> {
    pub fn new(config: AppConfig, backend: B) -> Self {
        let mut state = PlayerState::default();
        state.volume_percent = config.playback.default_volume_percent;
        state.speed = config.playback.default_speed;
        Self { config, backend, state, playlist: Playlist::default() }
    }

    pub fn state(&self) -> &PlayerState {
        &self.state
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn replace_playlist(
        &mut self,
        entries: Vec<PlaylistEntry>,
        start_index: usize,
    ) -> Result<(), AppError> {
        self.playlist.replace(entries, start_index);
        if let Some(entry) = self.playlist.current() {
            self.backend.open(&entry.locator).map_err(AppError::Message)?;
            self.state.current = Some(entry.locator.clone());
            self.state.paused = false;
        }
        Ok(())
    }

    pub fn handle_command(&mut self, command: AppCommand) -> Result<(), AppError> {
        match command {
            AppCommand::OpenFile(path) => {
                let locator = MediaLocator::File(path);
                self.backend.open(&locator).map_err(AppError::Message)?;
                self.state.current = Some(locator);
                self.state.paused = false;
            }
            AppCommand::OpenUrl(url) => {
                let locator = MediaLocator::from_url(&url)?;
                self.backend.open(&locator).map_err(AppError::Message)?;
                self.state.current = Some(locator);
                self.state.paused = false;
            }
            AppCommand::TogglePause => {
                self.state.paused = !self.state.paused;
                self.backend
                    .send(BackendCommand::SetPaused(self.state.paused))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SeekRelative(seconds) => {
                self.backend
                    .send(BackendCommand::SeekRelative(seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SeekAbsolute(seconds) => {
                self.backend
                    .send(BackendCommand::SeekAbsolute(seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetSpeed(speed) => {
                self.state.speed = speed;
                self.backend.send(BackendCommand::SetSpeed(speed)).map_err(AppError::Message)?;
            }
            AppCommand::ResetSpeed => {
                self.state.speed = self.config.playback.default_speed;
                self.backend
                    .send(BackendCommand::SetSpeed(self.state.speed))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetVolume(volume) => {
                self.state.volume_percent = volume;
                self.backend.send(BackendCommand::SetVolume(volume)).map_err(AppError::Message)?;
            }
            AppCommand::AdjustVolume(delta) => {
                let next = (self.state.volume_percent as i16 + delta as i16).clamp(0, 100) as u8;
                self.state.volume_percent = next;
                self.backend.send(BackendCommand::SetVolume(next)).map_err(AppError::Message)?;
            }
            AppCommand::CycleAudioChannel => {
                self.state.audio_channel = match self.state.audio_channel {
                    AudioChannelMode::Stereo => AudioChannelMode::MonoLeft,
                    AudioChannelMode::MonoLeft => AudioChannelMode::MonoRight,
                    AudioChannelMode::MonoRight => AudioChannelMode::Stereo,
                };
                self.backend
                    .send(BackendCommand::SetAudioChannel(self.state.audio_channel))
                    .map_err(AppError::Message)?;
            }
            AppCommand::RotateClockwise => {
                self.state.rotation = match self.state.rotation {
                    Rotation::Deg0 => Rotation::Deg90,
                    Rotation::Deg90 => Rotation::Deg180,
                    Rotation::Deg180 => Rotation::Deg270,
                    Rotation::Deg270 => Rotation::Deg0,
                };
                self.backend
                    .send(BackendCommand::SetRotation(self.state.rotation))
                    .map_err(AppError::Message)?;
            }
            AppCommand::ZoomIn => {
                self.state.zoom_step += 1;
                self.backend.send(BackendCommand::AdjustZoom(1)).map_err(AppError::Message)?;
            }
            AppCommand::ZoomOut => {
                self.state.zoom_step -= 1;
                self.backend.send(BackendCommand::AdjustZoom(-1)).map_err(AppError::Message)?;
            }
            AppCommand::SetABLoopPointA => {
                self.state.loop_state.point_a = Some(self.state.position_seconds);
                self.backend
                    .send(BackendCommand::SetABLoopPointA(self.state.position_seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetABLoopPointB => {
                self.state.loop_state.point_b = Some(self.state.position_seconds);
                self.backend
                    .send(BackendCommand::SetABLoopPointB(self.state.position_seconds))
                    .map_err(AppError::Message)?;
            }
            AppCommand::ClearABLoop => {
                self.state.loop_state = Default::default();
                self.backend.send(BackendCommand::ClearABLoop).map_err(AppError::Message)?;
            }
            AppCommand::ToggleFullscreen => {
                self.state.fullscreen = !self.state.fullscreen;
            }
            AppCommand::NextItem => {
                if let Some(entry) = self.playlist.next() {
                    self.backend.open(&entry.locator).map_err(AppError::Message)?;
                    self.state.current = Some(entry.locator.clone());
                }
            }
            AppCommand::PreviousItem => {
                if let Some(entry) = self.playlist.previous() {
                    self.backend.open(&entry.locator).map_err(AppError::Message)?;
                    self.state.current = Some(entry.locator.clone());
                }
            }
            AppCommand::OpenFolder(_) => {
                return Err(AppError::Message(
                    "OpenFolder must be expanded into a playlist by the desktop app".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn poll_backend(&mut self) -> Result<(), AppError> {
        for event in self.backend.drain_events() {
            match event {
                BackendEvent::PauseChanged(paused) => self.state.paused = paused,
                BackendEvent::PositionChanged(position) => self.state.position_seconds = position,
                BackendEvent::DurationChanged(duration) => self.state.duration_seconds = duration,
                BackendEvent::SpeedChanged(speed) => self.state.speed = speed,
                BackendEvent::VolumeChanged(volume) => self.state.volume_percent = volume,
                BackendEvent::AudioChannelChanged(mode) => self.state.audio_channel = mode,
                BackendEvent::RotationChanged(rotation) => self.state.rotation = rotation,
                BackendEvent::Warning(message) => self.state.status_message = Some(message),
                BackendEvent::Error(message) => self.state.last_error = Some(message),
                BackendEvent::EndOfFile => {
                    if let Some(entry) = self.playlist.next() {
                        self.backend.open(&entry.locator).map_err(AppError::Message)?;
                        self.state.current = Some(entry.locator.clone());
                    }
                }
            }
        }
        Ok(())
    }
}
