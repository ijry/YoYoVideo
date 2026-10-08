use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use tempfile::{TempDir, tempdir};
use yoyo_update_sign::{sign_release, verify_release, write_release};
use yoyo_updater::{Platform, VerifiedManifest};

const PASSWORD: &str = "test-only-password";
fn keys() -> &'static (String, String) {
    static KEYS: OnceLock<(String, String)> = OnceLock::new();
    KEYS.get_or_init(|| {
        let pair = minisign::KeyPair::generate_encrypted_keypair(Some(PASSWORD.into())).unwrap();
        (
            STANDARD.encode(pair.pk.to_box().unwrap().into_string()),
            STANDARD.encode(pair.sk.to_box(None).unwrap().into_string()),
        )
    })
}
fn fixture() -> (TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("YoYoVideo-0.0.2-full.nupkg"), b"abc").unwrap();
    let feed = json!({"Assets": [{"PackageId":"YoYoVideo", "Version":"0.0.2", "Type":"Full",
        "FileName":"YoYoVideo-0.0.2-full.nupkg", "Size":3,
        "SHA256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "SHA1":"a9993e364706816aba3e25717850c26c9cd0d89d", "NotesMarkdown":"Notes", "NotesHtml":""}]});
    let path = dir.path().join("releases.stable-windows-x64.json");
    std::fs::write(&path, serde_json::to_vec(&feed).unwrap()).unwrap();
    (dir, path)
}
fn make_signature(feed: &Path, dir: &Path) -> (Vec<u8>, String) {
    let (public, private) = keys();
    sign_release(feed, dir, Platform::WindowsX64, "0.0.2", public, private, PASSWORD).unwrap()
}
#[test]
fn encrypted_keys_produce_a_manifest_accepted_by_the_independent_verifier() {
    let (dir, feed) = fixture();
    let (raw, signature) = make_signature(&feed, dir.path());
    let verified =
        VerifiedManifest::verify(&raw, &signature, &keys().0, Platform::WindowsX64).unwrap();
    assert_eq!(verified.manifest().release_tag, "v0.0.2");
    assert_eq!(verified.asset().NotesMarkdown, "Notes");
    verify_release(&raw, &signature, &keys().0, Platform::WindowsX64, dir.path()).unwrap();
}
#[test]
fn mismatched_or_missing_package_is_not_signed() {
    let (dir, feed) = fixture();
    let (public, private) = keys();
    let package = dir.path().join("YoYoVideo-0.0.2-full.nupkg");
    std::fs::write(&package, b"abd").unwrap();
    assert!(
        sign_release(&feed, dir.path(), Platform::WindowsX64, "0.0.2", public, private, PASSWORD)
            .is_err()
    );
    std::fs::remove_file(&package).unwrap();
    assert!(
        sign_release(&feed, dir.path(), Platform::WindowsX64, "0.0.2", public, private, PASSWORD)
            .is_err()
    );
}
#[test]
fn wrong_password_and_key_pair_are_rejected_without_leaking_inputs() {
    let (dir, feed) = fixture();
    let (public, private) = keys();
    let error = sign_release(
        &feed,
        dir.path(),
        Platform::WindowsX64,
        "0.0.2",
        public,
        private,
        "wrong-sensitive-password",
    )
    .unwrap_err();
    assert!(!error.to_string().contains("wrong-sensitive-password"));
    assert!(!format!("{error:?}").contains(private));
    let other = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let wrong_public = STANDARD.encode(other.pk.to_box().unwrap().into_string());
    assert!(
        sign_release(
            &feed,
            dir.path(),
            Platform::WindowsX64,
            "0.0.2",
            &wrong_public,
            private,
            PASSWORD
        )
        .is_err()
    );
}
#[test]
fn feed_version_mismatch_and_empty_feed_are_rejected() {
    let (dir, feed) = fixture();
    let (public, private) = keys();
    assert!(
        sign_release(&feed, dir.path(), Platform::WindowsX64, "0.0.3", public, private, PASSWORD)
            .is_err()
    );
    assert!(
        sign_release(&feed, dir.path(), Platform::LinuxX64, "0.0.2", public, private, PASSWORD)
            .is_err()
    );
    std::fs::write(&feed, br#"{"Assets":[]}"#).unwrap();
    assert!(
        sign_release(&feed, dir.path(), Platform::WindowsX64, "0.0.2", public, private, PASSWORD)
            .is_err()
    );
}
#[test]
fn an_existing_release_is_never_overwritten() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("update.json");
    std::fs::write(&output, b"original").unwrap();
    assert!(write_release(&output, b"new", "sig").is_err());
    assert_eq!(std::fs::read(&output).unwrap(), b"original");
    assert!(!dir.path().join("update.json.sig").exists());
}
#[test]
fn a_signature_collision_does_not_leave_a_publishable_manifest() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("update.json");
    std::fs::write(dir.path().join("update.json.sig"), b"original").unwrap();
    assert!(write_release(&output, b"new", "new-signature").is_err());
    assert!(!output.exists());
    assert_eq!(std::fs::read(dir.path().join("update.json.sig")).unwrap(), b"original");
}
#[test]
fn cli_sign_and_verify_use_files_and_do_not_print_credentials() {
    let (dir, feed) = fixture();
    let (public, private) = keys();
    let output = dir.path().join("update.json");
    let public_path = dir.path().join("public.key");
    std::fs::write(&public_path, public).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_yoyo-update-sign"))
        .args(["sign", "--platform", "windows-x64", "--version", "0.0.2"])
        .arg("--feed")
        .arg(&feed)
        .arg("--assets-dir")
        .arg(dir.path())
        .arg("--output")
        .arg(&output)
        .arg("--public-key")
        .arg(&public_path)
        .env("YOYOVIDEO_UPDATER_PRIVATE_KEY", private)
        .env("YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD", PASSWORD)
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(output.exists());
    assert!(dir.path().join("update.json.sig").exists());
    assert!(!String::from_utf8_lossy(&result.stdout).contains(private));
    assert!(!String::from_utf8_lossy(&result.stderr).contains(PASSWORD));
    let verified = Command::new(env!("CARGO_BIN_EXE_yoyo-update-sign"))
        .args(["verify", "--platform", "windows-x64"])
        .arg("--manifest")
        .arg(&output)
        .arg("--assets-dir")
        .arg(dir.path())
        .arg("--public-key")
        .arg(&public_path)
        .output()
        .unwrap();
    assert!(verified.status.success(), "{}", String::from_utf8_lossy(&verified.stderr));
}
#[test]
fn cli_invalid_arguments_fail_without_creating_outputs() {
    for args in [
        vec!["sign"],
        vec!["unknown"],
        vec!["verify", "--private-key", "secret"],
        vec!["verify", "--platform", "windows-x64", "--platform", "linux-x64"],
    ] {
        let result =
            Command::new(env!("CARGO_BIN_EXE_yoyo-update-sign")).args(args).output().unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("secret"));
    }
}
