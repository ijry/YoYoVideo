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
