use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use yoyo_core::{PlaybackAccess, privacy::MediaKey};

type MediaAccess = (Arc<dyn PlaybackAccess>, Option<MediaKey>);

#[derive(Default)]
struct State {
    requested: AtomicBool,
    privacy_blocked: AtomicBool,
    media: Mutex<Option<MediaAccess>>,
}

/// Queued native work captures this shared permit, never a stale boolean.
/// Policy is rechecked at the actual output boundary as well as in the UI timer.
#[derive(Clone, Default)]
pub struct VisibilityPermit(Arc<State>);
impl VisibilityPermit {
    pub fn request_visible(&self, visible: bool) {
        self.0.requested.store(visible, Ordering::Release);
    }
    pub fn set_privacy_blocked(&self, blocked: bool) -> bool {
        self.0.privacy_blocked.swap(blocked, Ordering::AcqRel) != blocked
    }
    pub fn set_media_access(
        &self,
        access: Option<Arc<dyn PlaybackAccess>>,
        media: Option<MediaKey>,
    ) {
        if let Ok(mut binding) = self.0.media.lock() {
            *binding = access.map(|access| (access, media));
        }
    }
    pub fn visible(&self) -> bool {
        if !self.0.requested.load(Ordering::Acquire)
            || self.0.privacy_blocked.load(Ordering::Acquire)
        {
            return false;
        }
        let Ok(binding) = self.0.media.lock() else {
            return false;
        };
        match &*binding {
            Some((access, Some(media))) => !access.restricted(media),
            _ => true,
        }
    }
}
