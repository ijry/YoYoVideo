//! AppKit child windows are not Win32/X11 client-area subwindows.
//! They use screen coordinates, and orderOut: also removes their parent.
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow, NSWindowOrderingMode};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::window::Window;

use crate::{VideoHostBounds, VideoHostError};

pub(crate) struct MacVideoWindow {
    parent: Retained<NSWindow>,
    child: Retained<NSWindow>,
}

fn native_window(window: &Window) -> Result<Retained<NSWindow>, VideoHostError> {
    let handle = window.window_handle().map_err(|error| VideoHostError::new(error.to_string()))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err(VideoHostError::new("expected an AppKit video window"));
    };
    // SAFETY: winit owns this live NSView; all callers hold a MainThreadMarker.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    view.window().ok_or_else(|| VideoHostError::new("native view has no window"))
}

fn rect_in_view(bounds: VideoHostBounds, scale: f64, view: NSRect, flipped: bool) -> NSRect {
    let x = f64::from(bounds.x) / scale;
    let y = f64::from(bounds.y) / scale;
    let width = f64::from(bounds.width) / scale;
    let height = f64::from(bounds.height) / scale;
    NSRect::new(
        NSPoint::new(
            view.origin.x + x,
            view.origin.y + if flipped { y } else { view.size.height - y - height },
        ),
        NSSize::new(width, height),
    )
}

impl MacVideoWindow {
    pub(crate) fn new(
        parent: &Window,
        child: &Window,
        _mtm: MainThreadMarker,
    ) -> Result<Self, VideoHostError> {
        let parent = native_window(parent)?;
        let child = native_window(child)?;
        if std::ptr::eq(&*parent, &*child) {
            return Err(VideoHostError::new("a video host cannot parent itself"));
        }
        Ok(Self { parent, child })
    }

    pub(crate) fn set_bounds(&self, bounds: VideoHostBounds) -> Result<(), VideoHostError> {
        let Some(view) = self.parent.contentView() else {
            self.child.orderOut(None);
            return Err(VideoHostError::new("main window content view is unavailable"));
        };
        let scale = self.parent.backingScaleFactor();
        if !scale.is_finite() || scale <= 0.0 {
            self.child.orderOut(None);
            return Err(VideoHostError::new("main window has an invalid backing scale"));
        }
        let rect = rect_in_view(bounds, scale, view.bounds(), view.isFlipped());
        let in_window = view.convertRect_toView(rect, None);
        let on_screen = self.parent.convertRectToScreen(in_window);
        if self.child.frame() != on_screen {
            // The host is borderless, so its content frame and outer frame agree.
            // One native operation also avoids transient position/size mismatches.
            self.child.setFrame_display(on_screen, true);
        }
        Ok(())
    }

    pub(crate) fn set_visible(&self, requested: bool) {
        let parent = self.child.parentWindow();
        if !requested || !self.parent.isVisible() || self.parent.isMiniaturized() {
            if self.child.isVisible() || parent.is_some() {
                // Also detach an already-invisible child: otherwise restoring the
                // parent could briefly reveal privacy-suppressed video.
                self.child.orderOut(None);
            }
            return;
        }
        let attached = parent.as_ref().is_some_and(|parent| std::ptr::eq(&**parent, &*self.parent));
        let was_visible = self.child.isVisible();
        if !attached {
            if let Some(previous) = parent {
                previous.removeChildWindow(&self.child);
            }
            // SAFETY: the original main window and newly-created child are distinct;
            // the app never makes the main window a descendant of a video host.
            unsafe { self.parent.addChildWindow_ordered(&self.child, NSWindowOrderingMode::Above) };
        }
        if !was_visible || !attached {
            // Unlike winit::set_visible(true), this does not make the video window
            // key, activate a background app, or steal focus from a PIN/menu window.
            let parent_number = self.parent.windowNumber();
            self.child.orderWindow_relativeTo(NSWindowOrderingMode::Above, parent_number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_left_video_rect_is_flipped_into_appkit_content_coordinates() {
        let bounds = VideoHostBounds { x: 24, y: 62, width: 752, height: 448 };
        let view = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0));
        assert_eq!(
            rect_in_view(bounds, 1.0, view, false),
            NSRect::new(NSPoint::new(24.0, 90.0), NSSize::new(752.0, 448.0))
        );
    }

    #[test]
    fn retina_pixels_are_converted_using_the_parent_not_the_detached_host() {
        let bounds = VideoHostBounds { x: 48, y: 124, width: 1504, height: 896 };
        let view = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0));
        assert_eq!(
            rect_in_view(bounds, 2.0, view, false),
            NSRect::new(NSPoint::new(24.0, 90.0), NSSize::new(752.0, 448.0))
        );
    }

    #[test]
    fn already_flipped_views_and_nonzero_bounds_origins_are_respected() {
        let bounds = VideoHostBounds { x: 20, y: 40, width: 600, height: 400 };
        let view = NSRect::new(NSPoint::new(3.0, -4.0), NSSize::new(800.0, 600.0));
        assert_eq!(
            rect_in_view(bounds, 2.0, view, true),
            NSRect::new(NSPoint::new(13.0, 16.0), NSSize::new(300.0, 200.0))
        );
    }
}
