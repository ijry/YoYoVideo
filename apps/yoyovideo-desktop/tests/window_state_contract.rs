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
    let state = WindowState {
        width: 1200,
        height: 760,
        x: Some(-32000),
        y: Some(-32000),
        maximized: false,
    }
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

#[test]
fn restore_rehomes_the_reported_far_offscreen_and_oversized_window() {
    use yoyovideo_desktop::platform::DisplayBounds;
    let state =
        WindowState { width: 2910, height: 1650, x: Some(11830), y: Some(1001), maximized: false };
    let restored = state.restored_on(&[DisplayBounds { x: 0, y: 0, width: 1960, height: 1231 }]);
    assert_eq!(
        restored,
        WindowState { width: 1960, height: 1231, x: Some(0), y: Some(0), maximized: false }
    );
}

#[test]
fn restore_centers_a_window_from_a_disconnected_monitor() {
    use yoyovideo_desktop::platform::DisplayBounds;
    let state =
        WindowState { width: 1200, height: 760, x: Some(8000), y: Some(80), maximized: true };
    let restored = state.restored_on(&[DisplayBounds { x: 0, y: 0, width: 1920, height: 1080 }]);
    assert_eq!((restored.x, restored.y), (Some(360), Some(160)));
    assert!(restored.maximized);
}

#[test]
fn restore_preserves_a_valid_window_on_a_negative_coordinate_monitor() {
    use yoyovideo_desktop::platform::DisplayBounds;
    let state =
        WindowState { width: 1200, height: 760, x: Some(-2200), y: Some(80), maximized: false };
    let displays = [
        DisplayBounds { x: 0, y: 0, width: 1920, height: 1080 },
        DisplayBounds { x: -2560, y: 0, width: 2560, height: 1440 },
    ];
    assert_eq!(state.clone().restored_on(&displays), state);
}

#[test]
fn restore_does_not_treat_the_gap_between_monitors_as_a_screen() {
    use yoyovideo_desktop::platform::DisplayBounds;
    let state =
        WindowState { width: 900, height: 560, x: Some(2400), y: Some(50), maximized: false };
    let displays = [
        DisplayBounds { x: 0, y: 0, width: 1920, height: 1080 },
        DisplayBounds { x: 3840, y: 0, width: 1920, height: 1080 },
    ];
    let restored = state.restored_on(&displays);
    assert_eq!((restored.x, restored.y), (Some(510), Some(260)));
}

#[test]
fn restore_brings_an_unreachable_title_bar_back_onto_its_monitor() {
    use yoyovideo_desktop::platform::DisplayBounds;
    let state =
        WindowState { width: 1200, height: 760, x: Some(1850), y: Some(-700), maximized: false };
    let restored = state.restored_on(&[DisplayBounds { x: 0, y: 0, width: 1920, height: 1080 }]);
    assert_eq!((restored.x, restored.y), (Some(720), Some(0)));
}

#[test]
fn restore_handles_extreme_persisted_values_without_overflow() {
    use yoyovideo_desktop::platform::DisplayBounds;
    let state = WindowState {
        width: u32::MAX,
        height: u32::MAX,
        x: Some(i32::MAX),
        y: Some(i32::MAX),
        maximized: false,
    };
    let restored =
        state.restored_on(&[DisplayBounds { x: -1920, y: 0, width: 1920, height: 1080 }]);
    assert_eq!(
        (restored.width, restored.height, restored.x, restored.y),
        (1920, 1080, Some(-1920), Some(0))
    );
}

#[test]
fn restore_leaves_placement_to_the_window_system_when_monitors_are_unavailable() {
    let state =
        WindowState { width: 1200, height: 760, x: Some(11830), y: Some(1001), maximized: false };
    let restored = state.restored_on(&[]);
    assert_eq!((restored.x, restored.y), (None, None));
    assert_eq!((restored.width, restored.height), (1200, 760));
}
