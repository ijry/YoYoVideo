use std::path::Path;

use yoyo_core::{AppConfig, AppError, Shortcut, ShortcutAction, ValidationError};

#[derive(Default)]
pub struct SettingsController {
    config: AppConfig,
}

impl SettingsController {
    pub fn update_shortcut(
        &mut self,
        gesture: &str,
        action: ShortcutAction,
    ) -> Result<(), ValidationError> {
        let shortcut = Shortcut::parse(gesture)?;
        if let Some(existing) = self.config.shortcuts.bindings.get(&shortcut) {
            if *existing != action {
                return Err(ValidationError::InvalidShortcut(format!(
                    "duplicate shortcut: {}",
                    shortcut.as_str()
                )));
            }
        }
        self.config.shortcuts.bindings.insert(shortcut, action);
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        self.config.save(path)?;
        Ok(())
    }
}
