//! Slint/mpv lifecycle glue. Kept as a child of app so runtime state stays private.
use super::*;

pub(super) fn install_composited_video_notifier(
    app: &MainWindow,
    runtime: Rc<RefCell<DesktopRuntime>>,
) {
    let app_handle = app.as_weak();
    let callback_runtime = Rc::clone(&runtime);
    let result = app.window().set_rendering_notifier(move |state, graphics_api| {
        let mut runtime = callback_runtime.borrow_mut();
        if matches!(state, slint::RenderingState::RenderingTeardown) {
            if let Some(app) = app_handle.upgrade() {
                app.set_video_frame_active(false);
                app.set_video_frame(slint::Image::default());
            }
            if let Some(surface) = runtime.composited_video.as_mut() {
                // SAFETY: Slint guarantees the creating context is current here;
                // the image was cleared or its owning window is being destroyed.
                unsafe { surface.teardown() };
            }
            return;
        }
        let (blocked, access, media) = runtime
            .controller
            .as_ref()
            .map(|controller| {
                let session = controller.session();
                (
                    session.privacy_blocked(),
                    session.playback_access(),
                    session.current_media_key().cloned(),
                )
            })
            .unwrap_or((false, None, None));
        if let Some(surface) = runtime.composited_video.as_mut() {
            surface.set_media_access(access, media);
            surface.set_privacy_blocked(blocked);
        }
        if blocked {
            if let Some(app) = app_handle.upgrade() {
                app.set_video_frame_active(false);
                app.set_video_frame(slint::Image::default());
            }
            if let Some(controller) = runtime.controller.as_mut() {
                let _ = controller.session_mut().enforce_privacy();
            }
            return;
        }
        let Some(surface) = runtime.composited_video.as_mut() else {
            return;
        };
        if matches!(state, slint::RenderingState::AfterRendering) {
            surface.report_swap();
            return;
        }
        if !matches!(state, slint::RenderingState::BeforeRendering) || surface.has_failed() {
            return;
        }
        let Some(app) = app_handle.upgrade() else {
            return;
        };
        let size = physical_video_size(
            app.get_video_area_width(),
            app.get_video_area_height(),
            app.window().scale_factor(),
        );
        let Some((width, height)) = size else {
            return;
        };
        let result = (|| -> Result<(Option<slint::Image>, bool), String> {
            let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = graphics_api else {
                return Err("Wayland video requires Slint's native OpenGL renderer".into());
            };
            let DesktopRuntime { composited_video, controller, .. } = &mut *runtime;
            let surface = composited_video.as_mut().expect("surface checked above");
            let controller = controller.as_ref().ok_or("Playback backend is not initialized")?;
            let initialized = !surface.is_ready();
            // SAFETY: Slint's current GL context is used for both calls. The
            // controller is retained until teardown has freed the render context.
            unsafe {
                if initialized {
                    let handle = app_handle.clone();
                    surface.setup(
                        controller.session().backend(),
                        *get_proc_address,
                        move || {
                            let _ =
                                handle.upgrade_in_event_loop(|app| app.window().request_redraw());
                        },
                    )?;
                }
                Ok((surface.render(width, height)?, initialized))
            }
        })();
        match result {
            Ok((image, initialized)) => {
                if let Some(image) = image {
                    // Equal texture IDs compare equal in Slint. Reset first so
                    // in-place pixel updates invalidate cached image layers.
                    app.set_video_frame(slint::Image::default());
                    app.set_video_frame(image);
                }
                let has_media = runtime
                    .controller
                    .as_ref()
                    .is_some_and(|c| c.session().state().current.is_some());
                let allowed = runtime
                    .composited_video
                    .as_ref()
                    .is_some_and(|surface| surface.output_allowed());
                if !allowed {
                    app.set_video_frame(slint::Image::default());
                }
                app.set_video_frame_active(has_media && allowed && !runtime.grid.is_active());
                if initialized {
                    // Normal player commands can wait on mpv. Dispatch startup
                    // media after leaving the rendering callback, on the UI loop.
                    let handle = app_handle.clone();
                    let runtime = Rc::clone(&callback_runtime);
                    slint::Timer::single_shot(Duration::ZERO, move || {
                        let Some(app) = handle.upgrade() else {
                            return;
                        };
                        let mut runtime = runtime.borrow_mut();
                        if runtime.controller().is_none() {
                            return;
                        }
                        if let Some(locator) = runtime.pending_startup_open.take() {
                            let command = match locator {
                                MediaLocator::File(path) => AppCommand::OpenFile(path),
                                MediaLocator::Url(url) => AppCommand::OpenUrl(url),
                            };
                            if let Err(error) =
                                runtime.controller_mut().expect("ready").dispatch(command)
                            {
                                runtime.record_diagnostic("ERROR", error.to_string());
                                set_playback_status(&app, error.to_string().into());
                            }
                        }
                        refresh_runtime_window(&app, &runtime);
                        app.window().request_redraw();
                    });
                }
            }
            Err(error) => {
                runtime.composited_video.as_mut().expect("surface exists").mark_failed();
                app.set_video_frame_active(false);
                app.set_video_frame(slint::Image::default());
                let message = format!("Composited video failed: {error}");
                runtime.record_diagnostic("ERROR", &message);
                runtime.mark_error(message.clone());
                set_playback_status(&app, message.into());
            }
        }
    });
    if let Err(error) = result {
        runtime.borrow_mut().composited_notifier_error = Some(error.to_string());
    }
}
