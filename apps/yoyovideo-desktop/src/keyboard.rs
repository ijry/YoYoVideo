#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedDesktopKey {
    Space,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopKey {
    Named(NamedDesktopKey),
    Character(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInput {
    pub key: DesktopKey,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub repeat: bool,
    pub pressed: bool,
}

impl KeyboardInput {
    pub fn named(key: NamedDesktopKey) -> Self {
        Self {
            key: DesktopKey::Named(key),
            ctrl: false,
            alt: false,
            shift: false,
            repeat: false,
            pressed: true,
        }
    }

    pub fn character(key: char) -> Self {
        Self {
            key: DesktopKey::Character(key),
            ctrl: false,
            alt: false,
            shift: false,
            repeat: false,
            pressed: true,
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }
}

pub fn shortcut_allowed(url_focused: bool) -> bool {
    !url_focused
}

fn normalized_key_name(key: DesktopKey) -> Option<String> {
    match key {
        DesktopKey::Named(NamedDesktopKey::Space) => Some("Space".to_string()),
        DesktopKey::Named(NamedDesktopKey::Left) => Some("Left".to_string()),
        DesktopKey::Named(NamedDesktopKey::Right) => Some("Right".to_string()),
        DesktopKey::Named(NamedDesktopKey::Up) => Some("Up".to_string()),
        DesktopKey::Named(NamedDesktopKey::Down) => Some("Down".to_string()),
        DesktopKey::Character(ch) if !ch.is_control() => {
            let normalized = if ch.is_ascii_alphabetic() { ch.to_ascii_uppercase() } else { ch };
            Some(normalized.to_string())
        }
        _ => None,
    }
}

pub fn shortcut_gesture(input: KeyboardInput) -> Option<String> {
    if !input.pressed {
        return None;
    }

    let key = normalized_key_name(input.key)?;
    let mut parts = Vec::new();
    if input.ctrl {
        parts.push("Ctrl".to_string());
    }
    if input.alt {
        parts.push("Alt".to_string());
    }
    if input.shift {
        parts.push("Shift".to_string());
    }
    parts.push(key);
    Some(parts.join("+"))
}

pub mod winit_adapter {
    use slint::winit_030::winit::{
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{Key, ModifiersState, NamedKey},
    };

    use super::{DesktopKey, KeyboardInput, NamedDesktopKey};

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
                Key::Named(NamedKey::Space) => DesktopKey::Named(NamedDesktopKey::Space),
                Key::Named(NamedKey::ArrowLeft) => DesktopKey::Named(NamedDesktopKey::Left),
                Key::Named(NamedKey::ArrowRight) => DesktopKey::Named(NamedDesktopKey::Right),
                Key::Named(NamedKey::ArrowUp) => DesktopKey::Named(NamedDesktopKey::Up),
                Key::Named(NamedKey::ArrowDown) => DesktopKey::Named(NamedDesktopKey::Down),
                Key::Character(value) => {
                    let mut chars = value.chars();
                    let ch = chars.next()?;
                    if chars.next().is_some() {
                        return None;
                    }
                    DesktopKey::Character(ch)
                }
                _ => return None,
            };

            Some(KeyboardInput {
                key,
                ctrl: self.modifiers.control_key(),
                alt: self.modifiers.alt_key(),
                shift: self.modifiers.shift_key(),
                repeat: event.repeat,
                pressed: event.state == ElementState::Pressed,
            })
        }
    }
}
