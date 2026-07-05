#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopKey {
    Space,
    Left,
    Right,
    Up,
    Down,
    LeftBracket,
    RightBracket,
    Digit0,
    A,
    B,
    R,
    Z,
    X,
    C,
    F,
    O,
    U,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInput {
    pub key: DesktopKey,
    pub ctrl: bool,
    pub repeat: bool,
    pub pressed: bool,
}

impl KeyboardInput {
    pub fn pressed(key: DesktopKey) -> Self {
        Self { key, ctrl: false, repeat: false, pressed: true }
    }
}

pub fn shortcut_allowed(url_focused: bool) -> bool {
    !url_focused
}

pub fn shortcut_gesture(input: KeyboardInput) -> Option<&'static str> {
    if !input.pressed {
        return None;
    }

    match (input.key, input.ctrl) {
        (DesktopKey::Space, false) => Some("Space"),
        (DesktopKey::Left, false) => Some("Left"),
        (DesktopKey::Right, false) => Some("Right"),
        (DesktopKey::Up, false) => Some("Up"),
        (DesktopKey::Down, false) => Some("Down"),
        (DesktopKey::LeftBracket, false) => Some("["),
        (DesktopKey::RightBracket, false) => Some("]"),
        (DesktopKey::Digit0, false) => Some("0"),
        (DesktopKey::A, false) => Some("A"),
        (DesktopKey::B, false) => Some("B"),
        (DesktopKey::A, true) => Some("Ctrl+A"),
        (DesktopKey::R, false) => Some("R"),
        (DesktopKey::Z, false) => Some("Z"),
        (DesktopKey::X, false) => Some("X"),
        (DesktopKey::C, false) => Some("C"),
        (DesktopKey::F, false) => Some("F"),
        (DesktopKey::O, false) => Some("O"),
        (DesktopKey::U, false) => Some("U"),
        _ => None,
    }
}

pub mod winit_adapter {
    use slint::winit_030::winit::{
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{Key, ModifiersState, NamedKey},
    };

    use super::{DesktopKey, KeyboardInput};

    #[derive(Debug, Default, Clone, Copy)]
    pub struct WinitKeyboardState {
        modifiers: ModifiersState,
    }

    impl WinitKeyboardState {
        pub fn update(&mut self, event: &WindowEvent) -> Option<KeyboardInput> {
            match event {
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = modifiers.state();
                    None
                }
                WindowEvent::KeyboardInput { event, .. } => self.map_key_event(event),
                _ => None,
            }
        }

        fn map_key_event(&self, event: &KeyEvent) -> Option<KeyboardInput> {
            let key = match &event.logical_key {
                Key::Named(NamedKey::Space) => DesktopKey::Space,
                Key::Named(NamedKey::ArrowLeft) => DesktopKey::Left,
                Key::Named(NamedKey::ArrowRight) => DesktopKey::Right,
                Key::Named(NamedKey::ArrowUp) => DesktopKey::Up,
                Key::Named(NamedKey::ArrowDown) => DesktopKey::Down,
                Key::Character(value) if value == "[" => DesktopKey::LeftBracket,
                Key::Character(value) if value == "]" => DesktopKey::RightBracket,
                Key::Character(value) if value == "0" => DesktopKey::Digit0,
                Key::Character(value) if value.eq_ignore_ascii_case("a") => DesktopKey::A,
                Key::Character(value) if value.eq_ignore_ascii_case("b") => DesktopKey::B,
                Key::Character(value) if value.eq_ignore_ascii_case("r") => DesktopKey::R,
                Key::Character(value) if value.eq_ignore_ascii_case("z") => DesktopKey::Z,
                Key::Character(value) if value.eq_ignore_ascii_case("x") => DesktopKey::X,
                Key::Character(value) if value.eq_ignore_ascii_case("c") => DesktopKey::C,
                Key::Character(value) if value.eq_ignore_ascii_case("f") => DesktopKey::F,
                Key::Character(value) if value.eq_ignore_ascii_case("o") => DesktopKey::O,
                Key::Character(value) if value.eq_ignore_ascii_case("u") => DesktopKey::U,
                _ => return None,
            };

            Some(KeyboardInput {
                key,
                ctrl: self.modifiers.control_key(),
                repeat: event.repeat,
                pressed: event.state == ElementState::Pressed,
            })
        }
    }
}
