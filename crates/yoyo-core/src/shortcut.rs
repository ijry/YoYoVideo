use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ShortcutAction {
    TogglePause,
    SeekBackwardSmall,
    SeekForwardSmall,
    VolumeUp,
    VolumeDown,
    SpeedDown,
    SpeedUp,
    ResetSpeed,
    SetABLoopPointA,
    SetABLoopPointB,
    ClearABLoop,
    RotateClockwise,
    ZoomOut,
    ZoomIn,
    CycleAudioChannel,
    ToggleFullscreen,
    OpenFile,
    OpenUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Shortcut(String);

impl Shortcut {
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            return Err(ValidationError::InvalidShortcut(input.to_string()));
        }
        Ok(Self(normalized.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutMap {
    pub bindings: BTreeMap<Shortcut, ShortcutAction>,
}

impl Default for ShortcutMap {
    fn default() -> Self {
        let mut bindings = BTreeMap::new();
        bindings.insert(Shortcut("Space".into()), ShortcutAction::TogglePause);
        bindings.insert(Shortcut("Left".into()), ShortcutAction::SeekBackwardSmall);
        bindings.insert(Shortcut("Right".into()), ShortcutAction::SeekForwardSmall);
        bindings.insert(Shortcut("Up".into()), ShortcutAction::VolumeUp);
        bindings.insert(Shortcut("Down".into()), ShortcutAction::VolumeDown);
        bindings.insert(Shortcut("[".into()), ShortcutAction::SpeedDown);
        bindings.insert(Shortcut("]".into()), ShortcutAction::SpeedUp);
        bindings.insert(Shortcut("0".into()), ShortcutAction::ResetSpeed);
        bindings.insert(Shortcut("A".into()), ShortcutAction::SetABLoopPointA);
        bindings.insert(Shortcut("B".into()), ShortcutAction::SetABLoopPointB);
        bindings.insert(Shortcut("Ctrl+A".into()), ShortcutAction::ClearABLoop);
        bindings.insert(Shortcut("R".into()), ShortcutAction::RotateClockwise);
        bindings.insert(Shortcut("Z".into()), ShortcutAction::ZoomOut);
        bindings.insert(Shortcut("X".into()), ShortcutAction::ZoomIn);
        bindings.insert(Shortcut("C".into()), ShortcutAction::CycleAudioChannel);
        bindings.insert(Shortcut("F".into()), ShortcutAction::ToggleFullscreen);
        bindings.insert(Shortcut("O".into()), ShortcutAction::OpenFile);
        bindings.insert(Shortcut("U".into()), ShortcutAction::OpenUrl);
        Self { bindings }
    }
}

impl ShortcutMap {
    pub fn action_for(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        self.bindings.get(shortcut).copied()
    }
}
