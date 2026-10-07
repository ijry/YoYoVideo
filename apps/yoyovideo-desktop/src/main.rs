// This is a GUI application, including when installed via a Windows shortcut.
// Startup failures are still written to the diagnostic log below.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    velopack::VelopackApp::build().set_auto_apply_on_startup(false).run();
    match yoyovideo_desktop::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("Fatal startup error: {error}");
            eprintln!("{message}");
            let _ = yoyovideo_desktop::platform::append_diagnostic(None, "ERROR", &message);
            std::process::ExitCode::FAILURE
        }
    }
}
