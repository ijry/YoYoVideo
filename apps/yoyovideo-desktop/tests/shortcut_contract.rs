use yoyo_core::{AppCommand, FrameStepDirection, Shortcut, ShortcutAction, ShortcutMap};
use yoyovideo_desktop::{ShortcutDispatch, dispatch_shortcut, resolve_shortcut};

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

#[test]
fn video_tool_shortcuts_resolve_to_expected_dispatches() {
    let map = ShortcutMap::default();

    assert_eq!(resolve_shortcut(&map, "S"), Some(ShortcutDispatch::TakeScreenshot));
    assert_eq!(
        resolve_shortcut(&map, ","),
        Some(ShortcutDispatch::Command(AppCommand::StepFrame(FrameStepDirection::Previous)))
    );
    assert_eq!(
        resolve_shortcut(&map, "."),
        Some(ShortcutDispatch::Command(AppCommand::StepFrame(FrameStepDirection::Next)))
    );
}

#[test]
fn legacy_dispatch_shortcut_returns_none_for_screenshot_requiring_desktop_path() {
    let map = ShortcutMap::default();

    assert_eq!(dispatch_shortcut(&map, "S"), None);
}

#[test]
fn cinema_deck_shortcuts_resolve_to_dispatches() {
    let map = yoyo_core::ShortcutMap::default();

    assert_eq!(
        yoyovideo_desktop::dispatch_shortcut(&map, "M"),
        Some(yoyo_core::AppCommand::ToggleMute)
    );
    assert_eq!(
        yoyovideo_desktop::dispatch_shortcut(&map, "Ctrl+M"),
        Some(yoyo_core::AppCommand::AddMarkerAtCurrentPosition { created_at: "shortcut".into() })
    );
    assert_eq!(
        yoyovideo_desktop::dispatch_shortcut(&map, "Shift+Right"),
        Some(yoyo_core::AppCommand::SeekToNextChapterOrMarker)
    );
}
