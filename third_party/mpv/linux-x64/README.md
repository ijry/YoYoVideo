# Linux x64 libmpv Runtime Files

Expected files for runtime-enabled packaging:

- `lib/libmpv.so` or versioned `lib/libmpv.so*`: libmpv library used by Rust linking and copied into the package.

Keep binaries untracked until licensing is approved. Distribution package dependencies may still be needed for developer builds, but release packages should not silently rely on an unknown user-installed libmpv.
