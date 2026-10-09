use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use yoyo_core::{MediaLocator, PlaybackAccess, privacy::MediaKey};
use yoyovideo_desktop::{SuppressionAction, VideoHostSuppression, VisibilityPermit};

struct Gate(AtomicBool);
impl PlaybackAccess for Gate {
    fn restricted(&self, _: &MediaKey) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
#[test]
fn a_queued_show_reads_the_latest_privacy_permission() {
    let permit = VisibilityPermit::default();
    permit.request_visible(true);
    let queued = permit.clone();
    permit.set_privacy_blocked(true);
    assert!(!queued.visible());
    permit.request_visible(false);
    permit.set_privacy_blocked(false);
    assert!(!queued.visible());
    permit.request_visible(true);
    assert!(queued.visible());
}
#[test]
fn queued_native_work_rechecks_policy_even_before_the_ui_timer_updates() {
    let permit = VisibilityPermit::default();
    let gate = Arc::new(Gate(AtomicBool::new(false)));
    permit.set_media_access(
        Some(gate.clone()),
        Some(
            MediaKey::from_locator(&MediaLocator::Url("https://example.test/video".into()))
                .unwrap(),
        ),
    );
    permit.request_visible(true);
    assert!(permit.visible());
    let queued = permit.clone();
    gate.0.store(true, Ordering::SeqCst);
    assert!(!queued.visible());
}
#[test]
fn closing_a_popup_cannot_clear_privacy_suppression() {
    let mut suppression = VideoHostSuppression::default();
    assert_eq!(suppression.request_privacy(true), Some(SuppressionAction::Hide));
    assert_eq!(suppression.request(true), None);
    assert_eq!(suppression.request(false), None);
    assert!(suppression.is_suppressed());
    assert_eq!(suppression.request_privacy(false), Some(SuppressionAction::Reveal));
}
#[test]
fn unlocking_privacy_cannot_close_a_popup_suppression_reason() {
    let mut suppression = VideoHostSuppression::default();
    suppression.request(true);
    suppression.request_privacy(true);
    assert_eq!(suppression.request_privacy(false), None);
    assert!(suppression.is_suppressed());
    assert_eq!(suppression.request(false), Some(SuppressionAction::Reveal));
}
#[cfg(feature = "mpv-runtime")]
#[test]
fn grid_does_not_queue_denied_items_or_enter_grid_mode_for_a_denied_batch() {
    let mut grid = yoyovideo_desktop::GridRuntime::default();
    grid.set_playback_access(Arc::new(Gate(AtomicBool::new(true))));
    let dropped = grid.queue_open(vec![MediaLocator::Url("https://example.test/video".into())]);
    assert_eq!(dropped, 1);
    assert!(!grid.has_pending());
    assert!(!grid.is_active());
}
