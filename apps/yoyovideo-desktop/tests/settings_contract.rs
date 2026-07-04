use tempfile::tempdir;
use yoyo_core::ShortcutAction;
use yoyovideo_desktop::SettingsController;

#[test]
fn updating_shortcut_persists_to_config_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::default();
    controller
        .update_shortcut("Ctrl+P", ShortcutAction::TogglePause)
        .unwrap();
    controller.save(&path).unwrap();

    let saved = std::fs::read_to_string(path).unwrap();
    assert!(saved.contains("Ctrl+P"));
}

#[test]
fn duplicate_shortcut_is_rejected() {
    let mut controller = SettingsController::default();
    controller
        .update_shortcut("Space", ShortcutAction::TogglePause)
        .unwrap();

    let error = controller
        .update_shortcut("Space", ShortcutAction::SpeedUp)
        .unwrap_err();

    assert!(error.to_string().contains("duplicate"));
}
