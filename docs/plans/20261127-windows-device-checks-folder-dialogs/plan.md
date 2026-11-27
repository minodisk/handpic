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

# Record the Windows real-device checks of the status row, the folder drop and the folder dialogs (2026-11-27)

## Purpose

An agent session ran pending Windows checks of `todo.md` (the file is at the
repository root) on the Windows 11 host on 2026-11-27, through the MCP
companion, the `screenshot` tool and posted keys and clicks. It covered the
items from "App: real-device checks of the status row's StatusBar and progress
circle" up to, not including, "App: real-device checks of the tree's inline
folder rename (gpui-folder-dialogs Step 7)". The status row's canvas check and
the Step 3, 4 and 6 dialog checks passed whole, the Import checks passed but
for one finding (the strip does not fill during a run), and the folder drop and
the HiDPI icon check could not be run.

This plan records the run in the repository: `results.md` in this folder, the
passed boxes deleted from `todo.md`, the rest narrowed to what is left, and
`docs/agents/app.md` brought up to date. Docs and todo only, no code change,
no finding fixed: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261123-windows-device-checks-mcp-visual/plan.md`
(#1262) and `docs/plans/_archived/20261126-macos-device-checks-mcp/plan.md`
(#1307).

The run is already done; nothing is run again. Its results file is the source
of truth for every edit.

## Steps

- [x] Step 1: Copy the run's `results.md` into this folder and apply it to `todo.md` and `docs/agents/app.md`
  - Done when:
    - `results.md` in this folder is byte-identical to the session's results file (`cmp` reports no difference).
    - Every edit of lists B and C is applied, and `learnings.md` (list D) is written.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files.
    - In `todo.md`, every changed line lies between the heading `### App: real-device checks of the status row's StatusBar and progress circle` and the heading `### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)`; that second heading and everything after it, and everything before the first, are byte-for-byte `origin/main`'s.
    - In `docs/agents/app.md`, only the bullets of list C changed; the paragraph starting `` `wait_for_idle` after a confirmed import can return idle `` and the "Not verified in the running app" bullet of the folder drop section are unchanged.
    - No two `###` items in `todo.md` are left without exactly one blank line between them, the count of doubled blank lines in each changed file equals `origin/main`'s, and every changed file has LF line endings.
    - Every path to this run's results in `todo.md` and `docs/agents/app.md` is the archived one, in backticks.
    - The PR description records what was seen for each removed or narrowed box.
    - The PR's GitHub checks pass. `mise run ci:local` is not run for this docs-only change (see "Trade-offs and risks").
  - Implementation approach:
    - **No run.** The checks ran on 2026-11-27 at `f5cab4bf` (main, 2.2.0).
    - **Editing.**
      - `todo.md` is edited concurrently by other sessions. Rebase on `origin/main` right before editing and again before the PR; touch only the items named in list B.
      - Locate every item by its exact `###` line, never by line number. "Box N" counts the item's top-level `- [ ] ` lines from 1.
      - Keep every other heading, item, box and sentence byte-for-byte; no item heading is renamed.
      - `todo.md` paragraphs and boxes are single long lines; wrap text added to `docs/agents/app.md` at the file's width, as its neighbours.
      - `RESULTS` below stands for the archived path of this run's results, `_archived/20261127-windows-device-checks-folder-dialogs/results.md`: written `docs/plans/_archived/…` in `todo.md` and `../plans/_archived/…` in `docs/agents/app.md`, always in backticks as their neighbours are, never as a Markdown link (the folder is archived by this PR's wrap-up).
      - Suggested PR title: `docs: record the 2026-11-27 Windows real-device checks of the folder dialogs`.
    - **A. `results.md`**: copy `C:\Users\daisu\AppData\Local\Temp\claude\1c154d218f6a064f\4ca62dcf-c2c6-455e-bb0f-40709cf48790\scratchpad\results.md` into this folder unchanged (142 lines, LF, no Markdown link in it, checked while planning). Do not reword it.
    - **B. `todo.md`: three items removed whole, four edited, one untouched.**
      - `### App: real-device checks of the status row's StatusBar and progress circle`:
        - In the background, replace the sentence `What is left open is the canvas in the other pane layouts, below.` with: `On Windows 11 on 2026-11-27 (results in \`RESULTS\`) the canvas was recorded in the three other layouts, 1379 x 767 with the right pane hidden, 1600 x 767 with both side panes hidden and 1158 x 936 with the strip alone hidden, and did not resize when the circle appeared or went in any of them. What is left open is macOS and Linux, below.`
        - Delete box 1 (the Windows pane layouts). Box 2 (macOS and Linux) stays.
      - `### App: real-device checks of the burst bands, badges, burst keys and reject the rest`: no change.
      - `### App: real-device check of the folder drop (gpui-folder-drop)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261114-gpui-folder-drop/plan.md\``: `On Windows 11 on 2026-11-27 (results in \`RESULTS\`) a synthetic OLE drag started by an agent session did not reach the window (a \`DoDragDrop\` with the agent's own \`IDropSource\` failed with \`E_FAIL\`, and a WinForms \`DoDragDrop\` from an injected left press drew no outline and returned \`None\`), so the boxes below need a hand drag from Explorer.`
        - All six boxes stay as they are.
      - `### App: real-device checks of the filter menu's icons and headings (gpui-filter-menu-icons)`:
        - Append to its one box, after its last sentence: ` (Not run on 2026-11-27: the Windows host has one display at 100 %.)`
      - `### App: real-device checks of Rewrite Sidecars from Index… and Delete Sidecars… and the folder menu (gpui-folder-dialogs Step 3)`: all five boxes are closed (the setup with the run's own copies, Tab both ways, the dimmed buttons and the running note, `Ctrl+Z` after a Delete restoring the marks, `Ctrl+,` during a preview, the record). Remove the whole item (heading, `#### Background`, its paragraph, `#### TODO`, the boxes), leaving one blank line between its neighbours.
      - `### App: real-device checks of Sequence JPEG Timestamps… (gpui-folder-dialogs Step 4)`: all four boxes are closed. Remove the whole item the same way.
      - `### App: real-device checks of Import… (gpui-folder-dialogs Step 5)`:
        - In the background, replace the sentence `What is left open is in the boxes below.` with:
          - `On Windows 11 on 2026-11-27 (results in \`RESULTS\`), on a source with JPEGs of the two dates and a \`.dop\`: the dialog's Tab from \`Run\` to \`Cancel\` and back and its Escape; the \`reading …\` note of a 500-JPEG preview; a 375-file, 6.31 GB run not in the file cache, with the dialog stepping aside, the first date folder opening and the pane counting \`importing n / 375\`; \`Import…\` picked mid-run bringing the dialog back with \`Run\` dimmed and \`Cancel\` focused, Escape hiding it with the run going on; and \`Cancel\` stopping a second run with \`Import canceled, 78 files copied\` and no \`.handpic-tmp\` file left were seen.`
          - `The strip did not fill during the run: the open date folder read \`This folder has no RAW or JPEG files.\` until that folder's copies had all landed, because the open folder's watcher reports a burst \`DEBOUNCE\` (500 ms) after its last event and a copy that never pauses that long never settles; the fix, a maximum wait on the debounce, is delegated to the session \`dlg-c4-import-watch\`. What is left open is the box below.`
        - In the same paragraph's `Files:` list, append `, \`crates/engine/src/watch.rs\``.
        - Delete box 1 (setup), box 2 (the `reading …` note, Tab, Escape), box 4 (mid-run reopen and Cancel) and box 5 (record).
        - Box 3 becomes: `- [ ] On Windows, once the open folder's watcher reports during a burst that never pauses (a maximum wait on \`DEBOUNCE\` in \`crates/engine/src/watch.rs\`, delegated to the session \`dlg-c4-import-watch\`), \`Run\` an import on a source not in the file cache (or on a slow disk). Expect the strip of the opened date folder to fill while the pane counts \`importing n / m\`. (On 2026-11-27 the dialog stepped aside and the pane counted, but the strip stayed empty until that date folder's copies ended.)`
      - `### App: real-device checks of Rename by Capture Date… and the folder-rename aftermath (gpui-folder-dialogs Step 6)`: all four boxes are closed. Remove the whole item the same way, leaving one blank line before the Step 7 heading, which is not touched.
    - **C. `docs/agents/app.md`: bullets only; no section heading's tag changes.**
      - C1. Section `### The folder operations share one dialog model and live in \`folderops.rs\` (Inferred)`: insert a new bullet after the last `Seen (2026-11-22, Windows 11)` bullet (the one starting `an import opened its first date folder`) and before the `Not verified headless:` bullet. It starts `- Seen (2026-11-27, Windows 11, keys and clicks posted to the window, the state read through the MCP companion and \`screenshot\`):`, ends `(results in \`RESULTS\`).` and carries, from `results.md`:
        - Tab moved the outline from the run button to `Cancel` and back on the Rewrite, Sequence, Import and Rename by Capture Date dialogs, and Escape closed each.
        - A 125-file Rewrite and a date rename dimmed both buttons under their running note.
        - A Delete Sidecars run took the strip's stars, flag dots and labels and `Ctrl+Z` brought them back.
        - A colliding date rename had `Run` disabled and `Cancel` focused.
        - `ctrl+,` did nothing while an import preview read, with a Sequence dialog up and during a Sequence run.
        - A second `Sequence JPEG Timestamps…` during the preview did nothing.
        - The pane showed `reading …`, `sequencing n / m` and `importing n / m`.
        - A running import's dialog stepped aside, came back on `Import…` with `Run` dimmed and `Cancel` focused, Escape hid it with the run going on, and `Cancel` stopped a run with no `.handpic-tmp` file left.
        - The strip of the open date folder stayed empty until that folder's copies ended.
      - C2. Same section, the `- Not verified headless:` bullet: remove every item C1 now records. What stays: `a held sidecar request's dialog opening once the scan ends`, plus one new item, `the strip filling while an import into the open folder runs (it did not on 2026-11-27, above)`. Keep the bullet's closing sentence (`The hand checks are pending in …`) with its two paths exactly as they are.
      - C3. Section `### One progress circle sits in the status row's right region (Inferred)`, the first `- Rule:` bullet (the one ending `Verified on Windows 11 on 2026-11-23: the position \`1 / 8\` stayed at the same x …results.md\`).`): append `Measured in the other layouts on 2026-11-27: 1379 x 767 with the right pane hidden, 1600 x 767 with both side panes hidden and 1158 x 936 with the strip alone hidden, each the same with the circle shown and gone (\`RESULTS\`).`
      - C4. Section `### A folder rename releases the watches, then follows in the tree and reopens (Measured)`: replace the bullet `- Not verified headless: the reopen keeping the strip's judgments; the hand check is pending in \`../plans/20261113-gpui-folder-dialogs/learnings.md\`.` with `- Seen (2026-11-27, Windows 11): a one-date rename run from its dialog reopened the folder under its new name with the strip's two stars kept (results in \`RESULTS\`).` This bullet is outside the request's literal list; see "Trade-offs and risks".
      - Do not touch: the paragraph starting `` `wait_for_idle` after a confirmed import can return idle `` (the delegated watcher fix will), the "Not verified in the running app" bullet of the folder drop section (still not verified), and the older `Measured with the header (2026-11-22 …)` canvas bullet of the menu section.
    - **D. `learnings.md`**: this step's notes.
      - What posted input closed on Windows that MCP alone could not: Tab, Escape, Enter, `Ctrl+Z` and `Ctrl+,` as `WM_KEYDOWN` / `WM_KEYUP` with the modifier state set through `AttachThreadInput` + `SetKeyboardState`, and right and left clicks at client coordinates. What it cannot: an OLE drag (both `DoDragDrop` attempts failed; one left the left button down until released by hand).
      - The setup worth keeping: copies and hardlinks under `D:\handpic-c4-checks\`, outside the photo folders; a fresh `--index`, `--settings` and `--log` per launch with `--mcp-port` and `--import-settings`; `mcpAllowDestructive` off so every folder operation opens its dialog.
      - Timing: a short preview ends before a posted key or a capture arrives (the Delete preview of 125 files, the 9-file import preview), so the `ctrl+,` and `reading …` checks were taken on a 500-JPEG import preview.
      - The finding: the open folder's watcher never settles during an unbroken copy; delegated to the session `dlg-c4-import-watch`.
      - The checks' material was not under the `D:\Photos\tests\20261113-gpui-folder-dialogs\` paths the deleted setup boxes named.

## Trade-offs and risks

- **Two boxes are closed on a neighbouring case, as the requester asked.**
  - Step 3's `Ctrl+,` box: the key was posted during an Import preview, not a sidecar preview (the Delete preview of `canvas\` ended before the key arrived). Both previews go through the same `FolderOps::busy` gate.
  - Step 5's `reading …` note: it was seen on the 500-JPEG preview, not on `import-src` (whose 9-file preview ended before a capture).
  - The other option for each is to keep a narrowed box, which would keep the Step 3 item alive with one box.
- **The setup boxes are closed though the run used other paths** (`D:\handpic-c4-checks\`, not `D:\Photos\tests\20261113-gpui-folder-dialogs\`), and each "Record the results in the archived `learnings.md`" box is closed by this folder's `results.md`, not by an edit of `docs/plans/_archived/20261113-gpui-folder-dialogs/learnings.md`. The other option is to add a pointer line to that archived `learnings.md`; not planned, as the earlier device-check plans did not.
- **C4 goes beyond the request's literal list.** The bullet sits in the folder-rename section, not the folder dialogs one. Without C4 it would keep calling a hand check pending whose `todo.md` item this plan deletes. Decided: C4 is applied.
- **`a held sidecar request's dialog opening once the scan ends` moves out of "Not verified headless" into a `Seen (2026-11-23, …)` bullet.** It was seen on 2026-11-23 (`_archived/20261123-windows-device-checks-mcp-visual/results.md`), and the Step 3 background this plan deletes was the only other pointer to it.
- **The watcher finding appears in `docs/agents/app.md` (C1's last point and C2's new item) before its fix lands.** The delegated session `dlg-c4-import-watch` will edit the import paragraph and may want to reword these two.
- **The finding gets no `todo.md` item of its own**: it lives in the narrowed Import box and names the delegated session.
- **Pointers to `../plans/20261113-gpui-folder-dialogs/learnings.md` in `docs/agents/app.md` are left as they are** (C2's closing sentence). The folder is archived, so the backticked path is stale, but fixing it is outside this change.
- **No local `ci:local`.** GitHub Actions verifies. Every path this plan adds is backticked, so no new link is created.
- **`todo.md` is edited by other sessions**; an item reworded on `main` between planning (`f5cab4bf`) and the edit must be re-read, not overwritten.

## Progress

- 2026-11-27: Step 1 applied (A to D): the `docs/agents/app.md` bullets, the `todo.md` boxes closed, the results file and `learnings.md`. PR number: to be added once the PR exists.
