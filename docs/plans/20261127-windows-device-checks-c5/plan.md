<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Record the Windows real-device checks run of 2026-11-27 (windows-device-checks-c5)

## Purpose

A real-device run on Windows 11 on 2026-11-27 (`crates/app` at `f5cab4bf`, a
debug build with `--features screenshot`) exercised the pending Windows checks
of `todo.md` (the file is at the repository root) from
`### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)`
up to, not including, the item on the look of the switch and the `Kbd` keys
(native-component-controls). It was driven through the MCP companion, the
`screenshot` tool and synthetic mouse and keys. Twenty checks passed, two
failed and were handed to their own sessions (`dlg-titlebar-dialog`,
`dlg-empty-wrap`), and the IME, HiDPI and drop checks were not run.

This plan records the run in the repository: `results.md` in this folder, the
passed boxes deleted from `todo.md`, the partly done ones narrowed to what is
left, and the rules of `docs/agents/app.md` the run measured brought up to
date. Docs only, no code change: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261126-macos-device-checks-mcp/plan.md` (#1307).

The run is already done; nothing is run again.

## Steps

- [x] Step 1: Copy the run's `results.md` into this folder and apply it to `todo.md` and `docs/agents/app.md`
  - Done when:
    - `results.md` in this folder is a byte-for-byte copy of the run's results file (LF line endings).
    - Every edit of lists B and C is applied, each `todo.md` item found by its exact `###` line.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files. In `todo.md` only the items of list B changed, and `### App: the strip's stars have a dark outer outline and its face focus icons do not` is byte-for-byte as on `origin/main`. In `docs/agents/app.md` only the bullets of list C changed.
    - No two `###` items in `todo.md` are left without a blank line between them, the count of doubled blank lines in each changed file equals `origin/main`'s, and every changed file has LF line endings.
    - The PR description records what was seen for each removed or narrowed box.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **No run.** The checks ran on 2026-11-27 at `f5cab4bf` (main, 2.2.0). Source of truth for every edit: `C:\Users\daisu\AppData\Local\Temp\claude\7327e214b13d7ea7\b89a68f6-de29-433a-a129-9350ffb7c71f\scratchpad\results.md`.
    - **Editing.**
      - `todo.md` is edited concurrently by other sessions. Rebase on `origin/main` right before editing and again before the PR; touch only the items named below. If an item was reworded on `main`, re-read it and apply the intent, do not overwrite.
      - Locate every item by its exact `###` line, never by line number. "Box N" counts the item's top-level `- [ ] ` lines from 1, as they are on `origin/main`.
      - Keep every other heading, item, box and sentence byte-for-byte; no item heading is renamed.
      - `todo.md` paragraphs and boxes are single long lines; wrap text added to `docs/agents/app.md` at the file's width, as its neighbours.
      - `RESULTS` below stands for `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`, always written in backticks, not as a link (the folder is archived by this PR's wrap-up; a link would not resolve before that).
      - Each background sentence of list B starts `Seen on Windows 11 on 2026-11-27 (results in \`RESULTS\`):` and goes where the item says. Take its facts from `results.md` only.
      - Suggested PR title: `docs: record the 2026-11-27 Windows real-device checks run`.
    - **A. `results.md`**: copy the source file to `docs/plans/20261127-windows-device-checks-c5/results.md` unchanged (`cp`), then check it has LF endings. It holds no Markdown link, only backticked paths.
    - **B. `todo.md`: three items removed whole, eleven narrowed, one given a sentence, one untouched.**
      - `### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)`:
        - In the Background, replace `(its own item below)` with `(fixed since; 5 of 5 held renames succeeded on 2026-11-27 and its item was removed)`.
        - Background sentence, inserted immediately before ` The step's checkbox was ticked on the automated criteria only.`: the right-click `Rename…` put a field in the row with the name selected whole; Enter renamed the folder in place with the strip's files, judgments and position kept, no pass 1 work after it and the tree keeping the keyboard; a case-only rename worked; a single click on a root's name opened it with no field; a rename during an import run was refused with `Rename… is unavailable while an import runs`. The IME was not run.
        - Delete boxes 1 (the `step7` setup), 2 (right-click `Rename…`), 4 (single click on a root), 5 (import run) and 6 (`Record the results in the archived learnings.md`). Box 3 (the Japanese IME) stays as it is.
      - `### App: a held folder rename intermittently fails with Access denied when it runs as pass 2 ends`: its one box passed (5 fresh folders of 264 ARWs, every held rename ran at the scan's end, no `Access denied`), so remove the whole item (heading, background, `Files:` line, `#### TODO`, the box), leaving one blank line between its neighbours.
      - `### App: real-device checks of the app icon on Windows, macOS and Linux`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261114-native-app-icon/plan.md\``: the window class's big and small icons, the taskbar button and Explorer's icon of `crates/app/target/debug/handpic.exe` all showed the Handpic icon, and the title bar draws it at its left end.
        - Delete box 1 (Windows) and box 3 (update the "Pending manual checks" bullet; list C does it).
        - Box 2 (Linux) stays, with one sentence appended: ` Once it passes, remove the "Pending manual checks" bullet under \`## Packaging\` in \`docs/agents/app.md\`.`
      - `### App: real-device checks of the strip's face focus icon and cell tooltips`:
        - Background sentence, inserted immediately before ` What is left open is in the boxes below.`: rejecting a good frame dimmed its green icon with the cell; on a failed and Blue-labeled cell the error showed as a tooltip over the cell and `Blue` over its name band, the tooltip swapped to the next cell's band (`Red`), a wheel scroll under the still pointer removed it, and a cell cut by the window's left edge still showed its own inside the window; over the tooltip regions a click, a Ctrl+click, a Shift+click, a right-click and the wheel still selected, extended, opened the menu and scrolled. The 500 ms delay was not measured (a capture takes about 0.3 s) and the only error at hand was too short to wrap.
        - Box 1 (150 %) stays.
        - Delete box 2 (reject dimming), box 4 (swap, scroll, cut cell) and box 5 (clicks).
        - Box 3 becomes: `- [ ] On Windows, hover a cell the scan failed on with an error long enough to wrap. Expect the tooltip after about 500 ms (time it with something faster than the \`screenshot\` tool, whose call takes about 0.3 s), wrapping at 360 px and staying inside the window. (On 2026-11-27 a short error's tooltip, a failed and labeled cell, the swap, the scroll, a cut cell and the clicks were seen.)`
        - Box 6 (macOS and Linux) becomes self-contained: `- [ ] On macOS and Linux, when at hand: reject a good frame and expect its icon dimmed with the cell; hover a failed cell and a labeled cell's name band and expect the error and the label's name as tooltips, swapping from cell to cell, going when the strip scrolls under a still pointer, and shown for a cell cut by the window's left edge; over a cell with a tooltip, click, ctrl / shift click, right-click and wheel still select, open the menu and scroll. (Seen on Windows on 2026-11-22 and 2026-11-27.)`
      - `### App: the strip's stars have a dark outer outline and its face focus icons do not`: **do not touch** (another session owns it).
      - `### App: real-device checks of the gpui-component adoption (theme, Input, controls, menus, Root, the three Dialogs)`:
        - One background sentence, inserted in the first paragraph immediately before ` Plan: \`docs/plans/_archived/20261114-gpui-component-adoption/plan.md\``: seen in passing, in a 480 px tall window `Filter ▾` and the right-click menus stop under the title bar, the `Filter ▾` thumb drags and a press on its track scrolls it and leaves it open, and the `Lightroom language` `Select` opens and chooses by keys with Tab / Shift+Tab keeping the focus in the settings; the boxes below were not otherwise run.
        - No box changes.
      - `### App: real-device checks of the strip, folder tree and meta pane scrollbars`:
        - Background sentence, inserted immediately before ` What is left open is in the boxes below.`: in a 480 px tall window, dragging the thumbs of `Filter ▾`, the strip's right-click menu, the Keyboard Shortcuts page, the Sidecar page and the `Lightroom language` list scrolled them, a press on the `Filter ▾` track scrolled it and left it open, the tree's right-click menu overflowed the window and scrolled by wheel, and the tree's own thumb drag scrolled the tree.
        - Delete box 1 (Windows thumb drags).
        - Box 2 becomes self-contained: `- [ ] On macOS and Linux, when at hand, repeat the Windows checks the Background records (the strip's, the tree's and the meta pane's bars), and drag the thumbs of the settings' \`Lightroom language\` list and Keyboard Shortcuts page, of \`Filter ▾\` and of the strip's right-click menu in a short window, and make the tree's right-click menu overflow in a short window and scroll it. Expect the bars to scroll their areas.`
      - `### App: real-device check of the folder tree rows' name truncation inside the pane`: both boxes are closed (the thumb drag, the long pending name and the inset edge were measured; list C updates the rule), so remove the whole item, leaving one blank line between its neighbours.
      - `### App: real-device check of the meta pane's \`Analysis\` group and the AF rows in \`Maker note\``:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261114-native-meta-analysis/plan.md\``: a scanned ARW whose JPEG markers were then zeroed (size and mtime kept) showed the decode error in the preview area, a `Maker note` group holding only `AF point` / `AF frame`, and its full `Analysis` group.
        - Delete box 1 (failed preview).
        - Box 2 stays, with one sentence appended: ` (Inconclusive on 2026-11-27: the first capture landed 0.37 s after \`open_folder\`, when the new folder's preview was already shown, and both folders held files of the same names. Use folders with different file names and something faster than a screenshot.)`
      - `### App: real-device checks of the settings as a full-window layer (native-settings-component Step 1)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261115-native-settings-component/plan.md\``: with the settings open, a hover where a strip cell's tooltip would show showed none, and three wheel notches there left the strip unscrolled; the drop was not run.
        - The one box (with its two numbered points) becomes: `- [ ] On Windows, run the app with a test \`--settings\` store and an open folder, open the settings and drop a folder on the window. Expected: it opens nothing (record what actually happens; if it differs, correct the drop rule in \`docs/agents/app.md\`).`
      - `### App: real-device checks of the settings body drawn with gpui-component's \`Settings\` (native-settings-component Step 2)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261115-native-settings-component/plan.md\``: on Keyboard Shortcuts, `+` then `home` showed `"home" is taken by this app` and `+` then `end` the same for `end`, in red in the settings body, not on the window's status line (the full-window settings layer covers it), the keymap unchanged.
        - The one box becomes: `- [ ] On Windows, the user's visual check of the settings' look (group headers, label-left / control-right rows, the sidebar's colors).`
      - `### App: real-device checks of the drag-resizable tree and meta panes`:
        - Background sentence, inserted immediately before ` What is left is in the boxes below.`: with both panes at 480 px in a 1200 px wide window (the preview 240 px), a click on the no-folder message just inside each pane's 5 px band opened the folder picker and one inside a band did not; the message is one unwrapped line and overflows the 240 px preview over the panes, handed to its own session (`dlg-empty-wrap`).
        - Delete box 1 (the no-folder message click). Box 2 (macOS and Linux) stays.
        - Add a box after box 2: `- [ ] Failed on Windows 11 on 2026-11-27: in a 240 px preview the no-folder message is one unwrapped line, drawn over the tree pane's background on the left and hidden under the meta pane on the right, and a click on the part over the tree pane opens nothing. The fix is in flight in its own session (\`dlg-empty-wrap\`); re-check on Windows once it lands.`
      - `### App: real-device check of the folder tree's file counts on collapsed rows (native-tree-index-counts, native-folder-count-cache, native-folder-count-prune)`:
        - Background sentence, inserted immediately before ` Plans: \``: an index whose `folder_counts` table was dropped showed no count on collapsed rows; a RAW added outside the app followed a `Refresh` and the new count showed after a restart; a `Rename by Capture Date…` split showed each date folder's count while collapsed, the emptied original removed; a junction and a folder on a mapped network drive kept their counts after a restart; a counted folder deleted outside lost its row on its parent's `Refresh`, and a new folder of that name showed no count after a restart. Not run: a split whose undated files keep the original folder.
        - Box 1 (Windows) becomes: `- [ ] On Windows (\`mise run app:dev\`), run \`Rename by Capture Date…\` with a split that leaves undated files in the original folder: it shows no count until listed. (The rest of this check passed on 2026-11-27.)`
        - Boxes 2 (macOS) and 3 (Linux) stay.
      - `### App: real-device checks of the title bar (native-title-bar)`:
        - In the Background, delete the sentence `The theme item depends on \`feature/native-themes\` mapping \`title_bar\` / \`title_bar_border\`; until then the bar paints with gpui-component's load-time values.` (its box is deleted below; `theme.rs` maps both).
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261116-native-title-bar/plan.md\``: with a folder open the preview, `f`, `d`, `z`, the tree, the meta pane, the strip and the status row lay out below the bar, and in a 480 px tall window `Filter ▾` and the right-click menus stop under it; drags from all four edges (the top one is the client's top 1 px, over the bar) and all four corners resized the window, the maximize button's hover showed Snap Layouts, and the maximized bar spans the display; a stored `windowBounds` placed the window at the same rectangle on three launches; five themes each gave the bar and its border their colors at once. Failed: under the trash dialog or a folder operation's dialog, minimize, maximize and the bar's drag are dead. Not run: the drop outline.
        - Box 1 becomes: `- [ ] On Windows, open a folder and drag a folder over the window. Expect the drop outline to lay out below the bar with nothing clipped. (The rest of this layout check passed on 2026-11-27.)`
        - Delete box 2 (resize borders, Snap Layouts, maximized), box 4 (`windowBounds`) and box 5 (the theme).
        - Box 3 becomes: `- [ ] Failed on Windows 11 on 2026-11-27: while the trash dialog or a folder operation's dialog is open, minimize, maximize and the bar's drag do nothing under the backdrop (they work with no dialog and with the settings open). The fix is in flight in its own session (\`dlg-titlebar-dialog\`); re-check on Windows once it lands.`
        - Boxes 6 (macOS) and 7 (Linux) stay.
      - `### App: real-device checks of the title bar's settings button (native-title-bar-settings)`:
        - Background sentence, inserted immediately before ` The Step 1 checkbox was ticked on the automated criteria and the checks above.`: the gear, its hover fill and its tooltip `Settings Ctrl+,` showed in Neutral dark, Gruvbox Dark and Solarized Light, and a click on the gear during an inline rename opened the settings and closed the field with the name unchanged.
        - Delete box 1 (a shipped theme) and box 2 (the gear during an inline rename). Boxes 3 (macOS) and 4 (Linux) stay.
      - `### App: real-device check of the settings controls drawn with gpui-component (native-component-controls)`: both points passed (the language `Select` opened and chose by keys; Tab / Shift+Tab with the list open kept the focus in the settings), so remove the whole item, leaving one blank line before `### App: real-device check of the look of the switch and the \`Kbd\` keys (native-component-controls)`, which is not touched.
    - **C. `docs/agents/app.md`: the bullets below, nothing else; no section heading's tag changes.** Find each by its opening words.
      - Under `## Packaging`, the bullet starting `- Pending manual checks, not done here:`: replace it with a bullet saying the only pending check is the running window's icon after the install task on a Wayland and an X11 session; the Dock / Finder icon was seen on macOS on 2026-11-26 (keep its existing backticked results path) and the taskbar, title bar and Explorer icon on Windows on 2026-11-27 (`RESULTS`).
      - Strip tooltip section (`### A strip cell's tooltips are transparent \`div\`s over the canvas ...`), its bullet starting `Not verified by hand yet` (or the sentence of that wording): replace it with the measurement (2026-11-27, Windows 11, `RESULTS`): the hover, the swap between cells, a scroll under a still pointer removing it, a cell cut by the window's left edge and the clicks over the regions were seen; the wrapping at 360 px and the delay are still unmeasured.
      - Folder tree section, the bullet starting `- Rule: a tree row takes \`w_full()\``: replace its last sentence `Not yet observed: a drag of the scrollbar's thumb, a click on a row's inset edge and a long pending name.` with the measurement (2026-11-27, Windows 11, `RESULTS`): a drag of the scrollbar's thumb scrolls the tree; a held rename's long pending name truncates with its count whole; the left 0 to 5 px of the pane is the list's `px_1` padding and takes no press, the row starting at x = 6; the row's hover fill runs under the 16 px scrollbar track on the right, but a press there goes to the track, not the row.
      - Same section, the bullet starting `- Rule: a row shows its file count, expanded or collapsed`: replace its last sentence `Not yet observed: a folder reached through a mapped drive, a symlink or a junction, and an index upgraded from before the table.` with: measured (2026-11-27, Windows 11, `RESULTS`) a junction and a folder on a mapped network drive keeping their counts after a restart, and an index whose `folder_counts` table was dropped showing no count on a collapsed row; a symlink was seen on macOS on 2026-11-26.
      - Same section, the bullet starting `- Rule: every count the tree learns from disk is written to \`folder_counts\``: replace `Inferred, not yet observed in the running app.` with: measured (2026-11-27, Windows 11, `RESULTS`) a RAW added outside the app and a `Refresh` of the collapsed folder showed the new count, kept after a restart.
      - Same section, the bullet starting `- Rule: every successful tree listing (\`tree_listed\`'s \`Ok\` branch`: replace `Inferred, not yet observed in the running app.` with: measured (2026-11-27, Windows 11, `RESULTS`) a counted folder deleted outside the app lost its row on a `Refresh` of its parent, and a new folder of that name showed no count after a restart.
      - `### Driving the window in a real-device run on Windows (Measured)`: append four `- Rule:` bullets at the end of the list, each citing `RESULTS`:
        - A `cargo build` of `crates/app` can be stopped by the host's low-memory reaper while several other sessions build; build with `cargo build -j 2`.
        - The window has no frame above the client area, so the top resize band is the client's top pixel, over the title bar; drag from y = 0 of the client. A corner drag can do nothing on the first try; retry before calling it a failure.
        - A Python `subprocess` capture of a launcher that starts the app with redirected std handles hangs until the app exits, since the app inherits the handles; have the launcher write to files and read them instead.
        - A `screenshot` call takes about 0.3 s, so a delay or a transient shorter than that (a tooltip's 500 ms delay, the pane between a folder open and its first preview) cannot be measured with it.
      - Title bar section (`### The title bar is gpui-component's \`TitleBar\` ...`), the bullet starting `- Unmeasured (by-hand checks pending in the plan):`: rewrite it so that the resize edges over the bar (the client's top pixel), Snap Layouts and a stored `windowBounds` restoring without drift are stated as measured on 2026-11-27 (Windows 11, `RESULTS`); the controls and the drag under a dialog's backdrop are stated as observed dead under the trash dialog and a folder operation's dialog and working under the settings, the fix in flight; and the rest of the bullet's list (double-click to maximize, macOS's traffic lights, Linux's decorations) stays unmeasured as written. Keep it short; the fixing session will rewrite it.
    - **D. `learnings.md`**: this step's notes. The setup worth keeping (per-run `--index` / `--settings` / `--log`, `--mcp-port`, hardlinked ARWs, a preview made to fail after its scan by zeroing the JPEG markers with size and mtime kept); what synthetic input added over MCP alone (right-clicks, drags, keys in the settings) and what still needs the user (the IME, HiDPI, a drop from Explorer, the look); the two failures and the sessions they went to; anything that came up while applying the edits (an anchor moved on `main`, a review comment).

## Trade-offs and risks

- **The tree row truncation item is removed whole**, not narrowed: both of its boxes are closed. Its box asked for a click and a right-click on the inset edge; only a right-click was made, which shows the row's hit area.
- **Boxes that were not run are reworded** where they pointed at boxes this plan deletes: the macOS / Linux boxes of the strip tooltip and scrollbar items become self-contained, and the icon item's Linux box gains the clause about removing the `app.md` bullet.
- **Two Background sentences are edited beyond an insertion**: `(its own item below)` in the inline rename item, and the title bar item's sentence about "the theme item".
- **The inline rename's "record the results" box is dropped**; the next run's own `results.md` is the place.
- **`app.md`'s keymap section** says a refusal shows on the status line; the run saw it in red in the settings body (possibly the settings' own line). Left as is; only the `todo.md` Background notes it.
- **The settings-layer drop box keeps "opens nothing"** as its expectation, as on `main`.
- **The "upgraded index" was simulated** by dropping the `folder_counts` table with `user_version` kept, and the symlink case was seen on macOS only; `app.md` is worded to say so.
- **Colors were judged only as "visible"**, so the theme boxes of the title bar and the gear are closed on that.
- `todo.md` is edited by other sessions; an item reworded on `main` since planning must be re-read, not overwritten.

## Progress

- 2026-11-27: Step 1 done (results.md, learnings.md, todo.md, docs/agents/app.md bullets)
