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

# Record the eighth batch of Windows real-device checks (2026-11-22, drag-resizable panes, strip height and scrollbars)

## Purpose

An eighth real-device run on Windows 11 on 2026-11-22 exercised three pending
manual checks of `todo.md`: the drag-resizable tree and meta panes
(`native-pane-resize`), the drag-resizable strip height
(`native-strip-resize`) and the strip, folder tree and meta pane scrollbars
(`native-scrollbars`). The run is done. This plan removes the `todo.md` boxes
the run settled, narrows the rest to what stays open, adds the two findings as
new items, and records in `docs/agents/app.md` what settles a "Not observed"
bullet or corrects a real-device rule. Docs and todo only, no code change: one
step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-7/plan.md`.

The run's results are in [`results.md`](results.md) in this folder (Step 1
copies them verbatim from the scratchpad). They are the source of truth: the
lists below name their sections and do not restate them.

## Rule for sub-points the run did not exercise

A sub-point of a box that `results.md` does not mention, or lists as not run,
was **not** run and must stay open: carry it into the reworded box with a
short note of why. A box is deleted only when every one of its sub-points was
exercised and passed (or does not apply). A finding becomes a new item
(list D). Known cases (check for others while editing):

- Pane box 5: the empty-state message's click passed, but the message stayed
  80 px from each band, so the bands never came near it.
- Strip box 2: HiDPI (no HiDPI display).
- Scrollbars box 4: the thumbs of the settings lists and the menus were not
  dragged (wheel only), and the folder (tree) right-click menu was not made
  to overflow.
- The macOS / Linux box of each of the three items.

## Steps

- [x] Step 1: Copy `results.md`, apply the results to `todo.md` and `docs/agents/app.md`, and write `learnings.md`
  - Done when:
    - `results.md` in this folder is a verbatim copy of the scratchpad's `results.md` (`C:\Users\daisu\AppData\Local\Temp\claude\21695e1d7b4c51d2\b1fc40aa-02cb-466b-9335-2a975ae03bbd\scratchpad\results.md`).
    - Every edit of lists A to E is applied under the rule above.
    - `git diff origin/main --stat` (after `git fetch origin main`) shows only `todo.md`, `docs/agents/app.md` and this plan's files. In `todo.md` only the three items of lists A to C changed and the two items of list D were added; no other item changed, in particular not `### App: a shorter window leaves the preview under PREVIEW_MIN until the strip is dragged` and not `### App: real-device checks of the always-visible scrollbars`.
    - Every `###` item has exactly one blank line before it, no doubled blank lines are left where a box went, and the edited files hold no CR.
    - The PR description records what was seen for each removed box, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **Base.** #1232 (batch 7) is merged: `origin/main` is `ba9efeaf` or later. Put the branch on current `origin/main` before editing. The three items are byte-identical before and after #1232. Batch 7's `docs/plans/_archived/20261122-windows-device-checks-7/results.md` and the `docs/agents/app.md` section "Driving the window in a real-device run on Windows" exist on main.
    - **Editing.**
      - Locate every item by its `###` heading, never by line number. "Box N" counts the item's top-level `- [ ]` lines from 1.
      - Keep every heading, the neighbouring items and every sentence this plan does not name, byte-for-byte.
      - Write a Python script to the scratchpad with the Write tool and run it with `mise x -- python -I <script>`, asserting one match per `###` anchor and per replaced string.
      - Read and write bytes (or `newline=""`): Python on Windows writes CRLF through `Path.write_text`. Assert the result holds no `\r`.
      - Write in English, in the file's own style. The pane and strip items have no `#### Background` heading; their background is the paragraph under the heading. Do not add the heading.
      - Every "seen" sentence added to `todo.md` starts "Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`)", the path in backticks, no Markdown link.
    - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the pane and strip resize and the scrollbars`.
    - **A. `### App: real-device checks of the drag-resizable tree and meta panes`** (6 boxes, item kept):
      - Box 1 (hover) deleted: `SIZEWE` on the 5 px band of both boundaries, the 3 x 20 px pill on hover only, the 1 px `#1a1a1a` line judged acceptable in Neutral dark.
      - Box 2 (a jump at the first move) deleted: the question is answered, and the answer is list D's first item.
      - Box 3 (release positions and restarts) deleted: all five releases stored the clamped width at once, kept it over a `q` quit and came back at it; a release outside the window wrote at once.
      - Box 4 (`z`, then a drag) deleted: the crop was re-cut about 3 s after a drag that grew the canvas.
      - Box 5 narrowed to the one sub-point left: on Windows, in a window narrow enough that the no-folder message reaches the preview's edges (the preview near its 240 px minimum), click the message next to each band and expect the folder picker to open. Add the note of why: at the default 1600 px window with both panes at 480 the message stayed 80 px from each band. The other sub-points go: the preview has no context menu (`context.rs`'s `Target` is `Folder` or `Files`), the tree's thumb sits 4 px left of its band and dragged without changing the width, the tree's rows and inline rename input lay out at 160 px, and the meta pane's rows are list D's second item.
      - Box 6 (macOS and Linux) unchanged.
      - Intro paragraph: replace the clause "the handle's look on hover, a jump at the first move, the release positions, the 1:1 re-cut, the 5 px bands and the layout at 160 px are unchecked" with the 2026-11-22 "Seen" sentence. It summarizes the points above and says: the line was judged in Neutral dark only (3 levels from the tree pane's fill); the boundary ignores the first 3 to 5 px of a drag and then sits under the pointer (list D's item, by heading); the meta pane breaks words mid-word at 160 px (list D's item, by heading); the preview has no context menu; what is left is in the boxes below. Keep "The limits (160 / 480 / 240) were not judged by eye.", the `Plan:` and `Files:` sentences and the "Run each check with" paragraph.
    - **B. `### App: real-device checks of the drag-resizable strip height (native-strip-resize)`** (6 boxes, item kept):
      - Box 1 (hover) deleted: `SIZENS` on y 802 to 807 under the line at y 801, the `#1a1a1a` line reading on black in Neutral dark, a cell click and the scrollbar's thumb drag and track clicks still working.
      - Box 2 narrowed to HiDPI: on Windows, at 302 px on a HiDPI display, check the thumbnails' sharpness. Note that at 100 % (96 DPI) the 404 px thumbnails are drawn at about 1:1 in the 405 x 270 box and were legible, and that no HiDPI display was at hand.
      - Box 3 (release positions and restarts) deleted: all four releases stored the height at once (302 `MAX`, 179, 96 `MIN`, 302) and came back at it.
      - Box 4 deleted: the 1:1 re-cut after a strip edge drag was seen in batch 7 on the same day (`docs/plans/_archived/20261122-windows-device-checks-7/results.md`, the always-visible scrollbars' box 7) and the same re-cut after a pane drag in this run; `toggleStrip` hid the strip, its header and its line and brought them back at the dragged 302 px.
      - Box 5 (`--auto`) deleted: with 199 stored, `--auto 40` started at 128, and a drag wrote nothing, right after it and after the run quit.
      - Box 6 (macOS and Linux) unchanged.
      - Intro paragraph: replace the clause "the cursor and the handle's line, the sharpness of the largest cells, the release positions, the 1:1 re-cut and `--auto` are unchecked" with the 2026-11-22 "Seen" sentence. It summarizes the points above, names batch 7's results for the strip-drag re-cut, and says HiDPI sharpness is left. Keep the limits sentence, `Plan:`, `Files:` and the "Run each check with" paragraph.
    - **C. `### App: real-device checks of the strip, folder tree and meta pane scrollbars`** (5 boxes, item kept):
      - Box 1 deleted: the thumb dragged to the far end of 295 files showed the last cells' thumbnails within 3 s; the thumb followed `→`, `←`, `Home`, `End`, a refilter and its reset, and a 500 px narrower window, the current cell in view each time.
      - Box 2 deleted: a press on the thumb and two track clicks left the current file and the one selected file; the `Red` tooltip showed before and after.
      - Box 3 deleted: a click, a right press and an inline rename (Enter and `escape`) next to the tree's bar behaved as before.
      - Box 4 narrowed to what `results.md` lists as not done or does not mention: on Windows, drag the thumbs of the settings modal's lists (the `Lightroom language` `Select` list, the Keyboard Shortcuts page), of `Filter ▾` and of the strip's right-click menu in a short window, and make the folder (tree) right-click menu overflow in a short window and scroll it. Note that wheel scrolling of the first four, with the thumb moving, was seen on 2026-11-22 and that `Sort ▾` holds three rows and does not overflow.
      - Box 5 (macOS and Linux) unchanged.
      - `#### Background`: keep the existing sentences and append two. First: since `native-scrollbars-always` the mode is `ScrollbarMode::Always`, so the bars show at rest and the fade-out and "taking a press only while shown" above no longer apply (`### App: real-device checks of the always-visible scrollbars` holds that feature's checks). Second: the 2026-11-22 "Seen" sentence summarizing boxes 1 to 3 and box 4's wheel scrolling, ending "What is left open is in the boxes below."
    - **D. Two new items in `todo.md`**, inserted in this order right after the pane item of list A (before the next `###`), each with a background paragraph ending in a `Files:` sentence and a `#### TODO`. No existing item covers either ("mid-word", "word-break", "wrap", "first move" and "jump" give no match in `todo.md`).
      - D1 `### App: decide whether a pane drag keeps the press's offset from the boundary`.
        - Background: the 2026-11-22 "Seen" sentence, then the measurements of `results.md`'s pane box 2 (pressed at x 223 with the line at 220, no move at 224, 225 and 227, then the line at 230 for a pointer at 230; the meta pane's the same from 1379). So the first 3 to 5 px of a drag move nothing, and then the boundary sits under the pointer and the press's offset inside the 5 px band (up to 3 px here) is lost. By the source, `drag_pane` takes the pointer's `x` as the width (`panewidths::dragged`) and the handle is gpui-component's `resize_handle`, whose drag starts only after the pointer has moved. The strip's top edge (`drag_strip`, `stripheight::dragged`) follows the same pattern and was not measured.
        - Files: `crates/app/src/main.rs` (`pane_handle`, `drag_pane`, `strip_handle`, `drag_strip`, the `on_drag_move` listeners in `render`), `crates/app/src/panewidths.rs`, `crates/app/src/stripheight.rs`.
        - TODO, one box: decide whether to fix it; if so, keep the press's offset from the boundary through the drag for the panes and the strip's edge, and measure the strip's edge on Windows first; if not, record the decision in `docs/agents/app.md`'s "The side panes' widths are one `PaneWidths`, stored under `panelWidths`".
      - D2 `### App: the meta pane breaks words mid-word at its 160 px minimum`.
        - Background: the 2026-11-22 "Seen" sentence, then the examples of `results.md`'s pane box 5 (`Electroni` / `c`, `Standar` / `d`, `Compresse` / `d RAW`, `Mechani` / `cal`, `Manu` / `al`, the file name `ILCE-1M2_sony_a1_ii_0` / `3.arw`). By the source, each row of `meta_pane` is a `flex` row whose label is `flex_none` and whose value takes what is left (`min_w_0`, `text_right`), so at 160 px the value column is narrower than one word; the name is a plain `div` over the pane's width.
        - Files: `crates/app/src/main.rs` (`meta_pane`), `crates/app/src/panewidths.rs` (`MIN`).
        - TODO, one box: decide what a row does when its value does not fit beside its label (the value wrapping onto its own line under the label, a truncation with an ellipsis, or a larger `MIN`), apply it, and check the pane at 160 px on Windows.
    - **E. `docs/agents/app.md`** (find each section by heading; keep headings and their `(Inferred)` / `(Measured)` tags; each added bullet names `../plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md` in backticks):
      - "Driving the window in a real-device run on Windows": one bullet after the `GetCursorInfo` rule. Moving the pointer away and back with `SetCursorPos` alone (450 ms wait) read `ARROW` all along the pane boundaries; end the move with a few relative `mouse_event` moves before the read, and expect the band's edges to shift by about 1 px between reads.
      - "The side panes' widths are one `PaneWidths`, stored under `panelWidths`": replace the "Not observed in the running app" bullet with a `Seen (2026-11-22, Windows 11)` bullet. Content: `SIZEWE` on the preview's outer 5 px beside each pane (x 221 to 226 with the tree's line at 220, about 1374 to 1380 across reads with the meta pane's line at 1379; a press at 1380 moved nothing while 1379 and 1376 worked, so aim a synthetic press at the meta pane's boundary on or left of its line); the pill on hover; every release, outside the window included, writing at once (Windows delivers the release); the 1:1 re-cut after a drag; the tree at 160 px. End it with what is not yet seen: the bands against the no-folder message in a narrow window, macOS and Linux, and the two list D items by heading.
      - "The strip's height is one number, stored under `stripHeight`": replace its "Not observed in the running app" bullet the same way. Content: `SIZENS` on y 802 to 807 under the line at 801; the releases writing at once, outside the window included; `--auto 40` starting at 128 with 199 stored and writing nothing; the 302 px cells drawn at about 1:1 at scale 1. Not yet seen: a HiDPI display. Keep "the marks on the smallest cells" out of the Seen list unless the 2026-11-21 note in `todo.md` is cited for it.
      - Nothing else: `--auto` taking a value is already in the document (`--auto N`).
    - **F. `learnings.md`**: this step's notes, with the three incidents of `results.md` (the cursor probe, the meta pane band's side, `--auto N`) and that #1232 had merged before this plan's PR. `results.md` stays in the folder as the run's record.

## Trade-offs and risks

- **`docs/agents/app.md` (list E) or learnings only.** The plan was asked for on the premise that batch 7's section was on an unmerged branch; it is on main now, so list E edits it. Option: drop list E and keep the three incidents in `learnings.md` only. That keeps the diff to `todo.md` and the plan folder, but leaves two "Not observed in the running app" bullets that this run settles, and a cursor rule that failed as written on the pane boundaries.
- **Pane box 5 narrowed, not deleted.** The click on the message passed, but `results.md` says the bands could not reach the message in that window, so the check of the bands against the click was not really made. Option: delete the box and keep the caveat in the intro only. The cost is that nobody is asked to look at a narrow window again.
- **Scrollbars box 4 narrowed, not deleted.** The box says "scroll them", which the wheel did. `results.md` lists the thumb drags as not done and never made the folder menu overflow, so the rule keeps them open. Option: delete the box on the wheel scrolling alone.
- **Pane box 1 and strip box 1 deleted on Neutral dark only.** The 1 px line was judged in one theme, 3 levels from the tree pane's fill. The boxes say "the theme", so they close; the intro records the limit. Option: narrow each to "in Neutral light and one vendored theme".
- **D1 is a decision item.** `results.md` rates the jump minor. Option: no item, the behaviour recorded in the pane item's intro only. D1's background names the drag threshold as read from the source, not measured; the implementer of D1 verifies it.
- **D2 is a decision item**; this plan does not choose between wrapping, truncating and a larger `MIN`.
- **Strip box 4 closes on a citation.** The strip-drag re-cut was not re-run in this batch; it rests on batch 7's box 7 of the same day and this run's pane-drag re-cut.
- `todo.md` names this plan's `results.md` by its archived path, true only after the wrap-up's move.
- `results.md` calls itself "batch 8" while the folder is named after its subject; the copy is verbatim, so the title stays.

## Progress

- 2026-11-22: Step 1 done. The run is in `results.md`, the notes in `learnings.md`. Edits made:
  - A (tree and meta pane resize item): boxes 1 to 4 deleted (hover, the first-move jump, release positions and restarts, `z` then a drag); box 5 narrowed to the bands against the no-folder message in a narrow window; box 6 (macOS and Linux) kept; the intro's unchecked clause replaced with the "Seen" sentence.
  - B (strip height item): boxes 1, 3, 4 and 5 deleted; box 2 narrowed to HiDPI sharpness; box 6 kept; the intro updated.
  - C (scrollbars item): boxes 1 to 3 deleted; box 4 narrowed to the thumb drags and the folder menu overflow not done; box 5 kept; two Background sentences appended.
  - D (`todo.md`): two new items after the pane item, the pane drag's offset from the boundary and the meta pane's mid-word breaks at 160 px.
  - E (`docs/agents/app.md`): a cursor-probe bullet, and `Seen (2026-11-22, Windows 11)` bullets for the panes' widths and the strip's height replacing the "Not observed" ones.
  - F (`learnings.md`): the three incidents and the note that #1232 had merged first.
- 2026-11-22: Wrap-up. Learnings extraction and todo curation found nothing further; no settings to promote. Plan archived.
