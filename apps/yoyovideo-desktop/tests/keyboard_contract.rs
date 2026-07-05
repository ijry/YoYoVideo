use yoyovideo_desktop::{DesktopKey, KeyboardInput, shortcut_allowed, shortcut_gesture};

#[test]
fn keyboard_input_maps_to_existing_shortcut_gestures() {
    assert_eq!(shortcut_gesture(KeyboardInput::pressed(DesktopKey::Space)), Some("Space"));
    assert_eq!(shortcut_gesture(KeyboardInput::pressed(DesktopKey::Right)), Some("Right"));
    assert_eq!(
        shortcut_gesture(KeyboardInput {
            key: DesktopKey::A,
            ctrl: true,
            repeat: false,
            pressed: true,
        }),
        Some("Ctrl+A")
    );
}

#[test]
fn key_release_is_ignored() {
    assert_eq!(
        shortcut_gesture(KeyboardInput {
            key: DesktopKey::Space,
            ctrl: false,
            repeat: false,
            pressed: false,
        }),
        None
    );
}

#[test]
fn url_focus_suppresses_player_shortcuts() {
    assert!(shortcut_allowed(false));
    assert!(!shortcut_allowed(true));
}
