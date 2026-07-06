use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yoyo_core::{MediaMarker, StorageError};

use super::AppPaths;

pub const MAX_MARKER_SETS: usize = 200;
pub const MAX_MARKERS_PER_MEDIA: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaMarkerSet {
    pub locator_key: String,
    pub markers: Vec<MediaMarker>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MarkerStore {
    #[serde(skip)]
    path: Option<PathBuf>,
    pub items: Vec<MediaMarkerSet>,
}

impl MarkerStore {
    pub fn with_path(path: Option<PathBuf>) -> Self {
        Self { path, items: Vec::new() }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let Some(path_value) = path.clone() else {
            return Ok(Self::with_path(None));
        };
        if !path_value.exists() {
            return Ok(Self::with_path(Some(path_value)));
        }

        let raw = fs::read_to_string(&path_value)?;
        let mut store = toml::from_str::<MarkerStore>(&raw).unwrap_or_default();
        store.path = Some(path_value);
        store.normalize();
        Ok(store)
    }

    pub fn markers_for(&self, locator_key: &str) -> Vec<MediaMarker> {
        self.items
            .iter()
            .find(|set| set.locator_key == locator_key)
            .map(|set| set.markers.clone())
            .unwrap_or_default()
    }

    pub fn set_markers(&mut self, locator_key: String, markers: Vec<MediaMarker>) {
        let mut markers = markers
            .into_iter()
            .filter(|marker| marker.time_seconds.is_finite() && marker.time_seconds >= 0.0)
            .collect::<Vec<_>>();
        markers.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
        markers.truncate(MAX_MARKERS_PER_MEDIA);

        self.items.retain(|set| set.locator_key != locator_key);
        self.items.insert(0, MediaMarkerSet { locator_key, markers });
        self.items.truncate(MAX_MARKER_SETS);
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

    fn normalize(&mut self) {
        let mut normalized = Vec::new();
        for mut item in self.items.drain(..) {
            item.markers
                .retain(|marker| marker.time_seconds.is_finite() && marker.time_seconds >= 0.0);
            item.markers.sort_by(|left, right| left.time_seconds.total_cmp(&right.time_seconds));
            item.markers.truncate(MAX_MARKERS_PER_MEDIA);
            if !item.locator_key.trim().is_empty() {
                normalized.push(item);
            }
        }

        normalized.truncate(MAX_MARKER_SETS);
        self.items = normalized;
    }
}

pub fn marker_store_path(paths: Option<&AppPaths>) -> Option<PathBuf> {
    paths.map(|paths| paths.data_dir.join("markers.toml"))
}
