#[allow(dead_code)]
mod build_script {
    include!("../build.rs");
    #[test]
    fn macos_runtime_paths_follow_the_native_architecture() {
        assert_eq!(runtime_platform("macos", "aarch64"), Some("macos-aarch64"));
        assert_eq!(runtime_platform("macos", "x86_64"), Some("macos-x86_64"));
        assert_eq!(runtime_platform("linux", "x86_64"), Some("linux-x64"));
        assert_eq!(runtime_platform("windows", "x86_64"), Some("windows-x64"));
        assert_eq!(runtime_platform("macos", "unknown"), None);
    }
}
