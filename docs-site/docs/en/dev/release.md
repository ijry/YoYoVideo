---
title: Release process
description: How YoYoVideo versions releases, what the tag triggers, and what ends up on the release page.
---

# Release process

## One source for the version

The version lives in `[workspace.package]` at the root of the repository:

```toml
[workspace.package]
version = "0.0.1"
```

All three crates inherit it with `version.workspace = true`, so there is no second place to change.
`.github/workflows/release.yml` compares the tag against this value and refuses to publish on a
mismatch — "the code says 0.0.1 but the tag says 0.0.2" cannot happen quietly.

## Cutting a release

```powershell
# 1. Update version in Cargo.toml and write the changelog
# 2. Commit and push
git commit -am "release: 0.0.1"
git push origin main

# 3. Tag — the commit message *is* the changelog for this version
git tag -a v0.0.1 -m @"
## 0.0.1

First public release.

- Multi-tile batch playback
- Bundled libmpv runtime, no decoders to install
- Windows x64 portable package and installer
"@
git push origin v0.0.1
```

Pushing the tag starts `.github/workflows/release.yml`.

::: warning The tag commit's message is the changelog
The release body is taken verbatim from the tag commit's full message. GitHub's generated notes
for a release tag are only a compare link, which is not a changelog. Make sure the message is not
empty before you push.
:::

## What the pipeline does

1. **prepare** — validate the tag format, compare it against `Cargo.toml`, read the tag commit
   message as the changelog, and confirm which platforms may be published.
2. **build** (`windows-latest`) — fetch and verify the playback core, package, **run the playback
   smoke test**, build the NSIS installer, upload artifacts.
3. **publish** (`ubuntu-latest`) — download the artifacts, generate the release body with
   `scripts/create-release-body.mjs` (changelog, asset table, per-asset SHA-256), then create or
   update the Release.
4. Dispatch `docs.yml` so the documentation site redeploys.

The asset list is **generated from what was actually built**. If a platform's job fails, the release
page never links to a file that does not exist.

## Why only Windows ships

Every platform has an entry in `runtime/manifest.toml` with an `available` flag. macOS and Linux are
currently `available = false`, for reasons written into their own `notes`: macOS has no vetted
universal libmpv build (upstream publishes Windows builds only, and Homebrew ships per-architecture
bottles, so claiming "universal" would be a lie), and Linux would require bundling libmpv's entire
dependency closure.

`prepare` checks this up front, rather than discovering it halfway through a matrix.

## Adding a platform

1. Fill in a real `source_url`, `sha256` and `version` for that entry in `runtime/manifest.toml`, and
   flip `available` to `true`.
2. Add the normalization for the platform in `scripts/fetch-runtime.ps1` (the Windows branch renames
   the DLL and rebuilds the import library; other platforms drop the archive contents into `lib/`).
3. Run `fetch` → `package` → `smoke-package` locally and confirm playback really works.
4. Add the platform to the matrix in `release.yml` and to the platform list in `prepare`.

## Re-running a release

If a platform build failed and you want to retry the same tag:

```powershell
gh workflow run release.yml --ref v0.0.1 -f tag=v0.0.1
```

`allowUpdates: true`, so a re-run replaces the assets on the same Release.
