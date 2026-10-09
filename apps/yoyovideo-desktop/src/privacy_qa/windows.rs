//! Reads only a few pixels of this QA process's visible, foreground-owned surface.
//! There is no desktop capture or sampling of another process's windows.
use serde_json::{Value, json};
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn GetClientRect(hwnd: isize, rect: *mut Rect) -> i32;
    fn ClientToScreen(hwnd: isize, point: *mut Point) -> i32;
    fn WindowFromPoint(point: Point) -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn GetForegroundWindow() -> isize;
    fn GetParent(hwnd: isize) -> isize;
    fn GetAncestor(hwnd: isize, flags: u32) -> isize;
    fn GetDC(hwnd: isize) -> isize;
    fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetPixel(dc: isize, x: i32, y: i32) -> u32;
}

pub(super) fn color(hwnd: u64) -> Option<Value> {
    let hwnd = hwnd as isize;
    // SAFETY: all handles are borrowed from live QA-owned winit windows. We
    // validate visibility, foreground ancestry and point ownership before sampling.
    unsafe {
        if IsWindowVisible(hwnd) == 0
            || GetAncestor(GetForegroundWindow(), 2) != GetAncestor(hwnd, 2)
        {
            return None;
        }
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 || rect.right < 8 || rect.bottom < 8 {
            return None;
        }
        let dc = GetDC(0);
        if dc == 0 {
            return None;
        }
        let mut red = 0;
        let mut blue = 0;
        let mut samples = 0;
        for x in [1, 2, 3] {
            for y in [1, 2, 3] {
                let mut point = Point { x: rect.right * x / 4, y: rect.bottom * y / 4 };
                if ClientToScreen(hwnd, &mut point) == 0 {
                    continue;
                }
                let owner = WindowFromPoint(point);
                let mut pid = 0;
                GetWindowThreadProcessId(owner, &mut pid);
                if pid != std::process::id() {
                    continue;
                }
                let pixel = GetPixel(dc, point.x, point.y);
                if pixel == u32::MAX {
                    continue;
                }
                let r = pixel & 255;
                let g = (pixel >> 8) & 255;
                let b = (pixel >> 16) & 255;
                samples += 1;
                if r > 150 && g < 100 && b < 100 {
                    red += 1;
                }
                if b > 150 && r < 100 && g < 100 {
                    blue += 1;
                }
            }
        }
        ReleaseDC(0, dc);
        Some(json!({"samples":samples,"red":red,"blue":blue}))
    }
}

pub(super) fn has_parent(hwnd: u64, parent: u64) -> bool {
    unsafe { GetParent(hwnd as isize) == parent as isize }
}
