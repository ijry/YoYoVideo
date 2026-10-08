use std::ffi::OsString;
pub fn startup_report(args: &[OsString]) -> Option<String> {
    let [argument] = args else {
        return None;
    };
    match argument.to_str()? {
        "--version" => Some(format!("YoYoVideo {}", env!("CARGO_PKG_VERSION"))),
        "--build-info" => Some(
            serde_json::json!({
                // Keep the probe marker addressable in optimized x86_64 binaries.
                "schema": std::hint::black_box("yoyovideo-build-info-v1"),
                "version": env!("CARGO_PKG_VERSION"),
                "mpv_runtime": cfg!(feature = "mpv-runtime"),
                "updater": true,
                "updater_qa": yoyo_updater::qa_fixture_enabled(),
            })
            .to_string(),
        ),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_build_metadata_without_consuming_media_arguments() {
        let version = startup_report(&["--version".into()]).unwrap();
        assert_eq!(version, format!("YoYoVideo {}", env!("CARGO_PKG_VERSION")));
        let info: serde_json::Value =
            serde_json::from_str(&startup_report(&["--build-info".into()]).unwrap()).unwrap();
        assert_eq!(info["schema"], "yoyovideo-build-info-v1");
        assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(info["mpv_runtime"], cfg!(feature = "mpv-runtime"));
        assert_eq!(info["updater"], true);
        assert_eq!(info["updater_qa"], yoyo_updater::qa_fixture_enabled());
        assert!(startup_report(&["movie.mp4".into()]).is_none());
        assert!(startup_report(&["--version".into(), "movie.mp4".into()]).is_none());
    }
}
