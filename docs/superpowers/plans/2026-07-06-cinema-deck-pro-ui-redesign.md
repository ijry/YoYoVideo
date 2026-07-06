# Cinema Deck Pro UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework YoYoVideo's Slint desktop UI so the installed test build visually matches the confirmed `Cinema Deck Pro` direction.

**Architecture:** Keep the existing Rust + Slint + libmpv boundaries. This plan changes the main Slint surface and visual smoke documentation while preserving all existing callbacks, properties, and runtime wiring.

**Tech Stack:** Rust 2024, Slint 1.17.0, existing `std-widgets.slint` controls for text input/sliders, existing PowerShell packaging scripts.

## Global Constraints

- Do not change `yoyo-core` playback model.
- Do not change `yoyo-mpv` command/event translation.
- Do not change marker store format.
- Do not change shortcut defaults.
- Do not add a webview, web UI runtime, or new native UI toolkit.
- Keep existing playback, playlist/history, tracks, subtitles, screenshot, frame stepping, filters, markers, chapters, and packaging behavior intact.
- Keep the native video host rectangle properties stable: `video_area_x`, `video_area_y`, `video_area_width`, and `video_area_height`.
- Primary controls must remain one click away.
- Advanced controls must move into action panel or focused popups instead of cluttering the main deck.
- Use OLED black/near-black backgrounds, midnight blue surfaces, wine red primary playback emphasis, cyan status/progress feedback, amber marker ticks, thin luminous borders, and restrained glow.
- No emoji icons and no default-looking primary buttons.
- Avoid expensive animation loops; use static or event-driven visual effects.

---

## File Structure

- Modify `apps/yoyovideo-desktop/ui/main-window.slint`: define Cinema Deck Pro local components, refactor main shell, video stage, control deck, popups, and sidebar.
- Modify `apps/yoyovideo-desktop/tests/context_menu_contract.rs`: keep compile coverage for all public UI properties/callbacks after the refactor and add explicit progress/navigation row model coverage.
- Modify `docs/testing/manual-smoke-checklist.md`: add precise visual acceptance checks for the installed build.
- Use existing scripts only: `scripts/package.ps1`, `scripts/test-package-smoke.ps1`, and `scripts/smoke-runtime.ps1`.

---

### Task 1: Add UI Contract Coverage And Visual Smoke Checklist

**Files:**
- Modify: `apps/yoyovideo-desktop/tests/context_menu_contract.rs`
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes: existing `MainWindow` Slint public properties and callbacks.
- Produces: compile-time guard that the UI refactor preserves progress ticks, navigation rows, and Cinema Deck callbacks.

- [ ] **Step 1: Extend the Slint contract test with row model coverage**

Replace `exercise_cinema_deck_surface` in `apps/yoyovideo-desktop/tests/context_menu_contract.rs` with this version:

```rust
fn exercise_cinema_deck_surface(window: &MainWindow) {
    window.set_muted(true);
    assert!(window.get_muted());
    window.set_mute_label("Muted".into());
    window.set_osd_visible(true);
    window.set_osd_message("Muted".into());
    window.set_progress_preview_visible(true);
    window.set_progress_preview_label("01:15".into());
    window.set_progress_preview_value(0.5);
    window.set_action_panel_visible(true);
    window.set_jump_panel_visible(true);
    window.set_jump_input_text("01:15".into());
    window.set_progress_tick_rows(vec![
        yoyovideo_desktop::ProgressTickRowData {
            percent: 0.25,
            label: "Chapter 1".into(),
            is_marker: false,
        },
        yoyovideo_desktop::ProgressTickRowData {
            percent: 0.5,
            label: "Marker 00:30".into(),
            is_marker: true,
        },
    ]
    .into());
    window.set_navigation_rows(vec![yoyovideo_desktop::NavigationRowData {
        title: "Marker 00:30".into(),
        subtitle: "00:30".into(),
        id: "marker-30000".into(),
        is_marker: true,
    }]
    .into());
    assert_eq!(window.get_progress_tick_rows().row_count(), 2);
    assert_eq!(window.get_navigation_rows().row_count(), 1);

    window.on_toggle_mute_requested(|| {});
    window.on_progress_preview_requested(|_| {});
    window.on_progress_commit_requested(|_| {});
    window.on_progress_preview_cleared(|| {});
    window.on_jump_panel_requested(|| {});
    window.on_jump_input_changed(|_| {});
    window.on_jump_commit_requested(|_| {});
    window.on_action_panel_requested(|| {});
    window.on_action_panel_close_requested(|| {});
    window.on_add_marker_requested(|| {});
    window.on_remove_marker_requested(|_| {});
    window.on_navigation_row_requested(|_| {});
    window.on_previous_chapter_marker_requested(|| {});
    window.on_next_chapter_marker_requested(|| {});
}
```

- [ ] **Step 2: Run the contract test before UI changes**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
```

Expected: PASS. This verifies the test extension is only guarding public Slint API stability.

- [ ] **Step 3: Add manual visual checks**

Append these lines under the `## UX` section in `docs/testing/manual-smoke-checklist.md`:

```markdown
- Launch the installed test build and confirm the main player reads as `Cinema Deck Pro`: OLED black shell, dominant video stage, red primary play control, cyan progress/status accents, and amber marker ticks.
- Confirm the main deck has two clear priorities: row 1 for play/progress/time/volume/fullscreen, row 2 for secondary actions such as open, speed, jump, tracks, picture, markers, playlist, and actions.
- Confirm the main control strip no longer looks like default Slint buttons arranged in a long toolbar.
- Open the action panel, tracks popup, video tools popup, menu popup, and sidebar; confirm all use the same dark glass visual language.
- Collapse and reopen the sidebar; confirm the collapsed state is a slim rail and the video stage remains visually dominant.
```

- [ ] **Step 4: Run docs/test verification**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
git diff --check
```

Expected: PASS and no whitespace errors.

- [ ] **Step 5: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/tests/context_menu_contract.rs docs/testing/manual-smoke-checklist.md
git commit -m "test: guard cinema deck pro ui surface"
```

Expected: Commit succeeds.

---

### Task 2: Build Cinema Deck Pro Local Component System

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`

**Interfaces:**
- Consumes: existing Slint imports and public `MainWindow` properties/callbacks.
- Produces: reusable local components `DeckButton`, `DeckPrimaryButton`, `DeckPill`, `DeckSectionTitle`, `DeckPanelHeader`, and `DeckDivider`.

- [ ] **Step 1: Replace existing deck helper components**

In `apps/yoyovideo-desktop/ui/main-window.slint`, replace the current `DeckButton` and `DeckLabel` definitions with this block:

```slint
component DeckButton inherits Rectangle {
    in property <string> text;
    in property <bool> selected: false;
    callback clicked();
    min-width: 48px;
    height: 34px;
    border-radius: 17px;
    background: selected ? #132638 : (touch.has-hover ? #151f2d : #0b111a);
    border-width: 1px;
    border-color: selected ? #38bdf8 : (touch.has-hover ? #34546a : #233040);

    Text {
        text: root.text;
        color: selected ? #f8fafc : #cbd5e1;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 12px;
        font-weight: selected ? 700 : 600;
    }

    touch := TouchArea {
        clicked => { root.clicked(); }
    }
}

component DeckPrimaryButton inherits Rectangle {
    in property <string> text;
    callback clicked();
    width: 72px;
    height: 44px;
    border-radius: 22px;
    background: touch.has-hover ? #fb315f : #e11d48;
    border-width: 1px;
    border-color: #ff6b8d;

    Rectangle {
        width: parent.width - 8px;
        height: parent.height - 8px;
        x: 4px;
        y: 4px;
        border-radius: 18px;
        background: #ffffff12;
    }

    Text {
        text: root.text;
        color: #ffffff;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 13px;
        font-weight: 800;
    }

    touch := TouchArea {
        clicked => { root.clicked(); }
    }
}

component DeckPill inherits Rectangle {
    in property <string> text;
    min-width: 62px;
    height: 30px;
    border-radius: 15px;
    background: #050a12;
    border-width: 1px;
    border-color: #1f2c3a;

    Text {
        text: root.text;
        color: #94a3b8;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: 12px;
        font-weight: 600;
    }
}

component DeckSectionTitle inherits Text {
    color: #f8fafc;
    font-size: 13px;
    font-weight: 800;
}

component DeckPanelHeader inherits Rectangle {
    in property <string> title;
    in property <string> subtitle: "";
    height: 56px;
    background: transparent;

    VerticalBox {
        padding-left: 12px;
        padding-right: 12px;
        padding-top: 8px;
        spacing: 2px;

        Text {
            text: root.title;
            color: #f8fafc;
            font-size: 16px;
            font-weight: 800;
        }

        Text {
            text: root.subtitle;
            color: #64748b;
            font-size: 11px;
        }
    }
}

component DeckDivider inherits Rectangle {
    height: 1px;
    background: #1f2c3a;
}
```

- [ ] **Step 2: Run Slint compile check**

Run:

```powershell
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 3: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint
git commit -m "feat: add cinema deck pro components"
```

Expected: Commit succeeds.

---

### Task 3: Refactor Main Shell And Video Stage

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`

**Interfaces:**
- Consumes: `DeckButton`, `DeckPrimaryButton`, `DeckPill`, `DeckSectionTitle`, `DeckDivider`.
- Produces: video-first app shell, top chrome, empty-state stage, stable `video_area` geometry, and centered OSD.

- [ ] **Step 1: Update root window styling**

In `MainWindow`, keep all public properties and callbacks unchanged. Replace only the root window styling lines with:

```slint
title: "YoYoVideo";
width: 1200px;
height: 760px;
background: #000000;
```

- [ ] **Step 2: Replace the top-level `HorizontalBox` layout**

Replace the top-level `HorizontalBox { ... }` inside `MainWindow` with a top-level `Rectangle` shell:

```slint
Rectangle {
    background: #000000;

    Rectangle {
        x: -160px;
        y: -180px;
        width: 520px;
        height: 520px;
        border-radius: 260px;
        background: #0f172a66;
    }

    Rectangle {
        x: parent.width - 420px;
        y: parent.height - 360px;
        width: 520px;
        height: 420px;
        border-radius: 210px;
        background: #164e6333;
    }

    VerticalBox {
        padding: 14px;
        spacing: 12px;

        Rectangle {
            height: 44px;
            border-radius: 22px;
            background: #050a12cc;
            border-width: 1px;
            border-color: #1f2c3a;

            HorizontalBox {
                padding-left: 16px;
                padding-right: 12px;
                spacing: 10px;

                Text {
                    text: "YoYoVideo";
                    color: #f8fafc;
                    font-size: 18px;
                    font-weight: 900;
                    vertical-alignment: center;
                }

                Rectangle {
                    width: 1px;
                    height: 18px;
                    background: #263241;
                }

                Text {
                    text: status_label == "" ? "Cinema Deck Pro" : status_label;
                    color: #94a3b8;
                    font-size: 12px;
                    vertical-alignment: center;
                }

                Rectangle { }

                DeckButton { text: root.sidebar_visible ? "Hide List" : "Show List"; clicked => { root.toggle_sidebar_requested(); } }
                DeckButton { text: "Menu"; clicked => { menu_popup.show(); } }
                DeckButton { text: "Settings"; clicked => { root.settings_requested(); } }
            }
        }

        HorizontalBox {
            spacing: 12px;

            VerticalBox {
                spacing: 10px;

                video_area := Rectangle {
                    background: #05070b;
                    border-color: #263241;
                    border-width: 1px;
                    border-radius: 22px;
                    min-height: 520px;

                    Rectangle {
                        x: 12px;
                        y: 12px;
                        width: parent.width - 24px;
                        height: parent.height - 24px;
                        border-radius: 18px;
                        background: #00000033;
                        border-width: 1px;
                        border-color: #0f172a;
                    }

                    VerticalBox {
                        x: (parent.width - 320px) / 2;
                        y: (parent.height - 118px) / 2;
                        width: 320px;
                        height: 118px;
                        spacing: 8px;

                        Text {
                            text: "Drop video here";
                            color: #f8fafc;
                            horizontal-alignment: center;
                            font-size: 24px;
                            font-weight: 900;
                        }

                        Text {
                            text: "Open file, folder, URL, or use keyboard shortcuts";
                            color: #64748b;
                            horizontal-alignment: center;
                            font-size: 12px;
                        }

                        HorizontalBox {
                            spacing: 8px;
                            DeckButton { text: "Open"; clicked => { root.open_file_requested(); } }
                            DeckButton { text: "Folder"; clicked => { root.open_folder_requested(); } }
                            DeckPill { text: "URL opens from the deck" }
                        }
                    }

                    if root.osd_visible: Rectangle {
                        width: 260px;
                        height: 64px;
                        border-radius: 22px;
                        background: #020817e6;
                        border-width: 1px;
                        border-color: #38bdf866;
                        x: (parent.width - self.width) / 2;
                        y: (parent.height - self.height) / 2;

                        Text {
                            text: root.osd_message;
                            color: #f8fafc;
                            horizontal-alignment: center;
                            vertical-alignment: center;
                            font-size: 17px;
                            font-weight: 800;
                        }
                    }
                }

                // Task 4 expands this temporary deck shell into the full control deck.
                Rectangle {
                    height: 112px;
                    border-radius: 24px;
                    background: #080d14e6;
                    border-width: 1px;
                    border-color: #263241;
                }
            }

            // Task 5 expands this temporary sidebar shell into the full sidebar.
            Rectangle {
                width: root.sidebar_visible ? root.sidebar_expanded_width_px * 1px : 40px;
                border-radius: 22px;
                background: #080d14cc;
                border-width: 1px;
                border-color: #1f2c3a;
            }
        }
    }
}
```

- [ ] **Step 3: Run focused compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 4: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint
git commit -m "feat: add cinema deck pro stage"
```

Expected: Commit succeeds.

---

### Task 4: Build The Bottom Control Deck

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`

**Interfaces:**
- Consumes: existing callbacks `toggle_pause_requested`, `progress_preview_requested`, `progress_commit_requested`, `progress_preview_cleared`, `volume_changed`, `toggle_mute_requested`, `toggle_fullscreen_requested`, `open_file_requested`, `open_folder_requested`, `jump_panel_requested`, `action_panel_requested`, track/video tools popup show calls, marker/chapter navigation callbacks.
- Produces: two-row Cinema Deck Pro control surface.

- [ ] **Step 1: Replace the Task 3 temporary deck shell**

Replace the `Rectangle` comment `Task 4 expands this temporary deck shell into the full control deck.` and its temporary deck rectangle with:

```slint
Rectangle {
    height: 126px;
    border-radius: 26px;
    background: #080d14e6;
    border-width: 1px;
    border-color: #263241;

    VerticalBox {
        padding: 12px;
        spacing: 10px;

        HorizontalBox {
            spacing: 10px;

            DeckPrimaryButton { text: transport_label; clicked => { root.toggle_pause_requested(); } }
            DeckPill { text: time_label; }

            progress_rail := Rectangle {
                height: 34px;
                background: transparent;

                Rectangle {
                    x: 0;
                    y: 14px;
                    width: parent.width;
                    height: 7px;
                    border-radius: 4px;
                    background: #111827;
                    border-width: 1px;
                    border-color: #1f2c3a;
                }

                Rectangle {
                    x: 0;
                    y: 14px;
                    width: root.progress_value * parent.width;
                    height: 7px;
                    border-radius: 4px;
                    background: #38bdf8;
                }

                for tick in root.progress_tick_rows: Rectangle {
                    x: tick.percent * parent.width - 1px;
                    y: tick.is_marker ? 8px : 10px;
                    width: tick.is_marker ? 4px : 2px;
                    height: tick.is_marker ? 20px : 16px;
                    border-radius: 2px;
                    background: tick.is_marker ? #f59e0b : #7dd3fc;
                }

                if root.progress_preview_visible: Rectangle {
                    x: root.progress_preview_value * parent.width - 44px;
                    y: -20px;
                    width: 88px;
                    height: 24px;
                    border-radius: 12px;
                    background: #020817ee;
                    border-width: 1px;
                    border-color: #38bdf866;

                    Text {
                        text: root.progress_preview_label;
                        color: #f8fafc;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                        font-size: 11px;
                        font-weight: 700;
                    }
                }

                touch := TouchArea {
                    moved => {
                        root.progress_preview_requested(self.mouse-x / progress_rail.width);
                    }
                    clicked => {
                        root.progress_commit_requested(self.mouse-x / progress_rail.width);
                    }
                    pointer-event(event) => {
                        if event.kind == PointerEventKind.cancel {
                            root.progress_preview_cleared();
                        }
                    }
                }
            }

            DeckButton { text: root.mute_label; selected: root.muted; clicked => { root.toggle_mute_requested(); } }
            DeckPill { text: volume_label; }
            Slider {
                minimum: 0;
                maximum: 100;
                value: volume_value;
                changed(value) => { root.volume_changed(value); }
            }
            DeckButton { text: "Full"; clicked => { root.toggle_fullscreen_requested(); } }
        }

        HorizontalBox {
            spacing: 8px;

            DeckButton { text: "Open"; clicked => { root.open_file_requested(); } }
            DeckButton { text: "Folder"; clicked => { root.open_folder_requested(); } }
            url_input := LineEdit {
                placeholder-text: "Open URL";
                accepted => { root.open_url_requested(self.text); }
            }
            DeckButton { text: "Jump"; clicked => { jump_panel.show(); root.jump_panel_requested(); } }
            DeckButton { text: "Actions"; clicked => { action_panel.show(); root.action_panel_requested(); } }
            DeckButton { text: "Tracks"; clicked => { tracks_popup.show(); } }
            DeckButton { text: "Picture"; clicked => { video_tools_popup.show(); } }
            DeckButton { text: "Mark"; clicked => { root.add_marker_requested(); } }
            DeckButton { text: "Prev"; clicked => { root.previous_chapter_marker_requested(); } }
            DeckButton { text: "Next"; clicked => { root.next_chapter_marker_requested(); } }
            DeckPill { text: speed_label; }
            DeckButton { text: "-"; clicked => { root.speed_down_requested(); } }
            DeckButton { text: "+"; clicked => { root.speed_up_requested(); } }
            DeckButton { text: "1x"; clicked => { root.reset_speed_requested(); } }
            DeckPill { text: zoom_label; }
            DeckButton { text: "Zoom -"; clicked => { root.zoom_out_requested(); } }
            DeckButton { text: "Zoom +"; clicked => { root.zoom_in_requested(); } }
            DeckButton { text: "Rot"; clicked => { root.rotate_requested(); } }
            DeckPill { text: rotation_label; }
            DeckButton { text: "Audio"; clicked => { root.cycle_audio_requested(); } }
            DeckPill { text: audio_channel_label; }
        }
    }
}
```

- [ ] **Step 2: Add loop controls to the action panel path**

Do not add A/B loop controls back into the main deck. Task 5 adds them to the action panel so the primary strip stays clean.

- [ ] **Step 3: Run compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS. If Slint rejects `PointerEventKind.cancel`, remove the `pointer-event` block and keep `moved` plus `clicked`; progress preview clearing remains available through the Rust callback API and can be improved later.

- [ ] **Step 4: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint
git commit -m "feat: add cinema deck pro controls"
```

Expected: Commit succeeds.

---

### Task 5: Restyle Popups, Action Panel, And Sidebar

**Files:**
- Modify: `apps/yoyovideo-desktop/ui/main-window.slint`

**Interfaces:**
- Consumes: existing popup callbacks and row models.
- Produces: unified dark glass panels for menu, tracks, video tools, action panel, jump panel, playlist sidebar, history sidebar, and collapsed sidebar rail.

- [ ] **Step 1: Restyle `menu_popup`**

Inside `menu_popup`, keep the same callbacks but change the panel body to this structure:

```slint
Rectangle { width: parent.width; height: parent.height; background: #080d14; border-width: 1px; border-color: #263241; }

ScrollView {
    VerticalBox {
        padding: 12px;
        spacing: 8px;
        DeckPanelHeader { title: "Menu"; subtitle: "Open, recent, playback, settings"; }
        DeckButton { text: "Open File"; clicked => { root.open_file_requested(); menu_popup.close(); } }
        DeckButton { text: "Open Folder"; clicked => { root.open_folder_requested(); menu_popup.close(); } }
        DeckDivider { }
        DeckSectionTitle { text: "Recent"; }
        if root.recent_open_rows.length == 0: Text { text: "No Recent Items"; color: #64748b; }
        for row[index] in root.recent_open_rows: DeckButton {
            text: row.title;
            clicked => { root.recent_open_item_requested(index); menu_popup.close(); }
        }
        DeckDivider { }
        DeckButton { text: "Playlist"; clicked => { root.show_playlist_tab_requested(); menu_popup.close(); } }
        DeckButton { text: "History"; clicked => { root.show_history_tab_requested(); menu_popup.close(); } }
        DeckButton { text: "Settings"; clicked => { root.settings_requested(); menu_popup.close(); } }
    }
}
```

- [ ] **Step 2: Restyle `tracks_popup`**

Keep all existing sliders, checkbox, and callbacks. Wrap sections with `DeckPanelHeader`, `DeckSectionTitle`, and `DeckDivider`; replace track row `Button` instances with `DeckButton { selected: row.selected; ... }`.

Use this exact pattern for each row loop:

```slint
for row[idx] in root.audio_track_rows: DeckButton {
    text: row.label;
    selected: row.selected;
    clicked => { root.audio_track_requested(idx); }
}
```

Repeat with `subtitle_track_rows` and `video_track_rows`.

- [ ] **Step 3: Restyle `video_tools_popup`**

Replace default action `Button` widgets in `video_tools_popup` with `DeckButton` and add section titles:

```slint
DeckPanelHeader { title: "Picture"; subtitle: "Screenshot, frame step, filters"; }
DeckSectionTitle { text: "Capture"; }
DeckButton { text: "Screenshot"; clicked => { root.screenshot_requested(); } }
DeckSectionTitle { text: "Frame Step"; }
HorizontalBox {
    spacing: 8px;
    DeckButton { text: "Prev Frame"; clicked => { root.frame_step_previous_requested(); } }
    DeckButton { text: "Next Frame"; clicked => { root.frame_step_next_requested(); } }
}
```

Keep existing sliders for brightness, contrast, saturation, gamma, and hue. Replace filter buttons with `DeckButton` instances.

- [ ] **Step 4: Restyle `action_panel` and add secondary playback actions**

In `action_panel`, use the same glass background and add A/B loop controls that were removed from the primary deck:

```slint
DeckPanelHeader { title: "Actions"; subtitle: "Quick tools, navigation, and loop controls"; }
DeckSectionTitle { text: "Quick"; }
DeckButton { text: "Screenshot"; clicked => { root.screenshot_requested(); } }
DeckButton { text: "Previous Frame"; clicked => { root.frame_step_previous_requested(); } }
DeckButton { text: "Next Frame"; clicked => { root.frame_step_next_requested(); } }
DeckButton { text: "Jump To Time"; clicked => { jump_panel.show(); root.jump_panel_requested(); } }
DeckButton { text: "Add Marker"; clicked => { root.add_marker_requested(); } }
DeckDivider { }
DeckSectionTitle { text: "Loop"; }
HorizontalBox {
    spacing: 8px;
    DeckButton { text: "Set A"; clicked => { root.set_ab_point_a_requested(); } }
    DeckButton { text: "Set B"; clicked => { root.set_ab_point_b_requested(); } }
    DeckButton { text: "Clear"; clicked => { root.clear_ab_loop_requested(); } }
}
DeckPill { text: loop_label; }
DeckDivider { }
DeckSectionTitle { text: "Chapters & Markers"; }
if root.navigation_rows.length == 0: Text { text: "No chapters or markers"; color: #64748b; }
for row[idx] in root.navigation_rows: DeckButton {
    text: row.title + "  " + row.subtitle;
    selected: row.is_marker;
    clicked => { root.navigation_row_requested(idx); }
}
```

- [ ] **Step 5: Restyle `jump_panel`**

Add a dark glass background behind the existing `LineEdit`:

```slint
Rectangle { width: parent.width; height: parent.height; background: #080d14; border-width: 1px; border-color: #263241; }
VerticalBox {
    padding: 14px;
    spacing: 10px;
    DeckSectionTitle { text: "Jump To Time"; }
    jump_input := LineEdit {
        text: root.jump_input_text;
        placeholder-text: "ss, mm:ss, hh:mm:ss";
        edited => { root.jump_input_changed(self.text); }
        accepted => { root.jump_commit_requested(self.text); jump_panel.close(); }
    }
    DeckPrimaryButton { text: "Go"; clicked => { root.jump_commit_requested(jump_input.text); jump_panel.close(); } }
}
```

- [ ] **Step 6: Replace the Task 3 temporary sidebar shell**

Replace the `Rectangle` comment `Task 5 expands this temporary sidebar shell into the full sidebar.` and its temporary sidebar rectangle with:

```slint
Rectangle {
    width: root.sidebar_visible ? root.sidebar_expanded_width_px * 1px : 42px;
    border-radius: 22px;
    background: #080d14cc;
    border-width: 1px;
    border-color: #1f2c3a;

    if root.sidebar_visible: VerticalBox {
        padding: 12px;
        spacing: 10px;

        HorizontalBox {
            spacing: 8px;
            DeckButton { text: "Playlist"; selected: root.sidebar_tab_index == 0; clicked => { root.show_playlist_tab_requested(); } }
            DeckButton { text: "History"; selected: root.sidebar_tab_index == 1; clicked => { root.show_history_tab_requested(); } }
            DeckButton { text: "Close"; clicked => { root.toggle_sidebar_requested(); } }
        }

        ScrollView {
            if root.sidebar_tab_index == 0: VerticalBox {
                spacing: 6px;
                for row[idx] in root.playlist_rows: Rectangle {
                    min-height: 48px;
                    border-radius: 14px;
                    background: row.is_current ? #132638 : #0b111a;
                    border-width: 1px;
                    border-color: row.is_current ? #38bdf8 : #1f2c3a;

                    TouchArea { clicked => { root.playlist_item_requested(idx); } }

                    Text {
                        text: row.title;
                        color: row.is_current ? #f8fafc : #cbd5e1;
                        vertical-alignment: center;
                        x: 12px;
                        y: 14px;
                        font-size: 12px;
                        font-weight: row.is_current ? 800 : 600;
                    }
                }
            }

            if root.sidebar_tab_index == 1: VerticalBox {
                spacing: 6px;
                for row[idx] in root.history_rows: Rectangle {
                    min-height: 58px;
                    border-radius: 14px;
                    background: #0b111a;
                    border-width: 1px;
                    border-color: #1f2c3a;

                    TouchArea { clicked => { root.history_item_requested(idx); } }

                    VerticalBox {
                        padding-left: 12px;
                        padding-top: 8px;
                        spacing: 3px;
                        Text { text: row.title; color: #f8fafc; font-size: 12px; font-weight: 700; }
                        Text { text: row.subtitle; color: #64748b; font-size: 11px; }
                    }
                }
            }
        }
    }

    if !root.sidebar_visible: Rectangle {
        background: transparent;
        TouchArea { clicked => { root.toggle_sidebar_requested(); } }
        Text {
            text: "List";
            color: #94a3b8;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-size: 12px;
            font-weight: 800;
        }
    }
}
```

- [ ] **Step 7: Run compile checks**

Run:

```powershell
cargo test -p yoyovideo-desktop --test context_menu_contract
cargo test -p yoyovideo-desktop --test video_tools_window_contract
cargo check -p yoyovideo-desktop
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```powershell
git add apps/yoyovideo-desktop/ui/main-window.slint
git commit -m "feat: unify cinema deck pro panels"
```

Expected: Commit succeeds.

---

### Task 6: Full Verification, Package, And Local Test Install

**Files:**
- Modify only if needed: `docs/testing/manual-smoke-checklist.md`
- Update generated artifacts only under `dist/` and local install path when packaging.

**Interfaces:**
- Consumes: completed UI refactor.
- Produces: rebuilt package and installed local test copy at `%LOCALAPPDATA%\YoYoVideo-Test`.

- [ ] **Step 1: Run formatting and tests**

Run:

```powershell
cargo fmt --check
cargo test
```

Expected: PASS.

- [ ] **Step 2: Run runtime feature checks**

Run:

```powershell
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS.

- [ ] **Step 3: Run package smoke**

Run:

```powershell
pwsh -NoProfile -File scripts/test-package-smoke.ps1
```

Expected: PASS.

- [ ] **Step 4: Rebuild Windows package**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -FeatureFlags mpv-runtime
```

Expected: `dist\YoYoVideo-windows-x64\bin\yoyovideo-desktop.exe` exists and `dist\YoYoVideo-windows-x64\bin\mpv-2.dll` exists.

- [ ] **Step 5: Run runtime smoke**

Run:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64 -TimeoutSeconds 8
```

Expected: PASS and output includes `runtime_smoke=ok`.

- [ ] **Step 6: Install rebuilt portable package for user testing**

Run:

```powershell
$ErrorActionPreference = 'Stop'
$source = Resolve-Path -LiteralPath '.\dist\YoYoVideo-windows-x64'
$installRoot = Join-Path $env:LOCALAPPDATA 'YoYoVideo-Test'
New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
Get-ChildItem -LiteralPath $source.Path -Force | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $installRoot -Recurse -Force
}
$exe = Join-Path $installRoot 'bin\yoyovideo-desktop.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw "Installed executable not found: $exe" }
$desktop = [Environment]::GetFolderPath('Desktop')
$shortcutPath = Join-Path $desktop 'YoYoVideo Test.lnk'
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exe
$shortcut.WorkingDirectory = Split-Path -Parent $exe
$shortcut.IconLocation = $exe
$shortcut.Description = 'YoYoVideo local test build'
$shortcut.Save()
```

Expected: `%LOCALAPPDATA%\YoYoVideo-Test\bin\yoyovideo-desktop.exe` exists and desktop shortcut `YoYoVideo Test.lnk` points to it.

- [ ] **Step 7: Launch installed test build**

Run:

```powershell
$exe = Join-Path $env:LOCALAPPDATA 'YoYoVideo-Test\bin\yoyovideo-desktop.exe'
Start-Process -FilePath $exe -WorkingDirectory (Split-Path -Parent $exe)
```

Expected: app launches so the user can inspect the new UI.

- [ ] **Step 8: Commit any remaining tracked source changes**

Run:

```powershell
git status --short
```

Expected: no uncommitted tracked source changes. If `docs/testing/manual-smoke-checklist.md` changed after Task 1, commit it with:

```powershell
git add docs/testing/manual-smoke-checklist.md
git commit -m "docs: update cinema deck pro smoke checks"
```

Expected: Commit succeeds or no commit is needed.

---

## Self-Review

**Spec coverage:** Task 2 covers the reusable Cinema Deck Pro visual components. Task 3 covers the OLED shell, top chrome, dominant video stage, empty state, and OSD. Task 4 covers the two-row bottom deck, primary red play control, custom progress rail, cyan ticks, amber marker ticks, and primary/secondary hierarchy. Task 5 covers popups, action panel, A/B loop relocation, and sidebar visual hierarchy. Task 6 covers full verification, packaging, reinstall, and user test launch.

**Placeholder scan:** The plan contains concrete files, code snippets, commands, and expected results. It avoids deferred implementation wording.

**Type consistency:** All public Slint property and callback names match the existing `MainWindow` surface. New helper components are local to `main-window.slint` and are introduced before later tasks consume them.
