//! Read-only preflight: never kill another player process from the application.
use crate::UpdateError;
use std::path::Path;

fn process_refresh_kind() -> sysinfo::ProcessRefreshKind {
    // Linux task IDs are threads, not other application instances.
    sysinfo::ProcessRefreshKind::nothing().without_tasks().with_exe(sysinfo::UpdateKind::Always)
}
pub(crate) fn ensure_exclusive_installation(directory: &Path) -> Result<(), UpdateError> {
    use sysinfo::{RefreshKind, System, get_current_pid};
    let directory = directory.canonicalize()?;
    let current_exe = std::env::current_exe()?;
    let current_name = current_exe
        .file_name()
        .ok_or_else(|| UpdateError::Install("cannot identify current executable".into()))?;
    let current_pid =
        get_current_pid().map_err(|_| UpdateError::Install("cannot inspect processes".into()))?;
    let system =
        System::new_with_specifics(RefreshKind::nothing().with_processes(process_refresh_kind()));
    if system.process(current_pid).is_none() {
        return Err(UpdateError::Install("cannot inspect running player instances".into()));
    }
    for (pid, process) in system.processes() {
        if *pid == current_pid {
            continue;
        }
        let executable = process.exe().and_then(|p| p.canonicalize().ok());
        #[cfg(not(windows))]
        if matches!(process.status(), sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead)
        {
            continue;
        }
        let conflict = conflicts_with_installation(
            executable.as_deref(),
            process.name(),
            &directory,
            current_name,
        );
        #[cfg(target_os = "linux")]
        let conflict = conflict
            || appimage_conflict(
                *pid,
                process.exe().and_then(|p| p.file_name()).unwrap_or(process.name()),
                current_name,
            )?;
        #[cfg(windows)]
        let conflict = conflict && is_live_process(pid.as_u32())?;
        if conflict {
            return Err(UpdateError::Install(
                "close other instances or processes using this installation, then retry".into(),
            ));
        }
    }
    Ok(())
}

// Process enumeration can retain exited entries while another process holds a
// handle. A zero-time wait is authoritative; access errors are not proof of exit.
#[cfg(windows)]
fn is_live_process(pid: u32) -> Result<bool, UpdateError> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32;
    }
    // SYNCHRONIZE only: no process mutation, termination or memory access.
    let raw = unsafe { OpenProcess(0x00100000, 0, pid) };
    if raw.is_null() {
        return classify_open_error(std::io::Error::last_os_error().raw_os_error().unwrap_or(0));
    }
    // OpenProcess returned a non-null owned handle; RAII closes it exactly once.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    match unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } {
        0 => Ok(false),  // Signaled: all threads have exited.
        258 => Ok(true), // WAIT_TIMEOUT: process is still running.
        _ => Err(UpdateError::Install(
            "cannot determine whether an installation process is still running".into(),
        )),
    }
}
#[cfg(windows)]
fn classify_open_error(code: i32) -> Result<bool, UpdateError> {
    if code == 87 {
        // ERROR_INVALID_PARAMETER: the enumerated PID no longer exists.
        Ok(false)
    } else {
        Err(UpdateError::Install("cannot inspect a potentially conflicting process".into()))
    }
}

#[cfg(target_os = "linux")]
fn appimage_conflict(
    pid: sysinfo::Pid,
    name: &std::ffi::OsStr,
    current_name: &std::ffi::OsStr,
) -> Result<bool, UpdateError> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let Some(image) = std::env::var_os("APPIMAGE") else {
        return Ok(false);
    };
    if name != current_name {
        return Ok(false);
    }
    // Read only a candidate player's environment, never every system process.
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().without_tasks().with_environ(UpdateKind::Always),
    );
    let Some(process) = system.process(pid) else {
        return Ok(false);
    };
    if matches!(process.status(), sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead) {
        return Ok(false);
    }
    let value = process.environ().iter().find_map(|v| v.to_str()?.strip_prefix("APPIMAGE="));
    match value {
        Some(other) => Ok(Path::new(other).canonicalize()? == Path::new(&image).canonicalize()?),
        None if process.environ().is_empty() => {
            Err(UpdateError::Install("cannot inspect a potentially conflicting AppImage".into()))
        }
        None => Ok(false),
    }
}

fn conflicts_with_installation(
    executable: Option<&Path>,
    process_name: &std::ffi::OsStr,
    directory: &Path,
    current_name: &std::ffi::OsStr,
) -> bool {
    match executable {
        Some(path) => path.starts_with(directory),
        None => {
            process_name.to_string_lossy().eq_ignore_ascii_case(&current_name.to_string_lossy())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    #[test]
    fn preflight_lists_processes_not_the_players_own_threads() {
        assert!(!process_refresh_kind().tasks());
    }
    #[test]
    fn blocks_shared_version_directory_not_other_installations() {
        let dir = Path::new("C:/Apps/YoYo/current");
        let name = OsStr::new("yoyovideo-desktop.exe");
        assert!(conflicts_with_installation(
            Some(Path::new("C:/Apps/YoYo/current/helper.exe")),
            OsStr::new("helper.exe"),
            dir,
            name
        ));
        assert!(!conflicts_with_installation(
            Some(Path::new("C:/Apps/YoYo/current-other/yoyovideo-desktop.exe")),
            name,
            dir,
            name
        ));
    }
    #[test]
    fn unknown_executable_path_is_not_assumed_safe_for_another_player() {
        let name = OsStr::new("yoyovideo-desktop.exe");
        let dir = Path::new("C:/Apps/YoYo/current");
        assert!(conflicts_with_installation(None, OsStr::new("YOYOVIDEO-DESKTOP.EXE"), dir, name));
        assert!(!conflicts_with_installation(None, OsStr::new("unrelated.exe"), dir, name));
    }
    #[cfg(windows)]
    #[test]
    fn exited_process_handles_are_not_live_installation_conflicts() {
        use std::os::windows::process::CommandExt;
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "exit 0"])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        assert!(child.wait().unwrap().success());
        // Keep the Child/OS handle alive: exited entries can remain enumerable.
        assert!(!is_live_process(child.id()).unwrap());
        assert!(is_live_process(std::process::id()).unwrap());
        assert!(classify_open_error(5).is_err()); // ACCESS_DENIED remains fail-closed.
        assert!(!classify_open_error(87).unwrap()); // PID has gone away.
    }
}
