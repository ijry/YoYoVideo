// This is a GUI application, including when installed via a Windows shortcut.
// Startup failures are still written to the diagnostic log below.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    #[cfg(feature = "updater-qa")]
    yoyovideo_desktop::trace_updater_qa("before-velopack-startup");
    let startup = velopack::VelopackApp::build().set_auto_apply_on_startup(false);
    #[cfg(feature = "updater-qa")]
    let startup = startup.set_locator(
        yoyo_updater::QaFixture::from_env().and_then(|f| f.native_locator()).unwrap_or_default(),
    );
    let mut startup = startup;
    startup.run();
    #[cfg(feature = "updater-qa")]
    yoyovideo_desktop::trace_updater_qa("after-velopack-startup");
    if let Some(report) =
        yoyovideo_desktop::startup_report(&std::env::args_os().skip(1).collect::<Vec<_>>())
    {
        println!("{report}");
        return std::process::ExitCode::SUCCESS;
    }
    match yoyovideo_desktop::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            #[cfg(feature = "updater-qa")]
            yoyovideo_desktop::trace_updater_qa("fatal-startup-error");
            let message = format!("Fatal startup error: {error}");
            eprintln!("{message}");
            let _ = yoyovideo_desktop::platform::append_diagnostic(None, "ERROR", &message);
            std::process::ExitCode::FAILURE
        }
    }
}
