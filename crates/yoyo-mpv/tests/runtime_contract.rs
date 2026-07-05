use yoyo_mpv::{MpvBackend, MpvError};

#[test]
fn runtime_backend_requires_mpv_runtime_feature_by_default() {
    let error = MpvBackend::new_runtime().err().expect("runtime should be disabled by default");
    assert!(matches!(error, MpvError::RuntimeDisabled));
}
