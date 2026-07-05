use yoyovideo_desktop::{LogicalVideoRect, UnsupportedVideoHost, VideoHost, VideoHostBounds};

#[test]
fn logical_rect_converts_to_physical_bounds() {
    let rect = LogicalVideoRect { x: 10.0, y: 20.0, width: 300.0, height: 200.0 };

    assert_eq!(
        rect.to_physical(1.5),
        VideoHostBounds { x: 15, y: 30, width: 450, height: 300 }
    );
}

#[test]
fn unsupported_host_reports_clear_failure() {
    let mut host =
        UnsupportedVideoHost::new("Video embedding is not supported on this windowing backend yet");

    assert!(!host.is_available());
    assert!(host.mpv_window_id().is_err());
    assert!(host.show().is_err());
}
