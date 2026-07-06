use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::StorageError;

use super::AppPaths;

pub const MAX_RECENT_OPEN_ITEMS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecentOpenKind {
    File,
    Folder,
    Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentOpenItem {
    pub kind: RecentOpenKind,
    pub target: String,
    pub title: String,
    pub opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecentOpenStore {
    #[serde(skip)]
    path: Option<PathBuf>,
    pub items: Vec<RecentOpenItem>,
}

impl RecentOpenStore {
    pub fn with_path(path: Option<PathBuf>) -> Self {
        Self { path, items: Vec::new() }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let Some(path_value) = path else {
            return Ok(Self::default());
        };
        if !path_value.exists() {
            return Ok(Self::with_path(Some(path_value)));
        }
        let raw = fs::read_to_string(&path_value)?;
        let mut store = toml::from_str::<RecentOpenStore>(&raw).unwrap_or_default();
        store.path = Some(path_value);
        store.items.truncate(MAX_RECENT_OPEN_ITEMS);
        Ok(store)
    }

    pub fn remember(&mut self, item: RecentOpenItem) {
        self.items.retain(|existing| existing.kind != item.kind || existing.target != item.target);
        self.items.insert(0, item);
        self.items.truncate(MAX_RECENT_OPEN_ITEMS);
    }

    pub fn save(&self) -> Result<(), StorageError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}

pub fn recent_open_path(paths: Option<&AppPaths>) -> Option<PathBuf> {
    paths.map(|paths| paths.data_dir.join("recent-open.toml"))
}
