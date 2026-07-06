use crate::{
    AppCommand, AppConfig, AppError, AudioChannelMode, BackendCommand, BackendEvent,
    MARKER_DEDUPE_TOLERANCE_SECONDS, MediaLocator, MediaMarker, MediaTrack, PlaybackEndBehavior,
    PlayerBackend, PlayerState, Playlist, PlaylistEntry, PlaylistSnapshot, Rotation,
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

    fn reset_navigation_state_for_new_media(&mut self) {
        self.state.chapters.clear();
        self.state.markers.clear();
    }

    fn clamp_seek_target(&self, seconds: f64) -> Result<f64, AppError> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(AppError::Message("Invalid seek target".into()));
        }

        Ok(match self.state.duration_seconds {
            Some(duration) if duration.is_finite() && duration >= 0.0 => seconds.min(duration),
            _ => seconds,
        })
    }

    fn marker_id_for_position(seconds: f64) -> String {
        format!("marker-{}", (seconds * 1000.0).round() as u64)
    }

    fn marker_title_for_position(seconds: f64) -> String {
        let total = seconds.max(0.0) as u64;
        format!("Marker {:02}:{:02}", total / 60, total % 60)
    }

    fn add_marker_at_current_position(&mut self, created_at: String) {
        let position = self.state.position_seconds.max(0.0);
        if !position.is_finite() {
            self.state.status_message = Some("Cannot add marker at invalid position".into());
            return;
        }

        if self
            .state
            .markers
            .iter()
            .any(|marker| (marker.time_seconds - position).abs() <= MARKER_DEDUPE_TOLERANCE_SECONDS)
        {
            self.state.status_message = Some("Marker already exists near this position".into());
            return;
        }

        self.state.markers.push(MediaMarker {
            id: Self::marker_id_for_position(position),
            title: Self::marker_title_for_position(position),
            time_seconds: position,
            created_at,
        });
        self.state.markers.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
        self.state.status_message = Some("Marker added".into());
    }

    fn chapter_marker_points(&self) -> Vec<f64> {
        let mut points = self
            .state
            .chapters
            .iter()
            .map(|chapter| chapter.time_seconds)
            .chain(self.state.markers.iter().map(|marker| marker.time_seconds))
            .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
            .collect::<Vec<_>>();
        points.sort_by(|left, right| left.total_cmp(right));
        points.dedup_by(|left, right| (*left - *right).abs() <= 0.001);
        points
    }

    pub fn set_markers(&mut self, markers: Vec<MediaMarker>) {
        self.state.markers = markers
            .into_iter()
            .filter(|marker| marker.time_seconds.is_finite() && marker.time_seconds >= 0.0)
            .collect();
        self.state.markers.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
    }

    pub fn replace_playlist(
        &mut self,
        entries: Vec<PlaylistEntry>,
        start_index: usize,
    ) -> Result<(), AppError> {
        self.playlist.replace(entries, start_index);
        if let Some(entry) = self.playlist.current().cloned() {
            self.reset_track_state_for_new_media();
            self.reset_navigation_state_for_new_media();
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
        self.reset_navigation_state_for_new_media();
        self.backend.open(&entry.locator).map_err(AppError::Message)?;
        self.state.current = Some(entry.locator.clone());
        self.state.paused = false;
        Ok(())
    }

    fn open_single_locator(&mut self, locator: MediaLocator) -> Result<(), AppError> {
        let entry = PlaylistEntry::new(locator.clone());
        self.playlist.replace(vec![entry.clone()], 0);
        self.reset_track_state_for_new_media();
        self.reset_navigation_state_for_new_media();
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

    fn current_playlist_index(&self) -> Option<usize> {
        self.playlist.current_index
    }

    fn first_playlist_index(&self) -> Option<usize> {
        (!self.playlist.entries.is_empty()).then_some(0)
    }

    pub fn set_config(&mut self, config: AppConfig) {
        self.config = config;
    }

    fn handle_end_of_file(&mut self) -> Result<(), AppError> {
        match self.config.playback.end_behavior {
            PlaybackEndBehavior::PlayNext => {
                if let Some(index) = self.next_playlist_index() {
                    self.open_playlist_index(index)?;
                }
            }
            PlaybackEndBehavior::Stop => {
                self.state.paused = true;
                self.state.status_message = Some("Playback ended".to_string());
            }
            PlaybackEndBehavior::LoopCurrent => {
                if let Some(index) = self.current_playlist_index() {
                    self.open_playlist_index(index)?;
                }
            }
            PlaybackEndBehavior::LoopPlaylist => {
                if let Some(index) =
                    self.next_playlist_index().or_else(|| self.first_playlist_index())
                {
                    self.open_playlist_index(index)?;
                }
            }
        }
        Ok(())
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
            AppCommand::SetMuted(muted) => {
                self.state.muted = muted;
                self.backend.send(BackendCommand::SetMuted(muted)).map_err(AppError::Message)?;
            }
            AppCommand::ToggleMute => {
                let muted = !self.state.muted;
                self.state.muted = muted;
                self.backend.send(BackendCommand::SetMuted(muted)).map_err(AppError::Message)?;
            }
            AppCommand::JumpToTime(seconds) => {
                let target = self.clamp_seek_target(seconds)?;
                self.backend
                    .send(BackendCommand::SeekAbsolute(target))
                    .map_err(AppError::Message)?;
                self.state.status_message = Some(format!("Jumped to {:.1}s", target));
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
                let clamped = value.clamp(crate::MIN_VIDEO_ADJUSTMENT, crate::MAX_VIDEO_ADJUSTMENT);
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
            AppCommand::AddMarkerAtCurrentPosition { created_at } => {
                self.add_marker_at_current_position(created_at);
            }
            AppCommand::RemoveMarker(id) => {
                self.state.markers.retain(|marker| marker.id != id);
                self.state.status_message = Some("Marker removed".into());
            }
            AppCommand::SeekToChapter(index) => {
                if let Some(seconds) =
                    self.state.chapters.get(index).map(|chapter| chapter.time_seconds)
                {
                    let target = self.clamp_seek_target(seconds)?;
                    self.backend
                        .send(BackendCommand::SeekAbsolute(target))
                        .map_err(AppError::Message)?;
                }
            }
            AppCommand::SeekToMarker(id) => {
                if let Some(seconds) = self
                    .state
                    .markers
                    .iter()
                    .find(|marker| marker.id == id)
                    .map(|marker| marker.time_seconds)
                {
                    let target = self.clamp_seek_target(seconds)?;
                    self.backend
                        .send(BackendCommand::SeekAbsolute(target))
                        .map_err(AppError::Message)?;
                }
            }
            AppCommand::SeekToNextChapterOrMarker => {
                if let Some(target) = self
                    .chapter_marker_points()
                    .into_iter()
                    .find(|point| *point > self.state.position_seconds + 0.5)
                {
                    self.backend
                        .send(BackendCommand::SeekAbsolute(target))
                        .map_err(AppError::Message)?;
                }
            }
            AppCommand::SeekToPreviousChapterOrMarker => {
                if let Some(target) = self
                    .chapter_marker_points()
                    .into_iter()
                    .rev()
                    .find(|point| *point < self.state.position_seconds - 0.5)
                {
                    self.backend
                        .send(BackendCommand::SeekAbsolute(target))
                        .map_err(AppError::Message)?;
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
                BackendEvent::MutedChanged(muted) => self.state.muted = muted,
                BackendEvent::AudioChannelChanged(mode) => self.state.audio_channel = mode,
                BackendEvent::RotationChanged(rotation) => self.state.rotation = rotation,
                BackendEvent::TracksChanged { audio, subtitles, video } => {
                    self.state.audio_tracks = audio;
                    self.state.subtitle_tracks = subtitles;
                    self.state.video_tracks = video;
                    self.state.subtitle.external_path =
                        Self::selected_external_subtitle_path(&self.state.subtitle_tracks);
                }
                BackendEvent::ChaptersChanged(chapters) => {
                    self.state.chapters = chapters
                        .into_iter()
                        .filter(|chapter| {
                            chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0
                        })
                        .collect();
                    self.state
                        .chapters
                        .sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
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
                BackendEvent::EndOfFile => self.handle_end_of_file()?,
            }
        }
        Ok(())
    }
}
