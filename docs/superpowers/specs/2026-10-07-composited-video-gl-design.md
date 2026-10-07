# GL Composited Video Design

Approved direction: continue the existing GL texture/UI-frame work, preserving native paths.

- Select compositing from the actual Wayland window handle, never desktop environment variables. Windows/X11 and the macOS child-surface render path remain unchanged.
- Register the Slint notifier before showing the window. Resolve GL functions through Slint's current-context loader; mpv resolves its GL entry points synchronously during render-context creation. Fix the existing callback context pointer mismatch as part of this boundary.
- Request an OpenGL Slint renderer on Linux. Create the playback backend before the render context, but delay startup media until rendering is ready. Surface errors are surfaced once, without a repeated retry/log loop.
- Use mpv update notifications to enqueue UI redraw requests, not a permanent high-frequency timer. Consume update flags before rendering, publish the borrowed image, and report presentation after the UI draw. Use default (non-advanced) mpv control because the UI also makes synchronous player calls.
- Clear the published image before deleting its texture. Run all GL cleanup in RenderingTeardown with the owning context current; retain the mpv core if an exceptional missing teardown would otherwise leave a dangling render context.
- Render at the video area's physical-pixel dimensions; preserve framebuffer/viewport/texture bindings around custom GL calls. Keep the frame behind overlays and let existing Slint input handling work.
- Wayland grid playback remains unsupported: reject it explicitly before creating child windows. No claim of verified Wayland playback without a real Wayland smoke test.

Validation: behavior tests for loader invocation, route selection, physical dimensions, update coalescing and GL state/resource operations; default workspace tests; mpv-runtime build/tests on Windows; documented Wayland smoke checklist.
