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

# Record the sixth batch of Windows real-device checks (2026-11-22, strip, scan progress, resume, resync and the 1:1 view)

## Purpose

A sixth real-device run on Windows 11 on 2026-11-22 exercises three pending
manual checks of `todo.md`: the strip, the scan progress, the resume and the
resync (`gpui-strip-index`), the 1:1 view on files with a malformed full-size
JPEG, and the 1:1 view on older Sony ARW bodies that hold no full-size JPEG.
This plan runs them, removes the `todo.md` boxes the run settles, narrows the
rest to what stays open, adds whatever the run finds as new items, and records
what was seen in `docs/agents/app.md` only where a run teaches a new rule or
settles a "Not verified" bullet. Docs and todo only, no code change: one step,
one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-5/plan.md`.

Unlike the earlier batches, this plan is written before the run. The run's
results go to `results.md` in this folder (copied from the scratchpad). They
are the source of truth for every edit: lists A to D below give the rule per
box, and are tightened to the exact edits once `results.md` exists.

## Rule for sub-points the run did not exercise

A sub-point of a box that `results.md` does not mention, or lists as not run,
too fast to see or not comparable, was **not** run and must stay open: carry
it into the reworded box with a short note of why (or keep the box). A box is
deleted only when every one of its sub-points was exercised and passed. A
sub-point that failed stays open or becomes a new item (list C). Cases to
expect (check for others while editing):

- Strip box 1: the `scanning N/total` cadence ("about every 100 ms") on a
  debug build.
- Strip box 2: (2) the held `→` (synthetic keys do not auto-repeat; it counts
  only if the run sent repeated key-downs and says so); (3) the click in the
  8 px gap; (4) the memory sub-point, run as sampled private bytes, not Task
  Manager.
- Strip box 3: "within a second or so" on a debug build.
- Strip box 4: "From Explorer", if the copies and deletes were made from a
  shell.

## Steps

- [x] Step 1: Run the three checks, write `results.md`, and apply the results to `todo.md` (and `docs/agents/app.md` where a rule is learned)
  - Done when:
    - `results.md` in this folder holds the setup, the incidents and one section per `todo.md` item, each sub-point marked PASS, FAIL / FINDING, or not run with the reason.
    - Every edit of lists A to D is applied under the rule above, and this plan's lists name the exact boxes deleted, narrowed and added.
    - `git diff origin/main --stat` shows only `todo.md`, this plan's files and, if list D applied, `docs/agents/app.md`; no `todo.md` item outside the lists changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where a box or an item went (same counts as `origin/main`).
    - The PR description records what was seen for each removed box or item, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **The run.**
      - Build: Windows 11, debug build of `crates/app` with `--features screenshot`.
      - MCP: on port 41951 (`--mcp-port`), `"mcpEnabled": true` seeded through `--import-settings` into a fresh `--settings` store.
      - Per run: fresh `--index`, `--settings` and `--log` under the test folder.
      - Input and evidence: synthetic `SendInput` mouse and keys, window captures.
      - Follow the real-device rules in `docs/agents/app.md`: the MCP test-run rule, checking the target tree row before a click, the Git Bash `curl` / Python rules, and stopping a test instance by the PID whose command line names the test folder.
    - **Material.**
      - The app only ever opens copies or hardlinks under `D:\Photos\tests\20261122-strip6\`; nothing under `D:\photos\samples` or another test folder is opened.
      - Subfolders: `few\` (300 hardlinked ARWs), `big\` (1100 hardlinked ARWs), `corrupt\` (copies of `D:\Photos\tests\20261110-gpui-strip-index\ARW\`, holding `DSC-RX100M3_corrupt.ARW`), `manual4\` (5 ARW copies, spares to copy in), `zoom\` (copies of `NIKON_D70_Nikon.nef` and `CGO3P_YUN00007.dng`), `zoom-arw\` (one copy per listed body: `ILCE-9M2_sony_a9_ii_10.arw`, `ILCE-7RM4_DSC00395.ARW`, `ILCE-7RM4A_DSC00551.ARW`, `ILCE-7C_DSC00018.ARW`, `ILCE-6400_DSC00087.ARW`, `ILCE-6600__DSC0268.ARW`, `ZV-E10_DSC00002.ARW`).
      - The files the resync check deletes are copies, never the originals.
    - **Strip box 3(a) (resume).**
      - It needs the same `--index` on its two launches (the one exception to a fresh index per run).
      - It needs a clean quit in between, so the quit handlers drain the `last_viewed` writer. Close the test instance's window by its own PID; kill only if it does not exit, and then do not count the resume.
      - The box says "quit with `q`". Verify whether `q` quits in the current keymap, and record what was used.
    - **Strip box 2 (4) (memory).** Sample the test PID's private bytes from PowerShell (`PrivateMemorySize64`) before, during and after each of several end-to-end wheel passes; record the series in `results.md`.
    - **1:1 checks.** Per file, capture the fit view, then `z`; record the capture, the status line and pane text, and the `--log` lines of the crop read. Old Sony bodies: judge sharpness on the capture. Malformed files: record whether the app stays up, what the 1:1 view shows, and whether the `libjpeg fatal error: Not a JPEG file` text surfaces anywhere (UI or log).
    - **Editing.**
      - Locate every item by its `###` heading, never by line number. "Box N" counts the item's top-level `- [ ]` lines from 1.
      - Keep every heading, the neighbouring items and every sentence this plan does not name, byte-for-byte.
      - Write a Python script to the scratchpad with the Write tool and run it with `python -I`, asserting one match per `###` anchor.
      - Every "seen" sentence added to a Background says "on Windows 11 on 2026-11-22" and names the results as `docs/plans/_archived/20261122-windows-device-checks-6/results.md` (in backticks).
    - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the strip, the resume, the resync and the 1:1 view`.
    - **A. `### App: real-device checks of the strip, the scan progress, the resume and the resync`** (4 boxes, item kept):
      - Box 1 (`scanning N/total` on a few hundred files) deleted: the climb, its cadence (N changed on almost every 100 ms frame on `big\`, on the debug build) and the end state (empty, `1 failed` on `corrupt\`) passed.
      - Box 2 (the 1000+ folder) narrowed to sub-point (1), the placeholders turning into thumbnails from the current cell outward, now on files not in the OS file cache (`robocopy /J`), with the note that the cells in view already held thumbnails by the first capture. Sub-points (2) to (5) passed ((2) on 90 repeated key-downs with one key-up; (4) on sampled private bytes).
      - Box 3 (the resume and the steered scan) narrowed to (b), on files not in the OS file cache, with the note that pass 1 on cached files had ended before the window took keys. (a) passed after a `q` quit.
      - Box 4 (the resync) deleted: all three sub-points passed on PowerShell `Copy-Item` / `Remove-Item`; the method is recorded in the Background.
      - The Background got one 2026-11-22 paragraph before `#### TODO`: the material folder, the debug build with `--features screenshot`, the cadence, the private bytes series, the `q` quit, the resync's PowerShell copies and deletes, and "What is left open is in the boxes below."
    - **B. The two 1:1 items**, both deleted whole:
      - `### App: real-device check of the 1:1 view on files with a malformed full-size JPEG`: the app stayed up through `z` and paging, and the error surfaced in the meta pane and the status line; the black 1:1 canvas became the second list C item, in this item's place.
      - `### App: real-device check of the 1:1 view on older Sony ARW bodies (no full-size JPEG)`: all seven bodies gave a sharp crop of the 1616x1080 / 1920x1080 / 1440x1080 preview.
    - **C. New items** (no existing item covered either), in place of the malformed-JPEG item:
      - `### App: a strip cell of a file whose thumbnail encode panics reads the internal panic text and the Windows verbatim path` (`crates/core/src/scan.rs`'s `catch_unwind` text on `NIKON_D70_Nikon.nef` / `CGO3P_YUN00007.dng`).
      - `### App: the 1:1 view of a file whose crop read fails is a plain black canvas` (`z` hides `empty::PREVIEW_FAILED_TEXT`, the 1:1 view draws no text of its own).
    - **D. `docs/agents/app.md`**: nothing added. The run's real-device lessons are already rules there: the `page` lines go to stdout (Timings, "`Root` costs the pager nothing measurable"), and `cp`-made copies stay in the OS cache, so a cold pass 1 needs `robocopy /J` copies used once ("During pass 1 the preview reads what the scan already read").
    - **E. `learnings.md`**: this step's notes. `results.md` stays in the folder as the run's record.

## Trade-offs and risks

- `results.md` stays as its own file and `todo.md` names it.
- Debug build: behavioural sub-points close on it; timing / memory-level claims stay open unless they held on the debug build too.
- Memory sub-point closed on sampled private bytes rather than Task Manager's column.
- "From Explorer": closed on shell copies / deletes, method recorded.
- Held `→`: counts only if repeated key-downs were sent.
- The resume check uses one index twice and closes the instance through its window.
- A 1:1 item whose single box passes is deleted whole; what was seen lives in `results.md` and the PR description.

## Progress

- 2026-11-22: Step 1 done. The run's results are in `results.md`. In `todo.md`, the strip item lost boxes 1 and 4 and kept boxes 2 and 3 narrowed to the uncached-file sub-points; both 1:1 items were deleted; two new items were added (the strip cell's panic text with the verbatim path, the black 1:1 canvas of a failed crop). `docs/agents/app.md` unchanged.
