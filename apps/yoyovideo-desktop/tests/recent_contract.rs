use tempfile::tempdir;
use yoyovideo_desktop::platform::{
    MAX_RECENT_OPEN_ITEMS, RecentOpenItem, RecentOpenKind, RecentOpenStore,
};

fn item(kind: RecentOpenKind, target: &str, title: &str, opened_at: &str) -> RecentOpenItem {
    RecentOpenItem {
        kind,
        target: target.to_string(),
        title: title.to_string(),
        opened_at: opened_at.to_string(),
    }
}

#[test]
fn recent_store_deduplicates_newest_first_and_caps_at_ten() {
    let mut store = RecentOpenStore::default();

    for index in 0..12 {
        store.remember(item(
            RecentOpenKind::File,
            &format!("movie-{index}.mp4"),
            &format!("movie-{index}.mp4"),
            &format!("2026-07-06T10:{index:02}:00+08:00"),
        ));
    }
    store.remember(item(
        RecentOpenKind::File,
        "movie-5.mp4",
        "movie-5.mp4",
        "2026-07-06T11:00:00+08:00",
    ));

    assert_eq!(store.items.len(), MAX_RECENT_OPEN_ITEMS);
    assert_eq!(store.items[0].target, "movie-5.mp4");
    assert_eq!(store.items.iter().filter(|entry| entry.target == "movie-5.mp4").count(), 1);
}

#[test]
fn recent_store_round_trips_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recent.toml");
    let mut store = RecentOpenStore::with_path(Some(path.clone()));
    store.remember(item(RecentOpenKind::Folder, "D:/Media", "Media", "2026-07-06T10:00:00+08:00"));

    store.save().unwrap();
    let loaded = RecentOpenStore::load(Some(path)).unwrap();

    assert_eq!(loaded.items, store.items);
}

#[test]
fn recent_store_missing_file_loads_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing.toml");

    let store = RecentOpenStore::load(Some(path)).unwrap();

    assert!(store.items.is_empty());
}

#[test]
fn recent_store_corrupt_file_loads_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recent.toml");
    std::fs::write(&path, "not valid toml").unwrap();

    let store = RecentOpenStore::load(Some(path)).unwrap();

    assert!(store.items.is_empty());
}
