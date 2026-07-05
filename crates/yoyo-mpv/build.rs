use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if env::var_os("CARGO_FEATURE_MPV_RUNTIME").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let workspace_root = manifest_dir.parent().and_then(Path::parent).expect("workspace root");

    let Some(lib_dir) = staged_runtime_lib_dir(workspace_root) else {
        return;
    };

    if lib_dir.is_dir() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rerun-if-changed={}", lib_dir.display());
    }
}

fn staged_runtime_lib_dir(workspace_root: &Path) -> Option<PathBuf> {
    let target_os = env::var("CARGO_CFG_TARGET_OS").ok()?;
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").ok()?;

    let platform_dir = match (target_os.as_str(), target_arch.as_str()) {
        ("windows", "x86_64") => "windows-x64",
        ("macos", _) => "macos-universal",
        ("linux", "x86_64") => "linux-x64",
        _ => return None,
    };

    Some(workspace_root.join("third_party").join("mpv").join(platform_dir).join("lib"))
}
