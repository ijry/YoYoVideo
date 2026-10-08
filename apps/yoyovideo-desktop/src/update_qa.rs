//! Compiled only for updater-qa. Exercises real UI callbacks and playback state.
use crate::{MainWindow, UpdateWindow, platform::AppPaths};
use serde::Deserialize;
use slint::ComponentHandle;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    seq: u64,
    pid: u32,
    command: Command,
    #[serde(default)]
    path: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Command {
    Check,
    Download,
    Install,
    Later,
    DisableAuto,
    Open,
    ResumeHistory,
    Close,
}
fn parse_command(raw: &[u8], root: &Path, last_seq: u64, pid: u32) -> io::Result<Option<Request>> {
    if raw.len() > 16384 {
        return Err(io::Error::other("QA request exceeds limit"));
    }
    let mut request: Request = serde_json::from_slice(raw)?;
    if request.pid != pid || request.seq <= last_seq {
        return Ok(None);
    }
    if matches!(request.command, Command::Open) {
        let file = root
            .join(request.path.as_ref().ok_or_else(|| io::Error::other("Missing media path"))?)
            .canonicalize()?;
        if !file.starts_with(root.canonicalize()?) || !file.is_file() {
            return Err(io::Error::other("QA media must remain inside the fixture"));
        }
        request.path = Some(file);
    }
    Ok(Some(request))
}
pub(crate) fn paths() -> io::Result<AppPaths> {
    let fixture = yoyo_updater::QaFixture::from_env()?;
    fixture.public_key()?;
    let root = fixture.root().join("user");
    let paths = AppPaths {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        cache_dir: root.join("cache"),
    };
    for path in [&paths.config_dir, &paths.data_dir, &paths.cache_dir] {
        std::fs::create_dir_all(path)?;
    }
    Ok(paths)
}
pub(crate) struct QaSession {
    timer: slint::Timer,
}
impl QaSession {
    pub(crate) fn attach(
        main: &MainWindow,
        updates: &UpdateWindow,
        mut open: impl FnMut(PathBuf) + 'static,
        snapshot: impl Fn() -> serde_json::Value + 'static,
    ) -> io::Result<Self> {
        let fixture = yoyo_updater::QaFixture::from_env()?;
        let root = fixture.root().to_path_buf();
        let mut events = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("events.jsonl"))?;
        let main = main.as_weak();
        let updates = updates.as_weak();
        let pid = std::process::id();
        let mut last_seq = 0;
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(250), move || {
            let (Some(main), Some(updates)) = (main.upgrade(), updates.upgrade()) else { return; };
            let request_file = root.join("request.json");
            let request = (|| -> io::Result<Option<Request>> {
                match std::fs::metadata(&request_file) {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                    Err(e) => return Err(e),
                    Ok(m) if m.len() > 16384 => return Err(io::Error::other("Oversized QA request")),
                    _ => {}
                }
                parse_command(&std::fs::read(&request_file)?, &root, last_seq, pid)
            })();
            let mut error = None;
            match request {
                Ok(Some(request)) => {
                    last_seq = request.seq;
                    match request.command {
                        Command::Check => updates.invoke_check_requested(),
                        Command::Download => updates.invoke_download_requested(),
                        Command::Install => updates.invoke_install_requested(),
                        Command::Later => updates.invoke_later_requested(),
                        Command::DisableAuto => updates.invoke_automatic_check_changed(false),
                        Command::Open => { if let Some(path) = request.path { open(path); } },
                        Command::ResumeHistory => main.invoke_history_item_requested(0),
                        Command::Close => { let _ = main.hide(); let _ = slint::quit_event_loop(); },
                    }
                }
                Ok(None) => {}
                Err(e) => error = Some(e.to_string()),
            }
            let record = serde_json::json!({
                "kind": "snapshot", "pid": pid, "version": env!("CARGO_PKG_VERSION"), "seq": last_seq,
                "executable": std::env::current_exe().ok(), "appimage": std::env::var("APPIMAGE").ok(),
                "phase": updates.get_phase_index(), "automatic_check": updates.get_automatic_check(),
                "update_error": updates.get_status_message().as_str(), "qa_error": error,
                "playback": snapshot(),
            });
            let _ = writeln!(events, "{record}");
            let _ = events.flush();
        });
        Ok(Self { timer })
    }
}
impl Drop for QaSession {
    fn drop(&mut self) {
        eprintln!("QA: session drop begins");
        self.timer.stop();
        eprintln!("QA: session drop ends");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_are_pid_bound_non_replayable_and_cannot_open_outside_the_fixture() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("media.wav"), b"fixture").unwrap();
        assert!(
            parse_command(br#"{"seq":2,"pid":7,"command":"check"}"#, dir.path(), 1, 8)
                .unwrap()
                .is_none()
        );
        assert!(
            parse_command(br#"{"seq":1,"pid":7,"command":"check"}"#, dir.path(), 1, 7)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_command(br#"{"seq":2,"pid":7,"command":"check"}"#, dir.path(), 1, 7)
                .unwrap()
                .unwrap()
                .seq,
            2
        );
        assert!(
            parse_command(br#"{"seq":2,"pid":7,"command":"unknown"}"#, dir.path(), 1, 7).is_err()
        );
        assert!(
            parse_command(
                br#"{"seq":2,"pid":7,"command":"open","path":"../outside.wav"}"#,
                dir.path(),
                1,
                7
            )
            .is_err()
        );
        assert!(
            parse_command(
                br#"{"seq":2,"pid":7,"command":"open","path":"media.wav"}"#,
                dir.path(),
                1,
                7
            )
            .unwrap()
            .is_some()
        );
    }
}

pub(crate) fn record_shutdown() {
    if let Ok(fixture) = yoyo_updater::QaFixture::from_env() {
        if let Ok(mut file) =
            std::fs::OpenOptions::new().append(true).open(fixture.root().join("events.jsonl"))
        {
            let record = serde_json::json!({"kind":"shutdown","pid":std::process::id(),"version":env!("CARGO_PKG_VERSION")});
            let _ = writeln!(file, "{record}");
        }
    }
}
