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
4. Only after all four package targets, clean Windows Setup acceptance, and Windows/macOS/Linux native upgrades succeed:
   merge without overwrites, validate, then sign four manifests in an isolated step.
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

On 2026-10-08, commit `81d7f06` passed all **9/9 [native CI jobs](https://github.com/ijry/YoYoVideo/actions/runs/37736646064)**
and [regular CI](https://github.com/ijry/YoYoVideo/actions/runs/37736645755). Each macOS architecture passed three consecutive full upgrades,
and all four Linux cases passed. These tests did not use production private keys.

After separate maintainer authorization, [v0.0.1 was reissued](https://github.com/ijry/YoYoVideo/releases/tag/v0.0.1) from `4c04726`.
All **10/10 [release workflow jobs](https://github.com/ijry/YoYoVideo/actions/runs/37750925601)** passed.
All 29 public assets were downloaded and checksum-verified; the four signed update manifests and anonymous update endpoint were checked.
The previous release assets, metadata and annotated tag were backed up.

`updater-smoke.yml` covers native packaging/playback on all four targets, clean Windows Setup installation/shortcuts/uninstallation,
and real `0.0.1 → 0.0.2` upgrades: Windows portable layout, macOS ARM64/Intel,
and Linux 22.04/24.04 using both FUSE and extraction (containers have no system libmpv).
Every upgrade checks rejection of bad signatures/packages/cache tampering, postponement, instance protection,
the restarted process version, and playback-history restoration.
A separate job signs all four actual artifacts with disposable keys and verifies the complete release set;
test private keys and QA packages are never uploaded to a Release.
Publishing depends on the same package checks and both `updater-upgrade-windows.yml` and `updater-upgrade-unix.yml` gates.
See the [updater notes](https://github.com/ijry/YoYoVideo/blob/main/docs/development/updater.md) for commands, backups and current verification status.
