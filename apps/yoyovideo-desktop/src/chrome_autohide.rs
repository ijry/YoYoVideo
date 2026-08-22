use std::time::{Duration, Instant};

/// How long the pointer must sit still before the fullscreen chrome hides.
pub const CHROME_IDLE_HIDE_DELAY: Duration = Duration::from_millis(3000);

/// Pointer events arriving this soon after a visibility change are ignored.
///
/// Showing or hiding the bars resizes the video area, and the native video surface
/// re-delivers `CursorMoved` for a pointer that never actually moved. Without this
/// guard, hiding immediately re-shows the chrome and the two fight forever.
pub const CHROME_SETTLE: Duration = Duration::from_millis(400);

/// What should happen to the title bar and control deck right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeAction {
    /// Show the chrome and arm the idle timer for [`CHROME_IDLE_HIDE_DELAY`].
    ShowAndArm,
    /// Show the chrome and leave the timer disarmed (windowed mode never hides).
    ShowAndDisarm,
    /// Hide the chrome now.
    Hide,
    /// Leave things as they are.
    Nothing,
}

impl ChromeAction {
    /// Whether applying this action changes what is on screen, and therefore starts a
    /// settle window during which synthetic pointer events must be ignored.
    pub fn changes_visibility(self) -> bool {
        matches!(self, ChromeAction::Hide | ChromeAction::ShowAndArm | ChromeAction::ShowAndDisarm)
    }
}

/// Decides when the fullscreen chrome shows and hides.
///
/// Kept free of Slint and timer types so the policy is directly testable; the caller
/// owns the actual `slint::Timer` and the property writes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChromeAutoHide {
    settle_until: Option<Instant>,
}

impl ChromeAutoHide {
    /// Opens a settle window, so layout-induced pointer events are ignored.
    pub fn note_visibility_changed(&mut self, now: Instant) {
        self.settle_until = Some(now + CHROME_SETTLE);
    }

    fn settling(&self, now: Instant) -> bool {
        self.settle_until.is_some_and(|until| now < until)
    }

    /// Pointer moved somewhere over the window, or over the native video surface.
    pub fn on_pointer_activity(&self, now: Instant, fullscreen: bool) -> ChromeAction {
        if self.settling(now) {
            return ChromeAction::Nothing;
        }
        if fullscreen { ChromeAction::ShowAndArm } else { ChromeAction::ShowAndDisarm }
    }

    /// The idle timer elapsed.
    ///
    /// `chrome_hovered` keeps the deck alive while the pointer rests on it without
    /// moving, so controls never vanish from under the cursor.
    pub fn on_idle_elapsed(&self, fullscreen: bool, chrome_hovered: bool) -> ChromeAction {
        if fullscreen && !chrome_hovered { ChromeAction::Hide } else { ChromeAction::Nothing }
    }

    /// Fullscreen was entered or left.
    pub fn on_fullscreen_changed(&self, fullscreen: bool) -> ChromeAction {
        // Entering fullscreen still shows the chrome first, so the user can see where
        // the controls went before they fade out.
        if fullscreen { ChromeAction::ShowAndArm } else { ChromeAction::ShowAndDisarm }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, millis: u64) -> Instant {
        base + Duration::from_millis(millis)
    }

    #[test]
    fn windowed_pointer_activity_never_arms_the_timer() {
        let policy = ChromeAutoHide::default();
        assert_eq!(
            policy.on_pointer_activity(Instant::now(), false),
            ChromeAction::ShowAndDisarm
        );
    }

    #[test]
    fn fullscreen_pointer_activity_shows_and_arms() {
        let policy = ChromeAutoHide::default();
        assert_eq!(policy.on_pointer_activity(Instant::now(), true), ChromeAction::ShowAndArm);
    }

    #[test]
    fn idle_hides_only_in_fullscreen() {
        let policy = ChromeAutoHide::default();
        assert_eq!(policy.on_idle_elapsed(true, false), ChromeAction::Hide);
        assert_eq!(policy.on_idle_elapsed(false, false), ChromeAction::Nothing);
    }

    #[test]
    fn idle_does_not_hide_chrome_under_the_pointer() {
        let policy = ChromeAutoHide::default();
        assert_eq!(policy.on_idle_elapsed(true, true), ChromeAction::Nothing);
    }

    #[test]
    fn leaving_fullscreen_restores_the_chrome() {
        let policy = ChromeAutoHide::default();
        assert_eq!(policy.on_fullscreen_changed(false), ChromeAction::ShowAndDisarm);
    }

    #[test]
    fn entering_fullscreen_shows_chrome_then_arms() {
        let policy = ChromeAutoHide::default();
        assert_eq!(policy.on_fullscreen_changed(true), ChromeAction::ShowAndArm);
    }

    #[test]
    fn pointer_events_right_after_hiding_are_ignored() {
        // Hiding resizes the video area, which re-delivers CursorMoved for a pointer
        // that never moved. Acting on it would re-show the chrome instantly.
        let base = Instant::now();
        let mut policy = ChromeAutoHide::default();
        policy.note_visibility_changed(base);

        assert_eq!(policy.on_pointer_activity(base, true), ChromeAction::Nothing);
        assert_eq!(policy.on_pointer_activity(at(base, 399), true), ChromeAction::Nothing);
    }

    #[test]
    fn pointer_events_after_the_settle_window_show_the_chrome_again() {
        let base = Instant::now();
        let mut policy = ChromeAutoHide::default();
        policy.note_visibility_changed(base);

        assert_eq!(policy.on_pointer_activity(at(base, 400), true), ChromeAction::ShowAndArm);
        assert_eq!(policy.on_pointer_activity(at(base, 5000), true), ChromeAction::ShowAndArm);
    }

    #[test]
    fn settling_does_not_block_the_idle_timer() {
        // The timer fires 3s after the last activity, well past the settle window, but
        // the two must stay independent so a hide is never skipped.
        let base = Instant::now();
        let mut policy = ChromeAutoHide::default();
        policy.note_visibility_changed(base);

        assert_eq!(policy.on_idle_elapsed(true, false), ChromeAction::Hide);
    }

    #[test]
    fn only_visibility_changing_actions_open_a_settle_window() {
        assert!(ChromeAction::Hide.changes_visibility());
        assert!(ChromeAction::ShowAndArm.changes_visibility());
        assert!(ChromeAction::ShowAndDisarm.changes_visibility());
        assert!(!ChromeAction::Nothing.changes_visibility());
    }
}
