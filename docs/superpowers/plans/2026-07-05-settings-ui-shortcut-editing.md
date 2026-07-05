# Settings UI And Shortcut Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dedicated settings window to YoYoVideo that edits all current `AppConfig` fields, supports key-capture shortcut rebinding with conflict detection, persists to `config.toml`, and applies runtime-safe changes immediately without disturbing the current playback session.

**Architecture:** Keep persistent config rules and shortcut defaults in `yoyo-core`, refactor desktop keyboard routing to generate normalized gesture strings instead of a fixed built-in key list, and move settings editing into a pure draft-based `SettingsController` that the Slint window drives through callbacks. The desktop runtime continues to own live playback and history state; successful saves write the config first, then update shortcut routing and history behavior in memory.

**Tech Stack:** Rust 2024, Slint 1.17.0, `directories` 6.0, `rfd` 0.17, existing `yoyo-core` / `yoyovideo-desktop` crates, PowerShell verification commands.

## Global Constraints

- The settings surface is a dedicated window, not a sidebar tab or full-window overlay.
- Settings use explicit save semantics: `Apply`, `OK`, and `Cancel`.
- Expose every current `AppConfig` field for editing:
  - `playback.default_speed`
  - `playback.default_volume_percent`
  - `playback.prefer_hardware_decode`
  - `ui.remember_history`
  - `ui.show_playlist_on_startup`
  - `shortcuts`
- Each action has at most one shortcut, and each shortcut can belong to at most one action.
- Shortcut editing uses direct key capture instead of free-form text as the primary input.
- Shortcut conflicts block saving instead of silently reassigning bindings.
- `default_speed` must stay within `0.25x..=4.0x`.
- `default_volume_percent` must stay within `0..=100`.
- Settings that are safe to apply immediately do so after save; playback defaults only affect future playback state and future app launches.
- Turning off `remember_history` stops future history writes but does not delete the existing history file.
- Changing `show_playlist_on_startup` does not override the sidebar visibility the user already has in the current window.
- Changing default speed or volume affects newly created playback defaults, not the currently loaded media item.
- Changing hardware decode preference is stored for later runtime initialization rather than rebuilding the active backend in place.
- Save ordering is strict: validate the draft, write the config file, then apply runtime config updates.
- Slint stays declarative and receives simple view-model properties plus callbacks.

---

## File Structure

- `crates/yoyo-core/src/error.rs`: add config-validation and duplicate-shortcut error variants.
- `crates/yoyo-core/src/config.rs`: add speed constants and config validation helpers.
- `crates/yoyo-core/src/shortcut.rs`: add ordered action enumeration, labels, default-shortcut lookup, and action-to-shortcut helpers needed by the desktop settings draft.
- `crates/yoyo-core/src/lib.rs`: export the new config constants and shortcut helpers.
- `crates/yoyo-core/tests/config_shortcut_contract.rs`: regression coverage for config validation and shortcut editing semantics.
- `apps/yoyovideo-desktop/src/keyboard.rs`: replace the hardcoded shortcut key mapping with normalized named/character gesture generation suitable for custom bindings and capture.
- `apps/yoyovideo-desktop/src/settings_controller.rs`: own the draft model, dirty tracking, row-level conflict state, capture state, restore-default behavior, and persistence flow.
- `apps/yoyovideo-desktop/src/history_runtime.rs`: add runtime enable/disable behavior for `remember_history`.
- `apps/yoyovideo-desktop/src/app.rs`: export `SettingsWindow`, create/show the settings window, wire callbacks, refresh window properties, load validated startup config, and apply saved settings to live runtime state.
- `apps/yoyovideo-desktop/src/lib.rs`: export any new settings and window types needed by tests.
- `apps/yoyovideo-desktop/ui/main-window.slint`: add an exported `SettingsWindow` component plus row structs and callbacks.
- `apps/yoyovideo-desktop/tests/keyboard_contract.rs`: cover generic gesture normalization.
- `apps/yoyovideo-desktop/tests/shortcut_contract.rs`: verify custom bindings still dispatch through `dispatch_shortcut`.
- `apps/yoyovideo-desktop/tests/settings_contract.rs`: cover draft behavior, capture, conflict blocking, restore defaults, and persistence.
- `apps/yoyovideo-desktop/tests/settings_window_contract.rs`: compile-level contract for the exported Slint settings window.
- `apps/yoyovideo-desktop/tests/settings_runtime_contract.rs`: cover runtime shortcut replacement and history-enable toggling.
- `docs/testing/manual-smoke-checklist.md`: add settings-window and shortcut-editing smoke coverage.

---

### Task 1: Core Config Validation And Shortcut Editing Helpers

**Files:**
- Create: `crates/yoyo-core/tests/config_shortcut_contract.rs`
- Modify: `crates/yoyo-core/src/error.rs`
- Modify: `crates/yoyo-core/src/config.rs`
- Modify: `crates/yoyo-core/src/shortcut.rs`
- Modify: `crates/yoyo-core/src/lib.rs`

**Interfaces:**
- Produces: `pub const MIN_DEFAULT_SPEED: f32 = 0.25`
- Produces: `pub const MAX_DEFAULT_SPEED: f32 = 4.0`
- Produces: `ValidationError::InvalidConfig(String)`
- Produces: `ValidationError::DuplicateShortcut(String)`
- Produces: `AppConfig::validate(&self) -> Result<(), ValidationError>`
- Produces: `ShortcutAction::all() -> &'static [ShortcutAction]`
- Produces: `ShortcutAction::label(self) -> &'static str`
- Produces: `ShortcutAction::default_shortcut(self) -> Option<Shortcut>`
- Produces: `ShortcutMap::shortcut_for_action(&self, action: ShortcutAction) -> Option<Shortcut>`
- Produces: `ShortcutMap::set_binding(&mut self, action: ShortcutAction, shortcut: Option<Shortcut>) -> Result<(), ValidationError>`

- [ ] **Step 1: Write the failing core config/shortcut tests**

Create `crates/yoyo-core/tests/config_shortcut_contract.rs`:

```rust
use yoyo_core::{
    AppConfig, Shortcut, ShortcutAction, ShortcutMap, ValidationError, MAX_DEFAULT_SPEED,
    MIN_DEFAULT_SPEED,
};

#[test]
fn config_validation_rejects_default_speed_outside_supported_range() {
    let mut config = AppConfig::default();
    config.playback.default_speed = MAX_DEFAULT_SPEED + 0.5;

    let error = config.validate().unwrap_err();

    assert!(matches!(error, ValidationError::InvalidConfig(_)));
    assert!(error.to_string().contains("default speed"));
}

#[test]
fn config_validation_rejects_default_volume_above_one_hundred() {
    let mut config = AppConfig::default();
    config.playback.default_volume_percent = 101;

    let error = config.validate().unwrap_err();

    assert!(matches!(error, ValidationError::InvalidConfig(_)));
    assert!(error.to_string().contains("default volume"));
}

#[test]
fn shortcut_map_replaces_the_previous_binding_for_an_action() {
    let mut map = ShortcutMap::default();
    map.set_binding(
        ShortcutAction::TogglePause,
        Some(Shortcut::parse("Ctrl+P").unwrap()),
    )
    .unwrap();

    assert_eq!(
        map.action_for(&Shortcut::parse("Ctrl+P").unwrap()),
        Some(ShortcutAction::TogglePause)
    );
    assert!(map.action_for(&Shortcut::parse("Space").unwrap()).is_none());
    assert_eq!(
        map.shortcut_for_action(ShortcutAction::TogglePause),
        Some(Shortcut::parse("Ctrl+P").unwrap())
    );
}

#[test]
fn shortcut_map_rejects_duplicate_bindings_between_actions() {
    let mut map = ShortcutMap::default();
    map.set_binding(
        ShortcutAction::TogglePause,
        Some(Shortcut::parse("Ctrl+P").unwrap()),
    )
    .unwrap();

    let error = map
        .set_binding(
            ShortcutAction::SpeedUp,
            Some(Shortcut::parse("Ctrl+P").unwrap()),
        )
        .unwrap_err();

    assert!(matches!(error, ValidationError::DuplicateShortcut(_)));
}

#[test]
fn default_shortcut_lookup_matches_the_default_map() {
    let map = ShortcutMap::default();
    let action = ShortcutAction::TogglePause;
    let shortcut = action.default_shortcut().unwrap();

    assert_eq!(map.action_for(&shortcut), Some(action));
    assert!(MIN_DEFAULT_SPEED < 1.0);
}
```

- [ ] **Step 2: Run the failing core config/shortcut tests**

Run:

```powershell
cargo test -p yoyo-core --test config_shortcut_contract
```

Expected: FAIL because the validation constants, error variants, and shortcut helper methods do not exist yet.

- [ ] **Step 3: Add config validation error variants**

Modify `crates/yoyo-core/src/error.rs`:

```rust
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("unsupported url scheme: {0}")]
    UnsupportedUrlScheme(String),
    #[error("unsupported media path: {0}")]
    UnsupportedMediaPath(PathBuf),
    #[error("invalid shortcut: {0}")]
    InvalidShortcut(String),
    #[error("duplicate shortcut: {0}")]
    DuplicateShortcut(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}
```

- [ ] **Step 4: Add config validation constants and `AppConfig::validate()`**

Modify `crates/yoyo-core/src/config.rs`:

```rust
pub const MIN_DEFAULT_SPEED: f32 = 0.25;
pub const MAX_DEFAULT_SPEED: f32 = 4.0;

impl AppConfig {
    pub fn validate(&self) -> Result<(), StorageError> {
        todo!()
    }
}
```

Then replace the temporary signature with the real implementation:

```rust
use crate::{ShortcutMap, StorageError, ValidationError};

impl AppConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !self.playback.default_speed.is_finite()
            || self.playback.default_speed < MIN_DEFAULT_SPEED
            || self.playback.default_speed > MAX_DEFAULT_SPEED
        {
            return Err(ValidationError::InvalidConfig(format!(
                "default speed must be within {MIN_DEFAULT_SPEED:.2}x..={MAX_DEFAULT_SPEED:.2}x"
            )));
        }

        if self.playback.default_volume_percent > 100 {
            return Err(ValidationError::InvalidConfig(
                "default volume must be within 0..=100".to_string(),
            ));
        }

        Ok(())
    }
}
```

- [ ] **Step 5: Add ordered shortcut metadata and action-binding helpers**

Modify `crates/yoyo-core/src/shortcut.rs`:

```rust
impl ShortcutAction {
    pub fn all() -> &'static [ShortcutAction] {
        const ACTIONS: [ShortcutAction; 18] = [
            ShortcutAction::TogglePause,
            ShortcutAction::SeekBackwardSmall,
            ShortcutAction::SeekForwardSmall,
            ShortcutAction::VolumeUp,
            ShortcutAction::VolumeDown,
            ShortcutAction::SpeedDown,
            ShortcutAction::SpeedUp,
            ShortcutAction::ResetSpeed,
            ShortcutAction::SetABLoopPointA,
            ShortcutAction::SetABLoopPointB,
            ShortcutAction::ClearABLoop,
            ShortcutAction::RotateClockwise,
            ShortcutAction::ZoomOut,
            ShortcutAction::ZoomIn,
            ShortcutAction::CycleAudioChannel,
            ShortcutAction::ToggleFullscreen,
            ShortcutAction::OpenFile,
            ShortcutAction::OpenUrl,
        ];

        &ACTIONS
    }

    pub fn label(self) -> &'static str {
        match self {
            ShortcutAction::TogglePause => "Play / Pause",
            ShortcutAction::SeekBackwardSmall => "Seek Backward 5s",
            ShortcutAction::SeekForwardSmall => "Seek Forward 5s",
            ShortcutAction::VolumeUp => "Volume Up",
            ShortcutAction::VolumeDown => "Volume Down",
            ShortcutAction::SpeedDown => "Speed Down",
            ShortcutAction::SpeedUp => "Speed Up",
            ShortcutAction::ResetSpeed => "Reset Speed",
            ShortcutAction::SetABLoopPointA => "Set A-B Point A",
            ShortcutAction::SetABLoopPointB => "Set A-B Point B",
            ShortcutAction::ClearABLoop => "Clear A-B Loop",
            ShortcutAction::RotateClockwise => "Rotate Clockwise",
            ShortcutAction::ZoomOut => "Zoom Out",
            ShortcutAction::ZoomIn => "Zoom In",
            ShortcutAction::CycleAudioChannel => "Cycle Audio Channel",
            ShortcutAction::ToggleFullscreen => "Toggle Fullscreen",
            ShortcutAction::OpenFile => "Open File",
            ShortcutAction::OpenUrl => "Open URL",
        }
    }

    pub fn default_shortcut(self) -> Option<Shortcut> {
        ShortcutMap::default().shortcut_for_action(self)
    }
}

impl ShortcutMap {
    pub fn shortcut_for_action(&self, action: ShortcutAction) -> Option<Shortcut> {
        self.bindings
            .iter()
            .find_map(|(shortcut, mapped)| (*mapped == action).then_some(shortcut.clone()))
    }

    pub fn set_binding(
        &mut self,
        action: ShortcutAction,
        shortcut: Option<Shortcut>,
    ) -> Result<(), ValidationError> {
        self.bindings.retain(|_, mapped| *mapped != action);

        let Some(shortcut) = shortcut else {
            return Ok(());
        };

        if let Some(existing) = self.bindings.get(&shortcut) {
            if *existing != action {
                return Err(ValidationError::DuplicateShortcut(shortcut.as_str().to_string()));
            }
        }

        self.bindings.insert(shortcut, action);
        Ok(())
    }
}
```

- [ ] **Step 6: Export the new config constants**

Modify `crates/yoyo-core/src/lib.rs`:

```rust
pub use config::{
    AppConfig, PlaybackDefaults, UiPreferences, MAX_DEFAULT_SPEED, MIN_DEFAULT_SPEED,
};
```

- [ ] **Step 7: Run the core config/shortcut tests again**

Run:

```powershell
cargo test -p yoyo-core --test config_shortcut_contract
```

Expected: PASS for config range validation and shortcut editing helpers.

- [ ] **Step 8: Commit**

Run:

```powershell
git add crates/yoyo-core/src/error.rs crates/yoyo-core/src/config.rs crates/yoyo-core/src/shortcut.rs crates/yoyo-core/src/lib.rs crates/yoyo-core/tests/config_shortcut_contract.rs
git commit -m "feat: add config and shortcut validation helpers"
```

Expected: Commit succeeds.

---

### Task 2: Generic Keyboard Gesture Normalization For Custom Shortcuts

**Files:**
- Modify: `apps/yoyovideo-desktop/src/keyboard.rs`
- Modify: `apps/yoyovideo-desktop/tests/keyboard_contract.rs`
- Modify: `apps/yoyovideo-desktop/tests/shortcut_contract.rs`

**Interfaces:**
- Produces: `NamedDesktopKey`
- Produces: `DesktopKey::Named(NamedDesktopKey)`
- Produces: `DesktopKey::Character(char)`
- Produces: `KeyboardInput::named(key: NamedDesktopKey) -> Self`
- Produces: `KeyboardInput::character(key: char) -> Self`
- Produces: `KeyboardInput::with_ctrl(self) -> Self`
- Produces: `KeyboardInput::with_alt(self) -> Self`
- Produces: `KeyboardInput::with_shift(self) -> Self`
- Produces: `shortcut_gesture(input: KeyboardInput) -> Option<String>`

- [ ] **Step 1: Write the failing generic keyboard tests**

Replace `apps/yoyovideo-desktop/tests/keyboard_contract.rs` with:

```rust
use yoyovideo_desktop::{
    DesktopKey, KeyboardInput, NamedDesktopKey, shortcut_allowed, shortcut_gesture,
};

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
    assert_eq!(
        shortcut_gesture(KeyboardInput::character('[')),
        Some("[".to_string())
    );
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
```

Append to `apps/yoyovideo-desktop/tests/shortcut_contract.rs`:

```rust
use yoyo_core::ShortcutAction;

#[test]
fn custom_shortcut_binding_dispatches_through_the_same_command_path() {
    let mut map = ShortcutMap::default();
    map.set_binding(
        ShortcutAction::TogglePause,
        Some(Shortcut::parse("Ctrl+P").unwrap()),
    )
    .unwrap();

    assert_eq!(dispatch_shortcut(&map, "Ctrl+P"), Some(AppCommand::TogglePause));
    assert_eq!(dispatch_shortcut(&map, "Space"), None);
}
```

- [ ] **Step 2: Run the failing desktop keyboard tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
```

Expected: FAIL because the generic key constructors and `String`-based gesture normalization do not exist yet.

- [ ] **Step 3: Replace the fixed-key shortcut mapping with normalized gestures**

Modify `apps/yoyovideo-desktop/src/keyboard.rs`:

```rust
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
            let normalized = if ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
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
```

- [ ] **Step 4: Update the winit adapter to feed the generic key model**

Still in `apps/yoyovideo-desktop/src/keyboard.rs`, update the adapter section:

```rust
use slint::winit_030::winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
};

use super::{DesktopKey, KeyboardInput, NamedDesktopKey};
```

Replace `map_key_event()` with:

```rust
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
```

- [ ] **Step 5: Export the renamed keyboard types if needed**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
pub use keyboard::{
    DesktopKey, KeyboardInput, NamedDesktopKey, shortcut_allowed, shortcut_gesture,
};
```

- [ ] **Step 6: Run the keyboard and shortcut tests again**

Run:

```powershell
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
```

Expected: PASS. `Ctrl+P` is now normalized and dispatched through the same map lookup as the default shortcuts.

- [ ] **Step 7: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/keyboard.rs apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/keyboard_contract.rs apps/yoyovideo-desktop/tests/shortcut_contract.rs
git commit -m "feat: normalize generic keyboard shortcuts"
```

Expected: Commit succeeds.

---

### Task 3: Draft-Based Settings Controller With Conflict Visibility And Persistence

**Files:**
- Modify: `apps/yoyovideo-desktop/src/settings_controller.rs`
- Modify: `apps/yoyovideo-desktop/tests/settings_contract.rs`

**Interfaces:**
- Produces: `SettingsShortcutRow`
- Produces: `SettingsSnapshot`
- Produces: `SettingsController::new(config: AppConfig) -> Self`
- Produces: `SettingsController::snapshot(&self) -> SettingsSnapshot`
- Produces: `SettingsController::set_section(&mut self, index: i32)`
- Produces: `SettingsController::set_default_speed(&mut self, speed: f32)`
- Produces: `SettingsController::set_default_volume_percent(&mut self, volume: u8)`
- Produces: `SettingsController::set_prefer_hardware_decode(&mut self, value: bool)`
- Produces: `SettingsController::set_remember_history(&mut self, value: bool)`
- Produces: `SettingsController::set_show_playlist_on_startup(&mut self, value: bool)`
- Produces: `SettingsController::begin_shortcut_capture(&mut self, action: ShortcutAction)`
- Produces: `SettingsController::is_capturing(&self) -> bool`
- Produces: `SettingsController::capture_shortcut(&mut self, input: KeyboardInput) -> Result<bool, ValidationError>`
- Produces: `SettingsController::clear_shortcut(&mut self, action: ShortcutAction)`
- Produces: `SettingsController::restore_shortcut_default(&mut self, action: ShortcutAction)`
- Produces: `SettingsController::restore_defaults(&mut self)`
- Produces: `SettingsController::discard_changes(&mut self)`
- Produces: `SettingsController::save(&mut self, path: &Path) -> Result<AppConfig, AppError>`

- [ ] **Step 1: Replace the settings tests with draft-centric coverage**

Replace `apps/yoyovideo-desktop/tests/settings_contract.rs` with:

```rust
use tempfile::tempdir;
use yoyo_core::{AppConfig, Shortcut, ShortcutAction};
use yoyovideo_desktop::{KeyboardInput, SettingsController};

#[test]
fn save_persists_preferences_and_custom_shortcuts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.set_default_speed(1.25);
    controller.set_default_volume_percent(80);
    controller.set_prefer_hardware_decode(false);
    controller.set_remember_history(false);
    controller.set_show_playlist_on_startup(false);
    controller.begin_shortcut_capture(ShortcutAction::TogglePause);
    controller
        .capture_shortcut(KeyboardInput::character('p').with_ctrl())
        .unwrap();

    let saved = controller.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(saved.playback.default_speed, 1.25);
    assert_eq!(loaded.playback.default_speed, 1.25);
    assert_eq!(loaded.playback.default_volume_percent, 80);
    assert!(!loaded.playback.prefer_hardware_decode);
    assert!(!loaded.ui.remember_history);
    assert!(!loaded.ui.show_playlist_on_startup);
    assert_eq!(
        loaded.shortcuts.action_for(&Shortcut::parse("Ctrl+P").unwrap()),
        Some(ShortcutAction::TogglePause)
    );
}

#[test]
fn conflicting_shortcuts_stay_visible_in_the_snapshot_and_block_save() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.begin_shortcut_capture(ShortcutAction::TogglePause);
    controller
        .capture_shortcut(KeyboardInput::character('p').with_ctrl())
        .unwrap();
    controller.begin_shortcut_capture(ShortcutAction::SpeedUp);
    controller
        .capture_shortcut(KeyboardInput::character('p').with_ctrl())
        .unwrap();

    let snapshot = controller.snapshot();

    assert!(snapshot.dirty);
    assert!(!snapshot.can_apply);
    assert!(snapshot
        .shortcut_rows
        .iter()
        .filter_map(|row| row.conflict_message.as_ref())
        .any(|message| message.contains("Ctrl+P")));

    let error = controller.save(&path).unwrap_err();
    assert!(error.to_string().contains("duplicate shortcut"));
}

#[test]
fn restore_defaults_and_row_restore_reset_the_draft() {
    let mut controller = SettingsController::new(AppConfig::default());
    controller.set_remember_history(false);
    controller.begin_shortcut_capture(ShortcutAction::TogglePause);
    controller
        .capture_shortcut(KeyboardInput::character('p').with_ctrl())
        .unwrap();
    controller.restore_shortcut_default(ShortcutAction::TogglePause);

    let after_row_restore = controller.snapshot();
    let pause_row = after_row_restore
        .shortcut_rows
        .iter()
        .find(|row| row.action == ShortcutAction::TogglePause)
        .unwrap();

    assert_eq!(pause_row.binding_label, "Space");

    controller.restore_defaults();
    let snapshot = controller.snapshot();

    assert!(snapshot.remember_history);
    assert!(snapshot.show_playlist_on_startup);
}

#[test]
fn clear_shortcut_removes_the_binding_from_the_saved_config() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let mut controller = SettingsController::new(AppConfig::default());
    controller.clear_shortcut(ShortcutAction::TogglePause);
    controller.save(&path).unwrap();

    let loaded = AppConfig::load(&path).unwrap();
    assert!(loaded.shortcuts.action_for(&Shortcut::parse("Space").unwrap()).is_none());
}
```

- [ ] **Step 2: Run the failing settings tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_contract
```

Expected: FAIL because the current `SettingsController` only edits a live `AppConfig` and cannot model dirty state, capture state, conflicts, or row restore/default flows.

- [ ] **Step 3: Replace the simple controller with a draft model and view snapshot**

Replace `apps/yoyovideo-desktop/src/settings_controller.rs` with:

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{KeyboardInput, shortcut_gesture};
use yoyo_core::{AppConfig, AppError, Shortcut, ShortcutAction, ShortcutMap, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsShortcutRow {
    pub action: ShortcutAction,
    pub action_label: &'static str,
    pub binding_label: String,
    pub conflict_message: Option<String>,
    pub is_capturing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsSnapshot {
    pub section_index: i32,
    pub default_speed: f32,
    pub default_volume_percent: u8,
    pub prefer_hardware_decode: bool,
    pub remember_history: bool,
    pub show_playlist_on_startup: bool,
    pub dirty: bool,
    pub can_apply: bool,
    pub status_message: String,
    pub shortcut_rows: Vec<SettingsShortcutRow>,
}

#[derive(Debug, Clone, PartialEq)]
struct SettingsDraft {
    default_speed: f32,
    default_volume_percent: u8,
    prefer_hardware_decode: bool,
    remember_history: bool,
    show_playlist_on_startup: bool,
    shortcuts: BTreeMap<ShortcutAction, Option<String>>,
}
```

Append the draft helpers in the same file:

```rust
impl SettingsDraft {
    fn from_config(config: &AppConfig) -> Self {
        let shortcuts = ShortcutAction::all()
            .iter()
            .copied()
            .map(|action| {
                let gesture = config
                    .shortcuts
                    .shortcut_for_action(action)
                    .map(|shortcut| shortcut.as_str().to_string());
                (action, gesture)
            })
            .collect();

        Self {
            default_speed: config.playback.default_speed,
            default_volume_percent: config.playback.default_volume_percent,
            prefer_hardware_decode: config.playback.prefer_hardware_decode,
            remember_history: config.ui.remember_history,
            show_playlist_on_startup: config.ui.show_playlist_on_startup,
            shortcuts,
        }
    }

    fn to_config(&self) -> Result<AppConfig, ValidationError> {
        let mut config = AppConfig::default();
        config.playback.default_speed = self.default_speed;
        config.playback.default_volume_percent = self.default_volume_percent;
        config.playback.prefer_hardware_decode = self.prefer_hardware_decode;
        config.ui.remember_history = self.remember_history;
        config.ui.show_playlist_on_startup = self.show_playlist_on_startup;

        let mut map = ShortcutMap { bindings: Default::default() };
        for action in ShortcutAction::all().iter().copied() {
            let shortcut = self.shortcuts.get(&action).cloned().flatten();
            let parsed = shortcut
                .as_deref()
                .map(Shortcut::parse)
                .transpose()?;
            map.set_binding(action, parsed)?;
        }

        config.shortcuts = map;
        config.validate()?;
        Ok(config)
    }

    fn set_shortcut(&mut self, action: ShortcutAction, gesture: Option<String>) {
        self.shortcuts.insert(action, gesture);
    }

    fn restore_shortcut_default(&mut self, action: ShortcutAction) {
        let gesture = action.default_shortcut().map(|shortcut| shortcut.as_str().to_string());
        self.set_shortcut(action, gesture);
    }

    fn clear_shortcut(&mut self, action: ShortcutAction) {
        self.set_shortcut(action, None);
    }

    fn conflict_messages(&self) -> BTreeMap<ShortcutAction, String> {
        let mut seen = BTreeMap::<String, Vec<ShortcutAction>>::new();
        for (action, gesture) in &self.shortcuts {
            if let Some(gesture) = gesture.as_ref() {
                seen.entry(gesture.clone()).or_default().push(*action);
            }
        }

        let mut conflicts = BTreeMap::new();
        for (gesture, actions) in seen {
            if actions.len() < 2 {
                continue;
            }
            let labels = actions
                .iter()
                .map(|action| action.label())
                .collect::<Vec<_>>()
                .join(", ");
            for action in actions {
                conflicts.insert(action, format!("Conflicts with {labels}: {gesture}"));
            }
        }
        conflicts
    }
}
```

Replace the controller implementation in the same file:

```rust
pub struct SettingsController {
    baseline: AppConfig,
    draft: SettingsDraft,
    section_index: i32,
    capture_action: Option<ShortcutAction>,
    status_message: String,
}

impl SettingsController {
    pub fn new(config: AppConfig) -> Self {
        let draft = SettingsDraft::from_config(&config);
        Self {
            baseline: config,
            draft,
            section_index: 0,
            capture_action: None,
            status_message: String::new(),
        }
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        let conflicts = self.draft.conflict_messages();
        let dirty = self.draft != SettingsDraft::from_config(&self.baseline);
        let can_apply = dirty && conflicts.is_empty() && self.draft.to_config().is_ok();

        let shortcut_rows = ShortcutAction::all()
            .iter()
            .copied()
            .map(|action| SettingsShortcutRow {
                action,
                action_label: action.label(),
                binding_label: self
                    .draft
                    .shortcuts
                    .get(&action)
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| "Unbound".to_string()),
                conflict_message: conflicts.get(&action).cloned(),
                is_capturing: self.capture_action == Some(action),
            })
            .collect();

        SettingsSnapshot {
            section_index: self.section_index,
            default_speed: self.draft.default_speed,
            default_volume_percent: self.draft.default_volume_percent,
            prefer_hardware_decode: self.draft.prefer_hardware_decode,
            remember_history: self.draft.remember_history,
            show_playlist_on_startup: self.draft.show_playlist_on_startup,
            dirty,
            can_apply,
            status_message: self.status_message.clone(),
            shortcut_rows,
        }
    }

    pub fn set_section(&mut self, index: i32) {
        self.section_index = index.clamp(0, 2);
    }

    pub fn set_default_speed(&mut self, speed: f32) {
        self.draft.default_speed = speed;
    }

    pub fn set_default_volume_percent(&mut self, volume: u8) {
        self.draft.default_volume_percent = volume;
    }

    pub fn set_prefer_hardware_decode(&mut self, value: bool) {
        self.draft.prefer_hardware_decode = value;
    }

    pub fn set_remember_history(&mut self, value: bool) {
        self.draft.remember_history = value;
    }

    pub fn set_show_playlist_on_startup(&mut self, value: bool) {
        self.draft.show_playlist_on_startup = value;
    }

    pub fn begin_shortcut_capture(&mut self, action: ShortcutAction) {
        self.capture_action = Some(action);
        self.status_message = format!("Press a new shortcut for {}", action.label());
    }

    pub fn is_capturing(&self) -> bool {
        self.capture_action.is_some()
    }

    pub fn capture_shortcut(&mut self, input: KeyboardInput) -> Result<bool, ValidationError> {
        if input.repeat {
            return Ok(false);
        }

        let Some(action) = self.capture_action else {
            return Ok(false);
        };
        let Some(gesture) = shortcut_gesture(input) else {
            return Ok(false);
        };

        Shortcut::parse(&gesture)?;
        self.draft.set_shortcut(action, Some(gesture));
        self.capture_action = None;
        self.status_message.clear();
        Ok(true)
    }

    pub fn clear_shortcut(&mut self, action: ShortcutAction) {
        self.draft.clear_shortcut(action);
    }

    pub fn restore_shortcut_default(&mut self, action: ShortcutAction) {
        self.draft.restore_shortcut_default(action);
    }

    pub fn restore_defaults(&mut self) {
        self.draft = SettingsDraft::from_config(&AppConfig::default());
        self.capture_action = None;
        self.status_message.clear();
    }

    pub fn discard_changes(&mut self) {
        self.draft = SettingsDraft::from_config(&self.baseline);
        self.capture_action = None;
        self.status_message.clear();
    }

    pub fn save(&mut self, path: &Path) -> Result<AppConfig, AppError> {
        let config = self.draft.to_config()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        config.save(path)?;
        self.baseline = config.clone();
        self.draft = SettingsDraft::from_config(&config);
        self.capture_action = None;
        self.status_message = "Settings saved".to_string();
        Ok(config)
    }
}
```

- [ ] **Step 4: Export the richer settings controller API**

Modify `apps/yoyovideo-desktop/src/lib.rs` so the `SettingsController` re-export remains intact:

```rust
pub use settings_controller::SettingsController;
```

No new export line is needed beyond keeping this one present after the file is reformatted.

- [ ] **Step 5: Run the settings tests again**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_contract
```

Expected: PASS for persistence, conflict visibility, restore-default behavior, and cleared bindings.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/settings_controller.rs apps/yoyovideo-desktop/tests/settings_contract.rs
git commit -m "feat: add draft-based settings controller"
```

Expected: Commit succeeds.

---

### Task 4: Export A Dedicated Slint Settings Window Surface

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`
- Modify: `apps/yoyovideo-desktop/src/lib.rs`
- Create: `apps/yoyovideo-desktop/tests/settings_window_contract.rs`

**Interfaces:**
- Produces: `SettingsShortcutRowData`
- Produces: `SettingsWindow`
- Produces: `SettingsWindow::set_shortcut_rows(...)`
- Produces: `SettingsWindow` callbacks:
  - `section_requested(int)`
  - `default_speed_changed(float)`
  - `default_volume_changed(int)`
  - `prefer_hardware_decode_changed(bool)`
  - `remember_history_changed(bool)`
  - `show_playlist_on_startup_changed(bool)`
  - `edit_shortcut_requested(int)`
  - `clear_shortcut_requested(int)`
  - `restore_shortcut_requested(int)`
  - `restore_defaults_requested()`
  - `apply_requested()`
  - `ok_requested()`
  - `cancel_requested()`
- Produces: `pub use app::SettingsWindow`

- [ ] **Step 1: Add the failing Slint window contract test**

Create `apps/yoyovideo-desktop/tests/settings_window_contract.rs`:

```rust
use yoyovideo_desktop::SettingsWindow;

#[test]
fn settings_window_type_is_exported() {
    let constructor: fn() -> Result<SettingsWindow, slint::PlatformError> = SettingsWindow::new;
    let _ = constructor;
}
```

- [ ] **Step 2: Run the failing settings-window contract**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_window_contract
```

Expected: FAIL because `SettingsWindow` does not exist and is not exported from the desktop crate.

- [ ] **Step 3: Add the exported settings window and row struct**

Modify the import line at the top of `apps/yoyovideo-desktop/ui/main-window.slint`:

```slint
import {
    Button,
    CheckBox,
    HorizontalBox,
    LineEdit,
    ScrollView,
    Slider,
    VerticalBox
} from "std-widgets.slint";
```

Append to `apps/yoyovideo-desktop/ui/main-window.slint` after `MainWindow`:

```slint
export struct SettingsShortcutRowData {
    action_label: string,
    binding_label: string,
    conflict_label: string,
    is_capturing: bool,
}

export component SettingsWindow inherits Window {
    title: "Settings";
    width: 760px;
    height: 560px;

    in-out property <int> section_index: 0;
    in-out property <float> default_speed_value: 1.0;
    in-out property <string> default_speed_label: "1.00x";
    in-out property <int> default_volume_value: 100;
    in-out property <string> default_volume_label: "100%";
    in-out property <bool> prefer_hardware_decode: true;
    in-out property <bool> remember_history: true;
    in-out property <bool> show_playlist_on_startup: true;
    in-out property <bool> dirty: false;
    in-out property <bool> can_apply: false;
    in-out property <string> status_label: "";
    in-out property <[SettingsShortcutRowData]> shortcut_rows: [];

    callback section_requested(int);
    callback default_speed_changed(float);
    callback default_volume_changed(int);
    callback prefer_hardware_decode_changed(bool);
    callback remember_history_changed(bool);
    callback show_playlist_on_startup_changed(bool);
    callback edit_shortcut_requested(int);
    callback clear_shortcut_requested(int);
    callback restore_shortcut_requested(int);
    callback restore_defaults_requested();
    callback apply_requested();
    callback ok_requested();
    callback cancel_requested();

    HorizontalBox {
        spacing: 10px;
        padding: 12px;

        VerticalBox {
            width: 170px;
            spacing: 8px;

            Button { text: "Playback"; clicked => { root.section_requested(0); } }
            Button { text: "Interface"; clicked => { root.section_requested(1); } }
            Button { text: "Shortcuts"; clicked => { root.section_requested(2); } }
        }

        VerticalBox {
            spacing: 10px;

            if root.section_index == 0: VerticalBox {
                spacing: 8px;
                Text { text: "Default Speed"; }
                Slider {
                    minimum: 25;
                    maximum: 400;
                    value: root.default_speed_value * 100;
                    changed(value) => { root.default_speed_changed(value / 100); }
                }
                Text { text: root.default_speed_label; }
                Text { text: "Default Volume"; }
                Slider {
                    minimum: 0;
                    maximum: 100;
                    value: root.default_volume_value;
                    changed(value) => { root.default_volume_changed(value); }
                }
                Text { text: root.default_volume_label; }
                CheckBox {
                    text: "Prefer Hardware Decode";
                    checked: root.prefer_hardware_decode;
                    toggled => { root.prefer_hardware_decode_changed(self.checked); }
                }
            }

            if root.section_index == 1: VerticalBox {
                spacing: 8px;
                CheckBox {
                    text: "Remember Playback History";
                    checked: root.remember_history;
                    toggled => { root.remember_history_changed(self.checked); }
                }
                CheckBox {
                    text: "Show Playlist On Startup";
                    checked: root.show_playlist_on_startup;
                    toggled => { root.show_playlist_on_startup_changed(self.checked); }
                }
            }

            if root.section_index == 2: ScrollView {
                VerticalBox {
                    spacing: 6px;

                    for row[idx] in root.shortcut_rows: Rectangle {
                        border-width: 1px;
                        border-radius: 6px;
                        border-color: row.conflict_label == "" ? #26313a : #a44747;
                        background: row.is_capturing ? #1d2d38 : #141b20;
                        min-height: 64px;

                        VerticalBox {
                            padding: 8px;
                            spacing: 4px;

                            Text { text: row.action_label; color: #f2f5f7; }
                            Text { text: row.binding_label; color: #c7d1d8; }
                            Text { text: row.conflict_label; color: #f28b82; }

                            HorizontalBox {
                                spacing: 6px;
                                Button { text: "Edit"; clicked => { root.edit_shortcut_requested(idx); } }
                                Button { text: "Clear"; clicked => { root.clear_shortcut_requested(idx); } }
                                Button { text: "Restore Default"; clicked => { root.restore_shortcut_requested(idx); } }
                            }
                        }
                    }
                }
            }

            Rectangle { height: 1px; background: #26313a; }
            Text { text: root.status_label; color: #c7d1d8; }

            HorizontalBox {
                spacing: 8px;
                Button { text: "Restore Defaults"; clicked => { root.restore_defaults_requested(); } }
                Button { text: "Cancel"; clicked => { root.cancel_requested(); } }
                Button { text: "Apply"; enabled: root.can_apply; clicked => { root.apply_requested(); } }
                Button { text: "OK"; clicked => { root.ok_requested(); } }
            }
        }
    }
}
```

- [ ] **Step 4: Re-export the settings window type for tests**

Modify `apps/yoyovideo-desktop/src/lib.rs`:

```rust
pub use app::{
    DesktopController, SettingsWindow, build_desktop_backend,
    build_desktop_backend_with_video_window, dispatch_shortcut, refresh_window, run,
};
```

- [ ] **Step 5: Run the settings-window contract again**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_window_contract
```

Expected: PASS. The desktop crate now exports a compilable Slint settings window surface with the required properties.

- [ ] **Step 6: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint apps/yoyovideo-desktop/src/lib.rs apps/yoyovideo-desktop/tests/settings_window_contract.rs
git commit -m "feat: add settings window surface"
```

Expected: Commit succeeds.

---

### Task 5: Runtime Wiring, Immediate Apply Semantics, And History Toggle Behavior

**Files:**
- Modify: `apps/yoyovideo-desktop/src/history_runtime.rs`
- Modify: `apps/yoyovideo-desktop/src/app.rs`
- Modify: `apps/yoyovideo-desktop/tests/controller_contract.rs`
- Create: `apps/yoyovideo-desktop/tests/settings_runtime_contract.rs`

**Interfaces:**
- Produces: `HistoryRuntime::set_enabled(&mut self, enabled: bool)`
- Produces: `HistoryRuntime::enabled(&self) -> bool`
- Produces: `DesktopController::with_shortcuts(session: AppSession<B>, shortcuts: ShortcutMap) -> Self`
- Produces: `DesktopController::set_shortcuts(&mut self, shortcuts: ShortcutMap)`
- Produces: `SettingsWindow` runtime refresh in `app.rs`
- Produces: `handle_settings_save(runtime: &Rc<RefCell<DesktopRuntime>>, config_path: &PathBuf, close_after_save: bool)`
- Produces: `DesktopRuntime { settings_window: Option<SettingsWindow>, settings_controller: Option<SettingsController> }`
- Produces: validated startup config loading in `load_boot_config()`
- Produces: runtime save/apply flow that writes the config first, then updates shortcuts and history enablement in memory

- [ ] **Step 1: Add the failing runtime-application tests**

Create `apps/yoyovideo-desktop/tests/settings_runtime_contract.rs`:

```rust
use std::time::Duration;

use tempfile::tempdir;
use yoyo_core::{
    AppConfig, AppSession, BackendCommand, BackendEvent, MediaLocator, PlayerBackend, Shortcut,
    ShortcutAction, ShortcutMap,
};
use yoyovideo_desktop::{FlushReason, HistoryRuntime, DesktopController};

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
}

impl PlayerBackend for MockBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.opened.push(locator.clone());
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.commands.push(command);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        Vec::new()
    }
}

#[test]
fn controller_uses_replaced_shortcut_maps_immediately() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut controller = DesktopController::new(session);
    let mut shortcuts = ShortcutMap::default();
    shortcuts
        .set_binding(
            ShortcutAction::TogglePause,
            Some(Shortcut::parse("Ctrl+P").unwrap()),
        )
        .unwrap();

    controller.set_shortcuts(shortcuts);
    controller.dispatch_shortcut("Ctrl+P").unwrap();

    assert_eq!(
        controller.session().backend().commands,
        vec![BackendCommand::SetPaused(false)]
    );
}

#[test]
fn disabling_history_runtime_stops_future_history_writes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), Default::default(), true);

    runtime.set_enabled(false);
    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/new.mp4".into()),
        "New",
        Some(42.0),
    );

    assert!(!runtime.flush_if_needed(Duration::from_secs(5), FlushReason::Pause).unwrap());
    assert!(!path.exists());
}
```

Append to `apps/yoyovideo-desktop/tests/controller_contract.rs`:

```rust
#[test]
fn controller_can_start_with_non_default_shortcuts() {
    let session = AppSession::new(AppConfig::default(), MockBackend::default());
    let mut shortcuts = yoyo_core::ShortcutMap::default();
    shortcuts
        .set_binding(
            yoyo_core::ShortcutAction::TogglePause,
            Some(yoyo_core::Shortcut::parse("Ctrl+P").unwrap()),
        )
        .unwrap();

    let mut controller = DesktopController::with_shortcuts(session, shortcuts);
    controller.dispatch_shortcut("Ctrl+P").unwrap();

    assert_eq!(
        controller.session().backend().commands,
        vec![BackendCommand::SetPaused(false)]
    );
}
```

- [ ] **Step 2: Run the failing runtime tests**

Run:

```powershell
cargo test -p yoyovideo-desktop --test settings_runtime_contract
cargo test -p yoyovideo-desktop --test controller_contract
```

Expected: FAIL because `set_enabled()`, `set_shortcuts()`, and `with_shortcuts()` do not exist yet.

- [ ] **Step 3: Add live shortcut replacement and history-enable toggling**

Modify `apps/yoyovideo-desktop/src/history_runtime.rs`:

```rust
impl HistoryRuntime {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.dirty = false;
        }
    }
}
```

Modify `apps/yoyovideo-desktop/src/app.rs` inside `impl<B: PlayerBackend> DesktopController<B>`:

```rust
pub fn with_shortcuts(session: AppSession<B>, shortcuts: ShortcutMap) -> Self {
    Self { session, shortcuts, video_texture: VideoTexture::default() }
}

pub fn set_shortcuts(&mut self, shortcuts: ShortcutMap) {
    self.shortcuts = shortcuts;
}
```

Keep `new()` as the default wrapper:

```rust
pub fn new(session: AppSession<B>) -> Self {
    Self::with_shortcuts(session, ShortcutMap::default())
}
```

- [ ] **Step 4: Load only validated boot config and keep a settings window/controller in runtime**

Modify `load_boot_config()` in `apps/yoyovideo-desktop/src/app.rs`:

```rust
fn load_boot_config(paths: Option<&AppPaths>) -> AppConfig {
    let Some(path) = paths.map(config_file_path) else {
        return AppConfig::default();
    };
    let Ok(config) = AppConfig::load(&path) else {
        return AppConfig::default();
    };
    if config.validate().is_ok() {
        config
    } else {
        AppConfig::default()
    }
}
```

Extend `DesktopRuntime`:

```rust
struct DesktopRuntime {
    controller: Option<DesktopController<MpvBackend>>,
    video_host_error: Option<String>,
    app_handle: Option<slint::Weak<MainWindow>>,
    config: AppConfig,
    history: crate::HistoryRuntime,
    sidebar: crate::SidebarState,
    settings_window: Option<SettingsWindow>,
    settings_controller: Option<crate::SettingsController>,
    pending_resume: Option<crate::PendingResumeSeek>,
    last_seen_locator: Option<MediaLocator>,
    started_at: Instant,
    #[cfg(feature = "mpv-runtime")]
    video_host: Option<WinitVideoHost>,
}
```

Initialize the new fields in `DesktopRuntime::new()`:

```rust
settings_window: None,
settings_controller: None,
```

- [ ] **Step 5: Add settings-window refresh and saved-config application helpers**

Still in `apps/yoyovideo-desktop/src/app.rs`, add these helpers above `run()`:

```rust
fn refresh_settings_window(window: &SettingsWindow, controller: &crate::SettingsController) {
    let snapshot = controller.snapshot();
    window.set_section_index(snapshot.section_index);
    window.set_default_speed_value(snapshot.default_speed);
    window.set_default_speed_label(format!("{:.2}x", snapshot.default_speed).into());
    window.set_default_volume_value(i32::from(snapshot.default_volume_percent));
    window.set_default_volume_label(format!("{}%", snapshot.default_volume_percent).into());
    window.set_prefer_hardware_decode(snapshot.prefer_hardware_decode);
    window.set_remember_history(snapshot.remember_history);
    window.set_show_playlist_on_startup(snapshot.show_playlist_on_startup);
    window.set_dirty(snapshot.dirty);
    window.set_can_apply(snapshot.can_apply);
    window.set_status_label(snapshot.status_message.into());

    let rows = snapshot
        .shortcut_rows
        .into_iter()
        .map(|row| SettingsShortcutRowData {
            action_label: row.action_label.into(),
            binding_label: row.binding_label.into(),
            conflict_label: row.conflict_message.unwrap_or_default().into(),
            is_capturing: row.is_capturing,
        })
        .collect::<Vec<_>>();

    window.set_shortcut_rows(model_from_vec(rows));
}

fn apply_saved_settings(runtime: &mut DesktopRuntime, saved: AppConfig) {
    if let Some(controller) = runtime.controller_mut() {
        controller.set_shortcuts(saved.shortcuts.clone());
    }
    runtime.history.set_enabled(saved.ui.remember_history);
    runtime.config = saved;
}

fn handle_settings_save(
    runtime: &Rc<RefCell<DesktopRuntime>>,
    config_path: &PathBuf,
    close_after_save: bool,
) {
    let mut runtime = runtime.borrow_mut();
    let saved = {
        let Some(controller) = runtime.settings_controller.as_mut() else {
            return;
        };
        match controller.save(config_path) {
            Ok(saved) => saved,
            Err(error) => {
                if let Some(window) = runtime.settings_window.as_ref() {
                    window.set_status_label(error.to_string().into());
                }
                return;
            }
        }
    };

    apply_saved_settings(&mut runtime, saved);
    if let Some(app) = runtime.app_handle.as_ref().and_then(|handle| handle.upgrade()) {
        app.set_status_label("Settings saved".into());
    }
    if let (Some(window), Some(controller)) = (
        runtime.settings_window.as_ref(),
        runtime.settings_controller.as_ref(),
    ) {
        refresh_settings_window(window, controller);
        if close_after_save {
            let _ = window.hide();
        }
    }
}
```

- [ ] **Step 6: Create and wire the dedicated settings window from the main runtime**

Modify `run()` in `apps/yoyovideo-desktop/src/app.rs` so it computes a concrete save path once:

```rust
let paths = AppPaths::discover();
let config_path = paths
    .as_ref()
    .map(config_file_path)
    .unwrap_or_else(|| PathBuf::from("config.toml"));
let config = load_boot_config(paths.as_ref());
let history = load_history_runtime(paths.as_ref(), &config);
```

Update runtime initialization inside `DesktopWinitHandler::initialize_runtime()` so the loaded config’s shortcuts are used:

```rust
let config = runtime.config.clone();
let shortcuts = config.shortcuts.clone();
let result = (|| -> Result<(DesktopController<MpvBackend>, WinitVideoHost), String> {
    let video_host = WinitVideoHost::new_child(event_loop, parent_window)
        .map_err(|error| error.to_string())?;
    let window_id = video_host.mpv_window_id().map_err(|error| error.to_string())?;
    let backend = build_desktop_backend_with_video_window(window_id)
        .map_err(|error| error.to_string())?;
    let session = AppSession::new(config, backend);
    Ok((DesktopController::with_shortcuts(session, shortcuts), video_host))
})();
```

In the existing main-window keyboard callback, pass the normalized `String` gesture by reference:

```rust
with_runtime_controller(&app_handle, &runtime, move |controller| {
    controller.dispatch_shortcut(gesture.as_str())
});
```

Replace the placeholder `app.on_settings_requested` callback with:

```rust
app.on_settings_requested({
    let runtime = Rc::clone(&runtime);
    let config_path = config_path.clone();
    move || {
        let mut runtime = runtime.borrow_mut();
        runtime.settings_controller = Some(crate::SettingsController::new(runtime.config.clone()));

        if runtime.settings_window.is_none() {
            let window = SettingsWindow::new().expect("settings window");
            let keyboard_state = Rc::new(RefCell::new(
                crate::keyboard::winit_adapter::WinitKeyboardState::default(),
            ));

            window.on_section_requested({
                let runtime = Rc::clone(&runtime);
                move |index| {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.set_section(index);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_default_speed_changed({
                let runtime = Rc::clone(&runtime);
                move |speed| {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.set_default_speed(speed);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_default_volume_changed({
                let runtime = Rc::clone(&runtime);
                move |volume| {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.set_default_volume_percent(volume.clamp(0, 100) as u8);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_prefer_hardware_decode_changed({
                let runtime = Rc::clone(&runtime);
                move |value| {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.set_prefer_hardware_decode(value);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_remember_history_changed({
                let runtime = Rc::clone(&runtime);
                move |value| {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.set_remember_history(value);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_show_playlist_on_startup_changed({
                let runtime = Rc::clone(&runtime);
                move |value| {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.set_show_playlist_on_startup(value);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_edit_shortcut_requested({
                let runtime = Rc::clone(&runtime);
                move |index| {
                    let Some(action) = ShortcutAction::all().get(index as usize).copied() else {
                        return;
                    };
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.begin_shortcut_capture(action);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_clear_shortcut_requested({
                let runtime = Rc::clone(&runtime);
                move |index| {
                    let Some(action) = ShortcutAction::all().get(index as usize).copied() else {
                        return;
                    };
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.clear_shortcut(action);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_restore_shortcut_requested({
                let runtime = Rc::clone(&runtime);
                move |index| {
                    let Some(action) = ShortcutAction::all().get(index as usize).copied() else {
                        return;
                    };
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.restore_shortcut_default(action);
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_restore_defaults_requested({
                let runtime = Rc::clone(&runtime);
                move || {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.restore_defaults();
                        refresh_settings_window(window, controller);
                    }
                }
            });

            window.on_apply_requested({
                let runtime = Rc::clone(&runtime);
                let config_path = config_path.clone();
                move || handle_settings_save(&runtime, &config_path, false)
            });

            window.on_ok_requested({
                let runtime = Rc::clone(&runtime);
                let config_path = config_path.clone();
                move || handle_settings_save(&runtime, &config_path, true)
            });

            window.on_cancel_requested({
                let runtime = Rc::clone(&runtime);
                move || {
                    let mut runtime = runtime.borrow_mut();
                    if let (Some(window), Some(controller)) = (
                        runtime.settings_window.as_ref(),
                        runtime.settings_controller.as_mut(),
                    ) {
                        controller.discard_changes();
                        refresh_settings_window(window, controller);
                        let _ = window.hide();
                    }
                }
            });

            window.window().on_winit_window_event({
                let runtime = Rc::clone(&runtime);
                let keyboard_state = Rc::clone(&keyboard_state);
                move |_window, event| {
                    let Some(input) = keyboard_state.borrow_mut().update(event) else {
                        return slint::winit_030::EventResult::Propagate;
                    };

                    let mut runtime = runtime.borrow_mut();
                    let Some(controller) = runtime.settings_controller.as_mut() else {
                        return slint::winit_030::EventResult::Propagate;
                    };
                    if !controller.is_capturing() {
                        return slint::winit_030::EventResult::Propagate;
                    }

                    let consumed = controller.capture_shortcut(input).unwrap_or(false);
                    if consumed {
                        if let Some(window) = runtime.settings_window.as_ref() {
                            refresh_settings_window(window, controller);
                        }
                        slint::winit_030::EventResult::PreventDefault
                    } else {
                        slint::winit_030::EventResult::Propagate
                    }
                }
            });

            runtime.settings_window = Some(window);
        }

        if let (Some(window), Some(controller)) = (
            runtime.settings_window.as_ref(),
            runtime.settings_controller.as_ref(),
        ) {
            refresh_settings_window(window, controller);
            let _ = window.show();
        }
    }
});
```

- [ ] **Step 7: Run the runtime-focused tests and compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test controller_contract
cargo test -p yoyovideo-desktop --test settings_runtime_contract
cargo test -p yoyovideo-desktop --test settings_contract
cargo test -p yoyovideo-desktop --test settings_window_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS. The desktop build now compiles with a dedicated settings window, runtime shortcut replacement, and history-enable toggling.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/src/history_runtime.rs apps/yoyovideo-desktop/src/app.rs apps/yoyovideo-desktop/tests/controller_contract.rs apps/yoyovideo-desktop/tests/settings_runtime_contract.rs
git commit -m "feat: wire settings window into desktop runtime"
```

Expected: Commit succeeds.

---

### Task 6: Manual Smoke Checklist And Final Verification

**Files:**
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Produces: updated settings-window and shortcut-editing smoke coverage in the shared checklist

- [ ] **Step 1: Add the settings-specific smoke coverage**

Append these lines under `## UX` in `docs/testing/manual-smoke-checklist.md`:

```markdown
- Open the dedicated settings window and confirm `Apply`, `OK`, and `Cancel` behave like a desktop dialog.
- Change the play/pause shortcut in settings and confirm the new shortcut works immediately without restarting.
- Attempt to bind the same shortcut to two different actions and confirm the save is blocked with a clear conflict message.
- Disable playback history in settings, play a new media item, and confirm no new history entry is written.
- Change default speed or default volume in settings, open new media, and confirm the new defaults apply without changing media that was already playing before the save.
```

- [ ] **Step 2: Run a quick documentation coverage check**

Run:

```powershell
$content = Get-Content -Raw docs/testing/manual-smoke-checklist.md
$required = @(
  "Apply`, `OK`, and `Cancel`",
  "new shortcut works immediately",
  "save is blocked",
  "no new history entry",
  "new defaults apply"
)
$missing = $required | Where-Object { $content -notmatch [regex]::Escape($_) }
if ($missing.Count -gt 0) {
  Write-Error ("Missing checklist coverage: " + ($missing -join ", "))
  exit 1
}
```

Expected: PASS.

- [ ] **Step 3: Run final formatting and verification**

Run:

```powershell
cargo fmt --check
cargo test -p yoyo-core --test config_shortcut_contract
cargo test -p yoyovideo-desktop --test keyboard_contract
cargo test -p yoyovideo-desktop --test shortcut_contract
cargo test -p yoyovideo-desktop --test settings_contract
cargo test -p yoyovideo-desktop --test settings_window_contract
cargo test -p yoyovideo-desktop --test settings_runtime_contract
cargo check -p yoyovideo-desktop
cargo check -p yoyovideo-desktop --features mpv-runtime
git status --short
```

Expected:
- `cargo fmt --check`: PASS
- all targeted tests: PASS
- both `cargo check` commands: PASS
- `git status --short`: only the planned source/doc changes remain before commit

- [ ] **Step 4: Commit**

Run:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: add settings window smoke checks"
```

Expected: Commit succeeds.

---

## Self-Review

**Spec coverage:** The plan covers the dedicated settings window, explicit `Apply / OK / Cancel` flow, all current `AppConfig` fields, single-binding shortcut editing, row-level clear and restore-default actions, conflict visibility plus save blocking, strict save ordering, immediate runtime shortcut replacement, future-only playback defaults, `remember_history` runtime toggling, `show_playlist_on_startup` startup-only semantics, and manual smoke additions. It intentionally leaves multi-binding shortcuts, global hotkeys, import/export presets, subtitle UI, and live mpv backend rebuilds out of scope.

**Placeholder scan:** The plan does not use `TBD`, `TODO`, “implement later”, or “similar to Task N” placeholders. Each task names exact files, code snippets, commands, expected failures, expected passes, and commit messages.

**Type consistency:** The plan uses one consistent set of names across tasks: `MIN_DEFAULT_SPEED`, `MAX_DEFAULT_SPEED`, `SettingsController`, `SettingsSnapshot`, `SettingsShortcutRow`, `SettingsWindow`, `SettingsShortcutRowData`, `DesktopController::with_shortcuts`, `DesktopController::set_shortcuts`, and `HistoryRuntime::set_enabled`. The keyboard layer consistently produces gesture `String`s that flow through both capture and runtime dispatch.
