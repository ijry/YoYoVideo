//! Native observations for the isolated privacy-qa build only.
//! The expected viewport is derived from the actual main content rectangle,
//! independently of the production host's coordinate conversion.
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSView, NSWindow};
use objc2_foundation::{MainThreadMarker, NSRect};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde_json::{Value, json};
use slint::winit_030::winit::window::Window;

fn rect(rect: NSRect) -> Value {
    json!({"x":rect.origin.x,"y":rect.origin.y,"width":rect.size.width,"height":rect.size.height})
}
fn observe(window: &NSWindow, main: &NSWindow) -> Value {
    json!({
        "frame":rect(window.frame()),"visible":window.isVisible(),"key":window.isKeyWindow(),
        "parent_is_main":window.parentWindow().is_some_and(|parent|std::ptr::eq(&*parent,main)),
        "parent_title":window.parentWindow().map(|parent|parent.title().to_string()),
        "parent_frame":window.parentWindow().map(|parent|rect(parent.frame())),
    })
}
pub(super) fn snapshot(window: &Window, viewport: [f64; 4]) -> Value {
    let Some(mtm) = MainThreadMarker::new() else { return json!({"error":"not on main thread"}) };
    let Ok(handle) = window.window_handle() else {
        return json!({"error":"no main window handle"});
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return json!({"error":"not AppKit"});
    };
    // SAFETY: winit owns this live NSView and the caller holds its window on the main thread.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let Some(main) = view.window() else { return json!({"error":"main view is detached"}) };
    let content = main.contentRectForFrameRect(main.frame());
    let windows = NSApplication::sharedApplication(mtm).windows();
    let hosts: Vec<Retained<NSWindow>> = windows
        .iter()
        .filter(|candidate| candidate.title().to_string() == "YoYoVideo Video Host")
        .collect();
    let [x, y, width, height] = viewport;
    json!({
        "main":{
            "frame":rect(main.frame()),"content":rect(content),"visible":main.isVisible(),
            "key":main.isKeyWindow(),"minimized":main.isMiniaturized(),
            "fullscreen":window.fullscreen().is_some(),"scale_factor":main.backingScaleFactor(),
        },
        "visible_windows":windows.iter().filter(|candidate|candidate.isVisible()).count(),
        "host_count":hosts.len(),"hosts":hosts.iter().map(|host|observe(host,&main)).collect::<Vec<_>>(),
        "expected_host":{
            "x":content.origin.x+x,"y":content.origin.y+content.size.height-y-height,
            "width":width,"height":height,
        },
    })
}
