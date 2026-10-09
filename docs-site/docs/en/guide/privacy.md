---
title: One-click Privacy Mode
description: PIN-gated media protection, scheduled restrictions, and explicit manual overrides.
---

# One-click Privacy Mode

::: info Version requirement
One-click Privacy Mode is available starting with **v0.0.2**. It is not included in v0.0.1. Download the matching platform package from [Releases](https://github.com/ijry/YoYoVideo/releases/latest).
:::

Privacy mode controls media **explicitly marked as protected**. It is not incognito playback or file encryption.

## Getting started

1. Open **View / Settings → Privacy Settings…** and enter and confirm a **4-digit PIN**. Leading zeros are supported; there is no default PIN.
2. Choose **Protect Current Media**, or use the lock icon beside a playlist/history item. The unlock icon removes that item's protection. Changes require PIN verification.
3. Use the title-bar lock button or **One-click Privacy Mode** in the menu. After setup, turning protection **on needs no PIN**; manually turning it **off requires the PIN**.
4. You may bind **One-click Privacy Mode** in the normal shortcut settings. It has no default binding and takes over no existing key.

PIN prompts do not show protected media names. An authorized settings window can show its protection list, but closing it, losing focus, re-enabling protection or reaching a new period revokes that authorization and clears the displayed list.

## What happens to playback

Protected media is concealed and paused even if it was already loading, playing or paused. A separate privacy mute is applied. In grid mode only protected tiles are affected; ordinary tiles keep playing.

- Unlocking **never resumes playback automatically**. A new play action is required.
- Privacy mute does not overwrite the user's volume/mute preference.
- Resume, frame stepping, screenshots, history/recent opens, playlists, drops, command-line media and automatic next-item playback use the same access checks.
- Automatic playback stops at a protected item; it does not skip or unlock it silently.
- Displayed names, tracks, subtitle details and time/chapter information are masked. Existing history, progress, subtitle preferences and markers are retained.

## Schedules and manual overrides

Choose weekdays and `HH:mm` start/end times in authorized privacy settings. Add each period, then save the schedule. Rules use the system's local timezone, support overnight periods, reject equal start/end times, and merge overlapping or touching intervals.

**A manual override expires at the start of the next merged restricted period, not at the end of the current period.**

For a daily 09:00–18:00 schedule:

| Action | Result |
| --- | --- |
| Verify the PIN and turn privacy off at 10:00 | It stays off until 09:00 the next day, including after today's 18:00 end. |
| Turn privacy on at 20:00 | The schedule takes over at 09:00 the next day; protection stays on until that period ends at 18:00. |
| No enabled schedule | The manual state lasts until another manual action. |

Overrides and their expiry persist across restarts. Scheduled ends may release protection without prompting, but do not resume playback. Editing a schedule recomputes the next start for an existing manual override.

## PIN and failure handling

- Only an independently salted **Argon2id verifier** is saved. The PIN is not stored in plaintext and updater signing keys are not reused.
- Five failed attempts impose a persisted 30-second cooldown; restarting does not reset it.
- Cancelled, unfocused or expired verification requests cannot apply a late unlock reply.
- Privacy settings are stored separately in the app configuration directory's `privacy.toml`, using atomic replacement.
- Enabling still protects the current process if saving fails. Changes that release protection require a successful save first. Normal exit/updater restart is cancelled if a pending safety state cannot be saved.
- An existing corrupt or unsupported configuration fails closed rather than becoming an unprotected default. Back up and repair the configuration; there is no PIN-free in-app reset.

## Limits

Protection covers sessions and related windows **in the current process**. Separate processes load the configuration at startup; there is no instant cross-process broadcast.

Files are matched by normalized paths. Moving/renaming a file or changing its URL may require marking it again; URL query parameters are part of the identity. There is no folder inheritance or automatic content classification.

This does not encrypt media, history or configuration, prevent another player/file manager from reading files, resist someone who can modify local files, or provide OS-wide screenshot prevention. Native rendering must be validated separately on Windows, macOS and Linux; pure unit tests are not proof for every platform.
