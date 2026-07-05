use yoyo_core::{HistoryEntry, HistoryStore, MediaLocator, PlaylistEntry, PlaylistSnapshot};
use yoyovideo_desktop::{
    SidebarTab, build_history_rows, build_playlist_rows, expanded_sidebar_width,
    initial_sidebar_state,
};

#[test]
fn startup_visibility_uses_config_but_forces_narrow_windows_collapsed() {
    let wide = initial_sidebar_state(true, 1280.0);
    let narrow = initial_sidebar_state(true, 980.0);

    assert!(wide.visible);
    assert_eq!(wide.active_tab, SidebarTab::Playlist);
    assert!(!narrow.visible);
    assert_eq!(expanded_sidebar_width(980.0), 260.0);
    assert_eq!(expanded_sidebar_width(1280.0), 320.0);
}

#[test]
fn playlist_rows_highlight_only_the_valid_current_index() {
    let snapshot = PlaylistSnapshot {
        entries: vec![
            PlaylistEntry::new(MediaLocator::Url("https://example.com/a.mp4".into())),
            PlaylistEntry::new(MediaLocator::Url("https://example.com/b.mp4".into())),
        ],
        current_index: Some(1),
    };

    let rows = build_playlist_rows(&snapshot);

    assert_eq!(rows.len(), 2);
    assert!(!rows[0].is_current);
    assert!(rows[1].is_current);
}

#[test]
fn history_rows_format_resume_metadata() {
    let store = HistoryStore {
        items: vec![
            HistoryEntry {
                locator: MediaLocator::Url("https://example.com/movie.mp4".into()),
                title: "Movie".into(),
                last_position_seconds: Some(95.0),
            },
            HistoryEntry {
                locator: MediaLocator::Url("https://example.com/fresh.mp4".into()),
                title: "Fresh".into(),
                last_position_seconds: None,
            },
        ],
    };

    let rows = build_history_rows(&store);

    assert_eq!(rows[0].subtitle, "Resume 01:35");
    assert_eq!(rows[1].subtitle, "Resume start");
}
