use super::*;
use tempfile::tempdir;
fn runtime_for(history_path: PathBuf, marker_path: PathBuf) -> DesktopRuntime {
    DesktopRuntime::new(
        AppConfig::default(),
        crate::HistoryRuntime::new(Some(history_path), HistoryStore::default(), true),
        crate::platform::RecentOpenStore::load(None).unwrap(),
        crate::SubtitlePrefsRuntime::load(None).unwrap(),
        crate::platform::MarkerStore::with_path(Some(marker_path)),
        crate::initial_sidebar_state(false, 800.0),
        PathBuf::from("unused.log"),
        None,
    )
}
#[test]
fn update_shutdown_flushes_existing_dirty_history_and_marker_store() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let markers = dir.path().join("markers.toml");
    let mut rt = runtime_for(path.clone(), markers.clone());
    rt.history.remember_playback(
        &MediaLocator::Url("https://example.com/movie.mp4".into()),
        "Movie",
        Some(45.0),
    );
    persist_playback_for_shutdown(&mut rt).unwrap();
    let history = HistoryStore::load(&path).unwrap();
    assert_eq!(history.items[0].last_position_seconds, Some(45.0));
    assert!(markers.exists());
}
#[test]
fn update_shutdown_propagates_storage_failure_instead_of_ignoring_it() {
    let dir = tempdir().unwrap();
    let blocker = dir.path().join("blocked");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let mut rt = runtime_for(blocker.join("history.json"), dir.path().join("markers.toml"));
    rt.history.remember_playback(
        &MediaLocator::Url("https://example.com/movie.mp4".into()),
        "Movie",
        Some(45.0),
    );
    assert!(persist_playback_for_shutdown(&mut rt).is_err());
    assert!(dir.path().join("markers.toml").exists(), "other stores still get a save attempt");
}

#[test]
fn empty_unchanged_markers_do_not_create_or_evict_marker_entries() {
    let dir = tempdir().unwrap();
    let mut rt = runtime_for(dir.path().join("history.json"), dir.path().join("markers.toml"));
    let state = PlayerState {
        current: Some(MediaLocator::Url("https://example.com/movie.mp4".into())),
        ..Default::default()
    };
    rt.last_marker_locator_key = current_locator_key(&state);
    remember_shutdown_markers(&mut rt, &state);
    assert!(rt.marker_store.items.is_empty());
}
#[test]
fn markers_not_yet_restored_are_preserved_and_restored_edits_are_saved() {
    let dir = tempdir().unwrap();
    let mut rt = runtime_for(dir.path().join("history.json"), dir.path().join("markers.toml"));
    let mut state = PlayerState {
        current: Some(MediaLocator::Url("https://example.com/movie.mp4".into())),
        ..Default::default()
    };
    let key = current_locator_key(&state).unwrap();
    let marker = yoyo_core::MediaMarker {
        id: "m1".into(),
        title: "Scene".into(),
        time_seconds: 30.0,
        created_at: "2026-10-07T00:00:00Z".into(),
    };
    rt.marker_store.set_markers(key.clone(), vec![marker.clone()]);
    remember_shutdown_markers(&mut rt, &state);
    assert_eq!(rt.marker_store.markers_for(&key), vec![marker.clone()]);
    rt.last_marker_locator_key = Some(key.clone());
    state.markers = vec![yoyo_core::MediaMarker { time_seconds: 40.0, ..marker }];
    remember_shutdown_markers(&mut rt, &state);
    assert_eq!(rt.marker_store.markers_for(&key), state.markers);
}
