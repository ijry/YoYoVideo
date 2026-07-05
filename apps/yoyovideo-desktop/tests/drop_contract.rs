use std::fs;

use tempfile::tempdir;
use yoyo_core::{MediaLocator, PlaylistEntry};
use yoyovideo_desktop::platform::{DroppedMediaAction, classify_dropped_paths};

#[test]
fn single_supported_file_drop_opens_that_file() {
    let dir = tempdir().unwrap();
    let movie = dir.path().join("movie.mp4");
    fs::write(&movie, "media").unwrap();

    let action = classify_dropped_paths(&[movie.clone()]).unwrap();

    assert_eq!(action, DroppedMediaAction::OpenFile(movie));
}

#[test]
fn multiple_supported_files_drop_replaces_playlist_in_drop_order() {
    let dir = tempdir().unwrap();
    let first = dir.path().join("b.mp4");
    let second = dir.path().join("a.mkv");
    fs::write(&first, "media").unwrap();
    fs::write(&second, "media").unwrap();

    let action = classify_dropped_paths(&[first.clone(), second.clone()]).unwrap();

    assert_eq!(
        action,
        DroppedMediaAction::ReplacePlaylist(vec![
            PlaylistEntry::new(MediaLocator::File(first)),
            PlaylistEntry::new(MediaLocator::File(second)),
        ])
    );
}

#[test]
fn folder_drop_replaces_playlist_with_sorted_supported_media() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.webm"), "media").unwrap();
    fs::write(dir.path().join("cover.jpg"), "image").unwrap();
    fs::write(dir.path().join("a.mp4"), "media").unwrap();

    let action = classify_dropped_paths(&[dir.path().to_path_buf()]).unwrap();

    let DroppedMediaAction::ReplacePlaylist(entries) = action else {
        panic!("expected playlist replacement");
    };
    let titles: Vec<_> = entries.into_iter().map(|entry| entry.title).collect();
    assert_eq!(titles, vec!["a.mp4".to_string(), "z.webm".to_string()]);
}

#[test]
fn mixed_drop_ignores_unsupported_paths_when_supported_media_exists() {
    let dir = tempdir().unwrap();
    let unsupported = dir.path().join("notes.txt");
    let movie = dir.path().join("movie.mp4");
    fs::write(&unsupported, "notes").unwrap();
    fs::write(&movie, "media").unwrap();

    let action = classify_dropped_paths(&[unsupported, movie.clone()]).unwrap();

    assert_eq!(action, DroppedMediaAction::OpenFile(movie));
}

#[test]
fn unsupported_only_drop_reports_no_playable_media() {
    let dir = tempdir().unwrap();
    let unsupported = dir.path().join("cover.jpg");
    fs::write(&unsupported, "image").unwrap();

    let action = classify_dropped_paths(&[unsupported]).unwrap();

    assert_eq!(action, DroppedMediaAction::NoPlayableMedia { ignored_count: 1 });
}
