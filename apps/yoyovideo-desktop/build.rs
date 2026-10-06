fn main() {
    #[cfg(windows)]
    embed_windows_icon();

    slint_build::compile("ui/main-window.slint").expect("compile slint ui");
}

/// Embeds the app icon and version metadata into the Windows executable.
///
/// A warning rather than a failure when the resource compiler is missing: the
/// icon is cosmetic, and a checkout without the MSVC resource tools should still
/// build. The warning is loud because the alternative is a silently icon-less
/// binary that only shows up in a screenshot.
#[cfg(windows)]
fn embed_windows_icon() {
    let icon = std::path::Path::new("assets/icons/yoyovideo.ico");
    if !icon.exists() {
        println!(
            "cargo:warning={} is missing; run: node scripts/generate-icons.mjs",
            icon.display()
        );
        return;
    }

    println!("cargo:rerun-if-changed={}", icon.display());

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon.to_str().expect("icon path is valid UTF-8"));
    resource.set("FileDescription", "YoYoVideo");
    resource.set("ProductName", "YoYoVideo");
    resource.set("LegalCopyright", "GPL-3.0-or-later");

    match resource.compile() {
        Ok(()) => {}
        Err(error) => println!(
            "cargo:warning=could not embed the Windows icon ({error}); the executable will build without one"
        ),
    }
}
