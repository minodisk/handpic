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

# Record the fifth batch of Windows real-device checks (2026-11-22, folder operation dialogs)

## Purpose

A fifth real-device run on Windows 11 on 2026-11-22 exercised the pending
manual checks of the folder operation dialogs: the folder menu, `Rewrite
Sidecars from Index…`, `Delete Sidecars…`, `Sequence JPEG Timestamps…`,
`Import…`, `Rename by Capture Date…` and the OS notification on an import's
end. This plan removes the `todo.md` boxes the run settled, narrows the rest
to what stays open, adds the two findings the run raised as new items, and
records what was seen in `docs/agents/app.md` where a rule covers it. Docs and
todo only, no code change: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-4/plan.md`. The branch
was cut from `origin/main` at `14423cda` (the run used a build from before
`#1223`, which reverted only the keyed page's immediate paint and does not
touch the dialogs).

The run's results are in `learnings.md` in this folder (moved there by Step 1
from the `results.md` the run wrote). They are the source of truth: this plan
names their sections and does not restate them.

## Rule for sub-points the run did not exercise

A sub-point of a box this plan deletes or narrows that `results.md` does not
mention, or lists as not run or too fast to see, was **not** run and must stay
open: carry it into the reworded box (or keep the box). Known cases (check for
others while editing):

- Step 3 (sidecars): the dialog opening once the scan ends (the held Rewrite
  ended in an error, the held Delete was dropped by another open); Tab moving
  back from `Cancel` to `Rewrite`; the dimmed buttons and the running note
  during the run; the strip's marks going on a delete and returning on its
  undo; `Ctrl+,` while the preview is under way.
- Step 4 (Sequence): Tab and Escape on the dialog before a run; the pane's
  `sequencing n / m` text (only the MCP operation's `running 20/300` was
  read); a second request right after the first; `Ctrl+,` while the dialog or
  run is up.
- Step 5 (Import): the pane's `reading …` note; Tab and Escape on the dialog;
  the dialog stepping aside and the pane's `importing n / m` while the strip
  fills; the whole mid-run box (reopen, Escape, `Cancel`, no `.handpic-tmp`
  left).
- Step 6 (Rename by Capture Date): Tab and Escape on the dialog; the buttons
  dimming during the run; the strip's judgments kept after the rename's
  reopen; `Cancel` focused in the collide dialog.
- Notification: the click activation, the `Import canceled` title, no toast
  while the window is active, the taskbar icon and grouping, the two log
  warnings.

## Steps

- [x] Step 1: Apply the run's results to `todo.md` and `docs/agents/app.md`, and write `learnings.md`
  - Done when:
    - Every edit in the lists A to E below is applied (with the rule above).
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files, and no `todo.md` item outside the lists changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where a box went.
    - The PR description records what was seen for each removed box, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Locate every item by its `###` heading, never by line number. "Box N" counts the item's top-level `- [ ]` lines from 1. Keep every heading, the neighbouring items, and every sentence this plan does not name, byte-for-byte. A box with nothing left open is deleted; no item below is deleted whole.
    - Write in English, in the file's own style. Every "seen" sentence added to a Background says "on Windows 11 on 2026-11-22" and names the results as `docs/plans/_archived/20261122-windows-device-checks-5/learnings.md` (in backticks, no Markdown link to a plan folder). Each of the four gpui-folder-dialogs Backgrounds also says the run's material was under `D:\Photos\tests\20261122-folderops\`, not the `20261113-gpui-folder-dialogs` paths its setup box names, and ends its summary with "What is left open is in the boxes below."
    - Setup and record boxes: every one of the four gpui-folder-dialogs items keeps a check box, so each keeps its setup box (box 1) and its final `Record the results in the archived learnings.md` box unchanged.
    - As in the earlier batches, a Python script written to the scratchpad and run with `python -I`, asserting one match per `###` anchor, keeps the neighbours byte-for-byte.
    - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the folder operation dialogs`.
    - **A. Narrow in `todo.md`** (each Background gets a short 2026-11-22 summary of its `results.md` section):
      1. `### App: real-device checks of Rewrite Sidecars from Index… and Delete Sidecars… and the folder menu (gpui-folder-dialogs Step 3)`. Delete boxes 2 (menu order) and 6 (`jpeg-only`). Box 3 keeps only the dialog opening once the scan ends. Box 4 keeps only Tab moving back to `Rewrite` and the dimmed buttons and running note during the run. Box 5 keeps only the strip's marks going and returning. Box 7 keeps only `Ctrl+,` while the preview is under way. Background also says: under `sidecarFormat: both` the question reads `Rewrite the XMP and .dop sidecars of …`; a redo pressed while the roughly 3 s restore still ran took effect after it; the held Rewrite on a folder with nothing judged ended in the `No judgments indexed` message (item C2).
      2. `### App: Rewrite Sidecars from Index reported "4 files not in the index (skipped)" in a fully scanned folder`. Box unchanged, item stays open. Background adds: it did not reproduce on Windows 11 on 2026-11-22 (a fully scanned 12-file folder listed only the `with a judgment` / `without one` rows).
      3. `### App: real-device checks of Sequence JPEG Timestamps… (gpui-folder-dialogs Step 4)`. Delete boxes 4 (rebuilt) and 5 (cancel; the box reads "`Cancel` (or Escape)" and Escape ended a run). Box 2 keeps only Tab and Escape. Box 3 keeps only the pane counting `sequencing n / m`. Box 6 keeps only the second request and `Ctrl+,`. Background also names the list's blank band (item C1).
      4. `### App: real-device checks of Import… (gpui-folder-dialogs Step 5)`. Delete boxes 5 (`Nothing left to copy`, another size) and 6 (no root, the log line). Box 2 keeps only the `reading …` note and Tab / Escape. Box 3 keeps only the dialog stepping aside and the pane's `importing n / m` while the strip fills. Box 4 unchanged. Background also says: the source held no JPEGs of the two dates and no `.dop`; a 3.16 GB source copied from cache in a few seconds, too fast for the mid-run box; MCP `import_folder` is refused during a run; the `Source:` line and the status line's `no RAW files in` text showed the verbatim prefix (their two existing items, named, not edited); the list clipped its last row (item C1).
      5. `### App: real-device checks of Rename by Capture Date… and the folder-rename aftermath (gpui-folder-dialogs Step 6)`. Delete boxes 2 (during a scan; driven through MCP `rename_by_capture_date`, the same `folder_op` path), 5 (`three-dates`) and 6 (`two-dates`, already named). Box 3 keeps only Tab / Escape, the buttons dimming and the judgments kept after the reopen. Box 4 keeps only `Cancel` focused. Background also says: the renamed row re-sorted among its siblings rather than staying at its position, and the list clipped its last row (item C1).
      6. `### App: real-device check of the OS notification when an import ends while the window is inactive (native-import-notification)`. Box 1 keeps everything but the delivery of the `Import finished` toast (title and body). Box 2 unchanged. Background: the toast was delivered at the desktop's bottom-right with the header `Handpic`, the title `Import finished` and the status line's summary as body; its app icon is a generic one, not Handpic's (as `docs/agents/app.md` expects: no icon is registered).
    - **B. Checked, not covered by an existing item:** no `todo.md` item names the dialogs' blank band or clipped last row, or the `No judgments indexed` wording. The nearest is the "A long list" sub-point of the Step 7 box of the gpui-component adoption item, which checks that a long list scrolls; C1 names it, and that item is not edited.
    - **C. Add two items to `todo.md`**, each with `#### Background` (ending in a `Files:` line) and `#### TODO`:
      - C1, right after the Step 6 item: `### App: the folder dialogs' file list leaves a blank band above its first row and clips its last row`. Background: seen on Windows 11 on 2026-11-22; the Sequence dialog's list starts below a blank band (about one row on 10 files, about 360 px on 300 files, the scrollbar thumb mid-track while the first rows show); the same list in the Import and Rename by Capture Date dialogs shows a blank row above and cuts its last row mid-line until scrolled. The rows and lists are the `folder_dialog` body, which scrolls under the component `Dialog`'s scrollbar; the cause is not known. Files: `crates/app/src/folderops.rs` (`folder_dialog`), `crates/app/src/modal.rs`. TODO: find the cause and make the list start at its first row with its last row whole.
      - C2, right after the "4 files not in the index" item: ``### Engine: a held Rewrite Sidecars on an open folder with nothing judged says `open the folder first` ``. Background: seen on Windows 11 on 2026-11-22; at the scan's end the held request on the open folder `scan1` (no sidecars, nothing judged) answered `No judgments indexed for scan1; open the folder first`. `targets` returns it whenever the index has no row for the folder's RAW files, open or not. Files: `crates/engine/src/foldersidecars.rs` (`targets` and its test). TODO: reword the message so it does not tell the user to open an open folder.
    - **D. Record in `docs/agents/app.md`** (find each section by heading; keep headings and tags, the `(Inferred)` ones too; short `Seen (2026-11-22, Windows 11)` bullets naming `../plans/_archived/20261122-windows-device-checks-5/learnings.md` in backticks):
      - "The folder operations share one dialog model and live in `folderops.rs`": (a) during a scan a Rewrite and a Delete request each showed its waiting note with the scan not canceled, and another open dropped the waiting Delete with no dialog; with a dialog open a click on the tree or strip, `→`, `3` and `ctrl+,` did nothing; (b) a Delete Sidecars run's `Moved 24 sidecars to the Trash`, its undo's `Restored 24 sidecars from the Trash` and its redo; (c) a 300-file Sequence run dimmed `Run`, focused `Cancel`, and Escape ended it with `canceled, 60 of 300 written` and no Explorer window; (d) an import opened its first date folder during the run, and a reopen with nothing to copy had `Run` dimmed and `Cancel` focused. Narrow its "Not verified headless" bullet to what stays open (the rule's known cases above), keeping its two paths.
      - "A folder rename releases the watches, then follows in the tree and reopens": the `SCAN_RUNNING` refusal during a scan; a one-date rename with the row under its new name, the strip's files and current file kept, `todo=0` in the reconcile and `Ctrl+Z` restoring a trashed RAW into the renamed folder; a split keeping the folder with its undated file, and one removing it and opening the first date folder; an existing sibling of the date's name disabling `Run`. Narrow its "Not verified headless" bullet to the judgments kept after the reopen.
      - "An import's end posts a system-only notification while inactive": the toast's delivery, header, title, body and generic icon. The `(Inferred)` marker and the "hand check … is pending" sentence stay: the todo item's box 2 asks for the replacement only once all of box 1 is confirmed.
    - **E. Write `learnings.md`** with the content of `results.md` (setup, incidents, results) and this step's notes, then delete `results.md` and point this Purpose at `learnings.md`.

## Trade-offs and risks

- Step 6 box 2 is deleted although the refusal was driven through MCP, not the tree menu: both reach the same `folder_op` path.
- Step 4 box 5 is deleted on Escape alone: the box reads "`Cancel` (or Escape)".
- Step 4 box 3 keeps the pane's `sequencing n / m` and Step 5 box 3 the stepping aside and `importing n / m`: the results report the MCP operation's count, not the pane's text.
- The notification section keeps `(Inferred)`: only the delivery was seen.
- C2 is filed under `Engine:` because the text is in `crates/engine`; kept separate from the "4 files not in the index" item, which did not reproduce.
- The setup boxes keep the `20261113-gpui-folder-dialogs` paths; only the Backgrounds name the `20261122-folderops` material.
- Stale "Not verified" bullets and un-archived plan paths in `docs/agents/app.md` outside list D stay, as the earlier batches decided.
- `todo.md` names this plan's `learnings.md` by its archived path, true only after the wrap-up's move.

## Progress

- Step 1: applied the run's results to `todo.md` (lists A to C: boxes removed or narrowed in six items, the two new items C1 and C2) and `docs/agents/app.md` (list D), moved `results.md` into `learnings.md` and deleted it.
