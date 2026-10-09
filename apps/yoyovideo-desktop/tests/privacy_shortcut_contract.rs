use yoyo_core::{Shortcut, ShortcutAction, ShortcutMap};
use yoyovideo_desktop::{ShortcutDispatch, resolve_shortcut};
#[test]
fn privacy_action_is_configurable_without_stealing_an_existing_default() {
    assert!(ShortcutAction::all().contains(&ShortcutAction::TogglePrivacy));
    assert!(ShortcutAction::TogglePrivacy.default_shortcut().is_none());
    let mut map = ShortcutMap::default();
    map.set_binding(ShortcutAction::TogglePrivacy, Some(Shortcut::parse("Ctrl+Shift+P").unwrap()))
        .unwrap();
    assert_eq!(resolve_shortcut(&map, "Ctrl+Shift+P"), Some(ShortcutDispatch::TogglePrivacy));
    assert_eq!(
        resolve_shortcut(&map, "Space"),
        Some(ShortcutDispatch::Command(yoyo_core::AppCommand::TogglePause))
    );
}
