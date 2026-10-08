//! Generates disposable test credentials; never uses or prints production credentials.
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args_os().nth(1).ok_or("test output directory required")?,
    );
    std::fs::create_dir_all(&root)?;
    let pair = minisign::KeyPair::generate_encrypted_keypair(Some("fixture-only".into()))?;
    use base64::{Engine, engine::general_purpose::STANDARD};
    for (name, value) in [
        (
            "TEST-ONLY-private.key",
            STANDARD.encode(pair.sk.to_box(Some("DISPOSABLE TEST KEY"))?.into_string()),
        ),
        ("TEST-ONLY-public.key", STANDARD.encode(pair.pk.to_box()?.into_string())),
    ] {
        let mut file =
            std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(name))?;
        file.write_all(value.as_bytes())?;
    }
    Ok(())
}
