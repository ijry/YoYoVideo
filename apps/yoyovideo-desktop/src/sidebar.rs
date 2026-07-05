use yoyo_core::{HistoryStore, PlaylistSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Playlist,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarState {
    pub visible: bool,
    pub active_tab: SidebarTab,
}

impl SidebarState {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show_tab(&mut self, tab: SidebarTab) {
        self.active_tab = tab;
        self.visible = true;
    }

    pub fn tab_index(&self) -> i32 {
        match self.active_tab {
            SidebarTab::Playlist => 0,
            SidebarTab::History => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistSidebarRow {
    pub title: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySidebarRow {
    pub title: String,
    pub subtitle: String,
}

pub fn initial_sidebar_state(show_playlist_on_startup: bool, window_width: f32) -> SidebarState {
    SidebarState {
        visible: show_playlist_on_startup && window_width >= 1050.0,
        active_tab: SidebarTab::Playlist,
    }
}

pub fn expanded_sidebar_width(window_width: f32) -> f32 {
    if window_width < 1050.0 { 260.0 } else { 320.0 }
}

pub fn build_playlist_rows(snapshot: &PlaylistSnapshot) -> Vec<PlaylistSidebarRow> {
    let valid_current = snapshot.current_index.filter(|index| *index < snapshot.entries.len());

    snapshot
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| PlaylistSidebarRow {
            title: entry.title.clone(),
            is_current: valid_current == Some(index),
        })
        .collect()
}

pub fn build_history_rows(store: &HistoryStore) -> Vec<HistorySidebarRow> {
    store
        .items()
        .iter()
        .map(|entry| HistorySidebarRow {
            title: entry.title.clone(),
            subtitle: format_history_resume(entry.last_position_seconds),
        })
        .collect()
}

fn format_history_resume(seconds: Option<f64>) -> String {
    match seconds {
        Some(seconds) => {
            let total = seconds.max(0.0) as u64;
            format!("Resume {:02}:{:02}", total / 60, total % 60)
        }
        None => "Resume start".into(),
    }
}
