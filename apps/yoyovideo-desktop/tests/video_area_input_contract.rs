use std::time::{Duration, Instant};

use yoyovideo_desktop::{VideoAreaGesture, VideoAreaPointer};

const HOST_WIDTH: f64 = 800.0;
const HOST_HEIGHT: f64 = 450.0;

fn press_at(pointer: &mut VideoAreaPointer, now: Instant, x: f64, y: f64) {
    // Cursor position is only known from motion, so seed it before pressing.
    pointer.cursor_moved(x, y, 0, HOST_WIDTH, HOST_HEIGHT);
    pointer.pressed(now);
}

#[test]
fn small_movement_while_pressed_starts_no_gesture() {
    let mut pointer = VideoAreaPointer::default();
    press_at(&mut pointer, Instant::now(), 100.0, 100.0);

    assert_eq!(pointer.cursor_moved(101.0, 101.0, 0, HOST_WIDTH, HOST_HEIGHT), None);
}

#[test]
fn drag_without_zoom_moves_window_once_per_press() {
    let mut pointer = VideoAreaPointer::default();
    press_at(&mut pointer, Instant::now(), 100.0, 100.0);

    assert_eq!(
        pointer.cursor_moved(140.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT),
        Some(VideoAreaGesture::DragWindow)
    );
    // The OS drag loop owns the gesture from here, so do not ask again.
    assert_eq!(pointer.cursor_moved(180.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT), None);
}

#[test]
fn drag_while_zoomed_pans_picture_with_incremental_deltas() {
    let mut pointer = VideoAreaPointer::default();
    press_at(&mut pointer, Instant::now(), 100.0, 100.0);

    let first = pointer.cursor_moved(180.0, 100.0, 2, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(first, Some(VideoAreaGesture::PanPicture { delta_x: 0.1, delta_y: 0.0 }));

    // Same distance again yields the same delta: the value is incremental, not cumulative,
    // because AdjustVideoPan accumulates what it is given.
    let second = pointer.cursor_moved(260.0, 100.0, 2, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(second, Some(VideoAreaGesture::PanPicture { delta_x: 0.1, delta_y: 0.0 }));
}

#[test]
fn second_clean_click_toggles_fullscreen() {
    let mut pointer = VideoAreaPointer::default();
    let start = Instant::now();

    press_at(&mut pointer, start, 100.0, 100.0);
    assert_eq!(pointer.released(start + Duration::from_millis(30)), None);

    pointer.cursor_moved(100.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(
        pointer.pressed(start + Duration::from_millis(150)),
        Some(VideoAreaGesture::ToggleFullscreen)
    );
}

#[test]
fn slow_second_click_does_not_toggle_fullscreen() {
    let mut pointer = VideoAreaPointer::default();
    let start = Instant::now();

    press_at(&mut pointer, start, 100.0, 100.0);
    assert_eq!(pointer.released(start + Duration::from_millis(30)), None);

    pointer.cursor_moved(100.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(pointer.pressed(start + Duration::from_millis(900)), None);
}

#[test]
fn dragged_press_is_not_remembered_as_a_click() {
    let mut pointer = VideoAreaPointer::default();
    let start = Instant::now();

    press_at(&mut pointer, start, 100.0, 100.0);
    pointer.cursor_moved(160.0, 100.0, 2, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(pointer.released(start + Duration::from_millis(60)), None);

    pointer.cursor_moved(160.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(pointer.pressed(start + Duration::from_millis(120)), None);
}

#[test]
fn cancel_discards_the_press_so_no_click_is_recorded() {
    let mut pointer = VideoAreaPointer::default();
    let start = Instant::now();

    press_at(&mut pointer, start, 100.0, 100.0);
    assert_eq!(
        pointer.cursor_moved(150.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT),
        Some(VideoAreaGesture::DragWindow)
    );
    pointer.cancel();

    // A stray release after the OS drag loop must not leave a click behind.
    assert_eq!(pointer.released(start + Duration::from_millis(90)), None);
    pointer.cursor_moved(150.0, 100.0, 0, HOST_WIDTH, HOST_HEIGHT);
    assert_eq!(pointer.pressed(start + Duration::from_millis(140)), None);
}

#[test]
fn repeated_events_at_the_same_spot_are_not_real_movement() {
    let mut pointer = VideoAreaPointer::default();

    // Nothing seen yet, so the first event always counts.
    assert!(pointer.is_new_position(400.0, 300.0));
    pointer.cursor_moved(400.0, 300.0, 0, HOST_WIDTH, HOST_HEIGHT);

    // Resizing the video surface re-delivers CursorMoved at the same spot. Treating
    // that as activity makes the fullscreen chrome oscillate: hiding it grows the
    // video area, which re-emits this event, which re-shows the chrome.
    assert!(!pointer.is_new_position(400.0, 300.0));
    assert!(!pointer.is_new_position(400.4, 300.2), "sub-pixel jitter is not movement");
}

#[test]
fn genuine_movement_is_reported_as_new_position() {
    let mut pointer = VideoAreaPointer::default();
    pointer.cursor_moved(400.0, 300.0, 0, HOST_WIDTH, HOST_HEIGHT);

    assert!(pointer.is_new_position(402.0, 300.0));
    assert!(pointer.is_new_position(400.0, 305.0));
}
