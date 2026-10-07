use base64::{Engine, engine::general_purpose::STANDARD};
use semver::Version;
use serde_json::{Value, json};
use tempfile::tempdir;
use yoyo_updater::{Platform, VerifiedManifest};

fn fixture() -> Value {
    json!({
        "schema_version": 1, "app_id": "YoYoVideo", "platform": "windows-x64",
        "channel": "stable-windows-x64", "version": "0.0.2", "release_tag": "v0.0.2",
        "feed": {"Assets": [{
            "PackageId": "YoYoVideo", "Version": "0.0.2", "Type": "Full",
            "FileName": "YoYoVideo-0.0.2-full.nupkg", "Size": 3,
            "SHA256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "SHA1": "a9993e364706816aba3e25717850c26c9cd0d89d",
            "NotesMarkdown": "A signed release", "NotesHtml": ""
        }]}
    })
}
fn signed(raw: &[u8]) -> (String, String) {
    let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let key = STANDARD.encode(pair.pk.to_box().unwrap().into_string());
    let signature =
        minisign::sign(Some(&pair.pk), &pair.sk, raw, Some("yoyovideo test"), None).unwrap();
    (key, STANDARD.encode(signature.to_string()))
}
fn verified(value: &Value) -> Result<VerifiedManifest, yoyo_updater::UpdateError> {
    let raw = serde_json::to_vec(value).unwrap();
    let (key, signature) = signed(&raw);
    VerifiedManifest::verify(&raw, &signature, &key, Platform::WindowsX64)
}
#[test]
fn authenticated_manifest_and_package_are_accepted() {
    let manifest = verified(&fixture()).unwrap();
    assert_eq!(manifest.manifest().version, "0.0.2");
    let dir = tempdir().unwrap();
    let file = dir.path().join("package.nupkg");
    std::fs::write(&file, b"abc").unwrap();
    manifest.verify_package(&file).unwrap();
}
#[test]
fn modified_manifest_and_wrong_public_key_are_rejected() {
    let raw = serde_json::to_vec(&fixture()).unwrap();
    let (key, signature) = signed(&raw);
    let mut altered = raw.clone();
    altered.push(b' ');
    assert!(VerifiedManifest::verify(&altered, &signature, &key, Platform::WindowsX64).is_err());
    let (wrong_key, _) = signed(&raw);
    assert!(VerifiedManifest::verify(&raw, &signature, &wrong_key, Platform::WindowsX64).is_err());
}
#[test]
fn modified_trusted_comment_is_rejected() {
    let raw = serde_json::to_vec(&fixture()).unwrap();
    let (key, signature) = signed(&raw);
    let decoded = String::from_utf8(STANDARD.decode(signature).unwrap()).unwrap();
    let changed = STANDARD.encode(decoded.replace("yoyovideo test", "forged release"));
    assert!(VerifiedManifest::verify(&raw, &changed, &key, Platform::WindowsX64).is_err());
}
#[test]
fn malformed_or_missing_signature_material_is_rejected() {
    let raw = serde_json::to_vec(&fixture()).unwrap();
    let (key, signature) = signed(&raw);
    for bad in ["", "not base64!", "AA=="] {
        assert!(VerifiedManifest::verify(&raw, bad, &key, Platform::WindowsX64).is_err());
        assert!(VerifiedManifest::verify(&raw, &signature, bad, Platform::WindowsX64).is_err());
    }
    assert!(
        VerifiedManifest::verify(
            &raw,
            &signature[..signature.len() / 2],
            &key,
            Platform::WindowsX64
        )
        .is_err()
    );
}
#[test]
fn signed_but_wrong_release_context_is_rejected() {
    for (field, replacement) in [
        ("schema_version", json!(2)),
        ("app_id", json!("AnotherApp")),
        ("platform", json!("linux-x64")),
        ("channel", json!("beta-windows-x64")),
        ("version", json!("0.0.3")),
        ("version", json!("invalid")),
        ("version", json!("0.0.2-beta.1")),
        ("release_tag", json!("v0.0.3")),
    ] {
        let mut value = fixture();
        value[field] = replacement;
        assert!(verified(&value).is_err(), "accepted invalid field {field}");
    }
}
#[test]
fn a_signed_release_cannot_be_used_on_another_architecture() {
    let raw = serde_json::to_vec(&fixture()).unwrap();
    let (key, signature) = signed(&raw);
    assert!(VerifiedManifest::verify(&raw, &signature, &key, Platform::MacosArm64).is_err());
}
#[test]
fn rejects_missing_or_ambiguous_full_packages() {
    for assets in [
        json!([]),
        json!([fixture()["feed"]["Assets"][0].clone(), fixture()["feed"]["Assets"][0].clone()]),
    ] {
        let mut value = fixture();
        value["feed"]["Assets"] = assets;
        assert!(verified(&value).is_err());
    }
    for (field, replacement) in [
        ("PackageId", json!("Other")),
        ("Type", json!("Delta")),
        ("Version", json!("0.0.1")),
        ("SHA256", json!("")),
        ("SHA256", json!("z".repeat(64))),
        ("SHA1", json!("")),
        ("Size", json!(0)),
        ("Size", json!(2_u64 * 1024 * 1024 * 1024 + 1)),
    ] {
        let mut value = fixture();
        value["feed"]["Assets"][0][field] = replacement;
        assert!(verified(&value).is_err(), "accepted invalid asset field {field}");
    }
}
#[test]
fn rejects_package_paths_and_unsafe_names() {
    for name in [
        "../evil.nupkg",
        "dir/evil.nupkg",
        r"dir\evil.nupkg",
        "C:evil.nupkg",
        "/evil.nupkg",
        "evil.exe",
        "..",
        "evil%2fpackage.nupkg",
        "evil.nupkg ",
        "CON.nupkg",
    ] {
        let mut value = fixture();
        value["feed"]["Assets"][0]["FileName"] = json!(name);
        assert!(verified(&value).is_err(), "accepted {name}");
    }
}
#[test]
fn tampered_and_truncated_packages_are_rejected_even_after_prior_validation() {
    let manifest = verified(&fixture()).unwrap();
    let dir = tempdir().unwrap();
    let path = dir.path().join("package.nupkg");
    std::fs::write(&path, b"abc").unwrap();
    manifest.verify_package(&path).unwrap();
    std::fs::write(&path, b"abd").unwrap();
    assert!(manifest.verify_package(&path).is_err());
    std::fs::write(&path, b"ab").unwrap();
    assert!(manifest.verify_package(&path).is_err());
    assert!(manifest.verify_package(&dir.path().join("missing")).is_err());
}
#[test]
fn same_version_and_downgrades_are_not_updates() {
    let manifest = verified(&fixture()).unwrap();
    assert!(manifest.is_newer_than(&Version::parse("0.0.1").unwrap()));
    assert!(!manifest.is_newer_than(&Version::parse("0.0.2").unwrap()));
    assert!(!manifest.is_newer_than(&Version::parse("0.0.3").unwrap()));
}
#[test]
fn envelope_and_signature_limits_are_enforced() {
    let raw = vec![b' '; 8 * 1024 * 1024 + 1];
    assert!(VerifiedManifest::verify(&raw, "", "", Platform::WindowsX64).is_err());
    let raw = serde_json::to_vec(&fixture()).unwrap();
    let (key, _) = signed(&raw);
    assert!(
        VerifiedManifest::verify(&raw, &"A".repeat(16 * 1024 + 1), &key, Platform::WindowsX64)
            .is_err()
    );
}
