<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Record the second batch of 2026-11-21 Windows real-device checks

## Purpose

A second real-device run on Windows 11 on 2026-11-21 (a debug build with the
`screenshot` feature, driven by synthetic keyboard and mouse input
(`SendInput`), window captures and the MCP companion on a test port, on test
copies under `D:\Photos\tests\20261121-qa\`) exercised the pending manual
checks of the settings (store, layer, body, controls, Keyboard Shortcuts,
Cache), the restored window, the pane toggles and Open Folder, the pane and
strip resize, and the title bar. This plan removes the `todo.md` items the run
settled, rewords the ones it settled in part, adds the one question the run
raised as a new item, and records what was seen in `docs/agents/app.md` where
an item asks for it. Docs and todo only, no code change: one step, one PR.
It follows the conventions of
`docs/plans/_archived/20261121-windows-device-checks/plan.md`.

## Steps

- [x] Step 1: Apply the run's results to `todo.md` and `docs/agents/app.md`
  - Done when:
    - Every edit in the lists A, B, C and D below is applied.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files, and no `todo.md` item outside the lists changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where an item or a box went.
    - The PR description records what was seen for each removed item and each removed box (the "Seen" notes below), since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally (a `.md` change runs lychee) and the PR's GitHub checks pass.
  - Implementation approach:
    - Locate every item by its `###` heading, never by line number. An item runs from its `###` line to the line before the next `###`. "Box N" counts the top-level `- [ ]` lines of the item, from 1; "part N" is a numbered sub-line of a box.
    - Write everything in English, in the file's own style (`#### Background`, `Files:`, `#### TODO`, paths and identifiers in backticks). Keep the neighbouring items, and every sentence of an item this plan does not name, byte-for-byte.
    - The texts quoted below are the content to carry; their facts and quoted strings are exact, the surrounding wording may follow the file's style. Every "seen" sentence added to a Background says "on Windows 11 on 2026-11-21".
    - Three items (the settings store one, the pane resize one, the strip height one) have no `#### Background` heading: their intro paragraphs under the `###` line are the Background.
    - The previous plan's `learnings.md` notes that a script locating each item by its exact `###` line and asserting one match per anchor kept the neighbours byte-for-byte.
    - **A. Delete the whole item in `todo.md`:**
      1. `### App: real-device check of the Keyboard Shortcuts page's groups`. Seen: the five groups then `Reset all`; `+`, `×` and the row `Reset` in `Judgment` and `Panes`; `Reset all`; `ctrl+,` refused as `"ctrl+," is a menu accelerator`; a click on the sidebar's `Panes` scrolls to the group; the search works.
      2. `### App: real-device checks of the pane toggles and Open Folder (gpui-pane-toggles)`, all four boxes; the picker's result also goes to `docs/agents/app.md` (D2). Seen: `ctrl+o` opens the picker and a cancel changes nothing; a second `ctrl+o` while the picker is up opens no second dialog (one `#32770` window; the main window is disabled); picking a folder opens it and the keys work at once without a click; with the tree holding the keyboard, `tab` hides both sides and the arrows page; a seeded `{"panels":{"left":false}}` hides the tree and shows the other panes; `h` rebound to `toggleLeft` runs it.
    - **B. Reword in `todo.md`:**
      3. `### App: real-device checks of the settings store and the settings modal (sort / filter persistence, Culling, Sidecar, Keyboard Shortcuts, label names, Import)`: boxes 1, 2, 3, 4, 6 and 7 passed, and box 5 passed except its Japanese IME part.
         - Rename the heading to `### App: real-device check of the Japanese IME in the settings' label name fields`.
         - In the first intro paragraph, replace the sentences from "Only Step 1's store seed was hand-checked, headless." through "every step's checkbox was ticked on the automated criteria." with one saying: the store, the settings' shell, the Culling, Sidecar, Keyboard Shortcuts and Import pages, the text fields and the `Lightroom language` `Select` were exercised on Windows 11 on 2026-11-21; only the Japanese IME's composition in a text field was not.
         - In the second intro paragraph, delete its last sentence ("Record what was seen in `docs/agents/app.md`, above all the key names ... `ctrl+alt+1`)."), now done by D1.
         - Delete boxes 1, 2, 3, 4, 6 and 7.
         - Replace box 5 with: ``- [ ] On Windows, the Japanese IME in a label name field. Run on a folder of RAW copies with a fresh `--index`, `--settings` and `--log`, format XMP. Ctrl+, → Sidecar; with the Japanese IME on, type a name in the Yellow field: the composition is underlined, the candidate window sits at the field, Enter commits the composition (not the field), a second Enter commits the field.``
         - Seen, for the PR description:
           - Box 1: `Rating`, `Picked` and `Landscape` picked by mouse; the close button logged `quit: settings writes drained=true`. A restart with a fresh index logged a `settings:` line with `sortOrder=rating`, `flags: [Picked]` and `orientations: [Landscape]`, with the menu checks shown. After `Reset`, a quit and a restart, the filter was empty and the sort still `rating`.
           - Box 2: passes in today's UI. `ctrl+,` with `Filter ▾` open closes the menu and shows the settings, now a full-window layer with a sidebar, not a centered box over a dimmed window. The arrows, `q`, `1`, `x` and a strip click behind it changed nothing and the app did not quit. Escape and `×` close, and Right pages at once. The auto-advance switch toggles by click, Space and Enter; with it off, `1` rated and stayed; a restart logged `autoAdvance=false`. The box's tab-list, dimmed-area and focus-border wording is obsolete.
           - Box 3: with XMP 2 / `.dop` 4 files the XMP ratings showed first. `PhotoLab (.dop)` logged `sidecar format: xmp -> dop` and a `resync`, and the ratings showed 4; choosing it again logged nothing. `5` changed only the `.ARW.dop` (the `.xmp` mtime and bytes kept). `Both` then `3` left both holding 3. A restart without the seed logged `sidecarFormat=both` with `Both` selected. A switch in the JPEG-only folder gave no error and the view-only note. The optional MCP `set_judgment`-after-a-switch part was not done and is dropped.
           - Box 4: `+` then `k` adds `K` to Pick and `k` picks; a restart keeps `p`, `k`; `×` removes it. `x` gives `"x" is bound to reject`; `q` gives `"q" is taken by this app`. Escape in a capture ends it and the settings stay. Shift+1 arrives as `!` (added as a key); Ctrl+Alt+1 gives `"ctrl+alt+1" is bound to red`; Tab is captured (`"tab" is bound to toggleSides`). A press on the sidebar cancels a capture. `×` on the only key gives `"p" is the only key for pick; use Reset`. After `Reset all` and a restart the store holds no `shortcuts` key.
           - Box 5, less the IME: the file with `xmp:Label="Rot"` shows no red label until Red's name is set to `Rot` (`label names: ... -> ...` and a `resync`, then red). Ctrl+Alt+1 on another file writes `xmp:Label="Rot"`. `PhotoLab` hides the names block and `Lightroom` shows it. Typing, Backspace, Home / End, Shift+Home, Ctrl+C / V, Ctrl+X and Ctrl+A work. A typed `q` or `1` goes into the field and nothing happens behind. Tab commits and moves to the next field; Escape commits and closes. `日本語` gives the fields `レッド` ... `パープル` and the `Rot` file keeps no red label. A restart shows the last names.
           - Box 6: an empty root, the note, `%Y/%F`, the relative example `2026\2026-11-21`, four example buttons and the eleven-row table. `Choose…` opens the Windows folder dialog; picking sets the root and the example becomes absolute. Typing `%Y/%m` updates the example live. `%H` shows "`%Y/%m%H` uses a time, zone or timestamp specifier; only the date is known" in red; `%Y//%d` shows "the pattern makes an empty folder name". Escape keeps the last valid `%Y/%m`. The `%F` button's pattern is kept after a restart. Uncommitted typed text survived `Choose…` and the dialog. The window cannot be clicked while the folder dialog is up (it is modal), so the close-with-`×` part was skipped as the box allows.
           - Box 7: `日本語` fills the names at once and logs once; choosing it again logs nothing. Edited names make the `Select` show its placeholder ("Please select"). Typing the five English names back selects English. A reopen shows the language in force. No panic.
      4. `### App: real-device checks of the settings as a full-window layer (native-settings-component Step 1)`: parts 1, 2, 3, 4 and 7 passed; of part 5 only the keys and a strip click were seen blocked; part 6 was not done.
         - In the Background, replace "The view's keys, pointer, focus trap and the drop behavior were never exercised: the agent session could not drive the GUI, and the step's checkbox was ticked on the automated criteria only." with a sentence saying: the layer's opacity, typing, the Tab trap (30 Tabs and 2 Shift+Tabs stayed in the settings with the panes untouched), the `tab` capture, the blocked keys and strip click, and the close by Escape and `×` were seen on Windows 11 on 2026-11-21; hovers, wheel scrolls and the drop were not tried.
         - Replace the box's parts with two: (1) `Hovers (no strip tooltip) and wheel scrolls over the settings do not reach the panes underneath.` (2) the present part 6 (the drop), unchanged.
      5. `### App: real-device checks of the settings body drawn with gpui-component's `Settings` (native-settings-component Step 2)`: parts 1, 2, 3, 5 and 6 passed, and part 4 passed except that `home` / `end` were not tried.
         - In the Background, replace "Typing in the search field, Escape on a non-empty query, the capture cancel, the Tab trap, the search filtering and the look were never run on a real machine (the step's checkbox was ticked on the automated criteria only)." with a sentence saying: they were run on Windows 11 on 2026-11-21 (the sidebar now also lists Cache, MCP, Appearance and Updates); the refusal of `home` / `end` in a capture and the user's visual check of the look are left.
         - Replace the box's parts with two: (1) ``On Keyboard Shortcuts, `+` then `home` or `end` shows its refusal on the status line.`` (2) the present part 7 (the user's visual check), unchanged.
         - Seen, for the PR description: the placeholder reads "Search..."; Escape clears a non-empty search and a second Escape closes; Escape in a field commits and closes; "burst" lists its rows and "reset all" finds `Reset all`.
      6. `### App: real-device check of the settings controls drawn with gpui-component (native-component-controls)`: parts 1 (without the menu open), 2, 4, 5 and 6 passed; of part 3, the pointer open and Escape were seen; opening by keys was not tried; the `Reset` button no longer exists.
         - In the Background, replace the sentence beginning "The focus trap with the `Select` menu open, ..." through "... ticked on the automated criteria only." with one saying: on Windows 11 on 2026-11-21 the Tab trap with the menu closed, the switch's Space / Enter, the menu opened by pointer and Escape with it open (closing only the menu, then the settings), the blur commit and the read-only root field (selectable and copyable to the clipboard, typing doing nothing, Escape closing the settings) were seen; opening the language menu by keys and Tab with the menu open were not tried; the `Reset` button no longer exists (`native-label-language-apply`).
         - Replace the box's parts with two: (1) ``Sidecar (XMP on): open the language menu by keys (Space / Enter, Up / Down) and choose a language with them.`` (2) ``With the language menu open, Tab / Shift+Tab: the focus stays inside the settings.``
      7. `### App: real-device checks of the Cache page and Clear Cache (native-clear-cache)`: all four parts passed, except that part 3's tree-count clause was not looked at.
         - In the Background, replace "The dialog opened over the settings layer (Escape and focus routing), the held clear, the refresh and the size shrinking were never exercised: `--auto` cannot drive the GUI, and the step's checkbox was ticked on the automated criteria only." with a sentence saying: seen on Windows 11 on 2026-11-21; whether the tree's cached counts of unlisted folders disappear after a clear was not looked at.
         - Replace the box and its four parts with one box: ``- [ ] On Windows, run `mise run app:dev` with a test `--settings` / `--index`, expand the tree so that unlisted folders show cached counts, open the settings (`ctrl+,`), pick `Cache`, press `Clear Cache` and confirm. Expected: the tree's cached counts of unlisted folders disappear.``
         - Seen, for the PR description: `Index cache: 812.6 KB`; Escape closes only the dialog; Enter clears and logs `index cleared: EvictSummary { folders: 2, rows: 30, vacuumed: true }`, the size then 92.0 KB; during a scan the note shows and the dialog opens by itself at the scan's end; closing the settings while held gives `get_status` `failed` with "dropped: the settings closed" and no dialog.
      8. `### App: real-device checks of the restored window and last folder (gpui-session-restore)`: boxes 1, 2, 4 and 5 passed; box 3 (a second monitor) was not available; box 6 was measured and becomes C1.
         - Rename the heading to `### App: real-device check of the restored window on a second monitor (gpui-session-restore)`.
         - In the Background, replace the sentence beginning "Never exercised: quitting with `q`, ..." with one saying: a quit with `q`, a snapped window, a taskbar docked left, the paged file and `--auto` were seen on Windows 11 on 2026-11-21 and recorded in `docs/agents/app.md`; a second monitor, especially one with a different scale factor, was not available.
         - Delete boxes 1, 2, 4, 5 and 6; keep box 3 unchanged. Keep the "Record the results as `(Measured)` ..." sentence.
      9. `### App: real-device checks of the drag-resizable tree and meta panes`: boxes 2, 3, 5 and 8 passed, and a restart restored 160 / 480.
         - In the intro, replace the sentence beginning "Nobody has dragged a handle in the running window: ..." with one saying: on Windows 11 on 2026-11-21 a drag clamped at 480 and 160 px, a narrow window kept the preview at 240 px or more and widening restored the widths, a `Tab` hide and show came back at the width, a restart restored 160 / 480, and `--auto` started at 220 with a drag writing nothing; the handle's look on hover, a jump at the first move, the release positions, the 1:1 re-cut, the 5 px bands and the layout at 160 px are unchecked.
         - Keep "The limits (160 / 480 / 240) were not judged by eye."
         - Delete boxes 3, 5 and 8.
         - Replace box 2 with: ``- [ ] On Windows, drag the preview's left edge and the meta pane's edge. Note whether the boundary jumps a few px at the first move.``
         - Keep boxes 1, 4, 6, 7 and 9 unchanged.
      10. `### App: real-device checks of the drag-resizable strip height (native-strip-resize)`: box 2 and the 96 px half of box 3 passed.
          - In the intro, replace the sentence beginning "Nobody has dragged the handle in the running window: ..." with one saying: on Windows 11 on 2026-11-21 a drag moved the strip between 96 and 302 px with the current cell staying in view, and the marks were legible at 96 px; the cursor and the handle's line, the sharpness of the largest cells, the release positions, the 1:1 re-cut and `--auto` are unchecked.
          - Delete box 2.
          - Replace box 3 with: ``- [ ] On Windows, at 302 px, check the thumbnails' sharpness at 100 % and on a HiDPI display.``
          - Keep the other boxes unchanged.
      11. `### App: real-device checks of the title bar (native-title-bar)`: box 1 passed; of box 3, the double-click maximize / restore and the bottom-right corner resize were seen.
          - In the Background, replace the sentence beginning "Nobody has exercised the controls' clicks, drag, ..." with one saying: on Windows 11 on 2026-11-21 minimize, maximize, restore, close and the bar's drag worked, the main window title read "Handpic", a double-click on the bar maximized and restored, and the bottom-right corner resized; a folder's preview and overlays under the bar, the menus' height cap, the other resize borders, Snap Layouts, the bar under a dialog, saved window geometry, macOS and Linux are not exercised.
          - Delete box 1.
          - Replace box 3 with: ``- [ ] On Windows (gpui-pre 0.3.12), check the resize borders on all four edges and the other three corners (the top edge over the bar included), Snap Layouts on hover over the maximize button, and the bar under a maximized window.``
          - Keep the other boxes unchanged.
      12. `### App: real-device checks of the title bar's settings button (native-title-bar-settings)`: boxes 2 and 3 passed, and box 4's (a), (b) and (c) passed (the menu closes, the settings open).
          - In the Background, replace the sentence beginning "Never exercised: any shipped non-Neutral theme; ..." with one saying: seen on Windows 11 on 2026-11-21: the window controls and the bar's drag after the change, a judgment key and paging right after closing the settings the gear opened, and the gear pressed with the tree holding the keyboard, with `Sort ▾` open and with a right-click menu open; never exercised: a shipped non-Neutral theme, the gear during an inline rename, macOS, Linux.
          - Delete boxes 2 and 3.
          - Replace box 4 with: ``- [ ] On Windows, click the gear during an inline folder rename. Expect the settings to open and the rename to end as a press elsewhere ends it.``
          - Keep boxes 1, 5 and 6 unchanged.
      13. `### App: real-device checks of the title bar's smaller text and centered folder name (native-title-bar-folder-name)`: the centering, the drag on the name, the double-click on the name and the controls' and gear's clicks passed.
          - In the Background, replace from "Nobody has looked at the bar running. Never exercised: ..." through "... it was never seen running." with sentences saying: on Windows 11 on 2026-11-21 the name was centered, a press and drag starting on the name moved the window, a double-click on it maximized, and the window controls and the gear took their clicks; not checked: the text size by eye, the truncation in a narrow window and whether 160 px a side of `TITLE_NAME_MARGIN` is right (it was estimated from the control widths; on macOS the 80 px traffic-light padding leaves only about 4 px between the name's edge and "Handpic"), and the name following a rename.
          - Keep "The Step 1 checkbox ..." and the plan link.
          - Replace the box with: ``- [ ] On Windows 11 (`mise run app:dev` or a debug build of `crates/app`), open a folder. Expect "Handpic" next to the icon to read at 14 px with the icon still 16 px, and the folder's name to disappear with no folder open. Narrow the window: expect a long name to be truncated with an ellipsis before it reaches "Handpic" or the gear. If 160 px a side of `TITLE_NAME_MARGIN` looks wrong, tune it. Also open another folder, rename the open folder and run `Rename by Capture Date…` on it: expect the name to follow.``
    - **C. Add one item to `todo.md`**, a `###` section with `#### Background` (ending in a `Files:` sentence) and `#### TODO`, placed right after the reworded session-restore item (B8):
      - C1 `### App: the `--auto` default window is not centered when the taskbar is docked left`. Background: on Windows 11 on 2026-11-21, on one 5120x1440 display with the taskbar docked left (62 px), `--auto 3 <folder>` opened the window at 1616x1008 outer (1600x1000 client), vertically centered but 93 px right of the display's center. The default comes from `Bounds::centered(None, size(px(1600.), px(1000.)), cx)`. Do not state a cause. Files: `crates/app/src/main.rs` (the default window bounds), `crates/app/src/session.rs` (`place`). TODO: ``- [ ] Decide whether the default placement should center on the display's work area (the visible bounds) or on the whole display, measure where `Bounds::centered` puts the window with the taskbar at the bottom and at the left, and change the default if the decision differs from what is measured.``
    - **D. Record in `docs/agents/app.md`** (find each section by its heading; keep the headings and their tags unchanged):
      - D1 "Every keystroke goes through the keymap": as its own paragraph ahead of the "Seen in the running app ... under `RUST_LOG=debug`" paragraph (so the colon before its two bullets stays directly before them; moved there after review round 1), add that the Keyboard Shortcuts capture saw the same names on Windows 11 on 2026-11-21: Shift+1 arrived as `!` and was added as a key; Ctrl+Alt+1 arrived as `ctrl+alt+1` and was refused with `"ctrl+alt+1" is bound to red`; Tab arrived as `tab` and was refused with `"tab" is bound to toggleSides`. Keep the AZERTY / AltGr bullets as read from the source.
      - D2 "`open` shows the OS folder picker, a later open winning": replace the "Not verified in the running app: ..." bullet with a "Seen (2026-11-21, Windows 11)" bullet: a second `ctrl+o` while the picker is up opens no second dialog (one `#32770` window; the main window is disabled while the dialog is up), a cancel changes nothing, and after a pick the folder opens and the keys work at once without a click.
      - D3 "A launch restores the last folder and the window": after the 2026-11-14 "Measured" bullet, add a "Measured (2026-11-21, Windows 11, one 5120x1440 display, a test store)" bullet: a moved and resized window quit with `q` reopened at the same bounds; a window snapped with Win+Left reopened on-screen at its restore bounds; with the taskbar docked left (62 px) a moved window reopened without drifting; after paging to `_DSC1495` (position 4) a relaunch without arguments revealed the folder in the tree at that file with the same index, and a folder argument overrode it; `--auto 3` opened at 1616x1008 outer (1600x1000 client), vertically centered and 93 px right of the display's center with the taskbar docked left (the pending question in `todo.md`). In the "Not seen yet" bullet, remove "a quit with `q`, a snapped window," and reduce the taskbar clause to the top-docked case, so that the bullet lists a second monitor (with a different scale factor) and a taskbar docked at the top; keep its parenthesis, its link and the macOS / Linux / Wayland sentences as they are.

## Decisions

- B7 (Cache) keeps the unseen tree-count check as the item rather than deleting it.
- The other stale "Not verified headless" bullets in `docs/agents/app.md` outside list D are left alone.
- B3 and B8 headings are renamed to the one check each keeps.
- The stale `textinput.rs` mention in B3's intro is left as is.
- Dropped without a remaining box, noted in the PR description only: the optional MCP `set_judgment` after a format switch, the close-with-`×` while the folder dialog is up, and box 2's obsolete tab-list / dimmed-area / focus-border expectations.
- Headings are from `main` at `1cad6de8`; if one no longer matches, find the item by its text and note it in `learnings.md`.

## Progress

- (2026-11-21) Step 1 complete
