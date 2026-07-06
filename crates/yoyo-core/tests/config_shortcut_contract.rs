use yoyo_core::{
    AppConfig, MAX_DEFAULT_SPEED, MIN_DEFAULT_SPEED, Shortcut, ShortcutAction, ShortcutMap,
    ValidationError,
};

#[test]
fn config_validation_rejects_default_speed_outside_supported_range() {
    let mut config = AppConfig::default();
    config.playback.default_speed = MAX_DEFAULT_SPEED + 0.5;

    let error = config.validate().unwrap_err();

    assert!(matches!(error, ValidationError::InvalidConfig(_)));
    assert!(error.to_string().contains("default speed"));
}

#[test]
fn config_validation_rejects_default_volume_above_one_hundred() {
    let mut config = AppConfig::default();
    config.playback.default_volume_percent = 101;

    let error = config.validate().unwrap_err();

    assert!(matches!(error, ValidationError::InvalidConfig(_)));
    assert!(error.to_string().contains("default volume"));
}

#[test]
fn shortcut_map_replaces_the_previous_binding_for_an_action() {
    let mut map = ShortcutMap::default();
    map.set_binding(ShortcutAction::TogglePause, Some(Shortcut::parse("Ctrl+P").unwrap())).unwrap();

    assert_eq!(
        map.action_for(&Shortcut::parse("Ctrl+P").unwrap()),
        Some(ShortcutAction::TogglePause)
    );
    assert!(map.action_for(&Shortcut::parse("Space").unwrap()).is_none());
    assert_eq!(
        map.shortcut_for_action(ShortcutAction::TogglePause),
        Some(Shortcut::parse("Ctrl+P").unwrap())
    );
}

#[test]
fn shortcut_map_rejects_duplicate_bindings_between_actions() {
    let mut map = ShortcutMap::default();
    map.set_binding(ShortcutAction::TogglePause, Some(Shortcut::parse("Ctrl+P").unwrap())).unwrap();

    let error = map
        .set_binding(ShortcutAction::SpeedUp, Some(Shortcut::parse("Ctrl+P").unwrap()))
        .unwrap_err();

    assert!(matches!(error, ValidationError::DuplicateShortcut(_)));
}

#[test]
fn default_shortcut_lookup_matches_the_default_map() {
    let map = ShortcutMap::default();
    let action = ShortcutAction::TogglePause;
    let shortcut = action.default_shortcut().unwrap();

    assert_eq!(map.action_for(&shortcut), Some(action));
    assert!(MIN_DEFAULT_SPEED < 1.0);
}

#[test]
fn video_tool_default_shortcuts_are_registered() {
    let map = ShortcutMap::default();

    assert_eq!(
        map.action_for(&Shortcut::parse("S").unwrap()),
        Some(ShortcutAction::TakeScreenshot)
    );
    assert_eq!(
        map.action_for(&Shortcut::parse(",").unwrap()),
        Some(ShortcutAction::FrameStepBackward)
    );
    assert_eq!(
        map.action_for(&Shortcut::parse(".").unwrap()),
        Some(ShortcutAction::FrameStepForward)
    );
}

#[test]
fn default_playback_end_behavior_is_play_next() {
    let config = AppConfig::default();

    assert_eq!(config.playback.end_behavior, yoyo_core::PlaybackEndBehavior::PlayNext);
}

#[test]
fn default_ui_preferences_keep_compact_player_surface() {
    let config = AppConfig::default();

    assert!(config.ui.remember_history);
    assert!(!config.ui.show_playlist_on_startup);
}

#[test]
fn legacy_config_without_playback_end_behavior_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[playback]
default_speed = 1.25
default_volume_percent = 80
prefer_hardware_decode = true

[ui]
remember_history = true
show_playlist_on_startup = false

[shortcuts.bindings]
"#,
    )
    .unwrap();

    let config = AppConfig::load(&path).unwrap();

    assert_eq!(config.playback.default_speed, 1.25);
    assert_eq!(config.playback.default_volume_percent, 80);
    assert_eq!(config.playback.end_behavior, yoyo_core::PlaybackEndBehavior::PlayNext);
    assert!(!config.ui.show_playlist_on_startup);
}

#[test]
fn cinema_deck_shortcuts_are_registered() {
    let map = yoyo_core::ShortcutMap::default();

    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("M").unwrap()),
        Some(yoyo_core::ShortcutAction::ToggleMute)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("J").unwrap()),
        Some(yoyo_core::ShortcutAction::JumpToTime)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("Ctrl+M").unwrap()),
        Some(yoyo_core::ShortcutAction::AddMarker)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("P").unwrap()),
        Some(yoyo_core::ShortcutAction::OpenActionPanel)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("Shift+Right").unwrap()),
        Some(yoyo_core::ShortcutAction::NextChapterOrMarker)
    );
    assert_eq!(
        map.action_for(&yoyo_core::Shortcut::parse("Shift+Left").unwrap()),
        Some(yoyo_core::ShortcutAction::PreviousChapterOrMarker)
    );
}
