use yoyo_updater::Platform;
#[test]
fn release_channels_keep_architectures_separate() {
    for (platform, name, channel) in [
        (Platform::WindowsX64, "windows-x64", "stable-windows-x64"),
        (Platform::MacosArm64, "macos-aarch64", "stable-macos-aarch64"),
        (Platform::MacosX64, "macos-x86_64", "stable-macos-x86_64"),
        (Platform::LinuxX64, "linux-x64", "stable-linux-x64"),
    ] {
        assert_eq!(platform.as_str(), name);
        assert_eq!(platform.channel(), channel);
        assert_eq!(name.parse::<Platform>().unwrap(), platform);
    }
    assert!("windows-arm64".parse::<Platform>().is_err());
    assert!("../windows-x64".parse::<Platform>().is_err());
}
