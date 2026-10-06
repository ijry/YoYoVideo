---
layout: home
title: YoYoVideo
titleTemplate: Local video player

hero:
  name: YoYoVideo
  text: A local video player that takes playback seriously
  tagline: Built with Rust, Slint and libmpv. Multi-tile batch playback, subtitle and track switching, picture filters and A-B loop — all of it local.
  image:
    src: /player-default.png
    alt: The YoYoVideo window on startup
  actions:
    - theme: brand
      text: Download v0.0.1
      link: https://github.com/ijry/YoYoVideo/releases/latest
    - theme: alt
      text: Installation
      link: /en/guide/installation
    - theme: alt
      text: GitHub
      link: https://github.com/ijry/YoYoVideo

features:
  - title: Batch playback
    details: Drop in several files and each tile gets its own transport, volume and sizing.
  - title: Subtitles and tracks
    details: External subtitles, subtitle delay and scale, audio track and channel mode switching.
  - title: Picture tools
    details: Zoom, rotation, panning, and a set of common video filter presets.
  - title: A-B loop and markers
    details: Mark two points to loop between them, and drop jumpable markers on the timeline.
  - title: Frameless and quiet
    details: A self-drawn frameless shell that fades its chrome away when the pointer rests.
  - title: Bundled playback core
    details: libmpv ships inside the package. No separate mpv install, no data leaves your machine.
---

<div class="yv-shot-wrap">
  <img class="yv-shot" src="/player-default.png" alt="The YoYoVideo window on startup" />
</div>

<section class="yv-section">
  <div class="yv-facts">
    <div class="yv-fact"><b>GPL-3.0</b><span>open source license</span></div>
    <div class="yv-fact"><b>libmpv</b><span>playback core</span></div>
    <div class="yv-fact"><b>Rust</b><span>core language</span></div>
    <div class="yv-fact"><b>Zero</b><span>extra decoders to install</span></div>
  </div>
</section>

<section class="yv-section yv-section-alt">
  <div class="yv-section-inner">
    <h2>Why YoYoVideo</h2>
    <p class="yv-lede">
      Most players are either bloated, or they trade real playback capability for the word
      "simple". YoYoVideo takes a different route: keep the complexity in the engine, and reduce
      the interface to the picture plus the controls that matter.
    </p>

    <div class="yv-grid">
      <div class="yv-card">
        <span class="yv-icon">▶</span>
        <h3>Batch playback is a first-class feature</h3>
        <p>Throw a folder at the window and you get a wall of simultaneous playback. Every tile pauses, seeks and changes volume on its own.</p>
      </div>
      <div class="yv-card">
        <span class="yv-icon">⌨</span>
        <h3>Designed around the keyboard</h3>
        <p>Remappable shortcuts cover transport, seeking, tracks, subtitles and picture controls. Wheel for volume, hover for menus.</p>
      </div>
      <div class="yv-card">
        <span class="yv-icon">◧</span>
        <h3>Frameless, not bare</h3>
        <p>Title bar, control bar and side panel are all drawn by the app. The chrome fades out after a few idle seconds, leaving only the picture.</p>
      </div>
      <div class="yv-card">
        <span class="yv-icon">⚙</span>
        <h3>Reproducible releases</h3>
        <p>The version lives in <code>Cargo.toml</code> and nothing else; pushing a tag publishes. The runtime comes from a public upstream build pinned by checksum.</p>
      </div>
    </div>
  </div>
</section>

<section class="yv-section">
  <div class="yv-section-inner">
    <h2>Where it stands</h2>
    <p class="yv-lede">v0.0.1 is the first public release. The states below reflect the repository as it is, not a promise.</p>

    <div class="yv-roadmap">
      <div class="yv-step">
        <h4>Playback core and batch mode <span class="yv-tag">available</span></h4>
        <p>libmpv ships with the package; per-tile control works.</p>
      </div>
      <div class="yv-step">
        <h4>Subtitles, tracks, filters <span class="yv-tag">available</span></h4>
        <p>Track switching, subtitle delay and scale, and the common filter presets are implemented.</p>
      </div>
      <div class="yv-step">
        <h4>Windows x64 packages <span class="yv-tag">available</span></h4>
        <p>A portable zip and an NSIS installer are built and published by GitHub Actions.</p>
      </div>
      <div class="yv-step" data-state="next">
        <h4>macOS packages <span class="yv-tag" data-state="next">pending</span></h4>
        <p>libmpv comes from Homebrew, built per architecture. Video goes through mpv's render API — implemented and compile-verified, but not yet run on real hardware.</p>
      </div>
      <div class="yv-step" data-state="next">
        <h4>Linux packages <span class="yv-tag" data-state="next">pending</span></h4>
        <p>Declares libmpv as a .deb dependency rather than bundling it; video works under X11. Wayland embedding is not implemented.</p>
      </div>
    </div>
  </div>
</section>

<section class="yv-section yv-cta">
  <div class="yv-section-inner">
    <h2>Download v0.0.1</h2>
    <p class="yv-lede">
      A Windows x64 portable package and installer. Unpack and run — no decoders to install first.
    </p>
    <div class="yv-actions">
      <a class="yv-btn yv-btn-primary" href="https://github.com/ijry/YoYoVideo/releases/latest">Go to downloads</a>
      <a class="yv-btn yv-btn-ghost" href="https://github.com/ijry/YoYoVideo">Browse the source</a>
      <a class="yv-btn yv-btn-ghost" href="/en/guide/installation">Installation</a>
    </div>
  </div>
</section>
