//! UI-thread privacy coordination. Policies and PIN work live outside the player UI.
use super::*;
use crate::privacy::{
    PrivacyError, view,
    window::{DialogIntent, PrivacyUi},
};

pub(super) fn enforce_runtime_privacy(window: &MainWindow, runtime: &mut DesktopRuntime) {
    if let Some(service) = &runtime.privacy {
        service.tick();
        let snapshot = service.snapshot();
        window.set_privacy_enabled(snapshot.enabled);
        if !snapshot.dirty {
            runtime.privacy_save_error = false;
        }
        window.set_privacy_error(if snapshot.fail_closed {
            view::error_text(&PrivacyError::CorruptConfig, runtime.ui_language).into()
        } else if runtime.privacy_save_error {
            view::error_text(&PrivacyError::Persistence, runtime.ui_language).into()
        } else {
            "".into()
        });
        window.set_privacy_status(view::status_text(&snapshot, runtime.ui_language).into());
        if snapshot.enabled {
            let text = window.get_url_input_text();
            if MediaLocator::from_url(text.as_str())
                .ok()
                .is_some_and(|locator| view::locator_restricted(service.as_ref(), &locator))
            {
                window.set_url_input_text("".into());
            }
        }
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
    let newly_blocked = blocked && !window.get_privacy_blocked();
    window.set_privacy_blocked(blocked);
    if blocked {
        window.set_video_frame_active(false);
        window.set_video_frame(slint::Image::default());
        window.set_progress_preview_visible(false);
        window.set_progress_preview_label("".into());
        window.set_jump_input_text("".into());
        if newly_blocked {
            window.set_url_input_text("".into());
            let weak = window.as_weak();
            slint::Timer::single_shot(Duration::ZERO, move || {
                if let Some(window) = weak.upgrade() {
                    window.invoke_close_playback_panels();
                }
            });
        }
    }
    #[cfg(feature = "mpv-runtime")]
    {
        runtime.video_host_suppression.request_privacy(blocked);
        if let Some(host) = runtime.video_host.as_mut() {
            host.set_media_access(access.clone(), media.clone());
            let _ = host.set_privacy_blocked(blocked);
        }
        if let Some(surface) = runtime.composited_video.as_mut() {
            surface.set_media_access(access, media);
            surface.set_privacy_blocked(blocked);
        }
        runtime.grid.enforce_privacy();
    }
    #[cfg(not(feature = "mpv-runtime"))]
    let _ = (access, media);
    if newly_blocked {
        refresh_runtime_window(window, runtime);
        refresh_sidebar(window, runtime);
        refresh_recent_open_menu(window, runtime);
        refresh_tracks_popup(window, runtime);
    }
    // Actual surfaces have been concealed before any backend command can fail.
    if let Some(controller) = runtime.controller.as_mut() {
        let _ = controller.session_mut().enforce_privacy();
    }
}

pub(super) fn locator_allowed(runtime: &DesktopRuntime, locator: &MediaLocator) -> bool {
    runtime
        .privacy
        .as_ref()
        .is_none_or(|access| !view::locator_restricted(access.as_ref(), locator))
}

pub(super) fn deny_private_action(window: &MainWindow, runtime: &DesktopRuntime) {
    window.set_status_label(view::protected_label(runtime.ui_language).into());
}

fn refresh_all(window: &MainWindow, runtime: &mut DesktopRuntime) {
    enforce_runtime_privacy(window, runtime);
    refresh_runtime_window(window, runtime);
    refresh_sidebar(window, runtime);
    refresh_recent_open_menu(window, runtime);
    refresh_tracks_popup(window, runtime);
    #[cfg(feature = "mpv-runtime")]
    {
        sync_runtime_video_host(window, runtime);
        if runtime.grid.is_active() {
            sync_grid(window, runtime);
        }
    }
}

pub(super) fn attach_privacy_ui(
    app: &MainWindow,
    runtime: &Rc<RefCell<DesktopRuntime>>,
) -> Result<(), slint::PlatformError> {
    let Some(service) = runtime.borrow().privacy.clone() else {
        return Ok(());
    };
    let weak_runtime = Rc::downgrade(runtime);
    let app_handle = app.as_weak();
    let notify = Rc::new(move |error: Option<PrivacyError>| {
        let (Some(runtime), Some(app)) = (weak_runtime.upgrade(), app_handle.upgrade()) else {
            return;
        };
        let Ok(mut runtime) = runtime.try_borrow_mut() else {
            return;
        };
        if matches!(error, Some(PrivacyError::Persistence)) {
            runtime.privacy_save_error = true;
        }
        refresh_all(&app, &mut runtime);
        if let Some(error) = error {
            app.set_status_label(view::error_text(&error, runtime.ui_language).into());
        }
    });
    let ui = PrivacyUi::new(service, notify)?;
    runtime.borrow_mut().privacy_ui = Some(ui.clone());
    app.on_toggle_privacy_requested({
        let ui = ui.clone();
        let handle = app.as_weak();
        move || {
            let Some(app) = handle.upgrade() else {
                return;
            };
            let language = crate::UiLanguage::parse(app.get_ui_language_code().as_str());
            if let Err(error) = ui.borrow_mut().toggle(language) {
                app.set_status_label(view::error_text(&error, language).into());
            }
        }
    });
    app.on_privacy_settings_requested({
        let ui = ui.clone();
        let handle = app.as_weak();
        move || {
            let Some(app) = handle.upgrade() else {
                return;
            };
            let language = crate::UiLanguage::parse(app.get_ui_language_code().as_str());
            if let Err(error) = ui.borrow_mut().present(DialogIntent::Settings, language) {
                app.set_status_label(view::error_text(&error, language).into());
            }
        }
    });
    app.on_protect_current_requested({
        let ui = ui.clone();
        let handle = app.as_weak();
        let runtime = Rc::clone(runtime);
        move |protected| {
            let locator = {
                let runtime = runtime.borrow();
                #[cfg(feature = "mpv-runtime")]
                if runtime.grid.is_active() {
                    runtime.grid.active_locator()
                } else {
                    runtime.controller.as_ref().and_then(|c| c.session().state().current.clone())
                }
                #[cfg(not(feature = "mpv-runtime"))]
                runtime.controller.as_ref().and_then(|c| c.session().state().current.clone())
            };
            present_protection(&ui, &handle, locator, protected);
        }
    });
    app.on_protect_playlist_requested({
        let ui = ui.clone();
        let handle = app.as_weak();
        let runtime = Rc::clone(runtime);
        move |index, protected| {
            let locator = usize::try_from(index).ok().and_then(|index| {
                runtime.borrow().controller.as_ref().and_then(|c| {
                    c.session()
                        .playlist_snapshot()
                        .entries
                        .get(index)
                        .map(|entry| entry.locator.clone())
                })
            });
            present_protection(&ui, &handle, locator, protected);
        }
    });
    app.on_protect_history_requested({
        let ui = ui.clone();
        let handle = app.as_weak();
        let runtime = Rc::clone(runtime);
        move |index, protected| {
            let locator = usize::try_from(index).ok().and_then(|index| {
                runtime.borrow().history.store().entry(index).map(|entry| entry.locator.clone())
            });
            present_protection(&ui, &handle, locator, protected);
        }
    });
    Ok(())
}

fn present_protection(
    ui: &Rc<RefCell<PrivacyUi>>,
    handle: &slint::Weak<MainWindow>,
    locator: Option<MediaLocator>,
    protected: bool,
) {
    let Some(app) = handle.upgrade() else {
        return;
    };
    let language = crate::UiLanguage::parse(app.get_ui_language_code().as_str());
    let Some(locator) = locator else {
        app.set_status_label(
            if language == crate::UiLanguage::Chinese {
                "请先打开或选择媒体"
            } else {
                "Open or select media first"
            }
            .into(),
        );
        return;
    };
    // Capture the locator now; never replay a changing sidebar index after PIN verification.
    if let Err(error) = ui.borrow_mut().present(DialogIntent::Protect(locator, protected), language)
    {
        app.set_status_label(view::error_text(&error, language).into());
    }
}

pub(super) fn allow_privacy_safe_exit(
    app: &MainWindow,
    runtime: &Rc<RefCell<DesktopRuntime>>,
) -> bool {
    let privacy_ui = runtime.borrow().privacy_ui.clone();
    if let Some(ui) = privacy_ui {
        ui.borrow_mut().cancel_for_exit();
    }
    let mut runtime = runtime.borrow_mut();
    enforce_runtime_privacy(app, &mut runtime);
    match persist_playback_for_shutdown(&mut runtime) {
        Ok(()) => true,
        Err(error) => {
            runtime.privacy_save_error =
                runtime.privacy.as_ref().is_some_and(|service| service.snapshot().dirty);
            runtime.record_diagnostic("WARN", format!("Shutdown save failed: {error}"));
            app.set_status_label(
                if runtime.ui_language == crate::UiLanguage::Chinese {
                    "状态未能保存，已取消退出，请重试"
                } else {
                    "State could not be saved; exit cancelled. Please retry."
                }
                .into(),
            );
            false
        }
    }
}

/// Backend/file-operation statuses must not undo a privacy projection that was
/// applied earlier in the same callback. PIN/storage feedback uses its own channel.
pub(super) fn set_playback_status(window: &MainWindow, value: slint::SharedString) {
    if window.get_privacy_blocked() {
        let language = crate::UiLanguage::parse(window.get_ui_language_code().as_str());
        window.set_status_label(view::protected_label(language).into());
    } else {
        window.set_status_label(value);
    }
}
