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

# Record the Windows real-device checks run through MCP and screenshots (2026-11-23)

## Purpose

A real-device run on Windows 11 on 2026-11-23 exercised pending manual checks
of `todo.md` (the file is at the repository root) through the MCP companion
and its `screenshot` tool, with a few synthetic left clicks; the user judged
the visual points from the running window. It covered nine "real-device
check" items in part or whole and produced five findings (A to E). This plan
narrows those items to what was not run, adds a `todo.md` item for each of
findings A to D, appends finding E to an existing item, and marks three rules
of `docs/agents/app.md` as seen. Docs and todo only, no code change, no
finding fixed: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-8/plan.md`.

The run is already done. Its results are in the session scratchpad's
`results.md`; Step 1 copies that file into this folder as `results.md` and
applies it. Nothing is run again. `results.md` is the source of truth for
every edit: lists A to E below name the exact edits.

## Rule for sub-points the run did not exercise

A sub-point of a box that `results.md` does not mention, or lists as not run,
was **not** run and must stay open: carry it into a reworded box with a short
note of why. A box is deleted only when every one of its sub-points was
exercised and passed. The run pressed no key, used no IME, no drag and drop,
no right-click and no HiDPI display, and ran on Windows only, so every
sub-point that needs one of those, and every macOS / Linux box, stays open by
design. No item is deleted whole: each keeps at least one box. Cases found
while planning, and how each is settled here (see "Trade-offs and risks" for
the other option of each):

- A box the user judged OK by eye as a whole is closed on that judgment, even
  where `results.md` does not name each sub-point; the item's background
  sentence says it was a by-eye judgment. This covers the `f` overlay boxes 1
  and 2 (no Tauri reference build was at hand, so not side by side; the
  overlapping labels of small adjacent faces were accepted by the user) and
  the catalog icons' Windows box (Neutral dark only).
- `todo-three-decisions` box 1: `Sort ▾` opening in a RAW folder is not named
  in `results.md` (the filter's opening and the sort's dimming in a JPEG-only
  folder are). Closed: the sort menu's opening was seen on 2026-11-22
  (`native-strip-header`).
- `native-meta-analysis` box 1: the absence of `AF eye in focus` /
  `Focus candidate` rows is not stated; `results.md` lists the rows the group
  held. Closed on that list.
- `native-themes` boxes 1 and 2: the themes were set through MCP
  `set_settings`, not the Appearance page, and only the panes, the strip, the
  status row, the title bar and the menus are named. Both boxes are narrowed,
  not deleted.

## Steps

- [x] Step 1: Copy the run's `results.md` into this folder and apply it to `todo.md` and `docs/agents/app.md`
  - Done when:
    - `results.md` in this folder is the scratchpad's file with LF line endings and one change only: the Japanese gloss `(外枠)` in finding C removed (everything committed is English).
    - Every edit of lists A to E is applied.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files; in `todo.md` only the items of lists A to C changed or were added; in `docs/agents/app.md` only the three rule bullets of list D changed; nothing under `docs/plans/20261123-all-faces-check/` changed.
    - No two `###` items are left without a blank line between them, and the count of doubled blank lines in `todo.md` and `docs/agents/app.md` equals `origin/main`'s. Every changed file has LF line endings.
    - The PR description records what was seen for each removed or narrowed box, with its notes.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **No run.** The checks were run on Windows 11 on 2026-11-23 at `3918064c`; the source file is `C:\Users\daisu\AppData\Local\Temp\claude\596c5c3851d7158e\c1168b24-5ee7-4db7-bc39-d22ac0319fc0\scratchpad\results.md`. Copy it to `results.md` in this folder, converting to LF (assert no CR is left) and removing `(外枠)` with the space before it.
    - **Editing.**
      - `todo.md` is edited concurrently by other sessions. Rebase on `origin/main` right before editing and again before the PR; touch only the items named below.
      - Locate every item and section by its exact `###` line or the quoted anchor text, never by line number. "Box N" counts the item's top-level `- [ ] ` lines from 1.
      - Keep every other heading, item, box and sentence byte-for-byte.
      - Write a Python script to the scratchpad with the Write tool (not a Bash heredoc) and run it with `mise x -- python -I`, asserting exactly one match per anchor and writing LF.
      - `RESULTS` below stands for `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`, always written in backticks, not as a link (the folder is archived by this PR's wrap-up; a link would not resolve before that).
      - Each background sentence of list A is inserted into the item's background paragraph (the long paragraph that holds `Plan: \`docs/plans/_archived/...`) immediately before its sentence `What is left open is in the boxes below.` when it has one, else immediately before ` Plan: \``. It starts `Seen on Windows 11 on 2026-11-23 (results in \`RESULTS\`)`.
      - Wrap added text in `docs/agents/app.md` at the file's width, as its neighbours; `todo.md` paragraphs and boxes are single long lines.
      - Suggested PR title: `docs: record the 2026-11-23 Windows real-device checks run through MCP and screenshots`.
    - **A. `todo.md`: nine items narrowed** (no item deleted whole).
      - `### App: real-device check of the Sort / Filter menus with no folder, the status row's reserved slot and shortened strip names (todo-three-decisions)`:
        - Background sentence: with no folder both buttons were dimmed and a left click on either opened no menu, `Filter ▾` opened on a click with a folder open and `Sort ▾` was dimmed in a JPEG-only folder; the position `1 / 8` stayed at the same x with the progress circle shown (pass 2) and gone; at the default strip height `ILCE-6300_Free_ra…3068.arw` and `L1001880-DxO_D…ME 3.jpg` fitted inside their cells and short names stayed centered and unchanged; the first two rules are now marked seen in `docs/agents/app.md`, the third at the default height only.
        - Delete boxes 1 and 2.
        - Box 3 becomes: `- [ ] On Windows 11, open a folder with long file names and drag the strip shorter and taller. Expect a long name in a strip cell to stay fully inside the cell with no glyph clipped at either edge at each height, and a short name to stay centered and unchanged. If a glyph is clipped, tune \`NAME_INSET\`. (Seen at the default strip height on 2026-11-23; the run made no drag.)`
        - Box 4 becomes: `- [ ] After that check passes, drop the default-strip-height limit from the middle-ellipsis rule in \`docs/agents/app.md\`.`
      - `### App: real-device checks of the title bar's smaller text and centered folder name (native-title-bar-folder-name)`:
        - Background sentence: no name showed with no folder, `marks` was centered after an open, and the name followed another folder's open, `Rename by Capture Date…` (`onedate` to `2026-10-17`) and a folder rename to a 72-character name; at a 784 px window that name still fitted and at 604 px it ended in `…` before the gear with room to spare, so `TITLE_NAME_MARGIN` was left as is; the text size was not judged.
        - Box 1 becomes: `- [ ] On Windows 11 (\`mise run app:dev\` or a debug build of \`crates/app\`), expect "Handpic" next to the icon to read at 14 px with the icon still 16 px. (Not judged by eye on 2026-11-23; the rest of this check passed.)`
      - `### App: real-device check of the meta pane's \`Analysis\` group and the AF rows in \`Maker note\`` (in `todo.md` the heading has plain backticks):
        - Background sentence: on a fresh index of 125 ARWs the shown file's `Analysis` group appeared at `analyzing 27/125` with no input, holding `Eye focus` (`Sharp · 0.281`), `Eyes` (`Open · 76`), `Eyes closed`, `Eye aspect ratio`, `Yaw` / `Pitch` / `Roll`, `Mesh eye offset`, `Edge gap` and `Sharpness`, with `AF point` (`5913, 2964 of 8640 × 5760`) and `AF frame` directly after `AF tracking` in `Maker note` and neither in `Analysis`; in a folder under 10 files the group appears only when pass 2 ends, since pass 2 commits rows in batches of 10 (`BATCH` in `crates/engine/src/index.rs`), which is expected; a JPEG showed no `Analysis` group, a filter hiding every file left only `No files match the current filter.`, and the rows were the same with `f` on and off.
        - Delete box 1.
        - Box 2 becomes: `- [ ] On Windows, a file whose preview failed still shows its \`Analysis\` group and a \`Maker note\` group holding only the AF rows. (Not run on 2026-11-23; the box's other cases passed.)`
        - Box 3 stays.
      - `### App: real-device checks of the f focus mark, face boxes and face mesh`:
        - Background sentence: the user judged the `f` overlay OK by eye on the eight files, with no reference Tauri build at hand and so not side by side: the AF crosshair and frame bright green on `_DSC1758`, gray on `_DSC1616`, white on the rest and no AF mark on `ILCE-6400_DSC00405`; nine face boxes with cyan dots and labels on `_DSC1758`; the magenta mesh on `DSC03068` (closed eyes, no iris rings), `DSC00405`, `DSC01340` (orientation 6) and `_DSC1758`; the labels of small adjacent faces overlap on `_DSC1616` / `_DSC1758`, which the user accepted; the status line went `scanning N/8`, `analyzing N/8`, then empty.
        - Delete boxes 1 and 2.
        - Box 3 becomes: `- [ ] On Windows, hold \`→\` with \`f\` on and check that the log's \`eyes\` lines name only files the paging stopped on or passed slowly. (Not run on 2026-11-23: the run pressed no key.)`
        - Box 4 becomes: `- [ ] On Windows, on a folder of a few hundred RAW files with a fresh \`--index\`, check that switching to another folder mid-pass 2 from the folder tree stops it (reopening the first folder resumes the remaining files). (Not run on 2026-11-23; the status line's \`scanning\` then \`analyzing\` and its clean end were seen.)`
        - The item's second paragraph (the reference Tauri build) stays.
      - `### App: real-device checks of the strip's face focus icon and cell tooltips`:
        - Background sentence: on a fresh index the icons appeared on cells during pass 2 with no input, and none on neutral frames or JPEGs; no 150 % display was at hand.
        - Box 1 becomes: `- [ ] On Windows, expect the strip's 12 px face focus icon to be crisp at 150 % display scale. (Not run on 2026-11-23: no HiDPI display; the icons' appearance during pass 2 was seen.)`
        - Boxes 2 to 6 stay.
      - `### App: real-device check of the catalog-served icons (native-catalog-icons)`:
        - Background sentence: the `Filter ▾` menu's star rows (filled and outlined), the orientation rectangles and the `Face focus` icon, and the strip's good and low face icons, were judged OK by the user in Neutral dark; the label squares are gpui-component `Tag`s now and no longer catalog icons.
        - Delete box 1.
        - Box 2 becomes: `- [ ] On macOS and Linux (any platform at hand), open the \`Filter ▾\` menu and a scanned folder and confirm the star rows, the orientation rectangles, the \`Face focus\` faces and the strip's face focus icon on a good and a low frame look right.` (its "repeat the Windows check above" would point at the deleted box).
      - `### App: real-device checks of the filter menu's icons and headings (gpui-filter-menu-icons)`:
        - Background sentence: hovering the `Flag` heading drew no highlight, and a click on `Rating` changed nothing and left the menu open.
        - Delete box 1. Box 2 (HiDPI) stays.
      - `### App: real-device checks of Rewrite Sidecars from Index… and Delete Sidecars… and the folder menu (gpui-folder-dialogs Step 3)`:
        - Background sentence: a Rewrite requested during pass 1 of a fresh index noted `Rewrite Sidecars from Index: waiting for the scan to finish` and its dialog opened once pass 2 ended, the same for a Delete (`Move 3 sidecars (844 B) of longnames to the Trash?`); through MCP with `mcpAllowDestructive`, a confirmed Delete moved 4 `.xmp` to the Trash, the strip's stars, flags and labels went, and MCP `undo` restored the files and the marks.
        - Delete box 2.
        - Box 4 becomes: `- [ ] On Windows, after a \`Delete Sidecars…\` run on a folder whose files carry judgments, press \`Ctrl+Z\`: the marks return. (On 2026-11-23 the run and the MCP \`undo\` were seen; the key was not pressed.)`
        - Boxes 1, 3, 5 and 6 stay.
      - `### App: real-device checks of Sequence JPEG Timestamps… (gpui-folder-dialogs Step 4)`:
        - Background sentence: a confirmed run on 500 JPEGs showed `sequencing 109 / 500` in the pane's notes and ended `Wrote 500 of 500 files to …\jpeg500-sequenced`.
        - Delete box 3. Boxes 1, 2, 4 and 5 stay.
      - `### App: real-device checks of Rename by Capture Date… and the folder-rename aftermath (gpui-folder-dialogs Step 6)`:
        - Background sentence: with two files judged (4 stars; pick and Green), a run confirmed through MCP renamed `onedate` to `2026-10-17`, reopened it, and the strip kept both judgments and the current file.
        - Box 2 becomes: `- [ ] On Windows, pick the item on \`one-date\`. Expect Tab to move from \`Run\` to \`Cancel\` and Escape to close the dialog. \`Run\`: the buttons dim. (Not run on 2026-11-23: no key press, and the run was confirmed through MCP with no dialog; the strip keeping its judgments after the reopen was seen.)`
        - Boxes 1, 3 and 4 stay.
      - `### App: real-device checks of the Appearance tab (mode, shipped themes) (native-themes)`:
        - Background sentence: Neutral Light, Catppuccin Latte, Tokyo Night and Neutral dark, set through MCP `set_settings`, applied at once to the panes, the strip, the status row, the title bar and the menus; in the light themes a rejected cell's dim was too faint and the filter menu's `Face focus` `Good` green too bright (their own items below); System, a restart and the pickers' keys were not run.
        - Box 1 becomes: `- [ ] On Windows, open the settings (\`ctrl+,\`), Appearance, choose Light. Expect the dialogs and the settings box to turn light and stay readable, and check the strip's stars (\`#ffd050\`) and the pick green on the light strip, the tree cursor's ring, the menus' hover and the progress bar. Choose System and switch the OS appearance while the app runs: the app follows it without a restart. Choose Dark: it stays dark whatever the OS does. Restart: the choice is restored before the window is drawn. Record what is hard to read in the archived plan's \`learnings.md\` (the fixed judgment colors are not retuned in this plan). (On 2026-11-23 the panes, the strip, the status row, the title bar and the menus were seen turning light, set through MCP.)`
        - Box 2 becomes: `- [ ] On Windows, with a shipped dark theme (e.g. Tokyo Night) and a shipped light theme (e.g. Catppuccin Latte) chosen from the \`Dark theme\` and \`Light theme\` pickers, expect the dialogs and the settings box to take the theme and stay readable, the preview's surround taking the darkest of the theme's surfaces (\`Roles::preview\`; black in Neutral dark, per \`preview-theme-background\`, plan \`docs/plans/_archived/20261118-preview-theme-background/plan.md\`). Check the derived \`Roles\` on both (the strip's file names and failures, the burst band) and the fixed stars / pick / reject colors on each. Pick Neutral again: the app looks exactly as before (Neutral dark unchanged). In System mode with two non-Neutral themes chosen, switch the OS appearance: the app swaps between them. Restart: the choice is restored. Pickers' keys: Space / Enter open the menu, the arrows move, Enter picks, Escape with the menu open closes the menu only, with it closed closes the settings. (On 2026-11-23 the panes, the strip, the status row, the title bar and the menus were seen taking Tokyo Night and Catppuccin Latte at once, set through MCP; the dimmed reject has its own item below.)`
        - Boxes 3 and 4 (macOS, Linux) stay.
    - **B. `todo.md`: one sentence appended to an existing item (finding E).** In `### App: Rewrite Sidecars from Index reported "4 files not in the index (skipped)" in a fully scanned folder`, append to the end of the Background paragraph's first line (the line ending `... / \`without one\` rows.`, before the `Files:` line): ` Reproduced on Windows 11 on 2026-11-23 (results in \`RESULTS\`): on the 8-file \`marks\` folder after both passes, with four files judged, the preview read \`4 files with a judgment\` and \`4 files not in the index (skipped)\`; the other four files are indexed and unjudged.` The TODO box stays. **Not applied (found while implementing):** that item is no longer in `todo.md`; #1253 (`eb658f99`, merged after the run's `3918064c`) reworded the row to `N files with no judgment on record (skipped)` and removed the item, and its on-screen check is already a box of the `todo-four-display-fixes` item. Finding E stays in `results.md` and `learnings.md` only.
    - **C. `todo.md`: four new items** (findings A to D), each `### ` heading, `#### Background`, a `Files:` line, `#### TODO` with one box, one blank line around each. Each Background starts `Seen on Windows 11 on 2026-11-23 (results in \`RESULTS\`):` and states the finding as `results.md` does; none states a cause as fact.
      - Finding A, directly after the `native-themes` Appearance item (before `### App: check the upstream licenses of the shipped gpui-component themes before packaging (native-themes)`): `### App: a rejected strip cell's dim is too faint in light themes`. Background: in Neutral Light and Catppuccin Latte a rejected cell is dimmed so faintly that it melts into the light background; the dim is `Roles::rejected_dim`, which outside Neutral dark is the theme's `background` at 0.55 alpha (`Roles::of`). Files: `crates/app/src/theme.rs` (`Roles::of`, `rejected_dim`), `crates/app/src/main.rs` (`paint_strip`). TODO: `- [ ] Make a rejected cell read as rejected on light themes (Neutral Light, Catppuccin Latte) without changing Neutral dark, and check it on screen.`
      - Finding B, directly after finding A's item: `### App: the filter menu's \`Face focus\` \`Good\` green is too bright on light themes`. Background: the `Filter ▾` menu's `Good` row icon is drawn in `MarkState::color`'s fixed `#33ff33`, chosen for drawing over a photo, and is hard to read on a light menu surface. Files: `crates/app/src/mark.rs` (`MarkState::color`), `crates/app/src/main.rs` (`menu_row`, `Lead::Face`). TODO: `- [ ] Give the menu's \`Good\` icon a green that reads on light and dark menu surfaces, leaving the strip's and the \`f\` overlay's green over the photo as is, and check it on screen.`
      - Finding C, directly after the `### App: real-device checks of the strip's face focus icon and cell tooltips` item: `### App: the strip's stars have a dark outer outline and its face focus icons do not`. Background: on screen the strip's stars are drawn with a dark outer outline while the face focus icons (Good / Low) are not; the user wants the strip's icon drawing unified. In the code both paint dark copies under the mark (`paint_marks`, the stars "over a 1 px dark copy"; `paint_face_icon`, over dark copies for the `drop-shadow` halo, at `overlay::HALO`), so why the icon's halo does not show is not known. Files: `crates/app/src/main.rs` (`paint_marks`, `paint_face_icon`, `paint_burst_badge`), `crates/app/src/overlay.rs` (`HALO`). TODO: `- [ ] Find why the face focus icon's halo does not show, decide one drawing for the strip's marks (the stars, the face focus icon, the burst badge) with the user, and apply it.`
      - Finding D, directly after the `### App: real-device checks of the strip, the scan progress, the resume and the resync` item (before `### App: measure the scan pause in \`load\` ...`): `### App: a JPEG folder's strip cells in view stay gray until pass 1 ends`. Background: on `jpeg500` (500 JPEGs, hardlinks of cached files, fresh index, debug build) the strip cells in view at positions 3 to 9 stayed gray placeholders until pass 1 ended (`scan extract … in 89963ms`), while cells 1, 2, 10 and 11 had thumbnails early (seen at `scanning 492/500` and `498/500`); the hot list should have taken the cells in view first. Files: `crates/app/src/focus.rs` (`scan_focus_paths`), `crates/app/src/main.rs` (`scan_focus_paths`, the `SCAN_FOCUS_DELAY` task), `crates/app/src/thumbs.rs`, `crates/engine/src/index.rs` (`run_scan`), `crates/core/src/scan.rs` (`ScanFocus`). TODO: `- [ ] Reproduce on a JPEG folder with a fresh index, find whether the hot list misses the cells in view or their finished thumbnails are not read back until the pass ends, and fix it.`
    - **D. `docs/agents/app.md`**, these three rule bullets and nothing else; no section heading's tag changes (each of the three sections, `### The sort and filter menus are deferred popovers over the window (Inferred)`, `### The strip is a \`canvas\` over a pure model, not a \`uniform_list\` (Inferred)` and `### One progress circle sits in the status row's right region (Inferred)`, holds other statements this run did not see; confirm that while editing and leave the tags):
      - In the sort and filter menus section, the bullet starting `- Rule: with no folder open (\`Handpic.dir\` unset) both buttons are inert`: replace its last sentence `Inferred: not yet seen on screen.` (it wraps over two lines, `Inferred: not` / `yet seen on screen.`) with `Verified on Windows 11 on 2026-11-23: both buttons were dimmed and a left click on either opened no menu (\`RESULTS\`).`
      - In the progress circle section, the bullet starting `- Rule: the circle is in the row's flow inside the right region`: replace its last sentence `Inferred: the constant position is not yet seen on screen.` with `Verified on Windows 11 on 2026-11-23: the position \`1 / 8\` stayed at the same x with the circle shown (pass 2) and gone (\`RESULTS\`).`
      - In the strip section, the bullet starting `- Rule (Inferred, not yet seen on screen): a cell's name wider than the cell`: change the prefix to `- Rule (seen at the default strip height only; other heights not yet seen on screen):` and append at the bullet's end: `Seen on Windows 11 on 2026-11-23 at the default strip height: \`ILCE-6300_Free_ra…3068.arw\` and \`L1001880-DxO_D…ME 3.jpg\` fitted inside their cells and short names stayed centered and unchanged (\`RESULTS\`).`
    - **E. `learnings.md`**: this step's notes.
      - What MCP plus `screenshot` can and cannot close (no key presses, IME, drag, right-click, HiDPI), and that visual points still need the user's judgment.
      - The setup notes of `results.md` worth keeping: a fresh `--index` / `--settings` / `--log` and `--mcp-port` per launch, `mcpEnabled` through `--import-settings`, and Git Bash mangling `\\` in JSON arguments.
      - Pass 2's batches of 10 make a folder under 10 files report everything at once.
      - The decisions of the rule section above, and that findings A to D became items and E a sentence.
      - `results.md` stays in the folder as the run's record.

## Trade-offs and risks

- `f` overlay boxes 1 and 2 are closed on the user's by-eye judgment with no Tauri side-by-side, and `results.md` does not name every sub-point (the arm ends' dark outline, the label placement at the image's edges, the orientation 8 file's boxes, iris rings on the open-eyed frames). The other option is to keep a narrowed box for a side-by-side against Riffle 1.10.0; it would stay open until someone installs a reference build. The user judged the overlay OK, so this plan closes them and says so in the background.
- `todo-three-decisions` box 1 and `native-meta-analysis` box 1 are closed although one sub-point each is not named in `results.md` (see the rule section). Keeping a one-line box for each is the stricter option.
- `native-themes` boxes 1 and 2 are narrowed, not deleted: the themes were set through MCP, and the dialogs, the settings box and the readability points are not in `results.md`.
- `results.md` is copied with `(外枠)` removed, because the repository is English only.
- The catalog icons' macOS / Linux box is reworded although it was not run, because it referred to the deleted Windows box.
- The three `docs/agents/app.md` section headings keep `(Inferred)`.
- Finding C's background notes that the code paints a halo under both marks. That is read from the comments of `paint_marks` and `paint_face_icon`, not measured; the item asks to find the cause.
- A narrowed box's record lives in `results.md`, the background sentence and the PR description only.

## Progress

- 2026-11-23: Step 1 done. The run is in `results.md`, the notes in `learnings.md`. Edits made:
  - A: nine `todo.md` items narrowed (boxes deleted or reworded, background sentences added).
  - B: not applied; #1253 removed the item, so finding E stays in `results.md` and `learnings.md` only.
  - C: four items added to `todo.md` (findings A to D). Finding C's item also names the closed-eyes mark (#1257), which the run predates.
  - D (`docs/agents/app.md`): three rule bullets updated, no section tag changed.
  - E: `learnings.md` written.
