use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[cfg(feature = "mpv-runtime")]
use i_slint_backend_winit::{Backend as WinitBackend, CustomApplicationHandler, EventResult};
use slint::winit_030::WinitWindowAccessor;
use yoyo_core::{
    AppCommand, AppConfig, AppSession, FrameStepDirection, HistoryStore, MediaLocator,
    PlayerBackend, PlayerState, ShortcutAction, ShortcutMap, VideoAdjustmentKind,
    VideoFilterPreset,
};
use yoyo_mpv::{MpvBackend, MpvError};

use crate::NativeVideoWindowId;
#[cfg(feature = "mpv-runtime")]
use crate::VideoHost;
use crate::platform::{AppPaths, DialogService, RfdDialogService, scan_media_folder};
#[cfg(feature = "mpv-runtime")]
use crate::video_host_winit::WinitVideoHost;
use crate::video_texture::VideoTexture;

slint::include_modules!();

pub fn build_desktop_backend() -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime()
}

pub fn build_desktop_backend_with_video_window(
    window_id: NativeVideoWindowId,
) -> Result<MpvBackend, MpvError> {
    MpvBackend::new_runtime_with_options(yoyo_mpv::MpvClientOptions {
        video_window: Some(yoyo_mpv::MpvVideoWindow::new(window_id.0)),
        force_window: true,
        profile: None,
    })
}

pub fn refresh_window(window: &MainWindow, state: &PlayerState) {
    refresh_window_with_language(window, state, crate::UiLanguage::Chinese);
}

fn refresh_window_with_language(
    window: &MainWindow,
    state: &PlayerState,
    language: crate::UiLanguage,
) {
    window.set_ui_language_code(language.code().into());
    window.set_transport_label(crate::format_transport_label_for_language(state, language).into());
    window.set_speed_label(crate::format_speed_label(state).into());
    window.set_time_label(crate::format_time_label(state).into());
    window.set_volume_label(crate::format_volume_label_for_language(state, language).into());
    window.set_muted(state.muted);
    window.set_mute_label(
        match (language, state.muted) {
            (crate::UiLanguage::Chinese, true) => "静音",
            (crate::UiLanguage::Chinese, false) => "声音",
            (crate::UiLanguage::English, true) => "Muted",
            (crate::UiLanguage::English, false) => "Sound",
        }
        .into(),
    );
    window.set_rotation_label(crate::format_rotation_label_for_language(state, language).into());
    window.set_audio_channel_label(
        crate::format_audio_channel_label_for_language(state, language).into(),
    );
    window.set_zoom_label(crate::format_zoom_label_for_language(state, language).into());
    window.set_loop_label(crate::format_loop_label_for_language(state, language).into());
    window.set_progress_value(crate::progress_ratio(state));
    refresh_navigation_surfaces(window, state);
    window.set_volume_value(i32::from(state.volume_percent));
    window.set_brightness_value(i32::from(state.video_adjustments.brightness));
    window.set_contrast_value(i32::from(state.video_adjustments.contrast));
    window.set_saturation_value(i32::from(state.video_adjustments.saturation));
    window.set_gamma_value(i32::from(state.video_adjustments.gamma));
    window.set_hue_value(i32::from(state.video_adjustments.hue));
    window.set_brightness_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Brightness,
            state.video_adjustments.brightness,
            language,
        )
        .into(),
    );
    window.set_contrast_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Contrast,
            state.video_adjustments.contrast,
            language,
        )
        .into(),
    );
    window.set_saturation_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Saturation,
            state.video_adjustments.saturation,
            language,
        )
        .into(),
    );
    window.set_gamma_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Gamma,
            state.video_adjustments.gamma,
            language,
        )
        .into(),
    );
    window.set_hue_label(
        crate::format_video_adjustment_label_for_language(
            VideoAdjustmentKind::Hue,
            state.video_adjustments.hue,
            language,
        )
        .into(),
    );
    window.set_video_filter_label(
        crate::format_video_filter_preset_label_for_language(state.video_filter_preset, language)
            .into(),
    );
    window.set_status_label(
        state
            .last_error
            .clone()
            .or_else(|| state.status_message.clone())
            .unwrap_or_default()
            .into(),
    );
}

fn model_from_vec<T: Clone + 'static>(rows: Vec<T>) -> slint::ModelRc<T> {
    slint::ModelRc::new(slint::VecModel::from(rows))
}

fn current_locator_key(state: &PlayerState) -> Option<String> {
    state.current.as_ref().map(MediaLocator::as_label)
}

fn refresh_navigation_surfaces(window: &MainWindow, state: &PlayerState) {
    let rows = crate::build_navigation_rows(&state.chapters, &state.markers)
        .into_iter()
        .map(|row| NavigationRowData {
            title: row.title.into(),
            subtitle: row.subtitle.into(),
            id: row.id.into(),
            is_marker: row.is_marker,
        })
        .collect::<Vec<_>>();
    window.set_navigation_rows(model_from_vec(rows));

    let ticks =
        crate::build_progress_ticks(&state.chapters, &state.markers, state.duration_seconds)
            .into_iter()
            .map(|tick| ProgressTickRowData {
                percent: tick.percent,
                label: tick.label.into(),
                is_marker: tick.kind == crate::ProgressTickKind::Marker,
            })
            .collect::<Vec<_>>();
    window.set_progress_tick_rows(model_from_vec(ticks));
}

fn set_osd(window: &MainWindow, runtime: &mut DesktopRuntime, kind: crate::OsdKind) {
    runtime.osd.visible = true;
    runtime.osd.message = crate::format_osd_message_for_language(kind, runtime.ui_language);
    runtime.osd.generation = runtime.osd.generation.saturating_add(1);
    window.set_osd_visible(true);
    window.set_osd_message(runtime.osd.message.clone().into());
}

pub struct DesktopController<B: PlayerBackend> {
    session: AppSession<B>,
    shortcuts: ShortcutMap,
    #[allow(dead_code)]
    video_texture: VideoTexture,
}

impl<B: PlayerBackend> DesktopController<B> {
    pub fn new(session: AppSession<B>) -> Self {
        Self::with_shortcuts(session, ShortcutMap::default())
    }

    pub fn with_shortcuts(session: AppSession<B>, shortcuts: ShortcutMap) -> Self {
        Self { session, shortcuts, video_texture: VideoTexture::default() }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<(), yoyo_core::AppError> {
        self.session.handle_command(command)?;
        self.session.poll_backend()?;
        Ok(())
    }

    pub fn session(&self) -> &AppSession<B> {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut AppSession<B> {
        &mut self.session
    }

    pub fn set_config(&mut self, config: AppConfig) {
        self.session.set_config(config);
    }

    pub fn open_folder(&mut self, path: &std::path::Path) -> Result<(), yoyo_core::AppError> {
        let entries = scan_media_folder(path)?;
        self.session.replace_playlist(entries, 0)?;
        self.session.poll_backend()
    }

    pub fn open_playlist_entries(
        &mut self,
        entries: Vec<yoyo_core::PlaylistEntry>,
    ) -> Result<(), yoyo_core::AppError> {
        self.session.replace_playlist(entries, 0)?;
        self.session.poll_backend()
    }

    pub fn open_playlist_index(&mut self, index: usize) -> Result<(), yoyo_core::AppError> {
        self.session.open_playlist_index(index)?;
        self.session.poll_backend()
    }

    pub fn set_shortcuts(&mut self, shortcuts: ShortcutMap) {
        self.shortcuts = shortcuts;
    }

    pub fn dispatch_shortcut(&mut self, gesture: &str) -> Result<(), yoyo_core::AppError> {
        if let Some(ShortcutDispatch::Command(command)) = self.resolve_shortcut(gesture) {
            self.dispatch(command)?;
        }
        Ok(())
    }

    pub fn resolve_shortcut(&self, gesture: &str) -> Option<ShortcutDispatch> {
        resolve_shortcut(&self.shortcuts, gesture)
    }

    pub fn poll_backend(&mut self) -> Result<(), yoyo_core::AppError> {
        self.session.poll_backend()
    }

    pub fn set_subtitle_preferences_restored(&mut self, restored: bool) {
        self.session.set_subtitle_preferences_restored(restored);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutDispatch {
    Command(AppCommand),
    TakeScreenshot,
    OpenJumpPanel,
    OpenActionPanel,
    AddMarker,
}

pub fn resolve_shortcut(map: &ShortcutMap, gesture: &str) -> Option<ShortcutDispatch> {
    let shortcut = yoyo_core::Shortcut::parse(gesture).ok()?;
    match map.action_for(&shortcut)? {
        ShortcutAction::TogglePause => Some(ShortcutDispatch::Command(AppCommand::TogglePause)),
        ShortcutAction::SeekBackwardSmall => {
            Some(ShortcutDispatch::Command(AppCommand::SeekRelative(-5.0)))
        }
        ShortcutAction::SeekForwardSmall => {
            Some(ShortcutDispatch::Command(AppCommand::SeekRelative(5.0)))
        }
        ShortcutAction::VolumeUp => Some(ShortcutDispatch::Command(AppCommand::AdjustVolume(5))),
        ShortcutAction::VolumeDown => Some(ShortcutDispatch::Command(AppCommand::AdjustVolume(-5))),
        ShortcutAction::SpeedDown => Some(ShortcutDispatch::Command(AppCommand::SetSpeed(0.75))),
        ShortcutAction::SpeedUp => Some(ShortcutDispatch::Command(AppCommand::SetSpeed(1.25))),
        ShortcutAction::ResetSpeed => Some(ShortcutDispatch::Command(AppCommand::ResetSpeed)),
        ShortcutAction::SetABLoopPointA => {
            Some(ShortcutDispatch::Command(AppCommand::SetABLoopPointA))
        }
        ShortcutAction::SetABLoopPointB => {
            Some(ShortcutDispatch::Command(AppCommand::SetABLoopPointB))
        }
        ShortcutAction::ClearABLoop => Some(ShortcutDispatch::Command(AppCommand::ClearABLoop)),
        ShortcutAction::RotateClockwise => {
            Some(ShortcutDispatch::Command(AppCommand::RotateClockwise))
        }
        ShortcutAction::ZoomOut => Some(ShortcutDispatch::Command(AppCommand::ZoomOut)),
        ShortcutAction::ZoomIn => Some(ShortcutDispatch::Command(AppCommand::ZoomIn)),
        ShortcutAction::CycleAudioChannel => {
            Some(ShortcutDispatch::Command(AppCommand::CycleAudioChannel))
        }
        ShortcutAction::ToggleFullscreen => {
            Some(ShortcutDispatch::Command(AppCommand::ToggleFullscreen))
        }
        ShortcutAction::TakeScreenshot => Some(ShortcutDispatch::TakeScreenshot),
        ShortcutAction::FrameStepBackward => {
            Some(ShortcutDispatch::Command(AppCommand::StepFrame(FrameStepDirection::Previous)))
        }
        ShortcutAction::FrameStepForward => {
            Some(ShortcutDispatch::Command(AppCommand::StepFrame(FrameStepDirection::Next)))
        }
        ShortcutAction::ToggleMute => Some(ShortcutDispatch::Command(AppCommand::ToggleMute)),
        ShortcutAction::JumpToTime => Some(ShortcutDispatch::OpenJumpPanel),
        ShortcutAction::AddMarker => Some(ShortcutDispatch::AddMarker),
        ShortcutAction::OpenActionPanel => Some(ShortcutDispatch::OpenActionPanel),
        ShortcutAction::NextChapterOrMarker => {
            Some(ShortcutDispatch::Command(AppCommand::SeekToNextChapterOrMarker))
        }
        ShortcutAction::PreviousChapterOrMarker => {
            Some(ShortcutDispatch::Command(AppCommand::SeekToPreviousChapterOrMarker))
        }
        ShortcutAction::OpenFile | ShortcutAction::OpenUrl => None,
    }
}

pub fn dispatch_shortcut(map: &ShortcutMap, gesture: &str) -> Option<AppCommand> {
    match resolve_shortcut(map, gesture)? {
        ShortcutDispatch::Command(command) => Some(command),
        ShortcutDispatch::AddMarker => {
            Some(AppCommand::AddMarkerAtCurrentPosition { created_at: "shortcut".into() })
        }
        ShortcutDispatch::TakeScreenshot => None,
        ShortcutDispatch::OpenJumpPanel | ShortcutDispatch::OpenActionPanel => None,
    }
}

pub fn dropped_media_status(action: &crate::platform::DroppedMediaAction) -> String {
    match action {
        crate::platform::DroppedMediaAction::NoPlayableMedia { .. } => {
            "No playable media found in dropped items".to_string()
        }
        crate::platform::DroppedMediaAction::OpenFile(path) => {
            format!("Opened dropped file: {}", path.display())
        }
        crate::platform::DroppedMediaAction::ReplacePlaylist(entries) => {
            format!("Opened dropped playlist: {} items", entries.len())
        }
    }
}

pub fn recent_item_status(item: &crate::platform::RecentOpenItem) -> String {
    match item.kind {
        crate::platform::RecentOpenKind::File => {
            format!("Opening recent file: {}", item.target)
        }
        crate::platform::RecentOpenKind::Folder => {
            format!("Opening recent folder: {}", item.target)
        }
        crate::platform::RecentOpenKind::Url => format!("Opening recent URL: {}", item.target),
    }
}

struct DesktopRuntime {
    controller: Option<DesktopController<MpvBackend>>,
    video_host_error: Option<String>,
    app_handle: Option<slint::Weak<MainWindow>>,
    config: AppConfig,
    history: crate::HistoryRuntime,
    recent_open: crate::platform::RecentOpenStore,
    subtitle_prefs: crate::SubtitlePrefsRuntime,
    marker_store: crate::platform::MarkerStore,
    ui_language: crate::UiLanguage,
    osd: crate::OsdState,
    sidebar: crate::SidebarState,
    settings_window: Option<SettingsWindow>,
    settings_controller: Option<crate::SettingsController>,
    pending_resume: Option<crate::PendingResumeSeek>,
    last_seen_locator: Option<MediaLocator>,
    last_seen_subtitle_locator: Option<MediaLocator>,
    last_marker_locator_key: Option<String>,
    started_at: Instant,
    diagnostic_log_path: PathBuf,
    diagnostic_log_failed: bool,
    window_state_path: Option<PathBuf>,
    #[cfg(feature = "mpv-runtime")]
    video_host: Option<WinitVideoHost>,
}

impl DesktopRuntime {
    fn new(
        config: AppConfig,
        history: crate::HistoryRuntime,
        recent_open: crate::platform::RecentOpenStore,
        subtitle_prefs: crate::SubtitlePrefsRuntime,
        marker_store: crate::platform::MarkerStore,
        sidebar: crate::SidebarState,
        diagnostic_log_path: PathBuf,
        window_state_path: Option<PathBuf>,
    ) -> Self {
        Self {
            controller: None,
            video_host_error: initial_runtime_error(),
            app_handle: None,
            config,
            history,
            recent_open,
            subtitle_prefs,
            marker_store,
            ui_language: crate::UiLanguage::Chinese,
            osd: crate::OsdState::default(),
            sidebar,
            settings_window: None,
            settings_controller: None,
            pending_resume: None,
            last_seen_locator: None,
            last_seen_subtitle_locator: None,
            last_marker_locator_key: None,
            started_at: Instant::now(),
            diagnostic_log_path,
            diagnostic_log_failed: false,
            window_state_path,
            #[cfg(feature = "mpv-runtime")]
            video_host: None,
        }
    }

    fn controller(&self) -> Option<&DesktopController<MpvBackend>> {
        self.controller.as_ref()
    }

    fn controller_mut(&mut self) -> Option<&mut DesktopController<MpvBackend>> {
        self.controller.as_mut()
    }

    fn status_message(&self) -> String {
        self.video_host_error
            .clone()
            .unwrap_or_else(|| "Playback runtime is still initializing".to_string())
    }

    fn record_diagnostic(&mut self, level: &str, message: impl AsRef<str>) {
        if self.diagnostic_log_failed {
            return;
        }
        if crate::platform::append_diagnostic_line(
            &self.diagnostic_log_path,
            &crate::platform::diagnostic_timestamp_now(),
            level,
            message.as_ref(),
        )
        .is_err()
        {
            self.diagnostic_log_failed = true;
        }
    }

    #[cfg(feature = "mpv-runtime")]
    fn mark_error(&mut self, message: impl Into<String>) {
        self.video_host_error = Some(message.into());
    }

    #[cfg(feature = "mpv-runtime")]
    fn set_runtime(
        &mut self,
        controller: DesktopController<MpvBackend>,
        video_host: WinitVideoHost,
    ) {
        self.controller = Some(controller);
        self.video_host = Some(video_host);
        self.video_host_error = None;
    }
}

#[cfg(feature = "mpv-runtime")]
fn initial_runtime_error() -> Option<String> {
    None
}

#[cfg(not(feature = "mpv-runtime"))]
fn initial_runtime_error() -> Option<String> {
    Some("Playback runtime is disabled in this build".to_string())
}

pub fn format_runtime_startup_error(error: &str) -> String {
    if cfg!(target_os = "windows") {
        format!(
            "Playback runtime failed: {error}. Check that libmpv is staged at third_party/mpv/windows-x64/bin/mpv-2.dll for development or beside the packaged executable for release. Recovery: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force"
        )
    } else if cfg!(target_os = "macos") {
        format!(
            "Playback runtime failed: {error}. Check that libmpv.dylib is staged for macos-universal packaging or available to the app runtime."
        )
    } else {
        format!(
            "Playback runtime failed: {error}. Check that libmpv.so is staged for linux-x64 packaging or available through LD_LIBRARY_PATH."
        )
    }
}

fn config_file_path(paths: &AppPaths) -> PathBuf {
    paths.config_dir.join("config.toml")
}

fn history_file_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join("history.json")
}

fn subtitle_prefs_file_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join("subtitle_prefs.json")
}

fn load_boot_config(paths: Option<&AppPaths>) -> AppConfig {
    let Some(path) = paths.map(config_file_path) else {
        return AppConfig::default();
    };
    let Ok(config) = AppConfig::load(&path) else {
        return AppConfig::default();
    };
    if config.validate().is_ok() { config } else { AppConfig::default() }
}

fn load_history_runtime(paths: Option<&AppPaths>, config: &AppConfig) -> crate::HistoryRuntime {
    let history_path = paths.map(history_file_path);
    crate::HistoryRuntime::load(history_path, config.ui.remember_history)
        .unwrap_or_else(|_| crate::HistoryRuntime::new(None, HistoryStore::default(), false))
}

fn load_recent_open_store(paths: Option<&AppPaths>) -> crate::platform::RecentOpenStore {
    crate::platform::RecentOpenStore::load(crate::platform::recent_open_path(paths))
        .unwrap_or_default()
}

fn load_subtitle_prefs_runtime(paths: Option<&AppPaths>) -> crate::SubtitlePrefsRuntime {
    let prefs_path = paths.map(subtitle_prefs_file_path);
    crate::SubtitlePrefsRuntime::load(prefs_path.clone())
        .unwrap_or_else(|_| crate::SubtitlePrefsRuntime::new(prefs_path, Default::default()))
}

fn load_marker_store(paths: Option<&AppPaths>) -> crate::platform::MarkerStore {
    let marker_path = crate::platform::marker_store_path(paths);
    crate::platform::MarkerStore::load(marker_path.clone())
        .unwrap_or_else(|_| crate::platform::MarkerStore::with_path(marker_path))
}

fn refresh_runtime_window(window: &MainWindow, runtime: &DesktopRuntime) {
    if let Some(controller) = runtime.controller() {
        refresh_window_with_language(window, controller.session().state(), runtime.ui_language);
    } else {
        window.set_ui_language_code(runtime.ui_language.code().into());
        window.set_status_label(runtime.status_message().into());
        refresh_navigation_surfaces(window, &PlayerState::default());
    }
}

fn history_now(runtime: &DesktopRuntime) -> Duration {
    runtime.started_at.elapsed()
}

#[derive(Debug, Clone)]
struct PlaybackHistorySnapshot {
    current: Option<MediaLocator>,
    title: Option<String>,
    position_seconds: f64,
    paused: bool,
}

fn current_playlist_title(session: &AppSession<MpvBackend>) -> Option<String> {
    let snapshot = session.playlist_snapshot();
    let index = snapshot.current_index?;
    snapshot.entries.get(index).map(|entry| entry.title.clone())
}

fn capture_history_snapshot(session: &AppSession<MpvBackend>) -> PlaybackHistorySnapshot {
    PlaybackHistorySnapshot {
        current: session.state().current.clone(),
        title: current_playlist_title(session),
        position_seconds: session.state().position_seconds,
        paused: session.state().paused,
    }
}

fn window_width(window: &MainWindow) -> f32 {
    window
        .window()
        .with_winit_window(|winit_window| winit_window.inner_size().width as f32)
        .unwrap_or(1200.0)
}

fn refresh_sidebar(window: &MainWindow, runtime: &DesktopRuntime) {
    window.set_sidebar_visible(runtime.sidebar.visible);
    window.set_sidebar_tab_index(runtime.sidebar.tab_index());
    window.set_sidebar_expanded_width_px(crate::expanded_sidebar_width(window_width(window)));

    let playlist_rows = runtime
        .controller()
        .map(|controller| crate::build_playlist_rows(&controller.session().playlist_snapshot()))
        .unwrap_or_default()
        .into_iter()
        .map(|row| PlaylistSidebarRowData { title: row.title.into(), is_current: row.is_current })
        .collect::<Vec<_>>();

    let history_rows = crate::build_history_rows(runtime.history.store())
        .into_iter()
        .map(|row| HistorySidebarRowData { title: row.title.into(), subtitle: row.subtitle.into() })
        .collect::<Vec<_>>();

    window.set_playlist_rows(model_from_vec(playlist_rows));
    window.set_history_rows(model_from_vec(history_rows));
}

fn refresh_recent_open_menu(window: &MainWindow, runtime: &DesktopRuntime) {
    let rows = runtime
        .recent_open
        .items
        .iter()
        .map(|item| RecentOpenRowData {
            title: item.title.clone().into(),
            subtitle: item.target.clone().into(),
        })
        .collect::<Vec<_>>();

    window.set_recent_open_rows(model_from_vec(rows));
}

fn refresh_tracks_popup(window: &MainWindow, runtime: &DesktopRuntime) {
    let Some(state) = runtime.controller().map(|controller| controller.session().state()) else {
        window.set_audio_track_rows(model_from_vec(Vec::<TrackPopupRowData>::new()));
        window.set_subtitle_track_rows(model_from_vec(Vec::<TrackPopupRowData>::new()));
        window.set_video_track_rows(model_from_vec(Vec::<TrackPopupRowData>::new()));
        window.set_subtitle_visible(true);
        window.set_subtitle_delay_value(0.0);
        window.set_subtitle_delay_label(crate::format_subtitle_delay_label(0.0).into());
        window.set_subtitle_scale_value(1.0);
        window.set_subtitle_scale_label(crate::format_subtitle_scale_label(1.0).into());
        window.set_subtitle_position_value(100.0);
        window.set_tracks_status_label(runtime.status_message().into());
        return;
    };

    window.set_audio_track_rows(model_from_vec(
        crate::build_audio_track_rows(state)
            .into_iter()
            .map(|row| TrackPopupRowData { label: row.label.into(), selected: row.is_selected })
            .collect(),
    ));
    window.set_subtitle_track_rows(model_from_vec(
        crate::build_subtitle_track_rows(state)
            .into_iter()
            .map(|row| TrackPopupRowData { label: row.label.into(), selected: row.is_selected })
            .collect(),
    ));
    window.set_video_track_rows(model_from_vec(
        crate::build_video_track_rows(state)
            .into_iter()
            .map(|row| TrackPopupRowData { label: row.label.into(), selected: row.is_selected })
            .collect(),
    ));
    window.set_subtitle_visible(state.subtitle.visible);
    window.set_subtitle_delay_value(state.subtitle.delay_seconds as f32);
    window.set_subtitle_delay_label(
        crate::format_subtitle_delay_label(state.subtitle.delay_seconds).into(),
    );
    window.set_subtitle_scale_value(state.subtitle.scale);
    window
        .set_subtitle_scale_label(crate::format_subtitle_scale_label(state.subtitle.scale).into());
    window.set_subtitle_position_value(f32::from(state.subtitle.vertical_position_percent));
    window.set_tracks_status_label(
        state
            .last_error
            .clone()
            .or_else(|| state.status_message.clone())
            .unwrap_or_default()
            .into(),
    );
}

fn refresh_settings_window(window: &SettingsWindow, controller: &crate::SettingsController) {
    let snapshot = controller.snapshot();
    window.set_section_index(snapshot.section_index);
    window.set_default_speed_value(snapshot.default_speed);
    window.set_default_speed_label(format!("{:.2}x", snapshot.default_speed).into());
    window.set_default_volume_value(i32::from(snapshot.default_volume_percent));
    window.set_default_volume_label(format!("{}%", snapshot.default_volume_percent).into());
    window.set_playback_end_behavior_index(snapshot.playback_end_behavior_index);
    window.set_prefer_hardware_decode(snapshot.prefer_hardware_decode);
    window.set_remember_history(snapshot.remember_history);
    window.set_show_playlist_on_startup(snapshot.show_playlist_on_startup);
    window.set_dirty(snapshot.dirty);
    window.set_can_apply(snapshot.can_apply);
    window.set_status_label(snapshot.status_message.into());

    let rows = snapshot
        .shortcut_rows
        .into_iter()
        .map(|row| SettingsShortcutRowData {
            action_label: row.action_label.into(),
            binding_label: row.binding_label.into(),
            conflict_label: row.conflict_message.unwrap_or_default().into(),
            is_capturing: row.is_capturing,
        })
        .collect::<Vec<_>>();

    window.set_shortcut_rows(model_from_vec(rows));
}

fn refresh_runtime_settings_window(runtime: &DesktopRuntime) {
    if let (Some(window), Some(controller)) =
        (runtime.settings_window.as_ref(), runtime.settings_controller.as_ref())
    {
        refresh_settings_window(window, controller);
    }
}

fn mutate_settings_controller<F>(runtime: &mut DesktopRuntime, action: F)
where
    F: FnOnce(&mut crate::SettingsController),
{
    {
        let Some(controller) = runtime.settings_controller.as_mut() else {
            return;
        };
        action(controller);
    }
    refresh_runtime_settings_window(runtime);
}

fn apply_saved_settings(runtime: &mut DesktopRuntime, saved: AppConfig) {
    if let Some(controller) = runtime.controller_mut() {
        controller.set_shortcuts(saved.shortcuts.clone());
        controller.set_config(saved.clone());
    }
    runtime.history.set_enabled(saved.ui.remember_history);
    runtime.config = saved;
}

fn handle_settings_save(
    runtime: &Rc<RefCell<DesktopRuntime>>,
    config_path: &PathBuf,
    close_after_save: bool,
) {
    let mut runtime = runtime.borrow_mut();
    let saved = {
        let Some(controller) = runtime.settings_controller.as_mut() else {
            return;
        };
        match controller.save(config_path) {
            Ok(saved) => saved,
            Err(error) => {
                if let Some(window) = runtime.settings_window.as_ref() {
                    window.set_status_label(error.to_string().into());
                }
                return;
            }
        }
    };

    apply_saved_settings(&mut runtime, saved);
    if let Some(app) = runtime.app_handle.as_ref().and_then(|handle| handle.upgrade()) {
        app.set_status_label("Settings saved".into());
    }
    refresh_runtime_settings_window(&runtime);
    if close_after_save {
        if let Some(window) = runtime.settings_window.as_ref() {
            let _ = window.hide();
        }
    }
}

fn ensure_runtime_ready(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
) -> bool {
    let runtime = runtime.borrow();
    if runtime.controller().is_some() {
        return true;
    }

    if let Some(app) = app_handle.upgrade() {
        app.set_status_label(runtime.status_message().into());
    }
    false
}

fn sync_history_from_snapshot(
    runtime: &mut DesktopRuntime,
    snapshot: &PlaybackHistorySnapshot,
) -> Result<(), yoyo_core::StorageError> {
    let now = history_now(runtime);
    let current = snapshot.current.clone();
    let switched = current != runtime.last_seen_locator;
    runtime.last_seen_locator = current.clone();

    if let (Some(locator), Some(title)) = (current.as_ref(), snapshot.title.as_ref()) {
        runtime.history.remember_playback(locator, title, Some(snapshot.position_seconds));
    }

    if switched {
        runtime.history.flush_if_needed(now, crate::FlushReason::MediaSwitch)?;
    } else if snapshot.paused {
        runtime.history.flush_if_needed(now, crate::FlushReason::Pause)?;
    } else {
        runtime.history.flush_if_needed(now, crate::FlushReason::PeriodicTick)?;
    }

    Ok(())
}

fn sync_subtitle_prefs_from_state(
    runtime: &mut DesktopRuntime,
    state: &PlayerState,
) -> Result<(), yoyo_core::StorageError> {
    let now = history_now(runtime);
    let current = state.current.clone();
    let switched = current != runtime.last_seen_subtitle_locator;
    runtime.last_seen_subtitle_locator = current;

    if state.subtitle_preferences_restored {
        runtime.subtitle_prefs.remember_from_state(state);
    }

    runtime.subtitle_prefs.flush_if_needed(
        now,
        if switched {
            crate::SubtitlePrefsFlushReason::MediaSwitch
        } else {
            crate::SubtitlePrefsFlushReason::PeriodicTick
        },
    )?;

    Ok(())
}

fn restore_markers_for_current_media(
    runtime: &mut DesktopRuntime,
    controller: &mut DesktopController<MpvBackend>,
) {
    let locator_key = current_locator_key(controller.session().state());
    if locator_key == runtime.last_marker_locator_key {
        return;
    }

    runtime.last_marker_locator_key = locator_key.clone();
    let Some(locator_key) = locator_key else {
        return;
    };

    let markers = runtime.marker_store.markers_for(&locator_key);
    controller.session_mut().set_markers(markers);
}

fn persist_current_markers(runtime: &mut DesktopRuntime, state: &PlayerState) {
    let Some(locator_key) = current_locator_key(state) else {
        return;
    };

    if runtime.marker_store.markers_for(&locator_key) == state.markers {
        return;
    }

    runtime.marker_store.set_markers(locator_key, state.markers.clone());
    if let Err(error) = runtime.marker_store.save() {
        runtime.record_diagnostic("WARN", format!("Marker store save failed: {error}"));
    }
}

fn apply_subtitle_restore_if_needed(
    runtime: &mut DesktopRuntime,
    controller: &mut DesktopController<MpvBackend>,
    app: Option<&MainWindow>,
) -> Result<(), yoyo_core::AppError> {
    let state = controller.session().state().clone();
    let Some(locator) = state.current.as_ref() else {
        controller.set_subtitle_preferences_restored(true);
        return Ok(());
    };

    if state.subtitle_preferences_restored {
        return Ok(());
    }

    if state.audio_tracks.is_empty()
        && state.subtitle_tracks.is_empty()
        && state.video_tracks.is_empty()
    {
        return Ok(());
    }

    match runtime.subtitle_prefs.restore_plan_for(locator) {
        Ok(Some(plan)) => {
            for command in plan.commands {
                controller.dispatch(command)?;
            }
        }
        Ok(None) => {}
        Err(crate::SubtitleRestoreError::MissingExternalSubtitle(path)) => {
            if let Some(app) = app {
                app.set_status_label(
                    format!("Subtitle file is missing: {}", path.display()).into(),
                );
            }
        }
    }

    controller.set_subtitle_preferences_restored(true);
    Ok(())
}

fn apply_pending_resume(
    controller: &mut DesktopController<MpvBackend>,
    pending: Option<crate::PendingResumeSeek>,
) -> Result<Option<crate::PendingResumeSeek>, yoyo_core::AppError> {
    let Some(seek) = pending else {
        return Ok(None);
    };
    let Some(position) = seek.try_resolve(controller.session().state().duration_seconds) else {
        return Ok(Some(seek));
    };

    controller.dispatch(AppCommand::SeekAbsolute(position))?;
    Ok(None)
}

fn with_runtime_controller<F>(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    action: F,
) -> bool
where
    F: FnOnce(&mut DesktopController<MpvBackend>) -> Result<(), yoyo_core::AppError>,
{
    let mut runtime = runtime.borrow_mut();
    let pending_before = runtime.pending_resume.take();
    let Some(mut controller) = runtime.controller.take() else {
        runtime.pending_resume = pending_before;
        let message = runtime.status_message();
        runtime.record_diagnostic("WARN", &message);
        if let Some(app) = app_handle.upgrade() {
            app.set_status_label(message.into());
        }
        return false;
    };

    let app = app_handle.upgrade();
    let outcome = match action(&mut controller) {
        Ok(()) => match apply_pending_resume(&mut controller, pending_before) {
            Ok(pending_after) => {
                match apply_subtitle_restore_if_needed(&mut runtime, &mut controller, app.as_ref())
                {
                    Ok(()) => {
                        restore_markers_for_current_media(&mut runtime, &mut controller);
                        let state = controller.session().state().clone();
                        let history_snapshot = capture_history_snapshot(controller.session());
                        match sync_subtitle_prefs_from_state(&mut runtime, &state) {
                            Ok(()) => Ok((state, history_snapshot, pending_after)),
                            Err(error) => Err((error.into(), pending_after)),
                        }
                    }
                    Err(error) => Err((error, pending_after)),
                }
            }
            Err(error) => Err((error, pending_before)),
        },
        Err(error) => Err((error, pending_before)),
    };
    runtime.controller = Some(controller);

    match outcome {
        Ok((state, history_snapshot, pending_after)) => {
            runtime.pending_resume = pending_after;
            if let Err(error) = sync_history_from_snapshot(&mut runtime, &history_snapshot) {
                if let Some(app) = app_handle.upgrade() {
                    app.set_status_label(error.to_string().into());
                }
                return false;
            }
            persist_current_markers(&mut runtime, &state);
            if let Some(app) = app_handle.upgrade() {
                refresh_window_with_language(&app, &state, runtime.ui_language);
                refresh_sidebar(&app, &runtime);
                refresh_tracks_popup(&app, &runtime);
                apply_fullscreen_state(&app, &state);
            }
            true
        }
        Err((error, pending_restore)) => {
            runtime.pending_resume = pending_restore;
            runtime.record_diagnostic("ERROR", error.to_string());
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(error.to_string().into());
            }
            false
        }
    }
}

fn dispatch_screenshot(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    paths: Option<AppPaths>,
) {
    let path = match crate::platform::prepare_screenshot_path(paths.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            runtime
                .borrow_mut()
                .record_diagnostic("ERROR", format!("Screenshot path failed: {error}"));
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(format!("Screenshot path failed: {error}").into());
            }
            return;
        }
    };

    with_runtime_controller(app_handle, runtime, move |controller| {
        controller.dispatch(AppCommand::TakeScreenshot(path))
    });
}

fn dispatch_video_adjustment(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    kind: VideoAdjustmentKind,
    value: i32,
) {
    with_runtime_controller(app_handle, runtime, move |controller| {
        controller.dispatch(AppCommand::SetVideoAdjustment(kind, value.clamp(-100, 100) as i16))
    });
}

fn recent_title_for_target(kind: crate::platform::RecentOpenKind, target: &str) -> String {
    match kind {
        crate::platform::RecentOpenKind::File | crate::platform::RecentOpenKind::Folder => {
            std::path::Path::new(target)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(target)
                .to_string()
        }
        crate::platform::RecentOpenKind::Url => target.to_string(),
    }
}

fn remember_recent_open(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    kind: crate::platform::RecentOpenKind,
    target: String,
) {
    let opened_at = chrono::Local::now().to_rfc3339();
    let title = recent_title_for_target(kind, &target);
    let mut runtime = runtime.borrow_mut();
    runtime.recent_open.remember(crate::platform::RecentOpenItem {
        kind,
        target,
        title,
        opened_at,
    });
    if let Err(error) = runtime.recent_open.save() {
        runtime.record_diagnostic("WARN", format!("Recent open save failed: {error}"));
    }
    if let Some(app) = app_handle.upgrade() {
        refresh_recent_open_menu(&app, &runtime);
    }
}

fn save_current_window_state(runtime: &Rc<RefCell<DesktopRuntime>>, window: &slint::Window) {
    let size = window.size();
    let position = window.position();
    let state = crate::platform::WindowState {
        width: size.width,
        height: size.height,
        x: Some(position.x),
        y: Some(position.y),
        maximized: window.is_maximized(),
    }
    .clamped();

    let mut runtime = runtime.borrow_mut();
    if let Err(error) =
        crate::platform::save_window_state(runtime.window_state_path.clone(), &state)
    {
        runtime.record_diagnostic("WARN", format!("Window state save failed: {error}"));
    }
}

fn dispatch_dropped_paths(
    app_handle: &slint::Weak<MainWindow>,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    paths: Vec<PathBuf>,
) {
    if paths.is_empty() {
        return;
    }

    let action = match crate::platform::classify_dropped_paths(&paths) {
        Ok(action) => action,
        Err(error) => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(format!("Drop failed: {error}").into());
            }
            return;
        }
    };
    let status = dropped_media_status(&action);
    let single_folder_drop =
        if paths.len() == 1 && paths[0].is_dir() { Some(paths[0].clone()) } else { None };

    match action {
        crate::platform::DroppedMediaAction::NoPlayableMedia { .. } => {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(status.into());
            }
        }
        crate::platform::DroppedMediaAction::OpenFile(path) => {
            let target = path.display().to_string();
            let dispatched = with_runtime_controller(app_handle, runtime, move |controller| {
                controller.dispatch(AppCommand::OpenFile(path))
            });
            if dispatched && let Some(app) = app_handle.upgrade() {
                app.set_status_label(status.into());
            }
            if dispatched {
                remember_recent_open(
                    app_handle,
                    runtime,
                    crate::platform::RecentOpenKind::File,
                    target,
                );
            }
        }
        crate::platform::DroppedMediaAction::ReplacePlaylist(entries) => {
            let dispatched = with_runtime_controller(app_handle, runtime, move |controller| {
                controller.open_playlist_entries(entries)
            });
            if dispatched && let Some(app) = app_handle.upgrade() {
                app.set_status_label(status.into());
            }
            if dispatched && let Some(path) = single_folder_drop {
                remember_recent_open(
                    app_handle,
                    runtime,
                    crate::platform::RecentOpenKind::Folder,
                    path.display().to_string(),
                );
            }
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt().with_target(false).try_init();

    let paths = AppPaths::discover();
    let config_path =
        paths.as_ref().map(config_file_path).unwrap_or_else(|| PathBuf::from("config.toml"));
    let config = load_boot_config(paths.as_ref());
    let history = load_history_runtime(paths.as_ref(), &config);
    let recent_open = load_recent_open_store(paths.as_ref());
    let subtitle_prefs = load_subtitle_prefs_runtime(paths.as_ref());
    let marker_store = load_marker_store(paths.as_ref());
    let sidebar = crate::initial_sidebar_state(config.ui.show_playlist_on_startup, 1200.0);
    let diagnostic_log_path = crate::platform::default_log_file(paths.as_ref());
    let window_state_path = crate::platform::window_state_path(paths.as_ref());
    let runtime = Rc::new(RefCell::new(DesktopRuntime::new(
        config,
        history,
        recent_open,
        subtitle_prefs,
        marker_store,
        sidebar,
        diagnostic_log_path,
        window_state_path.clone(),
    )));
    configure_backend(Rc::clone(&runtime))?;

    let app = MainWindow::new()?;
    if let Ok(Some(state)) = crate::platform::load_window_state(window_state_path) {
        app.window().set_size(slint::PhysicalSize::new(state.width, state.height));
        if let (Some(x), Some(y)) = (state.x, state.y) {
            app.window().set_position(slint::PhysicalPosition::new(x, y));
        }
        app.window().set_maximized(state.maximized);
    }
    {
        let mut runtime = runtime.borrow_mut();
        runtime.app_handle = Some(app.as_weak());
        runtime.sidebar = crate::initial_sidebar_state(
            runtime.config.ui.show_playlist_on_startup,
            window_width(&app),
        );
    }
    let dialogs = Rc::new(RfdDialogService);

    refresh_runtime_window(&app, &runtime.borrow());
    refresh_sidebar(&app, &runtime.borrow());
    refresh_recent_open_menu(&app, &runtime.borrow());
    refresh_tracks_popup(&app, &runtime.borrow());

    app.on_open_file_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if !ensure_runtime_ready(&app_handle, &runtime) {
                return;
            }
            if let Some(path) = dialogs.pick_file() {
                let target = path.display().to_string();
                let dispatched =
                    with_runtime_controller(&app_handle, &runtime, move |controller| {
                        controller.dispatch(AppCommand::OpenFile(path))
                    });
                if dispatched {
                    remember_recent_open(
                        &app_handle,
                        &runtime,
                        crate::platform::RecentOpenKind::File,
                        target,
                    );
                }
            }
        }
    });

    app.on_open_folder_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if !ensure_runtime_ready(&app_handle, &runtime) {
                return;
            }
            if let Some(path) = dialogs.pick_folder() {
                let target = path.display().to_string();
                let dispatched =
                    with_runtime_controller(&app_handle, &runtime, move |controller| {
                        controller.open_folder(&path)
                    });
                if dispatched {
                    remember_recent_open(
                        &app_handle,
                        &runtime,
                        crate::platform::RecentOpenKind::Folder,
                        target,
                    );
                }
            }
        }
    });

    app.on_open_url_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |url| {
            let target = url.to_string();
            let command_target = target.clone();
            let dispatched = with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::OpenUrl(command_target))
            });
            if dispatched {
                remember_recent_open(
                    &app_handle,
                    &runtime,
                    crate::platform::RecentOpenKind::Url,
                    target,
                );
            }
        }
    });

    app.on_window_drag_requested({
        let runtime = Rc::clone(&runtime);
        let app_handle = app.as_weak();
        move || {
            let Some(app) = app_handle.upgrade() else {
                return;
            };
            let result = app.window().with_winit_window(|winit_window| winit_window.drag_window());
            if let Some(Err(error)) = result {
                runtime
                    .borrow_mut()
                    .record_diagnostic("WARN", format!("Window drag failed: {error}"));
            }
        }
    });

    app.on_window_minimize_requested({
        let app_handle = app.as_weak();
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.window().with_winit_window(|winit_window| winit_window.set_minimized(true));
            }
        }
    });

    app.on_window_maximize_restore_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.window().set_maximized(!app.window().is_maximized());
                save_current_window_state(&runtime, app.window());
            }
        }
    });

    app.on_window_close_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            if let Some(app) = app_handle.upgrade() {
                save_current_window_state(&runtime, app.window());
                let _ = app.hide();
            }
        }
    });

    app.on_language_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |language_code| {
            let language = crate::UiLanguage::parse(language_code.as_str());
            let Some(app) = app_handle.upgrade() else {
                return;
            };
            let mut runtime = runtime.borrow_mut();
            runtime.ui_language = language;
            if let Some(controller) = runtime.controller() {
                refresh_window_with_language(&app, controller.session().state(), language);
            } else {
                app.set_ui_language_code(language.code().into());
                app.set_status_label(runtime.status_message().into());
            }
        }
    });

    app.on_video_double_clicked(command_callback(&app, &runtime, AppCommand::ToggleFullscreen));

    app.on_video_dragged({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |delta_x, delta_y| {
            let Some(app) = app_handle.upgrade() else {
                return;
            };
            let width = (app.get_video_area_width() as f64).max(1.0);
            let height = (app.get_video_area_height() as f64).max(1.0);
            let pan_delta_x = f64::from(delta_x) / width;
            let pan_delta_y = f64::from(delta_y) / height;
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::AdjustVideoPan {
                    delta_x: pan_delta_x,
                    delta_y: pan_delta_y,
                })
            });
        }
    });

    app.on_reset_video_pan_requested(command_callback(&app, &runtime, AppCommand::ResetVideoPan));

    app.on_toggle_pause_requested(command_callback(&app, &runtime, AppCommand::TogglePause));
    app.on_toggle_mute_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            if with_runtime_controller(&app_handle, &runtime, |controller| {
                controller.dispatch(AppCommand::ToggleMute)
            }) && let Some(app) = app_handle.upgrade()
            {
                let mut runtime = runtime.borrow_mut();
                let muted = runtime
                    .controller()
                    .map(|controller| controller.session().state().muted)
                    .unwrap_or(false);
                set_osd(&app, &mut runtime, crate::OsdKind::Muted(muted));
            }
        }
    });
    app.on_speed_down_requested(command_callback(&app, &runtime, AppCommand::SetSpeed(0.75)));
    app.on_speed_up_requested(command_callback(&app, &runtime, AppCommand::SetSpeed(1.25)));
    app.on_reset_speed_requested(command_callback(&app, &runtime, AppCommand::ResetSpeed));
    app.on_seek_percent_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |percent| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                let Some(duration) = controller.session().state().duration_seconds else {
                    return Ok(());
                };
                let position = duration * f64::from(percent.clamp(0.0, 1.0));
                controller.dispatch(AppCommand::SeekAbsolute(position))
            });
        }
    });
    app.on_progress_preview_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |percent| {
            let Some(app) = app_handle.upgrade() else {
                return;
            };
            let runtime = runtime.borrow();
            let Some(controller) = runtime.controller() else {
                return;
            };
            let state = controller.session().state();
            let Some(duration) = state.duration_seconds else {
                return;
            };
            let percent = percent.clamp(0.0, 1.0);
            let seconds = duration * f64::from(percent);
            app.set_progress_preview_visible(true);
            app.set_progress_preview_value(percent);
            app.set_progress_preview_label(crate::format_preview_label(seconds, None).into());
        }
    });
    app.on_progress_commit_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |percent| {
            let percent = percent.clamp(0.0, 1.0);
            let target = {
                let runtime = runtime.borrow();
                runtime
                    .controller()
                    .and_then(|controller| controller.session().state().duration_seconds)
                    .map(|duration| duration * f64::from(percent))
            };
            let Some(target) = target else {
                return;
            };
            if with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::SeekAbsolute(target))
            }) && let Some(app) = app_handle.upgrade()
            {
                let mut runtime = runtime.borrow_mut();
                set_osd(&app, &mut runtime, crate::OsdKind::SeekedTo(target));
            }
        }
    });
    app.on_progress_preview_cleared({
        let app_handle = app.as_weak();
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.set_progress_preview_visible(false);
            }
        }
    });
    app.on_volume_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |volume| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::SetVolume(volume.clamp(0, 100) as u8))
            });
        }
    });
    app.on_jump_panel_requested({
        let app_handle = app.as_weak();
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.set_jump_panel_visible(true);
            }
        }
    });
    app.on_jump_input_changed({
        let app_handle = app.as_weak();
        move |input| {
            if let Some(app) = app_handle.upgrade() {
                app.set_jump_input_text(input);
            }
        }
    });
    app.on_jump_commit_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |input| match crate::parse_jump_time(input.as_str()) {
            Ok(seconds) => {
                if with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.dispatch(AppCommand::JumpToTime(seconds))
                }) && let Some(app) = app_handle.upgrade()
                {
                    let mut runtime = runtime.borrow_mut();
                    set_osd(&app, &mut runtime, crate::OsdKind::JumpedTo(seconds));
                    app.set_jump_panel_visible(false);
                }
            }
            Err(message) => {
                if let Some(app) = app_handle.upgrade() {
                    app.set_status_label(message.into());
                }
            }
        }
    });
    app.on_action_panel_requested({
        let app_handle = app.as_weak();
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.set_action_panel_visible(true);
            }
        }
    });
    app.on_action_panel_close_requested({
        let app_handle = app.as_weak();
        move || {
            if let Some(app) = app_handle.upgrade() {
                app.set_action_panel_visible(false);
            }
        }
    });
    app.on_add_marker_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            let created_at = chrono::Local::now().to_rfc3339();
            if with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::AddMarkerAtCurrentPosition { created_at })
            }) && let Some(app) = app_handle.upgrade()
            {
                let mut runtime = runtime.borrow_mut();
                set_osd(&app, &mut runtime, crate::OsdKind::MarkerAdded);
            }
        }
    });
    app.on_remove_marker_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |id| {
            let id = id.to_string();
            if with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::RemoveMarker(id))
            }) && let Some(app) = app_handle.upgrade()
            {
                let mut runtime = runtime.borrow_mut();
                set_osd(&app, &mut runtime, crate::OsdKind::MarkerRemoved);
            }
        }
    });
    app.on_navigation_row_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }
            let row = {
                let runtime = runtime.borrow();
                let Some(controller) = runtime.controller() else {
                    return;
                };
                let state = controller.session().state();
                crate::build_navigation_rows(&state.chapters, &state.markers)
                    .get(index as usize)
                    .cloned()
            };
            let Some(row) = row else {
                return;
            };
            let title = row.title.clone();
            let command = if row.is_marker {
                AppCommand::SeekToMarker(row.id)
            } else {
                let Ok(chapter_index) = row.id.parse::<usize>() else {
                    return;
                };
                AppCommand::SeekToChapter(chapter_index)
            };
            if with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(command)
            }) && let Some(app) = app_handle.upgrade()
            {
                let mut runtime = runtime.borrow_mut();
                set_osd(&app, &mut runtime, crate::OsdKind::Chapter(title));
            }
        }
    });
    app.on_previous_chapter_marker_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SeekToPreviousChapterOrMarker,
    ));
    app.on_next_chapter_marker_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SeekToNextChapterOrMarker,
    ));
    app.on_screenshot_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let paths = paths.clone();
        move || dispatch_screenshot(&app_handle, &runtime, paths.clone())
    });
    app.on_frame_step_previous_requested(command_callback(
        &app,
        &runtime,
        AppCommand::StepFrame(FrameStepDirection::Previous),
    ));
    app.on_frame_step_next_requested(command_callback(
        &app,
        &runtime,
        AppCommand::StepFrame(FrameStepDirection::Next),
    ));
    app.on_brightness_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |value| {
            dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Brightness, value)
        }
    });
    app.on_contrast_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |value| {
            dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Contrast, value)
        }
    });
    app.on_saturation_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |value| {
            dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Saturation, value)
        }
    });
    app.on_gamma_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |value| {
            dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Gamma, value)
        }
    });
    app.on_hue_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |value| {
            dispatch_video_adjustment(&app_handle, &runtime, VideoAdjustmentKind::Hue, value)
        }
    });
    app.on_reset_video_adjustments_requested(command_callback(
        &app,
        &runtime,
        AppCommand::ResetVideoAdjustments,
    ));
    app.on_filter_none_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SetVideoFilterPreset(VideoFilterPreset::None),
    ));
    app.on_filter_sharpen_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SetVideoFilterPreset(VideoFilterPreset::Sharpen),
    ));
    app.on_filter_light_denoise_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SetVideoFilterPreset(VideoFilterPreset::LightDenoise),
    ));
    app.on_filter_grayscale_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SetVideoFilterPreset(VideoFilterPreset::Grayscale),
    ));
    app.on_filter_invert_requested(command_callback(
        &app,
        &runtime,
        AppCommand::SetVideoFilterPreset(VideoFilterPreset::Invert),
    ));
    app.on_rotate_requested(command_callback(&app, &runtime, AppCommand::RotateClockwise));
    app.on_cycle_audio_requested(command_callback(&app, &runtime, AppCommand::CycleAudioChannel));
    app.on_zoom_in_requested(command_callback(&app, &runtime, AppCommand::ZoomIn));
    app.on_zoom_out_requested(command_callback(&app, &runtime, AppCommand::ZoomOut));
    app.on_set_ab_point_a_requested(command_callback(&app, &runtime, AppCommand::SetABLoopPointA));
    app.on_set_ab_point_b_requested(command_callback(&app, &runtime, AppCommand::SetABLoopPointB));
    app.on_clear_ab_loop_requested(command_callback(&app, &runtime, AppCommand::ClearABLoop));
    app.on_toggle_fullscreen_requested(command_callback(
        &app,
        &runtime,
        AppCommand::ToggleFullscreen,
    ));

    app.on_toggle_sidebar_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            let mut runtime = runtime.borrow_mut();
            runtime.sidebar.toggle();
            if let Some(app) = app_handle.upgrade() {
                refresh_sidebar(&app, &runtime);
                refresh_tracks_popup(&app, &runtime);
            }
        }
    });

    app.on_show_playlist_tab_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            let mut runtime = runtime.borrow_mut();
            runtime.sidebar.show_tab(crate::SidebarTab::Playlist);
            if let Some(app) = app_handle.upgrade() {
                refresh_sidebar(&app, &runtime);
                refresh_tracks_popup(&app, &runtime);
            }
        }
    });

    app.on_show_history_tab_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            let mut runtime = runtime.borrow_mut();
            runtime.sidebar.show_tab(crate::SidebarTab::History);
            if let Some(app) = app_handle.upgrade() {
                refresh_sidebar(&app, &runtime);
                refresh_tracks_popup(&app, &runtime);
            }
        }
    });

    app.on_playlist_item_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.open_playlist_index(index as usize)
            });
        }
    });

    app.on_history_item_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }

            let activation = {
                let runtime = runtime.borrow();
                runtime.history.activation_for(index as usize)
            };

            match activation {
                Ok(Some(activation)) => {
                    let command = activation.command;
                    let pending_seek = activation.pending_seek;
                    let dispatched =
                        with_runtime_controller(&app_handle, &runtime, move |controller| {
                            controller.dispatch(command)
                        });
                    if dispatched {
                        runtime.borrow_mut().pending_resume = pending_seek;
                    }
                }
                Ok(None) => {}
                Err(crate::HistoryActivationError::MissingLocalFile(path)) => {
                    if let Some(app) = app_handle.upgrade() {
                        app.set_status_label(
                            format!("History file is missing: {}", path.display()).into(),
                        );
                    }
                }
            }
        }
    });

    app.on_recent_open_item_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }

            let item = {
                let runtime = runtime.borrow();
                runtime.recent_open.items.get(index as usize).cloned()
            };
            let Some(item) = item else {
                return;
            };

            if let Some(app) = app_handle.upgrade() {
                app.set_status_label(recent_item_status(&item).into());
            }

            match item.kind {
                crate::platform::RecentOpenKind::File => {
                    let path = PathBuf::from(&item.target);
                    if !path.is_file() {
                        if let Some(app) = app_handle.upgrade() {
                            app.set_status_label(
                                format!("Recent item is missing: {}", path.display()).into(),
                            );
                        }
                        return;
                    }
                    let target = item.target.clone();
                    let dispatched =
                        with_runtime_controller(&app_handle, &runtime, move |controller| {
                            controller.dispatch(AppCommand::OpenFile(path))
                        });
                    if dispatched {
                        remember_recent_open(
                            &app_handle,
                            &runtime,
                            crate::platform::RecentOpenKind::File,
                            target,
                        );
                    }
                }
                crate::platform::RecentOpenKind::Folder => {
                    let path = PathBuf::from(&item.target);
                    if !path.is_dir() {
                        if let Some(app) = app_handle.upgrade() {
                            app.set_status_label(
                                format!("Recent item is missing: {}", path.display()).into(),
                            );
                        }
                        return;
                    }
                    let target = item.target.clone();
                    let dispatched =
                        with_runtime_controller(&app_handle, &runtime, move |controller| {
                            controller.open_folder(&path)
                        });
                    if dispatched {
                        remember_recent_open(
                            &app_handle,
                            &runtime,
                            crate::platform::RecentOpenKind::Folder,
                            target,
                        );
                    }
                }
                crate::platform::RecentOpenKind::Url => {
                    let target = item.target.clone();
                    let command_target = target.clone();
                    let dispatched =
                        with_runtime_controller(&app_handle, &runtime, move |controller| {
                            controller.dispatch(AppCommand::OpenUrl(command_target))
                        });
                    if dispatched {
                        remember_recent_open(
                            &app_handle,
                            &runtime,
                            crate::platform::RecentOpenKind::Url,
                            target,
                        );
                    }
                }
            }
        }
    });

    app.on_audio_track_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                let rows = crate::build_audio_track_rows(controller.session().state());
                let Some(id) = rows.get(index as usize).and_then(|row| row.track_id) else {
                    return Ok(());
                };
                controller.dispatch(AppCommand::SelectAudioTrack(id))
            });
        }
    });

    app.on_subtitle_track_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                let rows = crate::build_subtitle_track_rows(controller.session().state());
                let Some(row) = rows.get(index as usize) else {
                    return Ok(());
                };
                match row.track_id {
                    Some(id) => controller.dispatch(AppCommand::SelectSubtitleTrack(id)),
                    None => controller.dispatch(AppCommand::SetSubtitleVisible(false)),
                }
            });
        }
    });

    app.on_video_track_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |index| {
            if index < 0 {
                return;
            }
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                let rows = crate::build_video_track_rows(controller.session().state());
                let Some(id) = rows.get(index as usize).and_then(|row| row.track_id) else {
                    return Ok(());
                };
                controller.dispatch(AppCommand::SelectVideoTrack(id))
            });
        }
    });

    app.on_load_external_subtitle_requested({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let dialogs = Rc::clone(&dialogs);
        move || {
            if let Some(path) = dialogs.pick_subtitle_file() {
                with_runtime_controller(&app_handle, &runtime, move |controller| {
                    controller.dispatch(AppCommand::LoadExternalSubtitle(path))
                });
            }
        }
    });

    app.on_subtitle_visible_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |visible| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::SetSubtitleVisible(visible))
            });
        }
    });

    app.on_subtitle_delay_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |delay| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller
                    .dispatch(AppCommand::SetSubtitleDelay(f64::from(delay.clamp(-10.0, 10.0))))
            });
        }
    });

    app.on_subtitle_scale_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |scale| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::SetSubtitleScale(scale.clamp(0.5, 2.0)))
            });
        }
    });

    app.on_subtitle_position_changed({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move |position| {
            with_runtime_controller(&app_handle, &runtime, move |controller| {
                controller.dispatch(AppCommand::SetSubtitleVerticalPosition(
                    position.clamp(0.0, 100.0) as u8,
                ))
            });
        }
    });

    app.on_settings_requested({
        let runtime = Rc::clone(&runtime);
        let config_path = config_path.clone();
        move || {
            let mut runtime_ref = runtime.borrow_mut();
            let current_config = runtime_ref.config.clone();
            runtime_ref.settings_controller = Some(crate::SettingsController::new(current_config));

            if runtime_ref.settings_window.is_none() {
                let window = SettingsWindow::new().expect("settings window");
                let keyboard_state = Rc::new(RefCell::new(
                    crate::keyboard::winit_adapter::WinitKeyboardState::default(),
                ));

                window.on_section_requested({
                    let runtime = Rc::clone(&runtime);
                    move |index| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_section(index);
                        });
                    }
                });

                window.on_default_speed_changed({
                    let runtime = Rc::clone(&runtime);
                    move |speed| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_default_speed(speed);
                        });
                    }
                });

                window.on_default_volume_changed({
                    let runtime = Rc::clone(&runtime);
                    move |volume| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_default_volume_percent(volume.clamp(0, 100) as u8);
                        });
                    }
                });

                window.on_playback_end_behavior_changed({
                    let runtime = Rc::clone(&runtime);
                    move |index| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_playback_end_behavior_index(index);
                        });
                    }
                });

                window.on_prefer_hardware_decode_changed({
                    let runtime = Rc::clone(&runtime);
                    move |value| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_prefer_hardware_decode(value);
                        });
                    }
                });

                window.on_remember_history_changed({
                    let runtime = Rc::clone(&runtime);
                    move |value| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_remember_history(value);
                        });
                    }
                });

                window.on_show_playlist_on_startup_changed({
                    let runtime = Rc::clone(&runtime);
                    move |value| {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.set_show_playlist_on_startup(value);
                        });
                    }
                });

                window.on_edit_shortcut_requested({
                    let runtime = Rc::clone(&runtime);
                    move |index| {
                        if index < 0 {
                            return;
                        }
                        let Some(action) = ShortcutAction::all().get(index as usize).copied()
                        else {
                            return;
                        };
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.begin_shortcut_capture(action);
                        });
                    }
                });

                window.on_clear_shortcut_requested({
                    let runtime = Rc::clone(&runtime);
                    move |index| {
                        if index < 0 {
                            return;
                        }
                        let Some(action) = ShortcutAction::all().get(index as usize).copied()
                        else {
                            return;
                        };
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.clear_shortcut(action);
                        });
                    }
                });

                window.on_restore_shortcut_requested({
                    let runtime = Rc::clone(&runtime);
                    move |index| {
                        if index < 0 {
                            return;
                        }
                        let Some(action) = ShortcutAction::all().get(index as usize).copied()
                        else {
                            return;
                        };
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.restore_shortcut_default(action);
                        });
                    }
                });

                window.on_restore_defaults_requested({
                    let runtime = Rc::clone(&runtime);
                    move || {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.restore_defaults();
                        });
                    }
                });

                window.on_apply_requested({
                    let runtime = Rc::clone(&runtime);
                    let config_path = config_path.clone();
                    move || handle_settings_save(&runtime, &config_path, false)
                });

                window.on_ok_requested({
                    let runtime = Rc::clone(&runtime);
                    let config_path = config_path.clone();
                    move || handle_settings_save(&runtime, &config_path, true)
                });

                window.on_cancel_requested({
                    let runtime = Rc::clone(&runtime);
                    move || {
                        let mut runtime = runtime.borrow_mut();
                        mutate_settings_controller(&mut runtime, |controller| {
                            controller.discard_changes();
                        });
                        if let Some(window) = runtime.settings_window.as_ref() {
                            let _ = window.hide();
                        }
                    }
                });

                window.window().on_winit_window_event({
                    let runtime = Rc::clone(&runtime);
                    let keyboard_state = Rc::clone(&keyboard_state);
                    move |_window, event| {
                        let Some(input) = keyboard_state.borrow_mut().update(event) else {
                            return slint::winit_030::EventResult::Propagate;
                        };

                        let mut runtime = runtime.borrow_mut();
                        let is_capturing = runtime
                            .settings_controller
                            .as_ref()
                            .is_some_and(crate::SettingsController::is_capturing);
                        if !is_capturing {
                            return slint::winit_030::EventResult::Propagate;
                        }

                        let consumed = {
                            let controller =
                                runtime.settings_controller.as_mut().expect("checked above");
                            controller.capture_shortcut(input).unwrap_or(false)
                        };
                        if consumed {
                            refresh_runtime_settings_window(&runtime);
                            slint::winit_030::EventResult::PreventDefault
                        } else {
                            slint::winit_030::EventResult::Propagate
                        }
                    }
                });

                runtime_ref.settings_window = Some(window);
            }

            refresh_runtime_settings_window(&runtime_ref);
            if let Some(window) = runtime_ref.settings_window.as_ref() {
                let _ = window.show();
            }
        }
    });

    let keyboard_state =
        Rc::new(RefCell::new(crate::keyboard::winit_adapter::WinitKeyboardState::default()));
    let dropped_paths = Rc::new(RefCell::new(Vec::<PathBuf>::new()));
    let drop_timer = Rc::new(slint::Timer::default());
    app.window().on_winit_window_event({
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        let keyboard_state = Rc::clone(&keyboard_state);
        let paths = paths.clone();
        let dropped_paths = Rc::clone(&dropped_paths);
        let drop_timer = Rc::clone(&drop_timer);
        move |window, event| {
            match event {
                slint::winit_030::winit::event::WindowEvent::Moved(_)
                | slint::winit_030::winit::event::WindowEvent::Resized(_)
                | slint::winit_030::winit::event::WindowEvent::CloseRequested => {
                    save_current_window_state(&runtime, window);
                }
                _ => {}
            }

            if let slint::winit_030::winit::event::WindowEvent::DroppedFile(path) = event {
                dropped_paths.borrow_mut().push(path.clone());
                drop_timer.stop();
                drop_timer.start(slint::TimerMode::SingleShot, Duration::from_millis(120), {
                    let app_handle = app_handle.clone();
                    let runtime = Rc::clone(&runtime);
                    let dropped_paths = Rc::clone(&dropped_paths);
                    move || {
                        let paths = std::mem::take(&mut *dropped_paths.borrow_mut());
                        dispatch_dropped_paths(&app_handle, &runtime, paths);
                    }
                });
                return slint::winit_030::EventResult::PreventDefault;
            }

            let Some(app) = app_handle.upgrade() else {
                return slint::winit_030::EventResult::Propagate;
            };
            if !crate::shortcut_allowed(app.get_url_focused()) {
                return slint::winit_030::EventResult::Propagate;
            }
            let Some(input) = keyboard_state.borrow_mut().update(event) else {
                return slint::winit_030::EventResult::Propagate;
            };
            let Some(gesture) = crate::shortcut_gesture(input) else {
                return slint::winit_030::EventResult::Propagate;
            };

            let dispatch = {
                let runtime_ref = runtime.borrow();
                runtime_ref
                    .controller()
                    .and_then(|controller| controller.resolve_shortcut(gesture.as_str()))
            };

            match dispatch {
                Some(ShortcutDispatch::Command(command)) => {
                    with_runtime_controller(&app_handle, &runtime, move |controller| {
                        controller.dispatch(command)
                    });
                }
                Some(ShortcutDispatch::TakeScreenshot) => {
                    dispatch_screenshot(&app_handle, &runtime, paths.clone());
                }
                Some(ShortcutDispatch::AddMarker) => {
                    let created_at = chrono::Local::now().to_rfc3339();
                    with_runtime_controller(&app_handle, &runtime, move |controller| {
                        controller.dispatch(AppCommand::AddMarkerAtCurrentPosition { created_at })
                    });
                }
                Some(ShortcutDispatch::OpenJumpPanel) => {
                    app.set_jump_panel_visible(true);
                    app.set_status_label("Jump to time".into());
                }
                Some(ShortcutDispatch::OpenActionPanel) => {
                    app.set_action_panel_visible(true);
                    app.set_status_label("Action panel".into());
                }
                None => {}
            }
            slint::winit_030::EventResult::PreventDefault
        }
    });

    let poll_timer = slint::Timer::default();
    poll_timer.start(slint::TimerMode::Repeated, Duration::from_millis(250), {
        let app_handle = app.as_weak();
        let runtime = Rc::clone(&runtime);
        move || {
            let Some(app) = app_handle.upgrade() else {
                return;
            };

            let mut runtime = runtime.borrow_mut();
            let pending_before = runtime.pending_resume.take();
            let Some(mut controller) = runtime.controller.take() else {
                runtime.pending_resume = pending_before;
                refresh_runtime_window(&app, &runtime);
                refresh_sidebar(&app, &runtime);
                refresh_tracks_popup(&app, &runtime);
                return;
            };

            let outcome = match controller.poll_backend() {
                Ok(()) => match apply_pending_resume(&mut controller, pending_before) {
                    Ok(next_pending) => {
                        match apply_subtitle_restore_if_needed(
                            &mut runtime,
                            &mut controller,
                            Some(&app),
                        ) {
                            Ok(()) => {
                                restore_markers_for_current_media(&mut runtime, &mut controller);
                                let state = controller.session().state().clone();
                                let history_snapshot =
                                    capture_history_snapshot(controller.session());
                                let _ = sync_subtitle_prefs_from_state(&mut runtime, &state);
                                Ok((state, history_snapshot, next_pending))
                            }
                            Err(error) => Err((error, next_pending)),
                        }
                    }
                    Err(error) => Err((error, pending_before)),
                },
                Err(error) => Err((error, pending_before)),
            };
            runtime.controller = Some(controller);

            match outcome {
                Ok((state, history_snapshot, next_pending)) => {
                    runtime.pending_resume = next_pending;
                    if let Err(error) = sync_history_from_snapshot(&mut runtime, &history_snapshot)
                    {
                        app.set_status_label(error.to_string().into());
                    }
                    persist_current_markers(&mut runtime, &state);
                    refresh_window_with_language(&app, &state, runtime.ui_language);
                    refresh_sidebar(&app, &runtime);
                    refresh_tracks_popup(&app, &runtime);
                    #[cfg(feature = "mpv-runtime")]
                    sync_runtime_video_host(&app, &mut runtime);
                }
                Err((error, pending_restore)) => {
                    runtime.pending_resume = pending_restore;
                    runtime.record_diagnostic("ERROR", error.to_string());
                    app.set_status_label(error.to_string().into());
                }
            }
        }
    });

    app.run()?;

    {
        let mut runtime = runtime.borrow_mut();
        if let Some(controller) = runtime.controller() {
            let snapshot = capture_history_snapshot(controller.session());
            let state = controller.session().state().clone();
            let _ = sync_history_from_snapshot(&mut runtime, &snapshot);
            let _ = sync_subtitle_prefs_from_state(&mut runtime, &state);
            persist_current_markers(&mut runtime, &state);
        }
        let shutdown_now = history_now(&runtime);
        let _ = runtime.history.flush_if_needed(shutdown_now, crate::FlushReason::Shutdown);
        let _ = runtime
            .subtitle_prefs
            .flush_if_needed(shutdown_now, crate::SubtitlePrefsFlushReason::Shutdown);
    }

    Ok(())
}

fn command_callback(
    app: &MainWindow,
    runtime: &Rc<RefCell<DesktopRuntime>>,
    command: AppCommand,
) -> impl Fn() + 'static {
    let app_handle = app.as_weak();
    let runtime = Rc::clone(runtime);
    move || {
        with_runtime_controller(&app_handle, &runtime, |controller| {
            controller.dispatch(command.clone())
        });
    }
}

fn apply_fullscreen_state(app: &MainWindow, state: &PlayerState) {
    app.window().with_winit_window(|winit_window| {
        if state.fullscreen {
            winit_window.set_fullscreen(Some(
                slint::winit_030::winit::window::Fullscreen::Borderless(None),
            ));
        } else {
            winit_window.set_fullscreen(None);
        }
    });
}

#[cfg(feature = "mpv-runtime")]
fn current_video_rect(window: &MainWindow) -> crate::LogicalVideoRect {
    crate::LogicalVideoRect {
        x: window.get_video_area_x() as f32,
        y: window.get_video_area_y() as f32,
        width: window.get_video_area_width() as f32,
        height: window.get_video_area_height() as f32,
    }
}

#[cfg(feature = "mpv-runtime")]
fn sync_video_host_bounds<H: crate::VideoHost>(
    window: &MainWindow,
    host: &mut H,
    scale_factor: f64,
) -> Result<(), crate::VideoHostError> {
    let bounds = current_video_rect(window).to_physical(scale_factor);
    host.set_bounds(bounds)?;
    host.show()
}

#[cfg(feature = "mpv-runtime")]
fn sync_runtime_video_host(window: &MainWindow, runtime: &mut DesktopRuntime) {
    if runtime.video_host.is_none() {
        return;
    }
    let Some(scale_factor) =
        window.window().with_winit_window(|winit_window| winit_window.scale_factor())
    else {
        return;
    };

    let result = {
        let host = runtime.video_host.as_mut().expect("video host checked above");
        sync_video_host_bounds(window, host, scale_factor).map_err(|error| error.to_string())
    };

    if let Err(error) = result {
        if let Some(host) = runtime.video_host.as_mut() {
            let _ = host.hide();
        }
        runtime.mark_error(error.clone());
        window.set_status_label(error.into());
    }
}

#[cfg(feature = "mpv-runtime")]
fn configure_backend(
    runtime: Rc<RefCell<DesktopRuntime>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = WinitBackend::builder()
        .with_custom_application_handler(Box::new(DesktopWinitHandler::new(runtime)))
        .build()?;
    slint::platform::set_platform(Box::new(backend))?;
    Ok(())
}

#[cfg(not(feature = "mpv-runtime"))]
fn configure_backend(
    _runtime: Rc<RefCell<DesktopRuntime>>,
) -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new().backend_name("winit".into()).select()?;
    Ok(())
}

#[cfg(feature = "mpv-runtime")]
struct DesktopWinitHandler {
    runtime: Rc<RefCell<DesktopRuntime>>,
}

#[cfg(feature = "mpv-runtime")]
impl DesktopWinitHandler {
    fn new(runtime: Rc<RefCell<DesktopRuntime>>) -> Self {
        Self { runtime }
    }

    fn initialize_runtime(
        &mut self,
        event_loop: &slint::winit_030::winit::event_loop::ActiveEventLoop,
        winit_window: Option<&slint::winit_030::winit::window::Window>,
    ) {
        let Some(parent_window) = winit_window else {
            return;
        };

        let mut runtime = self.runtime.borrow_mut();
        if runtime.controller.is_some() || runtime.video_host_error.is_some() {
            return;
        }

        let config = runtime.config.clone();
        let shortcuts = config.shortcuts.clone();
        let result = (|| -> Result<(DesktopController<MpvBackend>, WinitVideoHost), String> {
            let video_host = WinitVideoHost::new_child(event_loop, parent_window)
                .map_err(|error| error.to_string())?;
            let window_id = video_host.mpv_window_id().map_err(|error| error.to_string())?;
            let backend = build_desktop_backend_with_video_window(window_id)
                .map_err(|error| error.to_string())?;
            let session = AppSession::new(config, backend);
            Ok((DesktopController::with_shortcuts(session, shortcuts), video_host))
        })();

        match result {
            Ok((controller, video_host)) => {
                runtime.set_runtime(controller, video_host);
                let app_handle = runtime.app_handle.clone();
                let (state, history_snapshot) = {
                    let controller = runtime.controller().expect("runtime just initialized");
                    (
                        controller.session().state().clone(),
                        capture_history_snapshot(controller.session()),
                    )
                };
                let _ = sync_history_from_snapshot(&mut runtime, &history_snapshot);

                if let Some(app_handle) = app_handle {
                    if let Some(app) = app_handle.upgrade() {
                        refresh_window_with_language(&app, &state, runtime.ui_language);
                        refresh_sidebar(&app, &runtime);
                        refresh_tracks_popup(&app, &runtime);
                        sync_runtime_video_host(&app, &mut runtime);
                    }
                }
            }
            Err(error) => {
                let message = format_runtime_startup_error(&error);
                runtime.record_diagnostic("ERROR", &message);
                runtime.mark_error(message.clone());
                if let Some(app_handle) = runtime.app_handle.clone() {
                    if let Some(app) = app_handle.upgrade() {
                        app.set_status_label(message.into());
                        refresh_sidebar(&app, &runtime);
                        refresh_tracks_popup(&app, &runtime);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "mpv-runtime")]
impl CustomApplicationHandler for DesktopWinitHandler {
    fn window_event(
        &mut self,
        event_loop: &slint::winit_030::winit::event_loop::ActiveEventLoop,
        _window_id: slint::winit_030::winit::window::WindowId,
        winit_window: Option<&slint::winit_030::winit::window::Window>,
        _slint_window: Option<&slint::Window>,
        _event: &slint::winit_030::winit::event::WindowEvent,
    ) -> EventResult {
        self.initialize_runtime(event_loop, winit_window);
        EventResult::Propagate
    }
}
