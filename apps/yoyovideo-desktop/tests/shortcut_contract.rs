use yoyo_core::{AppCommand, Shortcut, ShortcutAction, ShortcutMap};
use yoyovideo_desktop::dispatch_shortcut;

#[test]
fn control_a_clears_ab_loop() {
    let map = ShortcutMap::default();
    let command = dispatch_shortcut(&map, Shortcut::parse("Ctrl+A").unwrap().as_str());

    assert_eq!(command, Some(AppCommand::ClearABLoop));
}

#[test]
fn right_arrow_seeks_forward() {
    let map = ShortcutMap::default();
    let command = dispatch_shortcut(&map, Shortcut::parse("Right").unwrap().as_str());

    assert_eq!(command, Some(AppCommand::SeekRelative(5.0)));
}

#[test]
fn custom_shortcut_binding_dispatches_through_the_same_command_path() {
    let mut map = ShortcutMap::default();
    map.set_binding(ShortcutAction::TogglePause, Some(Shortcut::parse("Ctrl+P").unwrap())).unwrap();

    assert_eq!(dispatch_shortcut(&map, "Ctrl+P"), Some(AppCommand::TogglePause));
    assert_eq!(dispatch_shortcut(&map, "Space"), None);
}
