use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;
use std::collections::HashMap;
use std::io::Cursor;
use tempfile::tempdir;

#[derive(Default)]
struct FakeTransport {
    bodies: Mutex<HashMap<String, Vec<u8>>>,
    requested: Mutex<Vec<String>>,
}
impl Transport for FakeTransport {
    fn open(&self, url: &str, _: Duration) -> Result<Box<dyn Read + Send>, UpdateError> {
        self.requested.lock().unwrap().push(url.into());
        self.bodies
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .map(|v| Box::new(Cursor::new(v)) as Box<dyn Read + Send>)
            .ok_or(UpdateError::Network("test response missing"))
    }
}
fn fixture() -> (SignedSource, Arc<FakeTransport>, velopack::bundle::Manifest) {
    let raw = serde_json::to_vec(&json!({"schema_version":1,"app_id":"YoYoVideo","platform":"windows-x64","channel":"stable-windows-x64","version":"0.0.2","release_tag":"v0.0.2","feed":{"Assets":[{"PackageId":"YoYoVideo","Version":"0.0.2","Type":"Full","FileName":"YoYoVideo-0.0.2-full.nupkg","Size":3,"SHA256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","SHA1":"a9993e364706816aba3e25717850c26c9cd0d89d","NotesMarkdown":"","NotesHtml":""}]}})).unwrap();
    let key = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let signature =
        minisign::sign(Some(&key.pk), &key.sk, raw.as_slice(), Some("source contract"), None)
            .unwrap();
    let public = STANDARD.encode(key.pk.to_box().unwrap().into_string());
    let transport = Arc::new(FakeTransport::default());
    transport.bodies.lock().unwrap().insert(
        format!("{REPOSITORY}/releases/latest/download/yoyovideo-update.windows-x64.json"),
        raw,
    );
    transport.bodies.lock().unwrap().insert(
        format!("{REPOSITORY}/releases/latest/download/yoyovideo-update.windows-x64.json.sig"),
        STANDARD.encode(signature.to_string()).into_bytes(),
    );
    transport.bodies.lock().unwrap().insert(
        format!("{REPOSITORY}/releases/download/v0.0.2/YoYoVideo-0.0.2-full.nupkg"),
        b"abc".to_vec(),
    );
    let source = SignedSource::from_transport(Platform::WindowsX64, public, transport.clone());
    let app = velopack::bundle::Manifest {
        id: "YoYoVideo".into(),
        channel: "stable-windows-x64".into(),
        version: semver::Version::new(0, 0, 1),
        ..Default::default()
    };
    (source, transport, app)
}
#[test]
fn authenticates_feed_and_downloads_from_its_frozen_release_tag() {
    let (source, transport, app) = fixture();
    let feed = source.get_release_feed(&app.channel, &app, "not-sent").unwrap();
    assert_eq!(feed.Assets[0].Version, "0.0.2");
    transport.bodies.lock().unwrap().remove(&format!(
        "{REPOSITORY}/releases/latest/download/yoyovideo-update.windows-x64.json"
    ));
    let dir = tempdir().unwrap();
    let file = dir.path().join("download.partial");
    let (tx, rx) = std::sync::mpsc::channel();
    source.download_release_entry(&feed.Assets[0], &file, Some(tx)).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"abc");
    assert_eq!(rx.try_iter().last(), Some(100));
    assert_eq!(transport.requested.lock().unwrap().len(), 3);
}
#[test]
fn rejects_modified_assets_and_corrupted_downloads() {
    let (source, transport, app) = fixture();
    let feed = source.get_release_feed(&app.channel, &app, "").unwrap();
    let dir = tempdir().unwrap();
    let file = dir.path().join("download.partial");
    let mut changed = feed.Assets[0].clone();
    changed.FileName = "different.nupkg".into();
    assert!(source.download_release_entry(&changed, &file, None).is_err());
    assert!(!file.exists());
    transport.bodies.lock().unwrap().insert(
        format!("{REPOSITORY}/releases/download/v0.0.2/YoYoVideo-0.0.2-full.nupkg"),
        b"abd".to_vec(),
    );
    assert!(source.download_release_entry(&feed.Assets[0], &file, None).is_err());
    assert!(!file.exists());
}
#[test]
fn failed_recheck_cannot_fall_back_to_an_old_verified_snapshot() {
    let (source, transport, app) = fixture();
    let feed = source.get_release_feed(&app.channel, &app, "").unwrap();
    transport.bodies.lock().unwrap().insert(
        format!("{REPOSITORY}/releases/latest/download/yoyovideo-update.windows-x64.json.sig"),
        b"invalid".to_vec(),
    );
    assert!(source.get_release_feed(&app.channel, &app, "").is_err());
    assert!(source.snapshot().is_none());
    let dir = tempdir().unwrap();
    assert!(
        source.download_release_entry(&feed.Assets[0], &dir.path().join("partial"), None).is_err()
    );
}
#[test]
fn unknown_app_or_channel_never_uses_the_network() {
    let (source, transport, mut app) = fixture();
    app.id = "Other".into();
    assert!(source.get_release_feed(&app.channel, &app, "").is_err());
    app.id = "YoYoVideo".into();
    assert!(source.get_release_feed("stable-linux-x64", &app, "").is_err());
    assert!(transport.requested.lock().unwrap().is_empty());
}
#[test]
fn http_is_rejected_before_making_any_request() {
    assert!(HttpsTransport.open("http://127.0.0.1:1", Duration::from_secs(1)).is_err());
}
