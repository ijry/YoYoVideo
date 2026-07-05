use tempfile::tempdir;
use yoyovideo_desktop::platform::{next_screenshot_path, prepare_screenshot_path_in_dir};

#[test]
fn screenshot_path_uses_timestamped_png_name() {
    let dir = tempdir().unwrap();

    let path = next_screenshot_path(dir.path(), "20260705-211530");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("yoyovideo-20260705-211530.png")
    );
}

#[test]
fn screenshot_path_adds_suffix_when_file_exists() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("yoyovideo-20260705-211530.png"), b"existing").unwrap();
    std::fs::write(dir.path().join("yoyovideo-20260705-211530-1.png"), b"existing").unwrap();

    let path = next_screenshot_path(dir.path(), "20260705-211530");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("yoyovideo-20260705-211530-2.png")
    );
}

#[test]
fn prepare_screenshot_path_in_dir_creates_directory() {
    let dir = tempdir().unwrap().path().join("nested").join("screens");

    let path = prepare_screenshot_path_in_dir(&dir, "20260705-211530").unwrap();

    assert!(dir.is_dir());
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("yoyovideo-20260705-211530.png")
    );
}
