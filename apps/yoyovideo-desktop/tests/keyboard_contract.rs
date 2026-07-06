use yoyovideo_desktop::{KeyboardInput, NamedDesktopKey, shortcut_allowed, shortcut_gesture};

#[test]
fn keyboard_input_normalizes_named_and_character_shortcuts() {
    assert_eq!(
        shortcut_gesture(KeyboardInput::named(NamedDesktopKey::Space)),
        Some("Space".to_string())
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput::named(NamedDesktopKey::Right)),
        Some("Right".to_string())
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput::character('p').with_ctrl()),
        Some("Ctrl+P".to_string())
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput::character('u').with_ctrl().with_shift()),
        Some("Ctrl+Shift+U".to_string())
    );
    assert_eq!(shortcut_gesture(KeyboardInput::character('[')), Some("[".to_string()));
}

#[test]
fn key_release_is_ignored() {
    let mut input = KeyboardInput::named(NamedDesktopKey::Space);
    input.pressed = false;

    assert_eq!(shortcut_gesture(input), None);
}

#[test]
fn url_focus_still_suppresses_player_shortcuts() {
    assert!(shortcut_allowed(false));
    assert!(!shortcut_allowed(true));
}

#[test]
fn keyboard_input_normalizes_video_tool_shortcuts() {
    assert_eq!(shortcut_gesture(KeyboardInput::character('s')), Some("S".to_string()));
    assert_eq!(shortcut_gesture(KeyboardInput::character(',')), Some(",".to_string()));
    assert_eq!(shortcut_gesture(KeyboardInput::character('.')), Some(".".to_string()));
}

#[test]
fn keyboard_input_normalizes_shifted_chapter_marker_shortcuts() {
    assert_eq!(
        shortcut_gesture(KeyboardInput::named(NamedDesktopKey::Right).with_shift()),
        Some("Shift+Right".to_string())
    );
    assert_eq!(
        shortcut_gesture(KeyboardInput::named(NamedDesktopKey::Left).with_shift()),
        Some("Shift+Left".to_string())
    );
}
