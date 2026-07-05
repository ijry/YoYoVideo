use serde::{Deserialize, Serialize};

use crate::MediaLocator;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub locator: MediaLocator,
    pub title: String,
}

impl PlaylistEntry {
    pub fn new(locator: MediaLocator) -> Self {
        let title = locator.as_label();
        Self { locator, title }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistSnapshot {
    pub entries: Vec<PlaylistEntry>,
    pub current_index: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub entries: Vec<PlaylistEntry>,
    pub current_index: Option<usize>,
}

impl Playlist {
    pub fn replace(&mut self, entries: Vec<PlaylistEntry>, start_index: usize) {
        self.entries = entries;
        self.current_index = if self.entries.is_empty() {
            None
        } else {
            Some(start_index.min(self.entries.len() - 1))
        };
    }

    pub fn current(&self) -> Option<&PlaylistEntry> {
        self.current_index.and_then(|index| self.entries.get(index))
    }

    pub fn select(&mut self, index: usize) -> Option<&PlaylistEntry> {
        if index >= self.entries.len() {
            return None;
        }

        self.current_index = Some(index);
        self.entries.get(index)
    }

    pub fn snapshot(&self) -> PlaylistSnapshot {
        PlaylistSnapshot { entries: self.entries.clone(), current_index: self.current_index }
    }

    pub fn next(&mut self) -> Option<&PlaylistEntry> {
        let next_index = self.current_index?.saturating_add(1);
        if next_index < self.entries.len() {
            self.current_index = Some(next_index);
            self.entries.get(next_index)
        } else {
            None
        }
    }

    pub fn previous(&mut self) -> Option<&PlaylistEntry> {
        let current = self.current_index?;
        if current > 0 {
            self.current_index = Some(current - 1);
            self.entries.get(current - 1)
        } else {
            None
        }
    }
}
