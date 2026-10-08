use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use yoyo_update_sign::{
    SignError, read_bounded, sign_release, signature_path, verify_release, write_release,
};
use yoyo_updater::{MAX_MANIFEST_BYTES, MAX_SIGNATURE_BYTES, Platform};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<(), SignError> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or(SignError::Input("expected sign or verify"))?;
    let allowed: &[&str] = match command.as_str() {
        "sign" => {
            &["--feed", "--platform", "--version", "--assets-dir", "--output", "--public-key"]
        }
        "verify" => &["--manifest", "--platform", "--assets-dir", "--public-key"],
        _ => return Err(SignError::Input("expected sign or verify")),
    };
    let mut options = BTreeMap::new();
    while let Some(flag) = args.next() {
        if !allowed.contains(&flag.as_str()) {
            return Err(SignError::Input("unknown option"));
        }
        let value = args
            .next()
            .filter(|v| !v.starts_with("--"))
            .ok_or(SignError::Input("missing option value"))?;
        if options.insert(flag, value).is_some() {
            return Err(SignError::Input("duplicate option"));
        }
    }
    if options.len() != allowed.len() {
        return Err(SignError::Input("missing required options"));
    }
    let platform: Platform = options["--platform"].parse()?;
    let public_key = String::from_utf8(read_bounded(Path::new(&options["--public-key"]), 4096)?)
        .map_err(|_| SignError::Key)?;
    let assets_dir = Path::new(&options["--assets-dir"]);
    if command == "sign" {
        let private_key =
            std::env::var("YOYOVIDEO_UPDATER_PRIVATE_KEY").map_err(|_| SignError::Key)?;
        let password =
            std::env::var("YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD").map_err(|_| SignError::Key)?;
        let (raw, signature) = sign_release(
            Path::new(&options["--feed"]),
            assets_dir,
            platform,
            &options["--version"],
            &public_key,
            &private_key,
            &password,
        )?;
        write_release(Path::new(&options["--output"]), &raw, &signature)?;
    } else {
        let manifest_path = Path::new(&options["--manifest"]);
        let raw = read_bounded(manifest_path, MAX_MANIFEST_BYTES)?;
        let signature =
            String::from_utf8(read_bounded(&signature_path(manifest_path), MAX_SIGNATURE_BYTES)?)
                .map_err(|_| SignError::Input("invalid signature encoding"))?;
        verify_release(&raw, &signature, &public_key, platform, assets_dir)?;
    }
    println!("Release {} completed for {}", command, platform.as_str());
    Ok(())
}
