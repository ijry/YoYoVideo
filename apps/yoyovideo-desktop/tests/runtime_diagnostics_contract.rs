use yoyovideo_desktop::format_runtime_startup_error;

#[test]
fn windows_runtime_startup_error_mentions_mpv_dll_and_bootstrap_command() {
    let message = format_runtime_startup_error("backend init failed");

    if cfg!(target_os = "windows") {
        assert!(message.contains("mpv-2.dll"));
        assert!(message.contains("scripts/bootstrap-runtime.ps1"));
        assert!(message.contains("backend init failed"));
    } else {
        assert!(message.contains("backend init failed"));
        assert!(message.contains("libmpv"));
    }
}
