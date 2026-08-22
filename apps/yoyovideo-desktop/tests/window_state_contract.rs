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
fn window_state_drops_offscreen_minimized_position() {
    // Windows reports a minimized window at (-32000, -32000). Persisting that reopens
    // the window off-screen where the user cannot reach it.
    let state =
        WindowState { width: 1200, height: 760, x: Some(-32000), y: Some(-32000), maximized: false }
            .clamped();

    assert_eq!(state.x, None);
    assert_eq!(state.y, None);
    assert_eq!(state.width, 1200, "size is still preserved");
    assert_eq!(state.height, 760);
}

#[test]
fn window_state_drops_position_when_either_axis_is_offscreen() {
    let state =
        WindowState { width: 1200, height: 760, x: Some(400), y: Some(-32000), maximized: false }
            .clamped();

    assert_eq!((state.x, state.y), (None, None), "a half-valid position is still unreachable");
}

#[test]
fn window_state_keeps_small_negative_positions() {
    // A window nudged slightly off the top-left edge, or on a secondary monitor placed
    // left of the primary, is still legitimately reachable.
    let state =
        WindowState { width: 1200, height: 760, x: Some(-40), y: Some(-10), maximized: false }
            .clamped();

    assert_eq!(state.x, Some(-40));
    assert_eq!(state.y, Some(-10));
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
