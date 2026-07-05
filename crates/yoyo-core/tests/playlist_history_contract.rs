use std::path::PathBuf;

use yoyo_core::{
    AppCommand, AppConfig, AppSession, BackendCommand, BackendEvent, HistoryStore, MediaLocator,
    PlayerBackend, PlaylistEntry,
};

#[derive(Default)]
struct MockBackend {
    opened: Vec<MediaLocator>,
    commands: Vec<BackendCommand>,
}

impl PlayerBackend for MockBackend {
    fn open(&mut self, locator: &MediaLocator) -> Result<(), String> {
        self.opened.push(locator.clone());
        Ok(())
    }

    fn send(&mut self, command: BackendCommand) -> Result<(), String> {
        self.commands.push(command);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        Vec::new()
    }
}

#[test]
fn open_file_replaces_playlist_with_a_single_entry_snapshot() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("first.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("second.mp4"))),
            ],
            1,
        )
        .unwrap();

    session.handle_command(AppCommand::OpenFile(PathBuf::from("solo.mkv"))).unwrap();

    let snapshot = session.playlist_snapshot();
    assert_eq!(snapshot.current_index, Some(0));
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].locator, MediaLocator::File(PathBuf::from("solo.mkv")));
    assert_eq!(session.state().current, Some(MediaLocator::File(PathBuf::from("solo.mkv"))));
}

#[test]
fn open_playlist_index_switches_to_the_requested_queue_entry() {
    let mut session = AppSession::new(AppConfig::default(), MockBackend::default());
    session
        .replace_playlist(
            vec![
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("alpha.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("beta.mp4"))),
                PlaylistEntry::new(MediaLocator::File(PathBuf::from("gamma.mp4"))),
            ],
            0,
        )
        .unwrap();

    session.open_playlist_index(2).unwrap();

    let snapshot = session.playlist_snapshot();
    assert_eq!(snapshot.current_index, Some(2));
    assert_eq!(
        session.backend().opened.last(),
        Some(&MediaLocator::File(PathBuf::from("gamma.mp4")))
    );
    assert_eq!(session.state().current, Some(MediaLocator::File(PathBuf::from("gamma.mp4"))));
}

#[test]
fn remember_moves_an_existing_locator_to_the_front() {
    let mut store = HistoryStore::default();

    store.remember(
        MediaLocator::Url("https://example.com/first.mp4".into()),
        "First".into(),
        Some(12.0),
    );
    store.remember(
        MediaLocator::Url("https://example.com/second.mp4".into()),
        "Second".into(),
        Some(48.0),
    );
    store.remember(
        MediaLocator::Url("https://example.com/first.mp4".into()),
        "First renamed".into(),
        Some(99.0),
    );

    assert_eq!(store.items().len(), 2);
    assert_eq!(store.items()[0].title, "First renamed");
    assert_eq!(store.items()[0].last_position_seconds, Some(99.0));
    assert_eq!(store.items()[0].locator, MediaLocator::Url("https://example.com/first.mp4".into()));
}

#[test]
fn history_entry_lookup_is_bounds_checked() {
    let mut store = HistoryStore::default();
    store.remember(
        MediaLocator::Url("https://example.com/video.mp4".into()),
        "Video".into(),
        Some(35.0),
    );

    assert!(store.entry(0).is_some());
    assert!(store.entry(5).is_none());
}
