//! Read-only preflight: never kill another player process from the application.
use crate::UpdateError;
use std::path::Path;

pub(crate) fn ensure_exclusive_installation(directory: &Path) -> Result<(), UpdateError> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind, get_current_pid};
    let directory = directory.canonicalize()?;
    let current_exe = std::env::current_exe()?;
    let current_name = current_exe
        .file_name()
        .ok_or_else(|| UpdateError::Install("cannot identify current executable".into()))?;
    let current_pid =
        get_current_pid().map_err(|_| UpdateError::Install("cannot inspect processes".into()))?;
    let system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Always)),
    );
    if system.process(current_pid).is_none() {
        return Err(UpdateError::Install("cannot inspect running player instances".into()));
    }
    for (pid, process) in system.processes() {
        if *pid == current_pid {
            continue;
        }
        let executable = process.exe().and_then(|p| p.canonicalize().ok());
        let conflict = conflicts_with_installation(
            executable.as_deref(),
            process.name(),
            &directory,
            current_name,
        );
        if conflict {
            return Err(UpdateError::Install(
                "close other instances or processes using this installation, then retry".into(),
            ));
        }
    }
    Ok(())
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
}
