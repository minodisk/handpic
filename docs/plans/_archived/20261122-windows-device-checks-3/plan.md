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

# Record the third batch of Windows real-device checks (2026-11-22, folder tree group)

## Purpose

A third real-device run on Windows 11 on 2026-11-22 exercised the pending
manual checks of the folder tree group: the tree pane, its keyboard, its
right-click menu and its following of the disk, the strip's multi-selection
and right-click menu, Move to Trash… / Delete Permanently… and Trash undo /
redo, the tree's inline folder rename, the file counts on collapsed rows, the
rows' name truncation and the rows drawn as `ListItem`s. This plan removes the
`todo.md` items and boxes the run settled, rewords the ones it settled in
part, adds the three questions the run raised as new items, records what was
seen in `docs/agents/app.md` where a rule covers it, and keeps the full
results in this folder's `learnings.md`. Docs and todo only, no code change:
one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261121-windows-device-checks/plan.md` and
`docs/plans/_archived/20261121-windows-device-checks-2/plan.md`.

The run's full results are in [learnings.md](./learnings.md) in this folder
(moved there from the `results.md` the run wrote). It is the source of truth
for every fact below.

## Rule for sub-points the run did not exercise

The results name what was seen. A sub-point of a box this plan deletes or
narrows that `results.md` does not mention was **not** run and must stay open:
carry it into the reworded box (or keep the box) instead of deleting it.
Known cases (check for others while editing):

- gpui-tree-trash Step 3: a right press on another row opens that row's menu.
- Step 4: a click on the preview hands the keys back; the right-click rules
  hand back / keep the keys; a folder created in Explorer under an expanded
  one still appears after `Expand All`.
- Step 6: a left click on a cell still pages (with the menu closed by it).
- Step 7: clicks on the tree, strip and status menus do nothing with the dialog
  open; the `moving to Trash 0 / 1` text and the `Move to Trash: waiting for
  the scan to finish` note (too short-lived to capture).
- ListItem item: a click on an open leaf row does nothing; the outline also
  shows on the open row; the settings modal's text selection reads gray.
- The rename item's plain rename box (the `Shots` rename with the judgments,
  the position, the log line and the case-only rename) stays as is.

## Steps

- [x] Step 1: Apply the run's results to `todo.md` and `docs/agents/app.md`, and write `learnings.md`
  - Done when:
    - Every edit in the lists A, B, C and D below is applied (with the rule above), and `learnings.md` in this folder holds what list E names.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files, and no `todo.md` item outside the lists changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where an item or a box went.
    - The PR description records what was seen for each removed item and each removed box, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Locate every item by its `###` heading, never by line number. An item runs from its `###` line to the line before the next `###`. "Box N" counts the top-level `- [ ]` lines of the item, from 1; a "point" is an indented sub-line of a box.
    - Write everything in English, in the file's own style (`#### Background`, `Files:`, `#### TODO`, paths and identifiers in backticks). Keep the neighbouring items, and every sentence of an item this plan does not name, byte-for-byte.
    - Every "seen" sentence added to a Background says "on Windows 11 on 2026-11-22".
    - The earlier plans' `learnings.md` note that a Python script (written to the scratchpad with the Write tool, run with `python -I`) locating each item by its exact `###` line and asserting one match per anchor kept the neighbours byte-for-byte.
    - Do not write a Markdown link to another plan folder; name paths in backticks. `todo.md` refers to this plan's results as `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md` (where the wrap-up moves it).
    - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the folder tree`.
    - **A. Delete the whole item in `todo.md`:**
      1. `### App: real-device check of Left / Right paging the strip and Space toggling from the folder tree (native-tree-left-right)`. Every point passed (results section "native-tree-left-right").
    - **B. Reword in `todo.md`:**
      2. `### App: real-device checks of the folder tree, the strip's selection and right-click menu, and Move to Trash undo / redo`. Its two intro paragraphs are the Background; its seven boxes are Steps 2 to 8 in order.
         - Rename the heading to `### App: real-device checks of the folder tree and Move to Trash… left open (gpui-tree-trash)`.
         - In the first intro paragraph, replace "Nobody has clicked the tree, opened the menus or dialog, used Explorer, the clipboard or the Recycle Bin, or seen the keyboard focus model run: every step's checkbox was ticked on the automated criteria." with: on Windows 11 on 2026-11-22 these were exercised on a debug build with synthetic input (results in `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`); what is left open is in the boxes below; the run's findings are their own items (the Sort / Filter menus open with no folder; the Undo's count and reason after the Recycle Bin was emptied by hand; and, under the rename item, the held rename's `Access denied`). Keep the second intro paragraph unchanged.
         - Box 1 (Step 2) keeps only: the scan stop on a switch (a few hundred ARWs in `...\tree\a` so its scan runs for a while on a fresh `--index`; click `b` during it: no further `a` progress in the log) and closing the window with no folder quits.
         - Box 2 (Step 3) keeps only: under `RUST_LOG=debug` no re-list while collapsed; a right press on another row opens that row's menu; switch to another app and back with no flicker or collapse; a USB drive appears as a new root.
         - Box 3 (Step 4) keeps only the unexercised points (see the rule above); delete the box if none remain.
         - Delete box 4 (Step 5).
         - Box 5 (Step 6) keeps only: a left click on a cell still pages.
         - Box 6 (Step 7) keeps only: the clicks on the tree, strip and status menus with the dialog open; the short-lived `Move to Trash: waiting for the scan to finish` / `moving to Trash 0 / 1` notes; open another test folder before the held scan ends (nothing moved, the waiting note goes); a deleted file's earlier judgment no longer undoable.
         - Delete box 7 (Step 8); its one difference becomes item C2.
      3. `### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)`.
         - In the Background, replace the sentence saying nobody has used the editor in the running app with a 2026-11-22 summary: the refusals, Escape and an empty name, the field's keys and clipboard, Alt+Tab, the held rename's pending row, tooltip and note, its two cancels and the slow click were seen; the held rename failed once in two tries with `Access denied (os error 5)` (its own item below); two clicks about 480 ms apart opened the field (borderline with the 500 ms double-click time); the IME and a rename during an import run were not run.
         - Delete the boxes that passed in full (refusals; Escape / empty / keys in the field; Alt+Tab; the cancels; the slow click). The held-rename box: delete it (its pending look passed; the failure is item C3). Keep the setup, plain rename, IME, import and record boxes unchanged.
      4. `### App: real-device check of the folder tree's file counts on collapsed rows (...)`.
         - Add the 2026-11-22 observations to the Background (results section "file counts"), and say the prune was seen only as far as the outside rename shows.
         - Reword the Windows box to what remains: an index upgraded from before the cache table; `Refresh` after adding or removing a RAW outside the app (the count follows, and after a restart); a `Rename by Capture Date…` split; a mapped network drive, a symlink or a junction; then mark any remaining Inferred part of the `docs/agents/app.md` rules Measured. Keep the macOS and Linux boxes unchanged.
      5. `### App: real-device check of the folder tree rows' name truncation inside the pane`.
         - Background: the cause stays inferred from the gpui-pre 0.3.12 and taffy 0.15.0 sources; add what was seen on 2026-11-22 (results section "name truncation").
         - Reword to one box with three points left: dragging the vertical scrollbar's thumb scrolls the list; a held rename's long pending name truncates like any other; a click and a right-click on a row's inset edge behave as on the row. The second box becomes: once the three are confirmed, drop the "not yet observed" remainder from the `docs/agents/app.md` folder tree rule; on a failure, record what the layout does in `learnings.md` first.
      6. `### App: real-device check of the folder tree rows drawn as ListItems (native-tree-listitem)`.
         - Background: replace the "the app was never run ... unverified ... automated criteria only" sentences with the 2026-11-22 summary (results section "ListItems", three themes).
         - Reword box 1 to the points left: the open row's semibold (not clearly visible at the default size); a failed folder's red name, also on the open row; a click on an open leaf row does nothing; the outline also shows on the open row; the settings modal's text selection reads gray in Neutral. Keep box 2 unchanged.
    - **C. Add three items to `todo.md`**, each a `###` section with `#### Background` (ending in a `Files:` line) and `#### TODO`:
      - C1, after the reworded gpui-tree-trash item: `### App: decide whether the Sort / Filter menus open with no folder`. Background: launched with no folder on Windows 11 on 2026-11-22, `Sort ▾` and `Filter ▾` both open (Sort with `File name` checked, Filter with all its sections) while the gpui-tree-trash Step 2 check expected them to do nothing; every other key and click tried there did nothing and nothing panicked. Files: `crates/app/src/main.rs`, `crates/app/src/menu.rs`. TODO: decide; if not, keep the buttons from opening and pin it in a test; if so, record the rule in `docs/agents/app.md`.
      - C2, after C1: `### App: an Undo of Move to Trash whose files left the Recycle Bin counts the sidecars and misnames their reason`. Background: the facts of results Step 8's last bullet (`Moved 1 file to the Trash`; then `Restored 0 files from the Trash, 3 failed` with the quoted error lines; the check expected `1 failed`). Files: `crates/engine/src/trash.rs`, `crates/app/src/trash.rs` (`restored_status`, `trashed_status`). TODO: decide whether the restore's note counts as the move's does and apply it; give a sidecar that is itself no longer in the Trash a reason that says so.
      - C3, after the reworded rename item: `### App: a held folder rename intermittently fails with Access denied when it runs as pass 2 ends`. Background: the facts in results ("Fresh index, during the scan (264 files)..." bullet): the note and log lines quoted exactly, the hand rename right after succeeding in 62 ms, the second try (`slow3` to `Later4`) succeeding in 61 ms, one failure in two tries; on NTFS an open handle inside a folder refuses its rename (cite the `docs/agents/app.md` rename rule if it says so); the holder was not traced. Files: `crates/app/src/folderops.rs`, `crates/app/src/idle.rs`, `crates/app/src/scan.rs`, `crates/engine/src/rename.rs`, `crates/engine/src/index.rs`. TODO: reproduce it repeatedly and find the handle still open at pass 2's end; decide between releasing the gate only once it is closed and a brief retry on `ERROR_ACCESS_DENIED`, apply it, repeat the reproduction.
    - **D. Record in `docs/agents/app.md`** (find each section by heading; keep headings and their tags; one short bullet or sentence each, only where a rule exists to attach to):
      - The tree's `row_click` rule: Seen (2026-11-22, Windows 11) for the click rules that were run; not seen: a click on an open leaf row.
      - The "a tree row takes `w_full()`" rule: the cause inferred from sources; Measured (2026-11-22) for the truncation, the whole count, the inset highlights and the scrollbar position; not yet observed: thumb drag, inset-edge clicks, a long pending name.
      - The "a row shows its file count" rule: Measured (2026-11-22) for the listed count, cached counts after a restart, a renamed folder keeping its count, an outside-renamed one and a new folder of an old name showing none, a never-listed folder showing none; not yet observed: drives / links, an old index. Leave the writes and prune rules' Inferred text.
      - The tree keyboard section: a click does not move the cursor, which follows an open (`tree_cursor = tree_open`) and the arrow keys; Enter opens and keeps the keyboard; Right / Left page the strip; the swallowed keys; Escape and `toggleLeft` remove the ring.
      - The trash dialog section: Enter and Space on `Delete Permanently` do nothing, one click deletes; a double-click on the menu item opens the dialog and deletes nothing; a double-click on the dialog's own button deletes on its first click (the refused double-click is only the rest of the one that opened the dialog).
      - The inline rename section: the pending row's look, tooltip and note, the scan not canceled, the two cancels, and the one `Access denied` failure in two tries (pending in `todo.md`); the slow click observations including the ~480 ms borderline; replace its "Not verified headless" bullet with what stays unverified (the IME, a rename during an import run).
    - **E. Write `learnings.md` in this plan folder** with the content of `results.md` (method and results), the incident (a coordinate slip in the synthetic mouse input opened `D:\Photos\tests\2026-09-19-focus-sample-2`; it was only read, no file in it changed), and this step's implementation notes. Then delete `results.md` (its content lives in `learnings.md`), so no plan file links to it; update the link in this plan's Purpose to `learnings.md`.

## Trade-offs and risks

- The gpui-tree-trash heading is renamed, as batch 2 renamed the items it narrowed.
- Stale "Not verified headless" bullets in `docs/agents/app.md` outside list D stay, as batch 2 decided.
- `todo.md` names this plan's `learnings.md` by its archived path, true only after the wrap-up's move.
- Headings are from `main` at `f05d417c`; if one no longer matches, find the item by its text and note it in `learnings.md`.

## Progress

- Step 1 done: the run's results applied to `todo.md` (one item removed, five reworded, three added) and `docs/agents/app.md` (six folder tree, trash dialog and inline rename rules), and moved from `results.md` into `learnings.md`.
- 2026-11-22 Wrap-up: learnings-extractor proposed two `docs/agents/app.md` bullets (the trash gate and the trash undo sections, replacing their stale "Not verified headless" lines), applied; todo-curator found nothing to add or delete; the stale `native-tree-no-chevron` plan path in `todo.md` was pointed at `_archived`; no `.claude/settings.local.json` to promote. Plan archived.
