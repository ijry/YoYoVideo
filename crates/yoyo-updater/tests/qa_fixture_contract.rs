#![cfg(feature = "qa-fixture")]
use std::fs;
use tempfile::tempdir;
use yoyo_updater::QaFixture;

#[test]
fn fixtures_require_a_marked_root_and_do_not_fall_back_to_user_directories() {
    let dir = tempdir().unwrap();
    assert!(QaFixture::open(dir.path()).is_err());
    fs::write(dir.path().join("TEST-ONLY"), "not a fixture").unwrap();
    assert!(QaFixture::open(dir.path()).is_err());
    fs::write(dir.path().join("TEST-ONLY"), "YoYoVideo updater QA fixture v1\n").unwrap();
    let fixture = QaFixture::open(dir.path()).unwrap();
    assert_eq!(fixture.root(), dir.path().canonicalize().unwrap());
    assert!(fixture.source_file("../outside").is_err());
    assert!(fixture.source_file("C:outside").is_err());
    fs::create_dir(dir.path().join("source")).unwrap();
    fs::write(dir.path().join("source/release.json"), "test").unwrap();
    assert_eq!(fs::read(fixture.source_file("release.json").unwrap()).unwrap(), b"test");
}

#[test]
fn fixture_key_cannot_be_an_absent_or_unbounded_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("TEST-ONLY"), "YoYoVideo updater QA fixture v1\n").unwrap();
    let fixture = QaFixture::open(dir.path()).unwrap();
    assert!(fixture.public_key().is_err());
    fs::write(dir.path().join("public.key"), "x".repeat(8192)).unwrap();
    assert!(fixture.public_key().is_err());
    fs::write(dir.path().join("public.key"), "test-only-public-key").unwrap();
    assert_eq!(fixture.public_key().unwrap(), "test-only-public-key");
}
