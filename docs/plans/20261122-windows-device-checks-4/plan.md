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

# Record the fourth batch of Windows real-device checks (2026-11-22, strip and filter group)

## Purpose

A fourth real-device run on Windows 11 on 2026-11-22 exercised the pending
manual checks of the strip and filter group: the sort order, the filter and
its menus, the EXIF filter sections, the empty state, the status row and its
progress circle, the bursts, the filter menu's icons, the strip's face focus
icon and tooltips. This plan removes the `todo.md` boxes the run settled,
narrows the rest to what stays open, adds the three questions the run raised
as new items, and records what was seen in `docs/agents/app.md` where a rule
covers it. Docs and todo only, no code change: one step, one PR (single-PR
mode). It follows `docs/plans/_archived/20261122-windows-device-checks-3/plan.md`.

The run's results are in [`learnings.md`](learnings.md) in this folder (Step 1
moved them there from the run's `results.md`). They are the source of truth:
this plan names their sections and does not restate them.

## Rule for sub-points the run did not exercise

A sub-point of a box this plan deletes or narrows that `results.md` does not
mention, or lists as not run, was **not** run and must stay open: carry it
into the reworded box (or keep the box). Known cases (check for others while
editing):

- gpui-filter-sort: the strip starting empty and filling in during
  `analyzing n / m` (Step 3); a click on a menu item not also selecting a cell
  under it (the menu never overlapped the strip), a held `→` paging
  continuously (synthetic keys do not repeat), a narrowed strip on
  `menu\faces` after a `p`, the current file staying in view after `Sort ▾`.
- EXIF: the `Lens` section, the sections appearing after pass 1 on a fresh
  index, the name following the position, unchecking the item (only `Reset`
  was run), the `shown_after` race box.
- Empty state: rebinding / unbinding `open`, the click on the settings modal,
  the failed-preview message showing again on a second visit.
- Status row: the circle through pass 1 and its restart at empty, the 180 ms
  drop, paging cost, the strip-and-left-pane-hidden layout, the preview canvas
  not resizing, the long status text box, the macOS / Linux box.
- Filter menu icons: the headings not reacting to hover or a click; HiDPI.
- Face icon and tooltips: the icon appearing during pass 2 with no input,
  nothing on neutral frames or JPEGs, 150 % scale, the rejected-good dimming,
  the 500 ms delay, the 360 px wrapping, a failed and labeled cell, and boxes
  4 and 5 whole.
- Status row with no current-file info: `DROP_TEXT` and a scan suffix at the
  row's left edge.

## Steps

- [x] Step 1: Apply the run's results to `todo.md` and `docs/agents/app.md`, and write `learnings.md`
  - Done when:
    - Every edit in the lists A to D below is applied (with the rule above).
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files, and no `todo.md` item outside the lists changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where a box went.
    - The PR description records what was seen for each removed box, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Locate every item by its `###` heading, never by line number. "Box N" counts the item's top-level `- [ ]` lines from 1. Keep every heading, the neighbouring items, and every sentence this plan does not name, byte-for-byte. A box with nothing left open is deleted; no item below is deleted whole.
    - Write in English, in the file's own style. Every "seen" sentence added to a Background says "on Windows 11 on 2026-11-22" and names the results as `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md` (in backticks, no Markdown link to a plan folder). Items whose background is plain paragraphs under the heading (no `#### Background` line) get the sentence there; do not add the heading.
    - As in the earlier batches, a Python script written to the scratchpad and run with `python -I`, asserting one match per `###` anchor, keeps the neighbours byte-for-byte.
    - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the strip and filter`.
    - **A. Narrow in `todo.md`** (each Background gets a short 2026-11-22 summary of its `results.md` section):
      1. `### App: real-device checks of the strip's sort order, the filter and the filter / sort menus`. Delete boxes 1 and 2. Box 3 keeps only the fill-in during pass 2. Box 4 keeps only its unexercised points. Background also says: the good set is now `ILCE-1M2_sony_a1_ii_03` and `_DSC1758` (`DSC00405` is no longer Good; the rule changed after the check was written), the mark is the green `scan-face` icon, and a strip cell click that closes the menu also selects the cell.
      2. `### App: real-device checks of the EXIF filter sections and the refilter fixes`. Delete boxes 2, 3 and 6. Narrow boxes 1, 4 and 5 to the known cases above. Keep box 7 unchanged.
      3. `### App: real-device checks of the empty state in the preview area`. Delete boxes 2 and 4. Narrow boxes 1 and 3.
      4. `### App: real-device checks of the status row's StatusBar and progress circle`. Delete box 1. Narrow boxes 2 and 3. Keep boxes 4 and 5. Background notes the position shift (item C2) and the verbatim prefix (item C1).
      5. `### App: real-device checks of the burst bands, badges, burst keys and reject the rest`. Boxes unchanged (macOS stays open). Background: both checks passed on Windows on a Sony folder with `.xmp`; not run there: a Leica whole-second folder, the keys under a non-capture sort, in a JPEG folder and with the tree holding the keyboard, a `.dop`, the two-frame burst's undo move.
      6. `### App: real-device checks of the filter menu's icons and headings (gpui-filter-menu-icons)`. Delete boxes 2 and 3. Box 1 keeps only the headings not reacting to hover or a click. Keep box 4. Background: the color label rows are now gpui-component `Tag`s, not squares.
      7. `### App: real-device checks of the strip's face focus icon and cell tooltips`. Narrow boxes 1 and 3. Keep boxes 2, 4 and 5. Box 6 (the pending-rename tooltip): delete it, since `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md` records the tooltip `Renames when the scan finishes` seen on hover; confirm there first.
      8. `### App: real-device check of the status row with no current-file info`. Narrow the box to `DROP_TEXT` and the scan suffix.
    - **B.** The `### App: the Import dialog's `Source:` line shows the Windows verbatim prefix` item covers only the dialogs' `Source:` lines in `folderops.rs`; the status line's text comes from `format!("no RAW files in {}", listed.dir)` in `crates/app/src/main.rs`. C1 is its own item right after that one, each naming the other.
    - **C. Add three items to `todo.md`**, each with `#### Background` (ending in a `Files:` line) and `#### TODO`:
      - C1 `### App: the status line's `no RAW files in` text shows the Windows verbatim prefix`. Background: seen on 2026-11-22 after a folder emptied (`no RAW files in \\?\D:\Photos\tests\20261122-strip\cold`), and in batch 3 on an empty folder; `context.rs`'s `shown_path` strips the prefix elsewhere. Files: `crates/app/src/main.rs`, `crates/app/src/context.rs`. TODO: show the path through `shown_path`, and check the other status texts that print a folder path.
      - C2, after the StatusBar item: `### App: decide whether the status row reserves the progress circle's slot`. Background: the position text shifts right by about 20 px when the circle goes; `docs/agents/app.md`'s progress circle rule records the shift as the current behaviour. Files: `crates/app/src/main.rs`. TODO: decide; if reserved, keep the slot and update the rule; if not, record the decision in the rule.
      - C3, after the face icon and tooltips item: `### App: decide whether a strip cell's name longer than the cell ends in an ellipsis`. Background: long names are centered and clipped on both sides (the example in `results.md`). Files: `crates/app/src/main.rs`, `crates/app/src/strip.rs`. TODO: decide; if so, truncate with an ellipsis.
    - **D. Record in `docs/agents/app.md`** (find each section by heading; keep headings and tags; one short `Seen (2026-11-22, Windows 11)` bullet each):
      - The sort and filter menus section: Escape, a press on the preview, on a strip cell and on the menu's own button each close it (the button's stays closed); the strip cell press also selects that cell; the pager and `e` / `f` / `z` work with the filter menu open. Narrow its "Not verified headless" bullet to what stays: a press on an item reaching its `on_click` and not the strip.
      - The preview area's empty state section: the no-folder message and its click opening the picker, the no-files message's click doing nothing, the failed-preview message on the D70 NEF and `z` hiding it. Narrow its "Not verified" bullet to the key clause following a rebinding and the settings-modal click.
      - The progress circle section: the fill through pass 2 and a 600-file Move to Trash run, the circle going at the end, the position shifting about 20 px with it; pass 1 was too short to see.
      - The `rejectRest` rule in "A judgment goes view, row, writer": under a Picked filter the hidden member was rejected, the current file stayed, nothing advanced, the undo and redo named it as hidden by the filter, and a singleton did nothing.
    - **E. Write `learnings.md`** with the content of `results.md` (setup, results, the incident) and this step's notes, then delete `results.md` and point this Purpose at `learnings.md`.

## Trade-offs and risks

- C1 is a new item, not an extension of the Import `Source:` item: the code path differs, though the fix (`shown_path`) is the same.
- Headings are kept as they are; unchanged headings keep other references valid.
- The burst item gets no Windows box for the points not run on Windows; they are named in its Background only.
- Stale "Not verified" bullets in `docs/agents/app.md` outside list D stay, as batches 2 and 3 decided.
- C2 and C3 are decision items; this plan makes neither decision.
- `todo.md` names this plan's `learnings.md` by its archived path, true only after the wrap-up's move.

## Progress

- Step 1: applied the run's results to `todo.md` (lists A to C: boxes removed or narrowed in eight items, the three new items C1 to C3) and `docs/agents/app.md` (list D), moved `results.md` into `learnings.md` and deleted it.
