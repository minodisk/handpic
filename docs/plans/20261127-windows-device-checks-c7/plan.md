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

# windows-device-checks-c7

## Purpose

A real-device run on Windows 11 on 2026-11-27 exercised twelve pending
"real-device check" items of `todo.md` through the MCP companion's tools, with
no key press, click or drag. This plan records the results, deletes the items
the run closed, narrows the others to what was not run, and replaces the notes
in `docs/agents/` that said a value was not measured yet. Docs and todo only:
one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261123-windows-device-checks-mcp-visual/plan.md`.

The run is done and nothing is run again. Its results are in this folder's
`results.md`, the source of truth for every edit below.

## Steps

- [x] Step 1: Record the 2026-11-27 Windows results and update `todo.md` and `docs/agents/`
  - Done when:
    - `results.md` in this folder is unchanged.
    - `todo.md` holds the edits of lists A to C below and no other change.
    - `docs/agents/app.md` and `docs/agents/shared-crates.md` hold the edits
      of list D and no other change.
    - No file outside `todo.md`, `docs/agents/app.md`,
      `docs/agents/shared-crates.md` and this plan folder changes.
    - The PR's GitHub checks pass (no local `mise run ci:local` needed).
  - Implementation approach:
    - Rebase on `origin/main` before editing `todo.md` (other sessions edit it
      in parallel). Find sections by `###` heading, never by line number.
    - A deleted section takes its heading, body and the closing `---` (where
      it has one), keeping one separator between neighbours.
    - Every Background that stays gets one sentence in the existing style:
      "Seen on Windows 11 on 2026-11-27 (results in
      `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`):
      ...", the path in backticks. Sections without a `#### Background`
      heading get it at the end of their opening paragraph.
    - Anything needing a key press, click, drag, typing, PowerShell 7, another
      taskbar position, a real group photo folder or the user's eye stays open.
    - **A. Delete**
      1. "App: real-device checks of the title bar's smaller text and centered
         folder name (native-title-bar-folder-name)": its only box (14 px by
         eye) is superseded by `title-bar-tweak` (#1326: 13 px text, 18 px
         icon, seen and approved by the user on Windows 11,
         `docs/plans/_archived/20261127-title-bar-tweak/plan.md`); macOS and
         Linux stay under "App: real-device checks of the title bar's settings
         button (native-title-bar-settings)".
      2. "App: re-measure the prefetch ring after the wrap (todo-ten-closeouts)" (see D2).
      3. "App: real-device check of the Sort / Filter menus with no folder, the
         status row's reserved slot and shortened strip names
         (todo-three-decisions)" (see D3; the drag itself stays tracked under
         the native-strip-resize item).
    - **B. Narrow**
      1. "App: real-device check of the default window's centering on the
         visible area (todo-ten-closeouts)": delete the taskbar-left and record
         boxes; keep the taskbar-at-the-bottom box, noting it was not run
         because the host's taskbar setting is the user's. Background: the
         measurement (1616x1008 outer, centered to the pixel horizontally at x
         2591 and within the frame's 3.5 px vertically, 31 px right of the
         display's center, no longer 93 px); correct "the window's real
         rectangle was never read".
      2. "App: real-device checks of the Windows GUI-subsystem release build
         (windows-gui-subsystem)": delete the Explorer box; keep the `pwsh`
         box, adding that PowerShell 7 is still not installed on the host.
         Background: the installed release 2.1.0 launched through
         `explorer.exe` on a `.lnk` (ShellExecute, as a double-click does):
         parent `explorer.exe`, no child process, one window, no console.
      3. "App: real-device check of the strip's closed-eyes mark
         (eyes-mark-tier-mcp)": delete the three Windows boxes; keep the
         macOS / Linux box, restating the three expectations so it stands
         alone. Background: the results' three findings (mark = `Closed`
         filter = `get_view`'s closed state on `_DSC1926.ARW`; live during
         `analyzing 14/24` with no re-listing; clear of three-digit badges at
         `MIN`, dimmed when rejected).
      4. "Engine: real-device check of the chunked Delete Sidecars… move on
         Windows (chunked-sidecar-delete)": the timing and the undo / redo
         close (see D4). Keep one residual box: with the keys (`Ctrl+Z` and its
         redo key) rather than the MCP tools, and the progress circle seen
         ending full (the run saw it empty, then a partial arc, then gone).
    - **C. Keep, rewrite to what remains**
      1. "App: real-device check of Handpic installed beside Riffle 1.10.0 on
         Windows (native-release)": box unchanged; Background adds the two
         uninstall entries side by side, only Handpic's Start menu shortcut,
         no Riffle `settings.json` on this host, nothing uninstalled.
      2. "App: real-device check of the Windows one-line install script
         (install-scripts)": the install box stays, saying it was not run
         because it would replace the user's own install; the forced-failure
         box becomes "repeat the forced failure in PowerShell 7" (not
         installed here); the Arm box stays. Background: the PowerShell 5.1
         forced-failure result; correct "`install.ps1` was never run or even
         parsed".
      3. "App: real-device check of the group threshold setting
         (all-faces-check Step 4)": the Windows box keeps only typing in the
         field, Enter / Tab / Escape, `-` / `+`, the status-line refusal and
         digits not being shortcuts; the macOS / Linux box restates the whole
         check (default 5, lowering starts `checking faces`, the range, the
         restart). Seen sentence from the results.
      4. "...all-faces meta rows and the strip's closed-eyes mark
         (all-faces-check Step 5)": the Windows box keeps the counts agreeing
         with a real group photo and the mark from a closed count; macOS /
         Linux restates the whole check. Seen sentence from the results.
      5. "...Anyone closed / Anyone soft filter items (all-faces-check Step
         6)": the Windows box keeps `Anyone closed` on a folder with a real
         closed count and the readings judged against the photos; macOS /
         Linux restates the whole check. Seen sentence from the results.
    - **D. `docs/agents/`**
      1. `app.md`, "A launch restores the last folder and the window": replace
         "The fixed default is not measured on the host yet." with the
         measurement and the results path; add the bottom taskbar for the
         default window to "Not seen yet" if not implied.
      2. `app.md`, "A prefetch hit leaves only the frame": drop the "These
         numbers are from before `neighbours` wrapped" paragraph; give the
         2026-11-27 numbers as current (release at `f5cab4bf`, medians ARW
         13.64 / 22.22 ms against 28.01 / 36.30 ms, RAF 13.34 / 22.04 ms
         against 55.39 / 71.95 ms; 60 of 60 on every run but one RAF run of 58
         of 59 with a mid-list read on a busy machine); keep the pre-wrap
         numbers as one history sentence; keep the `paint` / `present` and
         memory sentences attributed to the earlier run; fix the first Rule
         bullet (the wraps now hit; a miss is a read on a busy machine; read
         the hits' latency from the p50).
      3. `app.md`, the middle-ellipsis rule under "The strip is a `canvas`
         over a pure model": mark it seen at `MIN` and `MAX` (heights set
         through the store, not a drag), `NAME_INSET` left as is.
      4. `shared-crates.md`, the `foldersidecars::delete` paragraph: add the
         2026-11-27 in-app measurement (300 `.xmp` in about 1.6 s, 5-7 ms a
         file against about 140 ms; undo 2759 ms, redo 2075 ms, through the
         MCP `undo` / `redo` tools), with the results path.

## Trade-offs and risks

- Strip names close on heights set through the store, not a drag; the prefetch
  closes on one 58-of-59 run whose miss was not a wrap; the Explorer box closes
  on a `.lnk` launch through `explorer.exe`; the closed-eyes mark's box 1 was
  compared with `get_view` and the filter, which read the same `Facts` as the
  meta row. Each is stated in the record.
- `todo.md` is edited in parallel; resolve a conflict by re-reading the section
  on the new `origin/main`.

## Progress
