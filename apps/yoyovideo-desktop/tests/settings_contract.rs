use tempfile::tempdir;
use yoyo_core::{AppConfig, Shortcut, ShortcutAction};
use yoyovideo_desktop::{KeyboardInput, SettingsController};

#[test]
fn save_persists_preferences_and_custom_shortcuts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.set_default_speed(1.25);
    controller.set_default_volume_percent(80);
    controller.set_prefer_hardware_decode(false);
    controller.set_remember_history(false);
    controller.set_show_playlist_on_startup(false);
    controller.begin_shortcut_capture(ShortcutAction::TogglePause);
    controller.capture_shortcut(KeyboardInput::character('p').with_ctrl()).unwrap();

    let saved = controller.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(saved.playback.default_speed, 1.25);
    assert_eq!(loaded.playback.default_speed, 1.25);
    assert_eq!(loaded.playback.default_volume_percent, 80);
    assert!(!loaded.playback.prefer_hardware_decode);
    assert!(!loaded.ui.remember_history);
    assert!(!loaded.ui.show_playlist_on_startup);
    assert_eq!(
        loaded.shortcuts.action_for(&Shortcut::parse("Ctrl+P").unwrap()),
        Some(ShortcutAction::TogglePause)
    );
}

#[test]
fn conflicting_shortcuts_stay_visible_in_the_snapshot_and_block_save() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.begin_shortcut_capture(ShortcutAction::TogglePause);
    controller.capture_shortcut(KeyboardInput::character('p').with_ctrl()).unwrap();
    controller.begin_shortcut_capture(ShortcutAction::SpeedUp);
    controller.capture_shortcut(KeyboardInput::character('p').with_ctrl()).unwrap();

    let snapshot = controller.snapshot();

    assert!(snapshot.dirty);
    assert!(!snapshot.can_apply);
    assert!(
        snapshot
            .shortcut_rows
            .iter()
            .filter_map(|row| row.conflict_message.as_ref())
            .any(|message| message.contains("Ctrl+P"))
    );

    let error = controller.save(&path).unwrap_err();
    assert!(error.to_string().contains("duplicate shortcut"));
}

#[test]
fn restore_defaults_and_row_restore_reset_the_draft() {
    let mut controller = SettingsController::new(AppConfig::default());
    controller.set_remember_history(false);
    controller.begin_shortcut_capture(ShortcutAction::TogglePause);
    controller.capture_shortcut(KeyboardInput::character('p').with_ctrl()).unwrap();
    controller.restore_shortcut_default(ShortcutAction::TogglePause);

    let after_row_restore = controller.snapshot();
    let pause_row = after_row_restore
        .shortcut_rows
        .iter()
        .find(|row| row.action == ShortcutAction::TogglePause)
        .unwrap();

    assert_eq!(pause_row.binding_label, "Space");

    controller.restore_defaults();
    let snapshot = controller.snapshot();

    assert!(snapshot.remember_history);
    assert!(snapshot.show_playlist_on_startup);
}

#[test]
fn clear_shortcut_removes_the_binding_from_the_saved_config() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.clear_shortcut(ShortcutAction::TogglePause);
    controller.save(&path).unwrap();

    let loaded = AppConfig::load(&path).unwrap();
    assert!(loaded.shortcuts.action_for(&Shortcut::parse("Space").unwrap()).is_none());
}

#[test]
fn save_persists_playback_end_behavior() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.set_playback_end_behavior(yoyo_core::PlaybackEndBehavior::LoopPlaylist);

    let saved = controller.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(saved.playback.end_behavior, yoyo_core::PlaybackEndBehavior::LoopPlaylist);
    assert_eq!(loaded.playback.end_behavior, yoyo_core::PlaybackEndBehavior::LoopPlaylist);
    assert_eq!(controller.snapshot().playback_end_behavior_index, 3);
}
