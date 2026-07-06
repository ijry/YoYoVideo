use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{KeyboardInput, shortcut_gesture};
use yoyo_core::{
    AppConfig, AppError, PlaybackEndBehavior, Shortcut, ShortcutAction, ShortcutMap, StorageError,
    ValidationError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsShortcutRow {
    pub action: ShortcutAction,
    pub action_label: &'static str,
    pub binding_label: String,
    pub conflict_message: Option<String>,
    pub is_capturing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsSnapshot {
    pub section_index: i32,
    pub default_speed: f32,
    pub default_volume_percent: u8,
    pub playback_end_behavior_index: i32,
    pub prefer_hardware_decode: bool,
    pub remember_history: bool,
    pub show_playlist_on_startup: bool,
    pub dirty: bool,
    pub can_apply: bool,
    pub status_message: String,
    pub shortcut_rows: Vec<SettingsShortcutRow>,
}

#[derive(Debug, Clone, PartialEq)]
struct SettingsDraft {
    default_speed: f32,
    default_volume_percent: u8,
    playback_end_behavior: PlaybackEndBehavior,
    prefer_hardware_decode: bool,
    remember_history: bool,
    show_playlist_on_startup: bool,
    shortcuts: BTreeMap<ShortcutAction, Option<String>>,
}

impl SettingsDraft {
    fn from_config(config: &AppConfig) -> Self {
        let shortcuts = ShortcutAction::all()
            .iter()
            .copied()
            .map(|action| {
                let gesture = config
                    .shortcuts
                    .shortcut_for_action(action)
                    .map(|shortcut| shortcut.as_str().to_string());
                (action, gesture)
            })
            .collect();

        Self {
            default_speed: config.playback.default_speed,
            default_volume_percent: config.playback.default_volume_percent,
            playback_end_behavior: config.playback.end_behavior,
            prefer_hardware_decode: config.playback.prefer_hardware_decode,
            remember_history: config.ui.remember_history,
            show_playlist_on_startup: config.ui.show_playlist_on_startup,
            shortcuts,
        }
    }

    fn to_config(&self) -> Result<AppConfig, ValidationError> {
        let mut config = AppConfig::default();
        config.playback.default_speed = self.default_speed;
        config.playback.default_volume_percent = self.default_volume_percent;
        config.playback.end_behavior = self.playback_end_behavior;
        config.playback.prefer_hardware_decode = self.prefer_hardware_decode;
        config.ui.remember_history = self.remember_history;
        config.ui.show_playlist_on_startup = self.show_playlist_on_startup;

        let mut shortcuts = ShortcutMap { bindings: BTreeMap::new() };
        for action in ShortcutAction::all().iter().copied() {
            let parsed = self
                .shortcuts
                .get(&action)
                .cloned()
                .flatten()
                .as_deref()
                .map(Shortcut::parse)
                .transpose()?;
            shortcuts.set_binding(action, parsed)?;
        }

        config.shortcuts = shortcuts;
        config.validate()?;
        Ok(config)
    }

    fn set_shortcut(&mut self, action: ShortcutAction, gesture: Option<String>) {
        self.shortcuts.insert(action, gesture);
    }

    fn restore_shortcut_default(&mut self, action: ShortcutAction) {
        let gesture = action.default_shortcut().map(|shortcut| shortcut.as_str().to_string());
        self.set_shortcut(action, gesture);
    }

    fn clear_shortcut(&mut self, action: ShortcutAction) {
        self.set_shortcut(action, None);
    }

    fn conflict_messages(&self) -> BTreeMap<ShortcutAction, String> {
        let mut seen = BTreeMap::<String, Vec<ShortcutAction>>::new();
        for (action, gesture) in &self.shortcuts {
            if let Some(gesture) = gesture.as_ref() {
                seen.entry(gesture.clone()).or_default().push(*action);
            }
        }

        let mut conflicts = BTreeMap::new();
        for (gesture, actions) in seen {
            if actions.len() < 2 {
                continue;
            }
            let labels = actions.iter().map(|action| action.label()).collect::<Vec<_>>().join(", ");
            for action in actions {
                conflicts.insert(action, format!("Conflicts with {labels}: {gesture}"));
            }
        }
        conflicts
    }
}

fn playback_end_behavior_index(value: PlaybackEndBehavior) -> i32 {
    match value {
        PlaybackEndBehavior::PlayNext => 0,
        PlaybackEndBehavior::Stop => 1,
        PlaybackEndBehavior::LoopCurrent => 2,
        PlaybackEndBehavior::LoopPlaylist => 3,
    }
}

fn playback_end_behavior_from_index(index: i32) -> PlaybackEndBehavior {
    match index {
        1 => PlaybackEndBehavior::Stop,
        2 => PlaybackEndBehavior::LoopCurrent,
        3 => PlaybackEndBehavior::LoopPlaylist,
        _ => PlaybackEndBehavior::PlayNext,
    }
}

pub struct SettingsController {
    baseline: AppConfig,
    draft: SettingsDraft,
    section_index: i32,
    capture_action: Option<ShortcutAction>,
    status_message: String,
}

impl SettingsController {
    pub fn new(config: AppConfig) -> Self {
        let draft = SettingsDraft::from_config(&config);
        Self {
            baseline: config,
            draft,
            section_index: 0,
            capture_action: None,
            status_message: String::new(),
        }
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        let conflicts = self.draft.conflict_messages();
        let dirty = self.draft != SettingsDraft::from_config(&self.baseline);
        let can_apply = dirty && conflicts.is_empty() && self.draft.to_config().is_ok();

        let shortcut_rows = ShortcutAction::all()
            .iter()
            .copied()
            .map(|action| SettingsShortcutRow {
                action,
                action_label: action.label(),
                binding_label: self
                    .draft
                    .shortcuts
                    .get(&action)
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| "Unbound".to_string()),
                conflict_message: conflicts.get(&action).cloned(),
                is_capturing: self.capture_action == Some(action),
            })
            .collect();

        SettingsSnapshot {
            section_index: self.section_index,
            default_speed: self.draft.default_speed,
            default_volume_percent: self.draft.default_volume_percent,
            playback_end_behavior_index: playback_end_behavior_index(
                self.draft.playback_end_behavior,
            ),
            prefer_hardware_decode: self.draft.prefer_hardware_decode,
            remember_history: self.draft.remember_history,
            show_playlist_on_startup: self.draft.show_playlist_on_startup,
            dirty,
            can_apply,
            status_message: self.status_message.clone(),
            shortcut_rows,
        }
    }

    pub fn set_section(&mut self, index: i32) {
        self.section_index = index.clamp(0, 2);
    }

    pub fn set_default_speed(&mut self, speed: f32) {
        self.draft.default_speed = speed;
    }

    pub fn set_default_volume_percent(&mut self, volume: u8) {
        self.draft.default_volume_percent = volume;
    }

    pub fn set_playback_end_behavior(&mut self, value: PlaybackEndBehavior) {
        self.draft.playback_end_behavior = value;
    }

    pub fn set_playback_end_behavior_index(&mut self, index: i32) {
        self.draft.playback_end_behavior = playback_end_behavior_from_index(index);
    }

    pub fn set_prefer_hardware_decode(&mut self, value: bool) {
        self.draft.prefer_hardware_decode = value;
    }

    pub fn set_remember_history(&mut self, value: bool) {
        self.draft.remember_history = value;
    }

    pub fn set_show_playlist_on_startup(&mut self, value: bool) {
        self.draft.show_playlist_on_startup = value;
    }

    pub fn begin_shortcut_capture(&mut self, action: ShortcutAction) {
        self.capture_action = Some(action);
        self.status_message = format!("Press a new shortcut for {}", action.label());
    }

    pub fn is_capturing(&self) -> bool {
        self.capture_action.is_some()
    }

    pub fn capture_shortcut(&mut self, input: KeyboardInput) -> Result<bool, ValidationError> {
        if input.repeat {
            return Ok(false);
        }

        let Some(action) = self.capture_action else {
            return Ok(false);
        };
        let Some(gesture) = shortcut_gesture(input) else {
            return Ok(false);
        };

        Shortcut::parse(&gesture)?;
        self.draft.set_shortcut(action, Some(gesture));
        self.capture_action = None;
        self.status_message.clear();
        Ok(true)
    }

    pub fn clear_shortcut(&mut self, action: ShortcutAction) {
        self.draft.clear_shortcut(action);
    }

    pub fn restore_shortcut_default(&mut self, action: ShortcutAction) {
        self.draft.restore_shortcut_default(action);
    }

    pub fn restore_defaults(&mut self) {
        self.draft = SettingsDraft::from_config(&AppConfig::default());
        self.capture_action = None;
        self.status_message.clear();
    }

    pub fn discard_changes(&mut self) {
        self.draft = SettingsDraft::from_config(&self.baseline);
        self.capture_action = None;
        self.status_message.clear();
    }

    pub fn save(&mut self, path: &Path) -> Result<AppConfig, AppError> {
        let config = self.draft.to_config()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(StorageError::from)?;
        }
        config.save(path)?;
        self.baseline = config.clone();
        self.draft = SettingsDraft::from_config(&config);
        self.capture_action = None;
        self.status_message = "Settings saved".to_string();
        Ok(config)
    }
}
