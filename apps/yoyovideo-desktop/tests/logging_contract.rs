use tempfile::tempdir;
use yoyovideo_desktop::platform::{
    AppPaths, append_diagnostic, append_diagnostic_line, default_log_file,
};

#[test]
fn default_log_file_uses_app_data_logs_directory() {
    let dir = tempdir().unwrap();
    let paths = AppPaths {
        config_dir: dir.path().join("config"),
        data_dir: dir.path().join("data"),
        cache_dir: dir.path().join("cache"),
    };

    let log = default_log_file(Some(&paths));

    assert_eq!(log, dir.path().join("data").join("logs").join("yoyovideo.log"));
}

#[test]
fn append_diagnostic_line_creates_parent_and_appends_text() {
    let dir = tempdir().unwrap();
    let log = dir.path().join("logs").join("yoyovideo.log");

    append_diagnostic_line(&log, "2026-07-06T10:11:12+08:00", "ERROR", "backend failed")
        .unwrap();
    append_diagnostic_line(&log, "2026-07-06T10:11:13+08:00", "WARN", "retrying").unwrap();

    let content = std::fs::read_to_string(log).unwrap();
    assert!(content.contains("2026-07-06T10:11:12+08:00 ERROR backend failed"));
    assert!(content.contains("2026-07-06T10:11:13+08:00 WARN retrying"));
}

#[test]
fn append_diagnostic_returns_actual_log_path() {
    let dir = tempdir().unwrap();
    let paths = AppPaths {
        config_dir: dir.path().join("config"),
        data_dir: dir.path().join("data"),
        cache_dir: dir.path().join("cache"),
    };

    let path = append_diagnostic(Some(&paths), "INFO", "startup").unwrap();

    assert_eq!(path, dir.path().join("data").join("logs").join("yoyovideo.log"));
    assert!(std::fs::read_to_string(path).unwrap().contains("INFO startup"));
}
