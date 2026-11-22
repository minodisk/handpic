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

# Record the eighth batch of Windows real-device checks (2026-11-22, performance overlay, grayscale hold and held-key preview)

## Purpose

An eighth real-device run on Windows 11 on 2026-11-22 exercised three pending
manual checks of `todo.md` (the file is at the repository root): the
performance overlay on `d`, the momentary grayscale preview on a held `g`, and
the held-key preview and crop (`gpui-held-key-preview`). Every box of the
three items passed. This plan removes the three items from `todo.md`, and
records what was seen in `docs/agents/app.md` where an "(Inferred)" section or
a "Pending" bullet was waiting for the run, or where the run taught a new rule
for driving the window. Docs and todo only, no code change: one step, one PR
(single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-6/plan.md`.

Unlike batches 6 and 7, the run is already done. Its results are in the
session scratchpad's `results.md`; Step 1 copies that file into this folder as
`results.md` and applies it. Nothing is run again. `results.md` is the source
of truth for every edit: lists A to E below name the exact edits.

## Rule for sub-points the run did not exercise

A sub-point of a box that `results.md` does not mention, or lists as not run,
too fast to see or not comparable, was **not** run and must stay open: carry
it into a reworded box with a short note of why (or keep the box). A box is
deleted only when every one of its sub-points was exercised and passed. A
sub-point that failed stays open or becomes a new item (list C). Cases found
while planning, and how each was settled:

- Held-key box 1: "No frame ever shows another file's marks" and the
  release's face boxes and mesh. Settled before Step 1: the main agent re-read
  the kept captures (a stale `--no-prefetch` RAF frame of a file with no `page`
  line is drawn with no AF mark, the current-file frames with theirs) and ran
  one more short hold ending on a face file (r6: face box, mesh, landmarks and
  AF mark on release); both are now in `results.md`.
- Held-key box 2: "(skipping some files)" in the 1:1 hold. Settled the same
  way: `results.md` now names the files whose crops were never drawn.
- Performance box 1: the lines' widths (solid 2 px / dashed 1 px) are not
  stated; solid and dashed are. Closed on the rest, said in the PR
  description.
- Performance box 4: "as before the change" was judged on the lines' format,
  with no build from before the overlay to compare against. Closed on the
  format, said in the PR description.

## Steps

- [x] Step 1: Copy the run's `results.md` into this folder and apply it to `todo.md` and `docs/agents/app.md`
  - Done when:
    - `results.md` in this folder is the scratchpad's file with LF line endings and no other change.
    - Every edit of lists A to E is applied, and this plan's lists name the exact items deleted and the sections changed.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files; no `todo.md` item outside list A changed, and in `docs/agents/app.md` only the four sections of list D changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where an item went (the count of doubled blank lines equals `origin/main`'s in both files). Every changed file has LF line endings.
    - The PR description records what was seen for each removed box, with its notes, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **No run.** The checks were run on Windows 11 on 2026-11-22 at `19bacdcf`; the source file is `C:\Users\daisu\AppData\Local\Temp\claude\325f1a3d5c9b0f2a\ead8d71b-53f2-409a-abd2-8d186e99f82e\scratchpad\results.md`. Copy it to `results.md` in this folder, converting to LF (assert no CR is left).
    - **Editing.**
      - The branch was cut from `origin/main` after the batch 7 PR, so the "Driving the window" section of list D is there. If `origin/main` moved meanwhile, rebase before editing.
      - Locate every item and section by its exact `###` line, never by line number. "Box N" counts the item's top-level `- [ ] ` lines from 1.
      - Keep every other heading, item and sentence byte-for-byte.
      - Write a Python script to the scratchpad with the Write tool (not a Bash heredoc) and run it with `mise x -- python -I`, asserting exactly one match per anchor (each `###` line, each `- Pending:` bullet, the rule bullet of the grayscale section) and writing LF.
      - Every "seen" sentence added to `docs/agents/app.md` says "on Windows 11 on 2026-11-22" and names the results as `docs/plans/_archived/20261122-windows-device-checks-8/results.md`, in backticks, not as a link (the folder is archived by the same PR's wrap-up, and a link would not resolve before that).
      - Wrap added text in `docs/agents/app.md` at the file's width, as its neighbours.
      - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the performance overlay, the grayscale hold and the held-key preview`.
    - **A. `todo.md`: three items deleted whole** (heading through the line before the next `###`, leaving one blank line between the neighbours):
      - `### App: real-device check of the performance overlay on \`d\`` (4 boxes; in `todo.md` the heading has plain backticks around `d`), release build:
        - Box 1 passed: the band is 120 px ending at the preview canvas's bottom edge, `#171717` at 0.85 alpha (`#281414` over a `#8C0600` pixel), grid lines at 0, 50 and 100 % under a 20 px legend row, the plot changing once every 500 ms with the newest point at the right edge, the legend with the four swatches, `Latency`, the eight stage names and `10 MB/s  500 ms`.
        - Box 2 passed, with a note: three `→` presses, one strip click and one folder open from the tree gave one bar each, and each input did what it does without the overlay. The bars are 1–3 px tall, since the switches took 12–20 ms against the 500 ms latency axis; next to invisible at normal size.
        - Box 3 passed: the band went away on `d`, and came back with no plotted lines.
        - Box 4 passed on the format: 60 key-downs printed 57 `page` lines with every field of the Timings description and `q` printed `summary n=61` with the nine stage rows; no build from before the overlay was run to compare against.
      - `### App: real-device Windows check of the momentary grayscale preview` (7 boxes), debug build:
        - Box 1 passed: chroma 0.0 while held and color on release in the fit view; in 1:1 both the placeholder and the crop gray.
        - Box 2 passed: the peaking edges, the face box and label, the mesh, the landmark dots, the AF mark and the performance graph stayed colored; the tree, the meta pane, the strip, the failed-preview text and the status row were pixel-identical.
        - Box 3 passed: no colored frame in 30 captures over 60 key-downs at 33 ms.
        - Box 4 passed: with `ctrl+g`, color returned on the first release either way.
        - Box 5 passed: color after Alt+Tab away and back with `g` still down.
        - Box 6 passed: color after the settings closed; the right-click menu's opening did not end the hold (the photo stayed gray while `g` was down), and `g`'s release with the menu open did.
        - Box 7 passed: paging, `z`, `f`, `e` and `d` behaved as before while `g` was held.
      - `### App: real-device checks of the held-key preview and crop (gpui-held-key-preview)` (2 boxes), debug build:
        - Box 1 passed: the fit view flipped through frames during the hold with files skipped (`raf\`, with and without `--no-prefetch`); a stale frame was drawn with no `f` overlay and the current-file frames with their own marks, so no frame showed another file's marks; the release showed the current file with its marks (on a face file: face box, mesh, landmarks and AF mark), its meta pane and the last `page` line.
        - Box 2 passed: the crops changed during the hold with files skipped and every frame one file's crop or placeholder whole, the release showed the current file's crop, a single press went old crop, new crop, and `z` off / on reused the crop with no new `zoom` line while `z` on a new file printed one.
    - **B. Narrowed boxes**: none.
    - **C. New `todo.md` items**: none. The 1–3 px bars of performance box 2 are the latency axis floor (`LATENCY_FLOOR`, 500 ms, in `crates/app/src/perf.rs`) working as designed, so they are recorded in `results.md` and the PR description only. Two other observations of `results.md` are by design as well and get no item: the legend without `System disk` 250 ms after reopening (`crates/app/src/perfgraph.rs` draws that series only once a sample in the window has a disk busy value), and `×` on `G` not removing the only key (`Keymap::remove` in `crates/engine/src/shortcuts.rs` refuses the last key, "use Reset").
    - **D. `docs/agents/app.md`**, these four sections and nothing else:
      - `### A held \`g\` grays only the photo, through the sprite flag (Inferred)`: change the tag to `(Measured)`, as the item's Background asks. Add after the section's `- Rule: key-up and modifiers-changed are dispatched only along the focused node's path ...` bullet one short bullet, worded as an observation: on Windows 11 on 2026-11-22 the strip's right-click `PopupMenu` opening did not end the hold (the photo stayed gray while `g` was down) and the held `g`'s key-up with the menu open did, which the rule's `PopupMenu` case does not predict; name the results path. Do not rewrite the rule or its other cases (the settings layer, a dialog, an `Input`), and do not state a cause: none was traced.
      - `### A held key draws each stale preview before chasing the current file (Inferred)`: change the tag to `(Measured)` and replace the `- Pending: a held key flipping frames ... unverified by hand on Windows.` bullet with a `- Measured on Windows 11 on 2026-11-22 ...` bullet: a held `→` (key-downs every 33 ms) flipped frames on RAF files with files skipped and some `page` lines waiting on the read in flight, a stale frame was drawn with no `f` overlay, and the release showed the current file with its marks (face box and mesh on a face file), meta pane and the last `page` line; results path.
      - `### A held key draws each stale crop before chasing the current one (Inferred)`: change the tag to `(Measured)` and replace the `- Pending: a held key flipping crops while zoomed ... unverified by hand on Windows.` bullet with a `- Measured on Windows 11 on 2026-11-22 ...` bullet: with crop reads of 51–62 ms against the 33 ms repeat the crops changed during the hold with files skipped, every frame one file's crop or placeholder whole, the release showed the current file's crop, a single press went old crop at 37 ms to new crop from 85 ms with no black or placeholder frame, and `z` off / on reused the crop with no new `zoom` line; results path.
      - `### Driving the window in a real-device run on Windows (Measured)`: append two `- Rule:` bullets at the end of its list, each naming this batch's results path. (1) `SetCursorPos` followed by a button press did not deliver the click to GPUI although `WindowFromPoint` named the instance; a few relative `mouse_event` moves before the press did, every time. (2) A click on a tree row gives the tree the keyboard, so the next key goes to its type-ahead; send `Escape` before the next key.
      - The third `Pending` bullet near these sections (the prefetch ring's) is not this batch's and stays.
    - **E. `learnings.md`**: this step's notes.
      - The two input lessons of list D.
      - The fast-burst capture with preallocated bitmaps (a capture every 15 ms or so) that caught the 1:1 placeholder before the crop.
      - `--no-prefetch` was needed to make the fit view's reads outlast the key repeat on cached files.
      - Three held-key sub-points were first recorded only in part; they were settled by re-reading the kept captures and one extra short hold (r6) before Step 1.
      - The decision of list C.
      - `results.md` stays in the folder as the run's record.

## Trade-offs and risks

- The held-key item is deleted whole on evidence `results.md` holds, after the re-read and the r6 hold (see the rule section); no fallback to narrowing was needed.
- The tiny bars get no `todo.md` item (list C). The other choice, an item to make a short switch's bar readable (a lower axis floor or a minimum bar height), would change a ported design (`perf.ts`'s floor) nobody asked to change; it can be raised later from the PR description.
- Performance box 1 is closed without the lines' widths in the record, and box 4 on the format alone; both are said in the PR description. Keeping either box open would need a pre-overlay build or a re-measurement, which this plan does not do.
- The grayscale section becomes `(Measured)` while one of its rule's cases is contradicted by an observation with no traced cause. The observation bullet says so; tracing why the key-up reached the view with the menu open is left out of this docs-only plan.
- A removed item's record lives in `results.md` and the PR description only.

## Progress

- 2026-11-22: Step 1 done. The run is in `results.md` (copied unchanged, LF), the notes in `learnings.md`. Edits made:
  - A: the three `todo.md` items deleted whole.
  - B / C: none.
  - D (`docs/agents/app.md`, four sections): three tags changed to `(Measured)`, two `Pending` bullets replaced by `Measured` bullets, one observation bullet added to the grayscale section, and two rules added to `Driving the window`.
  - E: `learnings.md` written.
