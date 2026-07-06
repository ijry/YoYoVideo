use tempfile::tempdir;
use yoyo_core::MediaMarker;
use yoyovideo_desktop::platform::{MarkerStore, marker_store_path};

fn marker(id: &str, seconds: f64) -> MediaMarker {
    MediaMarker {
        id: id.into(),
        title: format!("Marker {seconds}"),
        time_seconds: seconds,
        created_at: "2026-07-06T10:00:00+08:00".into(),
    }
}

#[test]
fn marker_store_round_trips_sorted_markers() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("markers.toml");
    let mut store = MarkerStore::with_path(Some(path.clone()));
    store.set_markers("file:movie.mp4".into(), vec![marker("b", 20.0), marker("a", 5.0)]);

    store.save().unwrap();
    let loaded = MarkerStore::load(Some(path)).unwrap();

    assert_eq!(loaded.markers_for("file:movie.mp4"), vec![marker("a", 5.0), marker("b", 20.0)]);
}

#[test]
fn marker_store_missing_and_corrupt_files_load_empty() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing.toml");
    assert!(MarkerStore::load(Some(missing)).unwrap().items.is_empty());

    let corrupt = dir.path().join("corrupt.toml");
    std::fs::write(&corrupt, "not valid toml").unwrap();
    assert!(MarkerStore::load(Some(corrupt)).unwrap().items.is_empty());
}

#[test]
fn marker_store_path_uses_app_data_when_paths_exist() {
    assert!(marker_store_path(None).is_none());
}
