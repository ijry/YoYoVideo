use slint::winit_030::WinitWindowAccessor;

use super::{DisplayBounds, WindowState};

/// One-shot restoration, deferred until the native window actually exists.
/// `show()` alone is insufficient: winit can create the window only after the
/// event loop resumes. Call `apply` from the window's native event callback.
pub struct PendingWindowRestore {
    saved: Option<WindowState>,
}

impl PendingWindowRestore {
    pub fn new(saved: Option<WindowState>) -> Self {
        Self { saved }
    }

    /// Returns true when geometry is ready to be persisted. A false result means
    /// native creation is still pending and this must be retried on a later event.
    pub fn apply(&mut self, window: &slint::Window) -> bool {
        let Some(saved) = self.saved.as_ref() else {
            return true;
        };
        let Some(displays) = window.with_winit_window(|native| {
            let convert = |monitor: slint::winit_030::winit::monitor::MonitorHandle| {
                let origin = monitor.position();
                let size = monitor.size();
                DisplayBounds { x: origin.x, y: origin.y, width: size.width, height: size.height }
            };
            let mut displays = Vec::new();
            if let Some(primary) = native.primary_monitor() {
                displays.push(convert(primary));
            }
            for monitor in native.available_monitors() {
                let bounds = convert(monitor);
                if !displays.contains(&bounds) {
                    displays.push(bounds);
                }
            }
            displays
        }) else {
            return false;
        };
        let restored = saved.clone().restored_on(&displays);
        self.saved = None;
        // Release the native-window borrow before invoking Slint setters.
        window.set_size(slint::PhysicalSize::new(restored.width, restored.height));
        if let (Some(x), Some(y)) = (restored.x, restored.y) {
            window.set_position(slint::PhysicalPosition::new(x, y));
        }
        window.set_maximized(restored.maximized);
        true
    }
}
