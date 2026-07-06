use tempfile::tempdir;
use yoyovideo_desktop::platform::{
    MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, WindowState, load_window_state, save_window_state,
};

#[test]
fn window_state_clamps_too_small_sizes() {
    let state = WindowState { width: 100, height: 100, x: Some(20), y: Some(30), maximized: false }
        .clamped();

    assert_eq!(state.width, MIN_WINDOW_WIDTH);
    assert_eq!(state.height, MIN_WINDOW_HEIGHT);
    assert_eq!(state.x, Some(20));
    assert_eq!(state.y, Some(30));
}

#[test]
fn window_state_round_trips_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("window-state.toml");
    let state = WindowState { width: 1280, height: 720, x: Some(10), y: Some(20), maximized: true };

    save_window_state(Some(path.clone()), &state).unwrap();
    let loaded = load_window_state(Some(path)).unwrap().unwrap();

    assert_eq!(loaded, state);
}

#[test]
fn window_state_missing_file_returns_none() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing.toml");

    assert_eq!(load_window_state(Some(path)).unwrap(), None);
}

#[test]
fn window_state_corrupt_file_returns_none() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("window-state.toml");
    std::fs::write(&path, "not valid toml").unwrap();

    assert_eq!(load_window_state(Some(path)).unwrap(), None);
}
