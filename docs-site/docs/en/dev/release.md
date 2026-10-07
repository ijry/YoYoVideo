---
title: Release process
description: Four-platform Velopack builds, authenticated updates and draft verification gates.
---

# Release process

The single version source is `[workspace.package]` in the root `Cargo.toml`; crates inherit it.
Only matching stable `vX.Y.Z` **annotated tags** are accepted. Release notes come from the tag annotation, not the target commit message.

## Pipeline

1. Commit a tested version, prepare nonempty tag notes and push the corresponding tag. Resolve it to an exact commit.
2. `updater-build.yml` builds Windows x64, macOS ARM64, macOS Intel and Linux x64.
   macOS architectures build natively; Linux AppImages use the Ubuntu 22.04 baseline.
3. Stage libmpv, build real playback, decode a test file, package with pinned **vpk 1.2.161**, and validate native artifacts.
   These jobs have no production signing keys. The release installer no longer uses the legacy NSIS flow.
4. Only after all four targets succeed: merge without overwrites, validate, then sign four manifests in an isolated step.
5. Verify the complete signed release, upload a new draft, download every draft asset, reverify signatures/package contents/all file hashes,
   recheck the tag commit, then publish. Any failure prevents publication.
6. Notify the documentation deployment after success.

Download tables and SHA-256 values are generated from actual artifacts. Each platform has its own stable channel; only full packages are produced, never deltas.

## Keys and OS signing

GitHub Secrets: `YOYOVIDEO_UPDATER_PRIVATE_KEY` and `YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD`.
Variable `YOYOVIDEO_UPDATER_PUBLIC_KEY` must match the client `assets/updater.pub`.
Only the signing step receives private keys; pull requests and regular builds do not.

Update authentication is not OS code signing. macOS uses ad-hoc signing without Apple notarization; Windows currently has no trusted Authenticode certificate. System warnings may remain.

## Retries and republishing

Manual dispatch accepts an existing stable tag, but **any existing release or draft for that tag is refused**.
There is no automatic clobber, deletion, or retagging. Investigate a failed draft instead of publishing it manually.
Reissuing the old 0.0.1 requires an explicit separate operator action.

## Verification status

`updater-smoke.yml` tests native packaging/playback, not actual old-to-new installation and restart.
Local Windows package/signature success does not prove native macOS/Linux success or end-to-end upgrades.
See the [updater notes](https://github.com/ijry/YoYoVideo/blob/main/docs/development/updater.md) for commands, backups and current verification status.
