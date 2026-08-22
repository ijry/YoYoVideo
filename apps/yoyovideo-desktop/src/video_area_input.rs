use std::time::{Duration, Instant};

/// Movement (physical px) needed while pressed before a drag gesture starts.
const DRAG_THRESHOLD_PX: f64 = 3.0;
/// Movement (physical px) a press may accumulate and still count as a click.
const CLICK_SLOP_PX: f64 = 4.0;
/// Maximum gap between two clicks for them to count as a double click.
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(400);
/// Maximum distance between two clicks for them to count as a double click.
const DOUBLE_CLICK_SLOP_PX: f64 = 8.0;
/// Movement below this is treated as the pointer standing still.
const REAL_MOVE_EPSILON_PX: f64 = 1.0;

/// What a pointer interaction over the native video surface should do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoAreaGesture {
    /// Hand the press off to the OS window-move loop on the parent window.
    DragWindow,
    /// Pan the zoomed picture by these deltas, normalized to the video area.
    PanPicture { delta_x: f64, delta_y: f64 },
    /// Second clean click within the double-click window.
    ToggleFullscreen,
}

#[derive(Debug, Clone, Copy)]
struct ActivePress {
    last_x: f64,
    last_y: f64,
    origin_x: f64,
    origin_y: f64,
    max_travel: f64,
    gesture_started: bool,
}

/// Tracks pointer state over the video surface and decides between moving the
/// window, panning a zoomed picture, and toggling fullscreen.
///
/// The video surface is a native child window, so these events never reach Slint
/// and the decision has to be made from raw winit events.
#[derive(Debug, Clone, Copy, Default)]
pub struct VideoAreaPointer {
    cursor: Option<(f64, f64)>,
    press: Option<ActivePress>,
    last_click: Option<(Instant, f64, f64)>,
}

fn distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

impl VideoAreaPointer {
    /// Whether this position differs from the last one seen.
    ///
    /// Resizing the video surface re-delivers `CursorMoved` at the same screen spot, so
    /// treating every event as pointer activity makes auto-hiding the fullscreen chrome
    /// oscillate: hiding grows the video area, which re-emits the event, which re-shows
    /// the chrome. Only genuine movement should count.
    pub fn is_new_position(&self, x: f64, y: f64) -> bool {
        match self.cursor {
            Some((last_x, last_y)) => distance(x, y, last_x, last_y) >= REAL_MOVE_EPSILON_PX,
            None => true,
        }
    }

    /// Records the latest cursor position, in physical pixels relative to the video
    /// surface. Returns a gesture once a press has travelled far enough to be a drag.
    pub fn cursor_moved(
        &mut self,
        x: f64,
        y: f64,
        zoom_step: i8,
        host_width: f64,
        host_height: f64,
    ) -> Option<VideoAreaGesture> {
        self.cursor = Some((x, y));
        let press = self.press.as_mut()?;

        let (previous_x, previous_y) = (press.last_x, press.last_y);
        press.last_x = x;
        press.last_y = y;
        press.max_travel = press
            .max_travel
            .max(distance(x, y, press.origin_x, press.origin_y));

        if press.max_travel < DRAG_THRESHOLD_PX {
            return None;
        }

        if zoom_step == 0 {
            // Only ask the OS to move the window once per press.
            if press.gesture_started {
                return None;
            }
            press.gesture_started = true;
            return Some(VideoAreaGesture::DragWindow);
        }

        press.gesture_started = true;
        // Incremental, because AdjustVideoPan accumulates what it is given.
        Some(VideoAreaGesture::PanPicture {
            delta_x: (x - previous_x) / host_width.max(1.0),
            delta_y: (y - previous_y) / host_height.max(1.0),
        })
    }

    /// Records a left-button press. Returns [`VideoAreaGesture::ToggleFullscreen`] when
    /// this is the second clean click inside the double-click window.
    pub fn pressed(&mut self, now: Instant) -> Option<VideoAreaGesture> {
        let (x, y) = self.cursor.unwrap_or((0.0, 0.0));
        self.press = Some(ActivePress {
            last_x: x,
            last_y: y,
            origin_x: x,
            origin_y: y,
            max_travel: 0.0,
            gesture_started: false,
        });

        let (clicked_at, click_x, click_y) = self.last_click?;
        if now.duration_since(clicked_at) <= DOUBLE_CLICK_WINDOW
            && distance(x, y, click_x, click_y) <= DOUBLE_CLICK_SLOP_PX
        {
            // Consume it so a third click does not toggle again.
            self.last_click = None;
            self.press = None;
            return Some(VideoAreaGesture::ToggleFullscreen);
        }

        self.last_click = None;
        None
    }

    /// Records a left-button release, remembering it as a click when the press barely
    /// moved. Never produces a gesture on its own.
    pub fn released(&mut self, now: Instant) -> Option<VideoAreaGesture> {
        let press = self.press.take()?;
        if press.max_travel <= CLICK_SLOP_PX {
            self.last_click = Some((now, press.origin_x, press.origin_y));
        }
        None
    }

    /// Forgets the active press without recording a click. Call this after handing the
    /// press to the OS drag loop, which swallows the release, and when the cursor leaves.
    pub fn cancel(&mut self) {
        self.press = None;
    }
}
