use yoyo_core::{AppCommand, Shortcut, ShortcutMap};
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
