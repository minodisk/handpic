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

# Fix four todo.md items: the thumbnail panic text, the black 1:1 view, the rewrite dialog's "not in the index" row and the strip under a shorter window

## Purpose

Four `todo.md` items from the Windows real-device checks of 2026-11-21 and
2026-11-22 are closed with a code fix and a test each:

1. `App: a strip cell of a file whose thumbnail encode panics reads the
   internal panic text and the Windows verbatim path`. The cell and its hover
   text read `panic while encoding the thumbnail of \\?\D:\...\file.dng`.
2. `App: the 1:1 view of a file whose crop read fails is a plain black
   canvas`. Under `z` nothing on the canvas says the crop read failed.
3. `App: Rewrite Sidecars from Index reported "4 files not in the index
   (skipped)" in a fully scanned folder`. The row's wording does not match
   what is counted.
4. `App: a shorter window leaves the preview under PREVIEW_MIN until the
   strip is dragged`. Nothing clamps the drawn strip when the window shrinks.

Once done: a malformed embedded JPEG gives the strip cell a plain reason with
no path, the 1:1 view says its read failed, the rewrite dialog's third row
says what it counts, and the preview keeps `stripheight::PREVIEW_MIN` in a
short window without the stored `stripHeight` changing.

Out of scope: every other `todo.md` item (other sessions edit `todo.md`
concurrently, so touch only these four headings and what this plan adds), and
`docs/plans/20261123-all-faces-check`.

### Decisions (approved by the user on 2026-11-23)

- Item 1: the panic payload's text is the message; a non-`String` payload
  gives `the thumbnail could not be made`. `EXTRACTOR_VERSION` is not bumped.
- Item 2: a dedicated state and text keyed to the crop read's own failure.
- Item 3: reword the row, count unchanged.
- Item 4: the user's height is kept; only the drawn height is clamped.

### Investigation results (2026-11-23)

- Item 1. `crates/core/src/scan.rs` `extract_unless` wraps the thumbnail
  encode in `catch_unwind` and, on a panic, formats
  `panic while encoding the thumbnail of {path.display()}`, discarding the
  payload. mozjpeg 0.10's error manager unwinds with
  `resume_unwind(Box::new(String))` holding `libjpeg fatal error: <message>`.
  `crates/core/src/partial.rs` `guarded` and `crates/app/src/main.rs`
  `guarded` already downcast that payload to `String`. The strip cell's text
  and its hover text are the same string: the index's error row, through
  `thumbs.rs` `read`, `Thumbs::failure`, `strip_cells`' `failure`, into
  `paint_failure` and `strip::tip_regions`. No other message of the extract
  path carries the path.
- Item 2. `crates/app/src/main.rs` `render` computes `preview_failed` as
  "the current file's `Shown::Failed`" `&& !self.zoom.is_on()`, so the
  message is hidden under `z`. `crop_arrived`'s `Arrived::Failed` only calls
  `self.fail(..)` (the status line). `zoom::Zoom` keeps no record that the
  current page's crop read failed, and `zoom_paint` then has neither a crop
  nor, when the preview failed too, a placeholder.
- Item 3. Root cause pinned from code. `foldersidecars::targets` counts the
  `ratings` rows of the listed RAW files (`Index::rows_of`), not the `files`
  rows. A `ratings` row exists only once a file is judged in the app
  (`set_rating`) or its sidecar is parsed at a folder open
  (`store_sidecar_ratings`); clean rows are also dropped by `evict_folder` /
  Clear Cache and `reset_sidecars`. A scanned file that was never judged and
  had no sidecar therefore has a `files` row and no `ratings` row, and is
  counted as `skipped` and shown as `not in the index`. The 2026-11-22 run
  did not reproduce it presumably because all 12 files had a `ratings` row.
  The count is consistent with what a rewrite does (nothing is written for
  such a file); only the wording is wrong.
- Item 4. The view's only strip height is `self.strip.geometry().height`.
  `stripheight::dragged` clamps a drag against the `shared` height the drag
  listener reads from the bounds of the container holding the middle row and
  the strip. `observe_window_bounds` re-clamps the strip's width only.
  `stripHeight` is written only in `end_pane_drag`, after a strip drag.

## Steps

- [x] Step 1: Fix the four behaviours, pin each with a test, and curate the four `todo.md` items
  - Done when:
    - Item 1: a file whose embedded preview is not a JPEG fails pass 1 with
      libjpeg's message from the panic payload (e.g. `libjpeg fatal error:
      Not a JPEG file: starts with 0x00 0x00`) and no path. A panic whose
      payload is not a `String` gives a short fixed reason with no path. A
      unit test in `crates/core/src/scan.rs` asserts the message of a
      non-JPEG preview starts with `libjpeg fatal error:` and contains
      neither `panic` nor the file's path.
    - Item 2: with `z` on and the current file's crop read failed and no crop
      of that file held, the preview area shows a message centered over the
      canvas. It goes away when another page is current, a later read of the
      file succeeds or `z` is turned off (the fit view then shows its own
      state as before). Unit tests cover the `Zoom` state's new rule in
      `crates/app/src/zoom.rs` and the empty state and its text in
      `crates/app/src/empty.rs`.
    - Item 3: the rewrite dialog's third row no longer says `not in the
      index`; it says the files have no judgment on record and are skipped.
      The unit tests of `crates/app/src/sidecars.rs` assert the new text.
      A unit test in `crates/engine/src/foldersidecars.rs` pins the cause: a
      listed RAW that pass 1 indexed (a `files` row) but that has no
      `ratings` row is counted in `skipped`, not in `unjudged`.
      `docs/humans/usage.md` and `docs/humans/usage.ja.md` describe the third
      count the same way in the same PR.
    - Item 4: the strip is drawn no taller than leaves the preview
      `PREVIEW_MIN` in the height the two share, never under `MIN`. The
      height the user set is kept, so a taller window draws it again and
      `stripHeight` is not written by a window resize. A unit test in
      `crates/app/src/stripheight.rs` covers the rule: a height that fits is
      unchanged, the todo's case (302 px cells with too little shared
      height) leaves the preview `PREVIEW_MIN`, and the result is never
      under `MIN`.
    - `CLAUDE.md`'s layout text and `docs/agents/app.md` (the strip height
      section, and the 1:1 view / empty state text where they describe the
      changed rules) match the code.
    - `todo.md`: the four headings are removed. One manual-check item is
      added in their place holding only what needs a real machine (see the
      approach). No other item is touched.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Item 1. `crates/core/src/scan.rs`, the `Err(_)` arm of the
      `catch_unwind` in `extract_unless`: downcast the payload to `String`
      as `partial.rs`'s `guarded` does, and use it as the `Failure` message.
      The existing test
      `a_file_whose_preview_is_not_a_jpeg_is_one_error_not_a_crash`
      (`fixture(6, &[0u8; 512])`) is the place to assert the text. Verify
      during implementation that this fixture takes the panic arm and not
      `Ok(Err(..))`. No change is needed in `crates/app/src/strip.rs` or
      `context::shown_path`: the path leaves the message at its source, and
      the hover text is the same string. `EXTRACTOR_VERSION` is not bumped.
    - Item 2. `crates/app/src/zoom.rs`: `Zoom` records the page (`seq`)
      whose read failed in `arrived`'s `Err` arm, and a query answers
      "the current page's read failed and no crop is held". After
      `Arrived::Failed` the held crop is either none or the current page's
      own, the latter being a failed re-cut after a resize, whose crop stays
      on screen with no message. A page bumps `seq`, which ends it; a
      successful `arrived` clears it. `crates/app/src/empty.rs`: a new state
      and text for it (e.g. `State::ZoomFailed`, `The 1:1 view of this file
      could not be shown.`), after the existing cases in `state`'s order.
      `crates/app/src/main.rs` `render`: feed it next to `preview_failed`;
      `empty_message` already centers any state's text over the canvas.
      Keep `crop_arrived`'s status line as it is. If a UI test is wanted on
      top of the pure tests, `uitest.rs`'s `dng` fixture would need a
      variant with a non-JPEG preview and the view a readable accessor for
      the empty state; add it only if it stays small.
    - Item 3. `crates/app/src/sidecars.rs` `rewrite_rows`: reword the third
      row, e.g. `{n} files with no judgment on record (skipped)`. Update the
      two tests there that quote the old text. In
      `crates/engine/src/foldersidecars.rs`, correct the doc comments of
      `Preview` / `targets` where they call these files unknown to the
      index, and add the test (the `Fixture` there has `raw` and `row`; the
      `files` row can be written through the index's batch write, or the
      test can state in its name and comment that the scan's row does not
      enter the count, whichever the fixture allows cheaply). The refusal
      `No judgments indexed for <name>` is kept. Update the sentence in
      `docs/humans/usage.md` ("the RAW files the index does not know yet
      (skipped)") and its Japanese pair.
    - Item 4. `crates/app/src/stripheight.rs`: a pure `drawn(height,
      shared)` in the manner of `panewidths::drawn`, with the same bound
      `dragged` uses (`shared - PREVIEW_MIN - HEADER - GUTTER`, never under
      `MIN`). `crates/app/src/main.rs`: the view keeps the user's height
      apart from the strip's geometry (today the geometry is the only copy,
      so clamping it in place would lose the height for a taller window).
      The geometry is set to `drawn(..)` through `Strip::set_geometry`
      followed by `strip_moved` and `zoom_resized`, as `drag_strip` does.
      It has to hold at startup too (a stored 302 px in a small restored
      window), not only on `observe_window_bounds`. `press_strip` starts
      from the drawn height (as `press_pane` starts from the drawn width),
      a drag sets the user's height, and `end_pane_drag` keeps writing only
      after a strip drag. Nothing is read or written under `--auto`, as now.
      To decide during implementation: where `shared` comes from outside a
      drag (see Trade-offs). Measure that the value used equals the
      `e.bounds.size.height` the `StripDrag` listener sees.
    - `todo.md` manual-check item (Windows, real machine), one heading, with
      these boxes:
      - on `NIKON_D70_Nikon.nef` / `CGO3P_YUN00007.dng`, the strip cell and
        its hover text read the libjpeg reason with no path;
      - `z` on those files shows the message over the canvas, and paging to
        a good file shows its crop;
      - with the strip at `MAX`, dragging the window's bottom edge up keeps
        the preview at 240 px or more, dragging it back down gives the strip
        its height back, and a relaunch still has the stored height;
      - the rewrite dialog's row in a folder with never-judged files reads
        the new wording.
      Check that no other `todo.md` item refers to the four removed
      headings by name before deleting them.
    - Commit type `fix`, Conventional Commits, English.

## Trade-offs and risks

- **Item 3: reword or change the count.** Taken here: reword, count
  unchanged. The alternative is to count a listed RAW with a `files` row and
  no `ratings` row as "without a judgment" and have the rewrite clear its
  sidecar. That needs `mark_dirty` (an `UPDATE`) to create rows, and it would
  clear a sidecar the index has never read (one written by another app since
  the last open), which can destroy a judgment. The reword has no such risk.
- **Item 1: bump `EXTRACTOR_VERSION` or not.** Without a bump, an error row
  already in a user's index keeps the old text (with the path) until Clear
  Cache or a change of the file. A bump (13 to 14) refreshes them but
  re-extracts every row of every folder on its next open, for a text-only
  change. Taken here: no bump.
- **Item 1: the fallback wording.** A non-`String` payload could be any Rust
  panic in the thumbnail code, so "the preview is not a JPEG" could be false.
  Taken here: `the thumbnail could not be made`.
- **Item 2: a dedicated message or the fit view's message under `z`.**
  Dropping `&& !self.zoom.is_on()` would draw "The preview of this file could
  not be shown." over a good 1:1 crop when the preview read fails and the
  full-size JPEG reads, and would still show nothing when only the crop read
  fails. Taken here: a state keyed to the crop read's own failure.
- **Item 4: where the shared height comes from.** (a) Compute it in `render`
  from the window's viewport height less the client-side frame's padding,
  `TITLE_BAR_HEIGHT` and `STATUS_ROW_HEIGHT`: correct from the first frame,
  but it must match the layout exactly. (b) Record the shared container's
  bounds during paint and clamp on the next frame: exact, but one frame late
  and it needs a notify. Left to the implementation, with the measurement
  named in the approach.
- **Item 4: the strip's cells change size on a window resize.** The
  thumbnail range and a 1:1 crop are recomputed as in a strip drag; a
  window-edge drag therefore triggers them repeatedly. `zoom_resized` is
  already debounced and the window resize already calls it.
- **One PR for four unrelated fixes.** Chosen for the small size of each;
  it makes a partial revert coarser.

## Progress

(2026-11-23) Step 1 complete
