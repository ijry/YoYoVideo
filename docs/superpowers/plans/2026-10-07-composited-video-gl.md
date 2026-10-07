# GL Composited Video Implementation Plan

> **For agentic workers:** Use executing-plans to implement this plan task-by-task, inline in this session.

**Goal:** Complete the existing Slint/mpv GL video integration for single-video Wayland playback.
**Architecture:** Preserve native host paths. A Slint rendering notifier owns the GL lifecycle; mpv update callbacks request UI redraws. Resolve GL symbols from the active Slint context rather than process-global lookup.
**Tech Stack:** Rust, Slint 1.17, winit 0.30, libmpv, OpenGL.
**Spec:** `docs/superpowers/specs/2026-10-07-composited-video-gl-design.md`

## Global Constraints
- Keep default builds independent of libmpv.
- Preserve Windows/X11/macOS video paths; no new runtime dependency.
- No GL operations without the owning context current.
- No Wayland grid or verified-platform claim in this change.

### Task 1: GL callback and context contract
Files: `crates/yoyo-mpv/src/{render_gl,client,gl_texture,options}.rs`, runtime/option contract tests
- [x] Add and run a failing missing-loader/runtime regression test.
- [x] Replace mismatched opaque callback pointer with a correctly typed scoped holder; add borrowed-loader construction while retaining the owned-loader API.
- [x] Use non-advanced render control for synchronous UI callers; preserve GL bindings around caller rendering.
- [x] Run runtime and GL unit tests.

### Task 2: Composited surface and runtime integration
Files: `apps/yoyovideo-desktop/src/{video_surface_gl,composited_video_runtime,video_surface_gl_smoke,app,lib}.rs`
- [x] Add failing behavioral tests for actual-handle routing, redraw coalescing and physical dimensions.
- [x] Resolve through Slint, implement setup/render/teardown and deferred startup.
- [x] Register notifier, request Linux OpenGL, preserve native host setup, reject Wayland grids explicitly.
- [x] Run targeted tests and feature-enabled build.

### Task 3: Regression validation and limitations
Files: platform documentation and `docs/testing/manual-smoke-checklist.md`
- [x] Run workspace default and feature-enabled suites; inspect diff and formatting.
- [x] Document renderer requirements, unsupported grid mode and unverified Wayland smoke scenarios.
- [x] Record actual verification results; leave changes uncommitted for review.


## Validation findings
- The new missing-GL-loader regression initially crashed with Windows STATUS_ACCESS_VIOLATION. The corrected opaque callback holder now returns the expected unsupported-GL error.
- The real GL smoke initially decoded video but kept a black FBO: mpv still auto-selected its own VO. Added an explicit `render_api` option emitting `vo=libmpv` for composited and macOS render-API backends. The smoke then read red pixels at 64x64 and 128x64 and confirmed teardown.
- Existing default-feature runtime tests incorrectly ran with mpv-runtime enabled; restricted those disabled-runtime assertions to the feature configuration they describe.


## Final verification (2026-10-07)
- `cargo test --workspace -j 1`: passed.
- `cargo test --workspace --features mpv-runtime -j 1`: passed; the desktop-only GL smoke is ignored in this ordinary suite.
- Opt-in `native_gl_decodes_resizes_and_tears_down`: passed against the local Windows OpenGL driver and staged libmpv, including real decoded pixels, resizing and teardown.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- macOS `x86_64-apple-darwin` and `aarch64-apple-darwin` cargo checks: passed. The cross-build emits the pre-existing Windows-icon resource warning; this is not a macOS runtime test.
- Initial concurrent link/cross-check attempts exhausted local memory. Serial retries passed without changing application code to work around the environment.
- Wayland compositor playback and macOS hardware playback remain unverified. Changes remain uncommitted on `feat/composited-video-gl`.
