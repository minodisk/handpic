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

# Record the Windows real-device checks run through MCP (2026-11-27)

## Purpose

A real-device run on Windows 11 on 2026-11-27 exercised seven pending items
of `todo.md` (the file is at the repository root) through the app's MCP
companion and its `screenshot` tool, plus a release-build A/B of pass 1 on
HDR PQ CR3 files. Five items closed whole and two closed in part. This plan
records the run in the repository: `results.md` in this folder, the closed
items deleted from `todo.md`, the partial ones narrowed to what is left, and
the matching sections of `docs/agents/shared-crates.md` and
`docs/agents/app.md` brought up to date. Docs and todo only, no code change:
one step, one PR (single-PR mode). It follows
`../_archived/20261126-macos-device-checks-mcp/plan.md` and
`../_archived/20261123-windows-device-checks-mcp-visual/plan.md`.

The run is already done; nothing is run again. Its results are the session
scratchpad's `results.md`, the source of truth for every edit.

## Steps

- [x] Step 1: Copy `results.md` into this folder and apply it to `todo.md`, `docs/agents/shared-crates.md` and `docs/agents/app.md`
  - Done when:
    - `results.md` in this folder is the scratchpad file's content, with LF line endings.
    - Every edit of lists B to E is applied.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md`, `docs/agents/shared-crates.md` and this plan's files. In `todo.md` only the seven items of list B changed, all of them between the heading `### App: measure pass 1 on a folder of HDR PQ CR3 files at cap 2, before and after the HEVC split` and, not including, `### App: real-device checks of the status row's StatusBar and progress circle`.
    - `CLAUDE.md`, `.claude/skills/develop` and `.claude/agents/todo-curator.md` are unchanged, and no file under `crates/` changed.
    - No two `###` items in `todo.md` are left without a blank line between them, and the count of doubled blank lines in each changed file equals `origin/main`'s. Every changed file has LF line endings.
    - The PR description records what was seen for each removed or narrowed box.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **No run.** The checks ran on 2026-11-27 on a debug build of `f5cab4bf` with `--features screenshot` (and release builds of `39474bdb` and `f5cab4bf` for the A/B). Source: `C:\Users\daisu\AppData\Local\Temp\claude\32479068b97e7400\afd0a0d9-512d-4a36-b48e-2b07cccfe9b4\scratchpad\results.md`.
    - **Editing.**
      - `todo.md` is edited concurrently by other sessions. Rebase on `origin/main` right before editing and again before the PR; touch only the items named in list B. An item reworded on `main` since planning (`f5cab4bf`) is re-read, not overwritten.
      - Locate every item by its exact `###` line, never by line number. "Box N" counts the item's top-level `- [ ] ` lines from 1.
      - Keep every other heading, item, box and sentence byte-for-byte; no item heading is renamed. Left untouched inside the range: `### Core: the cold-disk cost of pass 2's full-size eye read is unmeasured`, `### App: a JPEG folder's strip cells in view stay gray until pass 1 ends`, `### App: measure the scan pause in \`load\` on the SATA SSD with presses on files pass 1 has not read yet`, `### App: \`present\` p95 rose from about 0.2 ms to 0.55-0.85 ms on ARW with the strip`, `### App: real-device checks of the strip's sort order, the filter and the filter / sort menus`, `### App: real-device checks of the EXIF filter sections and the refilter fixes`, `### App: \`Camera\` / \`Lens\` order is a case-insensitive compare, not the Tauri \`localeCompare\``, `### App: real-device check of the Japanese IME in the settings' label name fields`, `### App: real-device check of the restored window on a second monitor (gpui-session-restore)`.
      - `todo.md` paragraphs and boxes are single long lines; wrap text added to `docs/agents/*.md` at the file's width, as its neighbours.
      - `RESULTS` below stands for `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`, always written in backticks, not as a link (the folder is archived by this PR's wrap-up; a link would not resolve before that).
      - Each background sentence of list B starts `Seen on Windows 11 on 2026-11-27 (results in \`RESULTS\`):`.
      - `mise run fmt` is `cargo fmt` only and the repository has no Markdown formatter, so the Markdown is checked by hand (LF, blank lines, table alignment as the neighbouring tables) and by lychee in `mise run ci:local`.
      - Suggested PR title: `docs: record the 2026-11-27 Windows device checks through MCP`.
    - **A. `results.md`**: copy the scratchpad file as it is (it is already LF and UTF-8); change nothing but line endings if the copy introduces CRLF. It refers to other plans by backticked names only, so it carries no link to break.
    - **B. `todo.md`: five items removed whole, two narrowed.** Removing an item takes its heading, background, `#### TODO` and boxes, leaving one blank line between its neighbours.
      - Remove whole: `### App: measure pass 1 on a folder of HDR PQ CR3 files at cap 2, before and after the HEVC split`.
      - Remove whole: `### App: real-device checks of the strip, the scan progress, the resume and the resync`.
      - Remove whole: `### App: private bytes grow while \`→\` is held in the 1:1 view`.
      - Remove whole: `### App: real-device checks of the f focus mark, face boxes and face mesh`.
      - Remove whole: `### App: real-device check of \`open_folder\` refused during a rename over MCP`.
      - Narrow `### App: by-eye checks of the scaled fit-view decode (fit-preview-downscale)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261121-fit-preview-downscale/plan.md\``: on the RAF compare folder, a debug build, through MCP `set_overlays` and `screenshot`: the grayscale latch the `g` key's hold shares drew the photo gray in the fit view and in the 1:1 crop with the strip thumbnails and the peaking overlay in color; the peaking layer showed in the fit view and at 1:1, and after `set_panes` hid both sides and the strip (the canvas growing from about 1150x765 to 1400x935 logical px) it was redrawn at the new size, its lines thin and on the edges. The boxes below were not run: they compare against a build from before `d3794643` or need captures during a resize.
        - Delete box 3 (`e`, the peaking layer) and box 4 (hold `g`). Boxes 1, 2, 5 and 6 stay as they are.
      - Narrow `### App: real-device checks of the folder tree and Move to Trash… left open (gpui-tree-trash)`:
        - Background sentence, inserted immediately before ` Windows only. Plan: \`docs/plans/_archived/20261113-gpui-tree-trash/plan.md\``:
          - Scan stop and quit: an `open_folder` (the tree click's `open_from_tree`) at `analyzing 45/400` stopped the first folder's pass 2 and reopening it resumed the 354 files left; a launch with no folder quit on the window's close.
          - Held run: a `trash_photos` confirmed during pass 1 was held with `Move to Trash: waiting for the scan to finish`, the scan was canceled and the file moved; with another folder opened before the scan ended the run was dropped (`dropped: another folder was opened`), the note cleared and the file stayed.
          - Undo and redo: a 3-star file restored by `undo` came back with its `.xmp` and its three stars in the strip; a run made in `a` and undone from `b` left `a` listing the file when reopened; a `redo` reported `moving to Trash 0 / 1`, then `Moved 1 file to the Trash`.
          - Not run: the dialog (an MCP-confirmed run opens none), the delete's judgment-undo and the Ctrl-click.
        - Delete box 1 (Step 2).
        - Box 5 (Step 7) becomes: `- [ ] On Windows, the rest of Move to Trash… and Delete Permanently… (Step 7). With the dialog open, clicks on the tree, strip and status menus do nothing. On \`...\trash-large\` with a fresh \`--index\`, during pass 1 pick \`Move to Trash…\` and confirm: the dialog closes at once and \`moving to Trash 0 / 1\` shows briefly once the held run starts. Judge a file, then \`Delete Permanently…\` it: the earlier judgment is no longer undoable. (The waiting note, the held run and the drop by a folder open were seen through MCP on 2026-11-27, with no dialog.)`
        - Box 6 (Steps 5 and 8) becomes: `- [ ] On Windows, the rest of the strip's selection (Step 5). With several files selected, Ctrl-click a middle one: it drops out and the focus stays on the current file. (The restore with its stars, the cross-folder undo and the redo's progress were seen through MCP on 2026-11-27.)`
        - Boxes 2, 3 and 4 (Steps 3, 4 and 6) stay as they are.
    - **C. `docs/agents/shared-crates.md`**, section `### The scan's reads are capped and yield to the preview's at the disk (Measured)`, two bullets, nothing else:
      - The bullet starting `- On NVMe the numbers do not confirm cap 2`: replace its last sentence `HDR PQ CR3 is untested.` with a pointer to the next bullet (for example `HDR PQ CR3 at cap 2 is measured in the next bullet, not on NVMe.`). The run did not compare cap 22 with cap 2, so no table goes here.
      - The bullet starting `- The cap does not cover \`hevc::to_jpeg\``: replace `Before the split such a folder paid a calculated 33-63 ms/file at a cap of 2 against 3-6 uncapped; pass 1's ms/file on such a folder at cap 2, before and after, is not measured yet (\`todo.md\`).` with the calculated figure kept, then the measurement: the method and the six-row table of `results.md`'s HDR PQ section (Build, CPU before, Pass 1 (300 files), ms/file), indented under the bullet as the section's other tables are. The method to carry, briefly:
        - Measured on 2026-11-27 with the app itself: release builds of `39474bdb` (before the split) and `f5cab4bf` (after), `READ_SLOTS` 2, 22 scan threads.
        - The folder: 300 files, 60 hardlinks each of the five sample CR3 that `handpic-cli info` reports with an HEVC preview and full image, on `D:`, so every read after the first run is a cache hit and the figures show the decode held inside the slot, not the disk.
        - Each run a fresh `--index`, the time from the `scan extract:` line over its files, the builds alternated three runs each, the total CPU load of other sessions sampled before each launch.
        - The reading: 94-102 ms/file before against 13-18 after, about 7x; the before figure is above the calculated 33-63 and the gap was not looked into.
        - The rest of the bullet (`The app's preview \`load\` also decodes after its read…`) stays.
      - Source line: `RESULTS`.
    - **D. `docs/agents/app.md`**: one short `Seen (2026-11-27, Windows 11, through MCP)` or `Measured on Windows 11 on 2026-11-27` bullet per behaviour, in the section's own style, each ending with `RESULTS`. No section heading's `(Measured)` / `(Inferred)` tag changes.
      - `### An on-demand judgment is superseded through a shared atomic (Measured)`:
        - Correct the sentence `a newer request stores its id and a clear (another folder, a re-listing) stores \`0\`, which no ticket has`: `EyesCache::request` issues no ticket while a judgment is in flight, so while one runs only a `clear` supersedes it; a newer request's id only follows a settle or a clear.
        - Add what was seen: with the `f` overlay on, 60 `navigate next` about 8 ms apart logged three `eyes` lines (the file the burst started on, the file current when the first judgment settled after 325 ms, about 40 pages in, and the last), and 40 about 35 ms apart logged four; the files passed between are never judged, about one judgment per 0.3-0.5 s.
        - The existing "60 presses 30 ms apart … logged 7 judgments" sentence stays.
      - `### A held key draws each stale crop before chasing the current one (Measured)`: add the memory plateau. Zoomed on 400 ARW, six series of 120 `navigate next` at about 37 ms and one of 400 `previous`, 1120 zoomed pages: `PrivateMemorySize64` went 242 MB before, 203-210 MB after each short series and 221 MB after the long one, no climb; a debug build. The 418 to 557 MB growth of the `gpui-zoom-peaking` smoke run did not repeat.
      - `### The hot list is set before the scan's workers start (Measured)`: add the steered thumbnails, on fresh indexes over 1520 and 1545 ARW hardlinks, pass 1 at about 120-150 files/s:
        - `navigate last` at `scanning 1/1545`: the ten cells in view gray at 0.5 s, nine of ten filled at 1.2 s (`scanning 124/1545`), all at 2.9 s.
        - A `show_photo` jump to the middle at `scanning 455/1520`: its cells filled at 1.0 s with pass 1 at 650/1520.
        - On an open, seven of the ten cells in view filled at 0.9 s (`scanning 59/1520`) and all at 1.4 s, not strictly from the current cell outward, since parallel reads finish out of order.
      - `### During pass 1 the preview reads what the scan already read (Measured)`, the rule that hardlinks of files an earlier run read end pass 1 before the window takes keys: only if it stays one clause, add that on 2026-11-27 hardlinks of 1500-file folders not opened that session, driven over MCP from the first answer, left pass 1 long enough (about 10 s) to see the placeholder phase and the steered scan. Skip this edit if it cannot be said that briefly; the cache state was not measured.
      - `### A trash or delete goes through the gate, and nothing lists under it (Inferred)`:
        - In the 2026-11-22 bullet, narrow `Not observed: opening another folder before the held run's scan ends, and an earlier judgment no longer being undoable after a delete (pending in \`todo.md\`).` to the delete half only.
        - Add: a `trash_photos` confirmed during pass 1 reported `held_for_scan` with `Move to Trash: waiting for the scan to finish`, the scan was canceled and the file moved; with another folder opened before the scan ended the operation became `failed` with `dropped: another folder was opened`, the note cleared and the file stayed.
      - `### A trash run is undone through the same gate, and outlives a folder switch (Inferred)`: add, in one bullet: a restored 3-star file's cell showed its stars; after a cross-folder undo, reopening the run's folder listed the file; a `redo` reported `moving to Trash 0 / 1` (operation `Redo Move to Trash`, running 0/1), then `Moved 1 file to the Trash`.
      - `### The tree's inline rename is a text input in the row, held by the gate (Inferred)`: add, in one bullet: while a rename was in flight (2148 index rows carried, 796 ms), eight `open_folder` calls on the folder 1-16 ms after the confirm answered all returned `A folder is being renamed; try again in a moment.`; a rename of a folder with no index rows ran in 10 ms, before any open arrived. (Opens sent concurrently with the confirm reached the view first, so the folder opened and the confirmed rename was held behind its scan; this is consistent with the section's rule and needs no rewording.)
      - `### The MCP companion answers from the view, one channel hop away (Measured)`, the paragraph on `open_folder` and `rename::RENAMING`: no rewording; at most a pointer to the rename section's new bullet if the section already lists what was seen per tool.
    - **E. `learnings.md`**: this step's notes.
      - Deferred, not edited here: `CLAUDE.md`'s `src/eyes.rs` description says "so a newer request or a clear supersedes the running judgment", and the doc comments of `EyesTicket` and `EyesCache::current` in `crates/app/src/eyes.rs` say the same; while a judgment runs no newer ticket is issued, so only a `clear` supersedes it. A later change that touches either should correct the wording.
      - Left as it is: the `Camera` / `Lens` item's box in `todo.md` still names the reference Tauri build "under" the removed `f` overlay item; the box also names the build inline, and the item belongs to another session.
      - What MCP plus `screenshot` closed on Windows and what it cannot: `present` p95 (only key presses print `page` lines), a dialog's own behaviour (an MCP-confirmed run opens none), the menus' clicks, a Ctrl-click, the IME; a second monitor and an idle `D:` were not at hand.
      - The setup worth keeping: per-run `--index` / `--settings` / `--log` / `--mcp-port`, `mcpEnabled` and `mcpAllowDestructive` through `--import-settings`, one MCP session for a series of `navigate` calls to page faster than key repeat, windows closed with WM_CLOSE, and hardlinks of a date folder not opened that session as a stand-in for `robocopy /J` copies.
      - To reach the rename refusal: the folder needs index rows (2148 gave 796 ms); with none the rename ends in 10 ms.
      - `mise run fmt` formats no Markdown.

## Trade-offs and risks

- **Stand-ins for a key or the wheel.** The grayscale box asked for a held `g`; the run set the `set_overlays` latch the key's hold shares. The strip's `End` box asked for a wheel to the middle; a `show_photo` jump stood in. The `f` overlay's hold box asked for a held `→`; `navigate` calls at and above key-repeat pace stood in. Closed on that basis (decided by the caller, following the 2026-11-26 precedent of MCP-driven key checks).
- **The cold-file condition of the strip boxes was not met as written.** The boxes asked for `robocopy /J` copies; the run used hardlinks of folders no run had opened that session, cache state unmeasured, pass 1 still climbing for about 10 s. The item is removed whole.
- **The HDR PQ figures are cache-hit figures under other sessions' CPU load** (37-63 % before each launch), and the before figure (94-102 ms/file) is above the calculated 33-63 with the gap unexplained. They are recorded with that caveat.
- **`CLAUDE.md` stays stale on the eyes supersede rule** (see list E); recorded in `learnings.md` only.
- **A dangling reference outside the range.** The `Camera` / `Lens` box names the removed `f` overlay item by heading; left untouched because another session owns the item.
- `todo.md` is edited by other sessions; the edits are confined to the range and re-read after each rebase.

## Progress
