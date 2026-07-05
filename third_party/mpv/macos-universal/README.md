# macOS Universal libmpv Runtime Files

Expected files for runtime-enabled packaging:

- `lib/libmpv.dylib`: libmpv library used by Rust linking and copied into the package.

Keep binaries untracked until licensing is approved. The package script copies `lib/*` into the package `bin/` directory for this foundation phase. A later app-bundle phase can move these files into a framework or bundle-specific location.
