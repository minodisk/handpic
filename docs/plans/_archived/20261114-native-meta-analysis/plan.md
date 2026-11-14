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

# GPUI meta pane: the Analysis group

## Purpose

The GPUI app's (`crates/native`) right-hand meta pane shows the `EXIF` and
`Maker note` groups but never an `Analysis` group: `meta_pane` in
`crates/native/src/main.rs` calls `meta::groups(meta, &[])` with an empty
analysis slice. The Tauri app shows what the scan computed for the shown file
there (`metaGroups` in `crates/app/ui/src/meta.ts`).

After this work the GPUI pane shows, for the file on screen, the values the
folder scan's pass 1 and pass 2 stored in the index: an `Analysis` group with
the rows the Tauri app has, with identical labels and value strings, plus the
computed values Tauri does not show, and the camera's AF point and AF frame
in the `Maker note` group. A file with no analysis value shows no `Analysis`
group.

Decisions already made by the user:

- The `Analysis` group uses only the values the scan stored. It never uses
  the on-demand eyes judgment (`eyes_of` / `marks.eyes(path)`), which Tauri
  falls back to. When the on-demand judgment runs does not change
  (`Marks::wanted_eyes` stays gated on the `f` overlay).
- The pane shows all the values pass 1 and pass 2 store, not only Tauri's six
  rows.
- `AF point` and `AF frame` go in the `Maker note` group, not in `Analysis`:
  the camera recorded them, and `Analysis` holds what Handpic computes.
- The personal ranking's rows (`Recommended`, `Reason`, `Confidence`) are out
  of scope: the native app has no ranking.

## Steps

- [x] Step 1: Show the scan's stored values in the GPUI meta pane (the `Analysis` group, and the AF point and frame in `Maker note`)
  - Done when:
    - Showing a RAW file pass 2 has scanned shows an `Analysis` group in the
      GPUI app whose Tauri-shared rows (`AF eye in focus`, `Eyes at full
      size`, `Eyes`, `Yaw`, `Pitch`, `Roll`) carry the same label and value
      string as `metaGroups` gives for the same stored values, together with
      the extra computed values listed below.
    - A file whose index row has an AF point shows `AF point`, and `AF frame`
      when one was recorded, in the `Maker note` group directly after
      `AF tracking`; the `Analysis` group holds neither.
    - A file with no analysis value shows no `Analysis` group, and a shown
      file whose index in `shown` is `None` (hidden by a refilter) shows
      neither the `Analysis` group nor the AF rows.
    - A file whose preview failed (`Shown::Failed`) still shows its
      `Analysis` group, as Tauri's does without `meta`, and a `Maker note`
      group holding only the AF rows when the index has them.
    - The rows appear without another key press when the scan reaches the
      file on screen.
    - The `metaGroups` cases of `crates/app/ui/src/meta.test.ts` that do not
      involve the ranking or the on-demand judgment are ported to
      `crates/native/src/meta.rs`'s tests, and the new rows have tests.
    - `CLAUDE.md`'s description of `crates/native/src/meta.rs` says what the
      `Analysis` group now holds, that the `Maker note` group also carries
      the index's AF point and frame, and where the values come from.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/meta.rs` (the row builders, `groups` and its
      tests), `crates/native/src/main.rs` (`meta_pane` only), `CLAUDE.md`.
      Nothing in `crates/backend`, `crates/core` or `crates/app` changes.
      `docs/humans/usage.md` describes the Tauri app's meta pane and
      `docs/agents/native-app.md` has no text about the group, so neither
      needs a change.
    - Data source: `Riffle.facts` (`filter::Facts`), its
      `focus: Option<index::Focus>` (whose `point` is the
      `index::FocusPoint`) and `sharpness: Option<f64>`. Every value below is
      already there, so no new index read is added. Do not read the overlay's
      `Marks.focus` (only read while `f` is on) and do not call
      `marks.eyes(..)`.
    - Target file: `self.shown` is `Option<(Option<usize>, Shown)>`, the
      index being into `self.files` and `None` once a refilter hid the file.
      Look the path up with `self.files.get(index)` and then `self.facts`;
      pass no rows when either is missing. `refilter` remaps the index with
      `filter::shown_after`. Verify during implementation that the open and
      re-listing paths never leave it pointing at another file of a new
      `files`.
    - Re-render: no change needed. The scan-event handler in `main.rs` ends
      with `cx.notify()` after `filter::apply_face_ready` in its
      `FacesProgress` arm, and after `read_entries` once pass 1 is done, and
      `meta_pane` reads `self.facts` on every render.
    - `groups` stays pure and gains one parameter, the Maker note rows that
      come from the index, in the same shape as `analysis`:
      `groups(meta, maker_note: &[(&str, Option<String>)], analysis: &[(&str, Option<String>)])`.
      - The Maker note group is `Shutter type`, `Focus mode`, `AF area`,
        `AF tracking` from `meta`, then the `maker_note` rows, then `Drive`
        through `Focus distance` from `meta`.
      - With `meta` `None` the group holds the `maker_note` rows alone, so it
        is built outside the `if let Some(m) = meta` block. This is the rule
        the `Analysis` group already follows: rows from the index do not need
        the file's metadata.
      - The group order stays `EXIF`, `Maker note`, `Analysis`; `None` rows
        and empty groups are dropped as before.
    - Add two pure builders in `meta.rs` whose results `meta_pane` passes to
      `groups`, for example:
      - `af_rows(point: Option<&index::FocusPoint>) -> Vec<(&'static str, Option<String>)>`
        for the Maker note rows;
      - `analysis(focus: Option<&index::Focus>, sharpness: Option<f64>) -> Vec<(&'static str, Option<String>)>`
        for the `Analysis` rows.
    - Maker note rows from the index (pass 1), after `AF tracking`:
      - `AF point`: `FocusPoint` in sensor coordinates,
        `3120, 2080 of 8640 × 5760`.
      - `AF frame`: `FocusPoint::frame`, `400 × 300`; no row without one.
      - Both show the stored point as it is, also on a manual-focus frame.
    - `Analysis` rows, in this order. It keeps Tauri's order, with the new
      rows placed next to the value they derive from:
      1. `AF eye in focus`: `eye_focus` as `87%` (Tauri's `focusPercent`).
      2. `Focus candidate` (new): `Yes` for `FocusCandidate::Candidate`, `No`
         for `NotCandidate`, no row for `Unknown`.
      3. `Eyes at full size`: `full_eye_sharp` as `Sharp · 0.162` /
         `Soft · 0.143`, cut at `mark::GOOD_FULL_EYE_SHARP` with `>=`
         (Tauri's `fullSharpValue`).
      4. `Eyes`: `Open · 41` / `Closed · 0`, from the stored state
         (`StoredEyes::eyes`) and the openness of `eyes_ear`; no row without
         an EAR (Tauri's `eyesValue`, stored branch only).
      5. `Eyes closed` (new): `eyes_closed`, the closed probability, as a
         percentage, formatted like row 1.
      6. `Eye aspect ratio` (new): `eyes_ear` with three decimals, `0.250`.
      7. `Yaw`, `Pitch`, `Roll`: the stored `pose` in whole signed degrees,
         `12°`, `-5°`, `0°` (Tauri's `degrees`).
      8. `Mesh eye offset` (new): `eye_offset` with three decimals, in face
         box sides.
      9. `Edge gap` (new): `edge_gap` with three decimals, signed, in face
         box sides.
      10. `Sharpness` (new): `Facts::sharpness`, the preview's Laplacian
          variance, with one decimal, `123.5`.
    - Pass 1 values left out: `manual_focus` (derived from the Sony
      `FocusMode` the Maker note group's `Focus mode` row already shows),
      the capture time and the shooting settings (already in the `EXIF`
      group), the orientation and the thumbnail.
    - Matching the JavaScript strings:
      - `Math.round(x)` is `(x + 0.5).floor()`. `f64::round` rounds halves
        away from zero and differs on negative halves (`-5.5` is `-5` in
        JavaScript). The floor form also never prints `-0` for an angle in
        `[-0.5, 0)`. Use it for the percentages, the openness and the
        degrees.
      - The openness is `eyesOpenness` of `crates/app/ui/src/focus.ts`:
        `(ear - EYES_CLOSED_EAR) / (EYES_WIDE_OPEN_EAR - EYES_CLOSED_EAR)`
        clamped to `[0, 1]`, times 100. `EYES_CLOSED_EAR` is
        `riffle_core::eyes::EYES_CLOSED_EAR` (0.137). `EYES_WIDE_OPEN_EAR`
        (0.41) does not exist in the native crate yet: add it next to the
        function in `meta.rs`, its only user.
      - `toFixed(3)` and `{:.3}` agree except on a value that is an exact
        decimal tie as a double (`0.0625`: `0.063` in JavaScript; Rust is
        expected to round half to even and print `0.062`). Test the tie
        during implementation and decide between matching JavaScript there
        and recording the divergence (see Trade-offs).
    - Existing tests in `meta.rs` call `groups(meta, &[])` or
      `groups(meta, &analysis)`; give them the new argument (`&[]`).
    - Tests to port from `meta.test.ts` (labels and values identical):
      - the Sony-like split into three groups with `AF eye in focus` `87%`;
      - the analysis group alone without metadata, and no group without a
        value;
      - `0.8701` gives `87%` and `0` gives `0%`;
      - an unknown candidate with no `eye_focus` gives no group;
      - the stored eye state, openness and pose (`0.09` closed gives
        `Closed · 0`, pose `12.4 / -5.2 / 3.5` gives `12°`, `-5°`, `4°`);
      - the openness figures (`0.25` gives 41, `0.3` gives 60, `0.45` gives
        100);
      - the pose `-110.2 / 0 / -1.6` gives `-110°`, `0°`, `-2°`;
      - a small negative angle (`-0.4`, `-0.2`, `-0.49`) gives `0°`;
      - the full-size score (`0.143` gives `Soft · 0.143`, the cut gives
        `Sharp · 0.162`, none gives no row);
      - `0.07` alone gives `7%`.
      The rows these Tauri cases produce now sit among the new rows, so
      assert on the shared rows by label or build the inputs so only they are
      set. The cases about the on-demand judgment (the fallback, and the
      stored values winning over it) and the ranking cases are not ported.
    - New tests:
      - each new `Analysis` row's format and its absence without a value;
        `Focus candidate` for the three states; a file with only a
        `sharpness` (no `Focus`) shows `Sharpness` alone; no `Focus` and no
        sharpness gives no `Analysis` group; a `Focus` with only an AF point
        gives no `Analysis` group;
      - `af_rows`: the `AF point` format, `AF frame` with and without a
        recorded frame, no rows without a point;
      - `groups`: the AF rows sit between `AF tracking` and `Drive`; with
        `meta` `None` the `Maker note` group holds the AF rows alone and
        comes before `Analysis`; with metadata that has no maker note field
        the group still shows for the AF rows; no AF rows and no maker note
        field gives no `Maker note` group.
    - Update the module comment of `meta.rs` and its `CLAUDE.md` description
      ("an `Analysis` group from the rows the caller passes") to name the
      stored values, the AF rows the `Maker note` group takes from the index,
      and that the on-demand judgment is not used.

## Trade-offs and risks

- **Where the AF rows sit in `Maker note`.** The plan puts them after
  `AF tracking`, with the other AF rows. Appending them after `Focus
  distance` is simpler (the metadata rows stay one block) but reads worse.
- **A `Maker note` group without metadata.** On a failed preview the group
  shows with only the AF rows, the same rule as `Analysis`. The alternative,
  hiding them without `meta`, would make index-sourced rows behave two ways.
- **The AF point on a manual-focus frame** is shown as stored, although the
  `f` mark does not draw it there. The `Focus mode` row two rows above says
  the frame was focused manually. Alternative: leave the AF rows out when
  `manual_focus` is set.
- **`manual_focus` is left out** as a duplicate of the Maker note's `Focus
  mode`. It is not strictly the same fact (the `DSC-` bodies whose
  `FocusMode` always reads 0 count as autofocus). Alternative: a `Manual
  focus` row with `Yes`, shown only when it is set.
- **`toFixed(3)` ties.** Matching JavaScript on an exact tie needs explicit
  handling; accepting the divergence touches only values that are exact
  dyadic ties, which a measured score practically never is. The step decides
  after testing the tie.
- **Labels and formats of the rows Tauri lacks** are this plan's choice.
  `Eye aspect ratio` could be `EAR`. `AF point` in sensor coordinates is the
  form the MCP `get_photo` tool reports, but may wrap in the pane; a
  percentage position is the alternative.
- **`Focus candidate` is derived** from `eye_focus` at the candidate cut, so
  it repeats part of `AF eye in focus`. It is included because the derived
  candidate was named among the values to show.
- **No on-demand fallback.** Unlike Tauri, a file pass 2 has not reached, or
  whose stored cue ran no mesh, shows no `Eyes` / pose rows even while the
  `f` overlay has a judgment for it. This is the user's decision.
- **A JPEG** has no `Focus` in the index unless it carries an AF point, so
  without one it shows at most `Sharpness`.
- **The Maker note group now differs from Tauri's**, which has no AF point or
  frame rows; the two apps' panes no longer list the same rows there.

## Progress

- (2026-11-14) Step 1 complete
