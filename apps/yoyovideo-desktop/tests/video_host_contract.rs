use yoyovideo_desktop::{
    LogicalVideoRect, SuppressionAction, UnsupportedVideoHost, VideoHost, VideoHostBounds,
    VideoHostSuppression,
};

#[test]
fn logical_rect_converts_to_physical_bounds() {
    let rect = LogicalVideoRect { x: 10.0, y: 20.0, width: 300.0, height: 200.0 };

    assert_eq!(rect.to_physical(1.5), VideoHostBounds { x: 15, y: 30, width: 450, height: 300 });
}

#[test]
fn unsupported_host_reports_clear_failure() {
    let mut host =
        UnsupportedVideoHost::new("Video embedding is not supported on this windowing backend yet");

    assert!(!host.is_available());
    assert!(host.mpv_window_id().is_err());
    assert!(host.show().is_err());
}

#[test]
fn suppression_starts_disabled_and_reports_transitions() {
    let mut suppression = VideoHostSuppression::default();
    assert!(!suppression.is_suppressed(), "video surface is visible until a popup opens");

    assert_eq!(suppression.request(true), Some(SuppressionAction::Hide));
    assert!(suppression.is_suppressed());

    assert_eq!(suppression.request(false), Some(SuppressionAction::Reveal));
    assert!(!suppression.is_suppressed());
}

#[test]
fn repeating_the_current_suppression_state_is_a_no_op() {
    let mut suppression = VideoHostSuppression::default();
    suppression.request(true);

    // Switching straight from one popup to another must not flash the video back on.
    assert_eq!(suppression.request(true), None);
    assert!(suppression.is_suppressed(), "still suppressed while a popup is open");

    suppression.request(false);
    assert_eq!(suppression.request(false), None);
}

#[test]
fn suppressed_surface_stays_hidden_across_repeated_bounds_syncs() {
    // Bounds syncing runs on a 250ms timer and always shows the surface, so callers must
    // skip it while suppressed. This encodes that contract.
    let mut suppression = VideoHostSuppression::default();
    suppression.request(true);

    let mut shows = 0;
    for _ in 0..8 {
        if !suppression.is_suppressed() {
            shows += 1;
        }
    }

    assert_eq!(shows, 0, "a suppressed surface must never be shown by bounds sync");
}
