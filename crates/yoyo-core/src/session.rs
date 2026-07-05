use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent, MediaLocator,
    MediaTrack, PlayerBackend, PlayerState, Playlist, PlaylistEntry, PlaylistSnapshot, Rotation,
    SubtitlePlaybackState,
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

    pub fn set_subtitle_preferences_restored(&mut self, restored: bool) {
        self.state.subtitle_preferences_restored = restored;
    }

    pub fn playlist_snapshot(&self) -> PlaylistSnapshot {
        self.playlist.snapshot()
    }

    fn reset_track_state_for_new_media(&mut self) {
        self.state.audio_tracks.clear();
        self.state.subtitle_tracks.clear();
        self.state.video_tracks.clear();
        self.state.subtitle = SubtitlePlaybackState::default();
        self.state.subtitle_preferences_restored = false;
    }

    pub fn replace_playlist(
        &mut self,
        entries: Vec<PlaylistEntry>,
        start_index: usize,
    ) -> Result<(), AppError> {
        self.playlist.replace(entries, start_index);
        if let Some(entry) = self.playlist.current().cloned() {
            self.reset_track_state_for_new_media();
            self.backend.open(&entry.locator).map_err(AppError::Message)?;
            self.state.current = Some(entry.locator.clone());
            self.state.paused = false;
        }
        Ok(())
    }

    pub fn open_playlist_index(&mut self, index: usize) -> Result<(), AppError> {
        let Some(entry) = self.playlist.select(index).cloned() else {
            return Ok(());
        };

        self.reset_track_state_for_new_media();
        self.backend.open(&entry.locator).map_err(AppError::Message)?;
        self.state.current = Some(entry.locator.clone());
        self.state.paused = false;
        Ok(())
    }

    fn open_single_locator(&mut self, locator: MediaLocator) -> Result<(), AppError> {
        let entry = PlaylistEntry::new(locator.clone());
        self.playlist.replace(vec![entry.clone()], 0);
        self.reset_track_state_for_new_media();
        self.backend.open(&entry.locator).map_err(AppError::Message)?;
        self.state.current = Some(locator);
        self.state.paused = false;
        Ok(())
    }

    fn next_playlist_index(&self) -> Option<usize> {
        self.playlist.current_index.and_then(|current| {
            let next = current.saturating_add(1);
            (next < self.playlist.entries.len()).then_some(next)
        })
    }

    fn previous_playlist_index(&self) -> Option<usize> {
        self.playlist.current_index.and_then(|current| current.checked_sub(1))
    }

    fn mark_selected(tracks: &mut [MediaTrack], id: i64) {
        for track in tracks {
            track.selected = track.id == id;
        }
    }

    fn selected_external_subtitle_path(tracks: &[MediaTrack]) -> Option<std::path::PathBuf> {
        tracks
            .iter()
            .find(|track| track.selected && track.external)
            .and_then(|track| track.source_path.clone())
    }

    pub fn handle_command(&mut self, command: AppCommand) -> Result<(), AppError> {
        match command {
            AppCommand::OpenFile(path) => {
                self.open_single_locator(MediaLocator::File(path))?;
            }
            AppCommand::OpenUrl(url) => {
                let locator = MediaLocator::from_url(&url)?;
                self.open_single_locator(locator)?;
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
                if let Some(index) = self.next_playlist_index() {
                    self.open_playlist_index(index)?;
                }
            }
            AppCommand::PreviousItem => {
                if let Some(index) = self.previous_playlist_index() {
                    self.open_playlist_index(index)?;
                }
            }
            AppCommand::SelectAudioTrack(id) => {
                Self::mark_selected(&mut self.state.audio_tracks, id);
                self.backend
                    .send(BackendCommand::SelectAudioTrack(id))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SelectSubtitleTrack(id) => {
                Self::mark_selected(&mut self.state.subtitle_tracks, id);
                self.state.subtitle.visible = true;
                self.state.subtitle.external_path =
                    Self::selected_external_subtitle_path(&self.state.subtitle_tracks);
                self.backend
                    .send(BackendCommand::SelectSubtitleTrack(id))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SelectVideoTrack(id) => {
                Self::mark_selected(&mut self.state.video_tracks, id);
                self.backend
                    .send(BackendCommand::SelectVideoTrack(id))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetSubtitleVisible(visible) => {
                self.state.subtitle.visible = visible;
                self.backend
                    .send(BackendCommand::SetSubtitleVisible(visible))
                    .map_err(AppError::Message)?;
            }
            AppCommand::LoadExternalSubtitle(path) => {
                self.backend
                    .send(BackendCommand::LoadExternalSubtitle(path))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetSubtitleDelay(delay) => {
                self.state.subtitle.delay_seconds = delay;
                self.backend
                    .send(BackendCommand::SetSubtitleDelay(delay))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetSubtitleScale(scale) => {
                self.state.subtitle.scale = scale;
                self.backend
                    .send(BackendCommand::SetSubtitleScale(scale))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetSubtitleVerticalPosition(position) => {
                self.state.subtitle.vertical_position_percent = position;
                self.backend
                    .send(BackendCommand::SetSubtitleVerticalPosition(position))
                    .map_err(AppError::Message)?;
            }
            AppCommand::TakeScreenshot(path) => {
                self.backend
                    .send(BackendCommand::TakeScreenshot(path.clone()))
                    .map_err(AppError::Message)?;
                self.state.last_error = None;
                self.state.status_message = Some(format!("Screenshot saved: {}", path.display()));
            }
            AppCommand::StepFrame(direction) => {
                self.backend
                    .send(BackendCommand::StepFrame(direction))
                    .map_err(AppError::Message)?;
            }
            AppCommand::SetVideoAdjustment(kind, value) => {
                let clamped =
                    value.clamp(crate::MIN_VIDEO_ADJUSTMENT, crate::MAX_VIDEO_ADJUSTMENT);
                self.backend
                    .send(BackendCommand::SetVideoAdjustment(kind, clamped))
                    .map_err(AppError::Message)?;
                self.state.video_adjustments.set_clamped(kind, clamped);
            }
            AppCommand::ResetVideoAdjustments => {
                self.backend
                    .send(BackendCommand::ResetVideoAdjustments)
                    .map_err(AppError::Message)?;
                self.state.video_adjustments = Default::default();
            }
            AppCommand::SetVideoFilterPreset(preset) => {
                self.backend
                    .send(BackendCommand::SetVideoFilterPreset(preset))
                    .map_err(AppError::Message)?;
                self.state.video_filter_preset = preset;
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
                BackendEvent::TracksChanged { audio, subtitles, video } => {
                    self.state.audio_tracks = audio;
                    self.state.subtitle_tracks = subtitles;
                    self.state.video_tracks = video;
                    self.state.subtitle.external_path =
                        Self::selected_external_subtitle_path(&self.state.subtitle_tracks);
                }
                BackendEvent::SubtitleVisibilityChanged(visible) => {
                    self.state.subtitle.visible = visible;
                }
                BackendEvent::SubtitleDelayChanged(delay) => {
                    self.state.subtitle.delay_seconds = delay;
                }
                BackendEvent::SubtitleScaleChanged(scale) => {
                    self.state.subtitle.scale = scale;
                }
                BackendEvent::SubtitleVerticalPositionChanged(position) => {
                    self.state.subtitle.vertical_position_percent = position;
                }
                BackendEvent::Warning(message) => self.state.status_message = Some(message),
                BackendEvent::Error(message) => self.state.last_error = Some(message),
                BackendEvent::EndOfFile => {
                    if let Some(index) = self.next_playlist_index() {
                        self.open_playlist_index(index)?;
                    }
                }
            }
        }
        Ok(())
    }
}
