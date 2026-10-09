//! Private test build only. Never compiled into ordinary development/release builds.
use super::*;
use crate::privacy::{PrivacyClock, PrivacyService, PrivacyStore};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};
#[cfg(target_os = "macos")]
#[path = "privacy_qa/macos.rs"]
mod macos;
#[cfg(windows)]
#[path = "privacy_qa/windows.rs"]
mod windows;

pub(super) struct QaClock(AtomicI64);
impl PrivacyClock for QaClock {
    fn now(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.0.load(Ordering::Acquire)).expect("validated QA time")
    }
}
fn local_time(value: &str) -> io::Result<DateTime<Utc>> {
    let local = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| io::Error::other("Invalid QA local time"))?;
    Local
        .from_local_datetime(&local)
        .single()
        .map(|time| time.with_timezone(&Utc))
        .ok_or_else(|| io::Error::other("Ambiguous QA time"))
}
fn root() -> io::Result<PathBuf> {
    let root = std::env::var_os("YOYOVIDEO_PRIVACY_QA_ROOT")
        .ok_or_else(|| io::Error::other("privacy-qa requires an explicit isolated fixture"))?;
    let root = std::fs::canonicalize(root)?;
    let marker = std::fs::read_to_string(root.join("fixture.txt"))?;
    if marker.trim() != "yoyovideo-privacy-qa-v1" {
        return Err(io::Error::other("Not a privacy QA fixture"));
    }
    Ok(root)
}
pub(super) fn paths() -> io::Result<AppPaths> {
    let user = root()?.join("user");
    let paths = AppPaths {
        config_dir: user.join("config"),
        data_dir: user.join("data"),
        cache_dir: user.join("cache"),
    };
    for path in [&paths.config_dir, &paths.data_dir, &paths.cache_dir] {
        std::fs::create_dir_all(path)?;
    }
    Ok(paths)
}
pub(super) fn load_service(store: PrivacyStore) -> io::Result<(Arc<PrivacyService>, Arc<QaClock>)> {
    let value = std::fs::read_to_string(root()?.join("clock.txt"))?;
    let clock = Arc::new(QaClock(AtomicI64::new(local_time(value.trim())?.timestamp_millis())));
    Ok((Arc::new(PrivacyService::with_clock(store, clock.clone())), clock))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    pid: u32,
    seq: u64,
    command: Command,
    path: Option<String>,
    local_time: Option<String>,
    index: Option<i32>,
    volume: Option<i32>,
    window_position: Option<[i32; 2]>,
    window_size: Option<[u32; 2]>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Command {
    Open,
    ProtectCurrent,
    Toggle,
    Settings,
    SubmitGood,
    SubmitBad,
    Cancel,
    SaveDailySchedule,
    Clock,
    Play,
    Pause,
    Screenshot,
    OpenPopup,
    ClosePopup,
    Fullscreen,
    History,
    Recent,
    Playlist,
    Next,
    SeekEnd,
    Grid,
    GridPlay,
    GridVolume,
    FocusMain,
    MoveMain,
    ResizeMain,
    MinimizeMain,
    RestoreMain,
    Close,
}

fn media(root: &Path, name: &str) -> io::Result<PathBuf> {
    let directory = root.join("media").canonicalize()?;
    let file = directory.join(name).canonicalize()?;
    if !directory.starts_with(root) || !file.starts_with(&directory) || !file.is_file() {
        return Err(io::Error::other("QA media must remain inside the fixture"));
    }
    Ok(file)
}
fn pin_window(runtime: &Rc<RefCell<DesktopRuntime>>) -> Option<crate::PrivacyWindow> {
    let ui = runtime.borrow().privacy_ui.clone()?;
    let window = ui.borrow().qa_window();
    Some(window)
}
fn act(
    request: Request,
    root: &Path,
    clock: &QaClock,
    app: &MainWindow,
    runtime: &Rc<RefCell<DesktopRuntime>>,
) -> io::Result<()> {
    let command = |command| {
        with_runtime_controller(&app.as_weak(), runtime, |controller| controller.dispatch(command));
    };
    match request.command {
        Command::Open => dispatch_dropped_paths(
            &app.as_weak(),
            runtime,
            vec![media(
                root,
                request.path.as_deref().ok_or_else(|| io::Error::other("Missing fixture media"))?,
            )?],
        ),
        Command::ProtectCurrent => app.invoke_protect_current_requested(true),
        Command::Toggle => app.invoke_toggle_privacy_requested(),
        Command::Settings => app.invoke_privacy_settings_requested(),
        Command::SubmitGood | Command::SubmitBad => {
            let window =
                pin_window(runtime).ok_or_else(|| io::Error::other("Missing PIN window"))?;
            if window.get_busy() || window.get_cooldown_seconds() > 0 {
                window.invoke_submit_requested();
                return Ok(());
            }
            // The only accepted test PIN is synthetic and stays in this test-only source.
            // Requests and diagnostic output contain neither the digits nor a hash.
            let pin = if matches!(request.command, Command::SubmitGood) { "0123" } else { "9999" };
            window.set_pin(pin.into());
            if window.get_mode() == 0 || window.get_mode() == 3 {
                window.set_confirmation(pin.into());
            }
            window.invoke_submit_requested();
        }
        Command::Cancel => {
            if let Some(window) = pin_window(runtime) {
                window.invoke_cancel_requested();
            }
        }
        Command::SaveDailySchedule => {
            let window =
                pin_window(runtime).ok_or_else(|| io::Error::other("Missing settings window"))?;
            if window.get_mode() != 2 {
                return Err(io::Error::other("Settings are not authorized"));
            }
            window.set_start_time("09:00".into());
            window.set_end_time("18:00".into());
            window.set_monday(true);
            window.set_tuesday(true);
            window.set_wednesday(true);
            window.set_thursday(true);
            window.set_friday(true);
            window.set_saturday(true);
            window.set_sunday(true);
            window.set_schedule_enabled(true);
            window.invoke_add_rule_requested();
            window.invoke_save_schedule_requested();
        }
        Command::Clock => {
            let text = request.local_time.ok_or_else(|| io::Error::other("Missing QA time"))?;
            let time = local_time(&text)?;
            std::fs::write(root.join("clock.txt"), text)?;
            clock.0.store(time.timestamp_millis(), Ordering::Release);
        }
        Command::Play => command(AppCommand::SetPaused(false)),
        Command::Pause => command(AppCommand::SetPaused(true)),
        Command::Screenshot => {
            command(AppCommand::TakeScreenshot(root.join("requested-screenshot.png")))
        }
        Command::OpenPopup => app.invoke_open_menu_popup(),
        Command::ClosePopup => app.invoke_close_playback_panels(),
        Command::Fullscreen => command(AppCommand::ToggleFullscreen),
        Command::History => app.invoke_history_item_requested(request.index.unwrap_or(0)),
        Command::Recent => app.invoke_recent_open_item_requested(request.index.unwrap_or(0)),
        Command::Playlist => {
            let entries = ["blue.mp4", "red.mp4"]
                .into_iter()
                .map(|name| {
                    media(root, name)
                        .map(|path| yoyo_core::PlaylistEntry::new(MediaLocator::File(path)))
                })
                .collect::<io::Result<Vec<_>>>()?;
            with_runtime_controller(&app.as_weak(), runtime, |controller| {
                controller.open_playlist_entries(entries)
            });
        }
        Command::Next => command(AppCommand::NextItem),
        Command::SeekEnd => {
            let duration = runtime
                .borrow()
                .controller
                .as_ref()
                .and_then(|c| c.session().state().duration_seconds)
                .ok_or_else(|| io::Error::other("No duration"))?;
            command(AppCommand::SeekAbsolute((duration - 0.25).max(0.0)));
        }
        Command::Grid => {
            let locators = ["red.mp4", "blue.mp4"]
                .into_iter()
                .map(|name| media(root, name).map(MediaLocator::File))
                .collect::<io::Result<Vec<_>>>()?;
            let mut runtime = runtime.borrow_mut();
            if locators.iter().any(|locator| !runtime.grid.can_open(locator)) {
                return Err(io::Error::other("Grid fixture is restricted"));
            }
            runtime.grid.clear();
            if let Some(controller) = runtime.controller.as_mut() {
                let _ = controller.dispatch(AppCommand::Stop);
            }
            if let Some(host) = runtime.video_host.as_mut() {
                let _ = host.hide();
            }
            runtime.grid.queue_open(locators);
            app.set_grid_mode(true);
        }
        Command::GridPlay => runtime.borrow_mut().grid.set_all_paused(false),
        Command::GridVolume => app.invoke_grid_tile_volume_changed(
            request.index.unwrap_or(0),
            request.volume.unwrap_or(37),
        ),
        Command::FocusMain => {
            app.window().with_winit_window(|window| {
                if window.is_visible() == Some(false) {
                    window.set_visible(false);
                    window.set_visible(true);
                }
                window.focus_window();
            });
        }
        Command::MoveMain => {
            let [x, y] =
                request.window_position.ok_or_else(|| io::Error::other("Missing QA position"))?;
            app.window().with_winit_window(|window| {
                window.set_outer_position(slint::winit_030::winit::dpi::LogicalPosition::new(x, y));
            });
        }
        Command::ResizeMain => {
            let [width, height] =
                request.window_size.ok_or_else(|| io::Error::other("Missing QA size"))?;
            app.window().with_winit_window(|window| {
                let _ = window.request_inner_size(slint::winit_030::winit::dpi::LogicalSize::new(
                    width, height,
                ));
            });
        }
        Command::MinimizeMain => app.window().set_minimized(true),
        Command::RestoreMain => app.window().set_minimized(false),
        Command::Close => app.invoke_window_close_requested(),
    }
    Ok(())
}

fn surface_color(hwnd: Option<u64>) -> Option<Value> {
    #[cfg(windows)]
    {
        hwnd.and_then(windows::color)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        None
    }
}
fn snapshot(app: &MainWindow, runtime: &Rc<RefCell<DesktopRuntime>>, root: &Path) -> Value {
    let ui=pin_window(runtime).map(|window|json!({
        "visible":window.window().is_visible(),"focused":window.window().with_winit_window(|w|w.has_focus()).unwrap_or(false),
        "mode":window.get_mode(),"busy":window.get_busy(),"can_submit":window.get_submit_enabled(),
        "pin_empty":window.get_pin().is_empty() && window.get_confirmation().is_empty(),
        "protected_rows":window.get_protected_items().row_count(),"rules":window.get_rules().row_count(),
    }));
    #[cfg(target_os = "macos")]
    let native_macos = app.window().with_winit_window(|window| {
        macos::snapshot(
            window,
            [
                f64::from(app.get_video_area_x()),
                f64::from(app.get_video_area_y()),
                f64::from(app.get_video_area_width()),
                f64::from(app.get_video_area_height()),
            ],
        )
    });
    #[cfg(not(target_os = "macos"))]
    let native_macos: Option<Value> = None;
    let runtime = runtime.borrow();
    let privacy=runtime.privacy.as_ref().map(|service|{let state=service.snapshot();json!({"configured":state.configured,"enabled":state.enabled,"manual":state.manual,"dirty":state.dirty,"fail_closed":state.fail_closed,"cooldown":state.cooldown_seconds})});
    let state = runtime.controller.as_ref().map(|controller| controller.session().state());
    let filename =
        state.and_then(|state| state.current.as_ref()).and_then(|locator| match locator {
            MediaLocator::File(path) if path.starts_with(root.join("media")) => {
                path.file_name().map(|name| name.to_string_lossy().into_owned())
            }
            _ => None,
        });
    let native_visible = runtime.video_host.as_ref().is_some_and(|host| host.native_visible());
    let hwnd = if filename.is_some() {
        runtime.video_host.as_ref().and_then(|host| host.mpv_window_id().ok()).map(|id| id.0)
    } else {
        None
    };
    #[cfg(windows)]
    let parent_ok = hwnd
        .zip(
            app.window()
                .with_winit_window(|window| {
                    window.window_handle().ok().and_then(|handle| match handle.as_raw() {
                        raw_window_handle::RawWindowHandle::Win32(handle) => {
                            Some(handle.hwnd.get() as u64)
                        }
                        _ => None,
                    })
                })
                .flatten(),
        )
        .map(|(child, parent)| windows::has_parent(child, parent));
    #[cfg(not(windows))]
    let parent_ok: Option<bool> = None;
    let flags = runtime
        .controller
        .as_ref()
        .and_then(|controller| controller.session().backend().qa_output_flags().ok());
    let label = crate::privacy::view::protected_label(runtime.ui_language);
    let history = app.get_history_rows();
    let playlist = app.get_playlist_rows();
    let recent = app.get_recent_open_rows();
    let mut grid = runtime.grid.qa_snapshot(&root.join("media"));
    if let Some(tiles) = grid.as_array_mut() {
        for tile in tiles {
            let color = surface_color(tile.get("hwnd").and_then(Value::as_u64));
            tile["color"] = json!(color);
        }
    }
    json!({
        "ready":runtime.controller.is_some(),"main_visible":app.window().with_winit_window(|window|window.is_visible()).flatten(),"suppressed":runtime.video_host_suppression.is_suppressed(),"permit":runtime.video_host.as_ref().map(|host|host.qa_visibility_permitted()),"privacy":privacy,"ui":ui,"current_file":filename,
        "paused":state.map(|state|state.paused),"user_muted":state.map(|state|state.muted),"position":state.map(|state|state.position_seconds),
        "backend_paused":flags.map(|flags|flags.0),"backend_muted":flags.map(|flags|flags.1),"backend_idle":flags.map(|flags|flags.2),
        "native_visible":native_visible,"host_parent_is_main":parent_ok,"color":surface_color(hwnd),"blocked":app.get_privacy_blocked(),"frame_active":app.get_video_frame_active(),
        "status_mentions_red":app.get_status_label().contains("red.mp4"),
        "history_redacted":(0..history.row_count()).map(|i|history.row_data(i).is_some_and(|r|r.title==label && r.subtitle.is_empty())).collect::<Vec<_>>(),
        "playlist_redacted":(0..playlist.row_count()).map(|i|playlist.row_data(i).is_some_and(|r|r.title==label)).collect::<Vec<_>>(),
        "recent_redacted":(0..recent.row_count()).map(|i|recent.row_data(i).is_some_and(|r|r.title==label && r.subtitle.is_empty())).collect::<Vec<_>>(),
        "popup":app.get_any_popup_open(),"grid":grid,"macos":native_macos,
        "host_error":runtime.video_host_error,"status":app.get_status_label().to_string(),
    })
}

pub(super) struct QaSession {
    _timer: slint::Timer,
}
impl QaSession {
    pub(super) fn attach(
        app: &MainWindow,
        runtime: Rc<RefCell<DesktopRuntime>>,
        clock: Arc<QaClock>,
    ) -> io::Result<Self> {
        let root = root()?;
        app.set_window_brand("YoYoVideo · PRIVACY QA ONLY".into());
        if let Some(window) = pin_window(&runtime) {
            window.set_window_brand("YoYoVideo PRIVACY QA ONLY".into());
        }
        let mut events = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("events.jsonl"))?;
        let app = app.as_weak();
        let pid = std::process::id();
        let mut seq = 0_u64;
        let mut last_error = None;
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated,Duration::from_millis(150),move || {
            let Some(app)=app.upgrade() else {return;};
            let request=(||->io::Result<Option<Request>> {
                let file=root.join("request.json");
                let metadata=match std::fs::metadata(&file){Ok(value)=>value,Err(error) if error.kind()==io::ErrorKind::NotFound=>return Ok(None),Err(error)=>return Err(error)};
                if metadata.len()>16384 {return Err(io::Error::other("Oversized QA request"));}
                let request:Request=serde_json::from_slice(&std::fs::read(file)?)?;
                Ok((request.pid==pid && request.seq>seq).then_some(request))
            })();
            match request {
                Ok(Some(request))=>{seq=request.seq;last_error=None;if let Err(err)=act(request,&root,&clock,&app,&runtime){last_error=Some(err.to_string());}},
                Err(err)=>last_error=Some(err.to_string()),_=>{},
            }
            {let mut runtime=runtime.borrow_mut(); enforce_runtime_privacy(&app,&mut runtime);}
            let record=json!({"pid":pid,"seq":seq,"qa_error":last_error,"state":snapshot(&app,&runtime,&root)});
            let _=writeln!(events,"{record}"); let _=events.flush();
            if let Ok(mut file)=tempfile::NamedTempFile::new_in(&root) {if write!(file,"{record}").is_ok(){let _=file.persist(root.join("state.json"));}}
        });
        Ok(Self { _timer: timer })
    }
}
