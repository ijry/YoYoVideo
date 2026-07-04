use yoyo_mpv::{MpvRenderBridge, RenderTarget};

#[test]
fn render_target_keeps_dimensions() {
    let target = RenderTarget {
        framebuffer_object: 7,
        width: 1280,
        height: 720,
        flipped: false,
    };

    assert_eq!(target.width, 1280);
    assert_eq!(target.height, 720);
}

#[test]
fn render_bridge_starts_without_pending_redraw() {
    let bridge = MpvRenderBridge::default();
    assert!(!bridge.needs_redraw());
}
