use std::path::PathBuf;
use std::time::Duration;

use tempfile::tempdir;
use yoyo_core::{HistoryEntry, HistoryStore, MediaLocator};
use yoyovideo_desktop::{FlushReason, HistoryActivationError, HistoryRuntime, PendingResumeSeek};

#[test]
fn periodic_flush_is_throttled_to_two_seconds() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), HistoryStore::default(), true);

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(10.0),
    );
    assert!(runtime.flush_if_needed(Duration::from_secs(0), FlushReason::PeriodicTick).unwrap());

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(12.0),
    );
    assert!(!runtime.flush_if_needed(Duration::from_secs(1), FlushReason::PeriodicTick).unwrap());
    assert!(runtime.flush_if_needed(Duration::from_secs(2), FlushReason::PeriodicTick).unwrap());
}

#[test]
fn pause_flush_bypasses_the_periodic_throttle_window() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), HistoryStore::default(), true);

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(10.0),
    );
    runtime.flush_if_needed(Duration::from_secs(0), FlushReason::PeriodicTick).unwrap();

    runtime.remember_playback(
        &MediaLocator::Url("https://example.com/a.mp4".into()),
        "A",
        Some(11.0),
    );
    assert!(runtime.flush_if_needed(Duration::from_secs(1), FlushReason::Pause).unwrap());
}

#[test]
fn activation_rejects_missing_files_and_resume_seek_clamps_to_duration() {
    let missing = PathBuf::from("Z:/missing/movie.mp4");
    let runtime = HistoryRuntime::new(
        None,
        HistoryStore {
            items: vec![HistoryEntry {
                locator: MediaLocator::File(missing.clone()),
                title: "Missing".into(),
                last_position_seconds: Some(999.0),
            }],
        },
        true,
    );

    let error = runtime.activation_for(0).unwrap_err();
    assert_eq!(error, HistoryActivationError::MissingLocalFile(missing));

    let pending = PendingResumeSeek::new(999.0).unwrap();
    assert_eq!(pending.try_resolve(Some(120.0)), Some(120.0));
    assert_eq!(pending.try_resolve(None), None);
}

fn stored_entry(title: &str) -> HistoryEntry {
    HistoryEntry {
        locator: MediaLocator::Url(format!("https://example.com/{title}.mp4")),
        title: title.into(),
        last_position_seconds: Some(12.0),
    }
}

#[test]
fn clearing_history_writes_the_purge_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let store = HistoryStore { items: vec![stored_entry("a"), stored_entry("b")] };
    let mut runtime = HistoryRuntime::new(Some(path.clone()), store, true);

    assert!(runtime.clear().unwrap());
    assert!(runtime.store().items().is_empty());

    // The purge must survive a restart, so it has to be on disk already.
    let reloaded = HistoryStore::load(&path).unwrap();
    assert!(reloaded.items().is_empty());
}

#[test]
fn clearing_history_persists_even_when_recording_is_disabled() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let store = HistoryStore { items: vec![stored_entry("a")] };
    // flush_if_needed() is a no-op while disabled, so clear() must write through.
    let mut runtime = HistoryRuntime::new(Some(path.clone()), store, false);

    assert!(runtime.clear().unwrap());
    assert!(HistoryStore::load(&path).unwrap().items().is_empty());
}

#[test]
fn clearing_empty_history_is_a_no_op() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut runtime = HistoryRuntime::new(Some(path.clone()), HistoryStore::default(), true);

    assert!(!runtime.clear().unwrap());
    // Nothing to purge, so no file should have been created.
    assert!(!path.exists());
}
