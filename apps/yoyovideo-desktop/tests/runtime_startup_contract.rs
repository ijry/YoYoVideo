use yoyovideo_desktop::build_desktop_backend;

#[test]
fn desktop_backend_requires_runtime_feature_by_default() {
    let error = build_desktop_backend().err().expect("runtime should be disabled by default");
    assert!(error.to_string().contains("disabled"));
}
