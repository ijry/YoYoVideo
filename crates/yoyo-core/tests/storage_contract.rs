use std::path::PathBuf;

use tempfile::tempdir;
use yoyo_core::{
    AppConfig, HistoryEntry, HistoryStore, MediaLocator, Playlist, PlaylistEntry, Shortcut,
    ShortcutAction, ShortcutMap, ValidationError,
};

#[test]
fn invalid_url_is_rejected_before_backend_open() {
    let error = MediaLocator::from_url("notaurl").unwrap_err();
    assert!(matches!(error, ValidationError::InvalidUrl(_)));
}

#[test]
fn playlist_next_advances_current_item() {
    let mut playlist = Playlist::default();
    playlist.replace(
        vec![
            PlaylistEntry::new(MediaLocator::File(PathBuf::from("a.mp4"))),
            PlaylistEntry::new(MediaLocator::File(PathBuf::from("b.mp4"))),
        ],
        0,
    );

    let next = playlist.next().expect("next item");
    assert_eq!(next.locator, MediaLocator::File(PathBuf::from("b.mp4")));
}

#[test]
fn default_shortcuts_include_required_bindings() {
    let shortcut = Shortcut::parse("Space").unwrap();
    let map = ShortcutMap::default();
    assert_eq!(map.action_for(&shortcut), Some(ShortcutAction::TogglePause));
}

#[test]
fn history_round_trip_preserves_resume_position() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");

    let mut history = HistoryStore::default();
    history.items.push(HistoryEntry {
        locator: MediaLocator::File(PathBuf::from("movie.mkv")),
        title: "movie.mkv".to_string(),
        last_position_seconds: Some(84.0),
    });

    history.save(&path).unwrap();
    let loaded = HistoryStore::load(&path).unwrap();

    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items[0].last_position_seconds, Some(84.0));
}

#[test]
fn config_round_trip_keeps_speed_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let config = AppConfig::default();
    config.save(&path).unwrap();
    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(loaded.playback.default_speed, 1.0);
}
