fn main() -> std::process::ExitCode {
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
