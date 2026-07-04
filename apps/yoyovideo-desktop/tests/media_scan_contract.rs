use std::fs;

use tempfile::tempdir;
use yoyovideo_desktop::scan_media_folder;

#[test]
fn folder_scan_only_keeps_supported_media_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("movie.mp4"), "x").unwrap();
    fs::write(dir.path().join("cover.jpg"), "x").unwrap();
    fs::write(dir.path().join("song.flac"), "x").unwrap();

    let entries = scan_media_folder(dir.path()).unwrap();
    let titles: Vec<_> = entries.into_iter().map(|entry| entry.title).collect();

    assert_eq!(titles, vec!["movie.mp4".to_string(), "song.flac".to_string()]);
}
