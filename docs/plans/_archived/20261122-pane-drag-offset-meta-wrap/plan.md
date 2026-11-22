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

# Keep the press's offset through a pane or strip drag, and keep the meta pane readable at 160 px

## Purpose

The Windows real-device run of 2026-11-22
([results](../20261122-windows-device-checks-resize-scrollbars/results.md),
pane box 2 and box 5) left two findings in `todo.md`, and the user decided to
fix both:

- `### App: decide whether a pane drag keeps the press's offset from the boundary`:
  a drag of the tree's or the meta pane's boundary ignores the first 3 to 5 px
  and then puts the boundary under the pointer, so the press's offset inside
  the 5 px band is lost (pressed at x 223 with the line at 220, the line sat at
  230 for a pointer at 230). The strip's top edge follows the same code
  pattern.
- `### App: the meta pane breaks words mid-word at its 160 px minimum`: at
  160 px the values break inside words (`Electroni` / `c`, `Mechani` / `cal`)
  and the file name heading breaks mid-name (`ILCE-1M2_sony_a1_ii_0` /
  `3.arw`).

Once done, a boundary moves by the pointer's delta from the press, and the meta
pane at its minimum width shows every value whole, losing no information.

What the source says about the first finding (read at planning time, not
measured again):

- The no-move threshold is GPUI's. `gpui-pre` 0.3.12's `elements/div.rs` has a
  private `const DRAG_THRESHOLD: f64 = 2.;`: a drag starts in the bubble phase
  of the first mouse move more than 2 px from the press. `on_drag_move`
  listeners run in the capture phase and only while a drag is active, so the
  first `DragMoveEvent` is the move after the one that started the drag. That
  matches the run: press at 223, moves to 224 and 225 (within 2 px), 227
  (starts the drag), 230 (the first drag-move).
- The jump is the app's: `drag_pane` hands the pointer's absolute `x` to
  `panewidths::dragged` as the width, and `drag_strip` hands
  `bottom - pointer y` to `stripheight::dragged` as the pane height.
- `resize_handle` (gpui-base 0.9.1's `resizable/resize_handle.rs`, re-exported
  by gpui-component) implements `InteractiveElement` over its band's `Div`;
  its doc says listeners a caller attaches through the interactive element
  traits land on the band. Its own mouse-down listener does not stop
  propagation.

## Steps

- [x] Step 1: A pane or strip drag moves the boundary by the pointer's delta from the press
  - Done when:
    - A drag of the tree's boundary, the meta pane's boundary or the strip's
      top edge gives the width or height at the press plus the pointer's
      movement since the press (minus it for the meta pane and for the strip,
      whose pane grows as the pointer goes left or up), then the existing
      rounding and clamps. The pointer's absolute position is no longer taken
      as the width or the height.
    - Unit tests on the pure arithmetic in `panewidths.rs` and
      `stripheight.rs` cover: the measured case (a left pane 220 wide pressed
      at x 223, pointer at 230, gives 227); the right pane (width 220 in a
      1600 px window pressed at 1376, pointer at 1370, gives 226); a pointer
      back at the press position gives the starting width exactly; a drag past
      a clamp and back gives the same width as without the excursion (no
      drift); the strip (a pointer 50 px above the press gives a pane 50 px
      higher, before the clamps). The existing clamp and `PREVIEW_MIN` tests
      still hold, rewritten for the new parameters where the signature
      changed.
    - `docs/agents/app.md` is updated: in "The side panes' widths are one
      `PaneWidths`, stored under `panelWidths`" the rule that says the
      pointer's `x` is the left width is replaced by the press-relative rule,
      with a note that the first moves of a drag change nothing because of
      GPUI's 2 px `DRAG_THRESHOLD` and that the first drag-move arrives one
      move later (Inferred, from the source and the 2026-11-22 run); in "The
      strip's height is one number, stored under `stripHeight`" the "wanted
      pane height is that bottom less the pointer's `y`" rule likewise. The
      "Seen" bullet's reference to the `todo.md` item is updated to the item's
      new state.
    - `todo.md`: the item `### App: decide whether a pane drag keeps the
      press's offset from the boundary` no longer holds a decision box. It is
      narrowed to (or replaced by) one Windows real-device check box: press
      each pane boundary and the strip's top edge off the line's own column or
      row, drag, and expect the line to keep that offset from the pointer once
      it starts moving, in both directions, with the first 3 to 5 px moving
      nothing; the strip's edge was never measured before the fix. The
      references to the old heading in the `### App: real-device checks of the
      drag-resizable tree and meta panes` background follow the new heading.
    - `CLAUDE.md`'s Layout text is updated only if a name it lists
      (`dragged` of `panewidths.rs` / `stripheight.rs`) changes meaning or a
      new public helper is added.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Keep `resize_handle` and its `on_drag`; keep GPUI's threshold. It is a
      private constant, and a drag that starts only after 2 px keeps a plain
      click on the band from nudging the width. After the threshold the
      boundary catches up to the press-relative position on the first
      drag-move (pressed at 223 on a line at 220, the line is at 227 for a
      pointer at 230).
    - Record the press: attach `on_mouse_down(MouseButton::Left,
      cx.listener(..))` to the handles where `render` builds them
      (`pane_handle(..)` and `strip_handle()` return a `ResizeHandle`, an
      `InteractiveElement`), storing in the view the press's pointer
      coordinate and the starting size: for a pane its drawn width (the same
      `self.pane_widths.drawn(window width, self.panels)` `render` uses, so a
      pane narrowed by a small window does not jump either), for the strip
      the cells' height (`self.strip.geometry().height`) or the pane height
      derived from it with `HEADER` and `GUTTER`.
    - Verify during implementation that the handle's `on_mouse_down` listener
      fires (the band `occlude`s, and base replaces only `id`, `group`, the
      hitbox behavior and `on_drag`). If it does not, fall back to taking the
      first `DragMoveEvent` of a drag as the origin and say so in
      `docs/agents/app.md` and the PR; see "Trade-offs and risks".
    - The delta form needs no window geometry for the pane's origin: the left
      pane wants `start + (x - press)`, the right `start - (x - press)`, the
      strip pane `start - (y - press)`. It also drops the assumption that the
      panes are flush with the window's edges (the Linux client-side frame's
      padding). `panewidths::dragged` still needs the window width and
      `stripheight::dragged` the `shared` height for the `PREVIEW_MIN` clamps;
      the strip's `on_drag_move` keeps reading `e.bounds.size.height` for it.
    - Put the arithmetic in the pure modules so the tests cover it: either
      change `dragged` to take the wanted width (pane) and keep
      `stripheight::dragged(wanted, shared)`, with a small helper computing
      the wanted size from start, press and pointer, or give `dragged` the
      press parameters. The step decides; `main.rs` keeps only the wiring.
    - `pane_dragged` / `strip_dragged` and `end_pane_drag` (the one write at
      the release) stay as they are. A press without a drag leaves a recorded
      press behind, which the next press overwrites.
    - Files: `crates/app/src/main.rs` (the view's fields near `pane_dragged`,
      `drag_pane`, `drag_strip`, the two `on_drag_move` listeners and the
      handle call sites in `render`), `crates/app/src/panewidths.rs`,
      `crates/app/src/stripheight.rs`, `docs/agents/app.md`, `todo.md`.
    - Not on the page path (a drag is not paging): no A/B latency gate.
- [x] Step 2: The meta pane wraps a value under its label and breaks the file name at its separators
  - Done when:
    - In `meta_pane`, a row whose value does not fit whole beside its label
      puts the value on its own line under the label, right-aligned, where it
      wraps at spaces if it is still wider than the pane. A row whose value
      fits looks as it does today (label left, value right, one line).
    - The file name heading (the `Shown::Read` and `Shown::Failed` name alike)
      breaks after `_`, `-` or a space instead of mid-name; a single segment
      wider than the pane still wraps inside itself, so nothing is cut off or
      elided.
    - `panewidths::MIN` stays 160.
    - A pure helper in `meta.rs` splits a file name into its segments, with
      unit tests: `ILCE-1M2_sony_a1_ii_03.arw` gives `ILCE-`, `1M2_`, `sony_`,
      `a1_`, `ii_`, `03.arw`; the segments concatenate back to the name; a
      name without a separator is one segment; an empty name and a name
      ending in a separator do not give an empty segment.
    - `docs/agents/app.md` is updated: "The meta pane is `div` rows, and its
      cost lands in `present`" (or a new rule beside it) says what a row does
      when the value does not fit and how the name is split, marked
      (Inferred) since nobody has seen it in the running window; "The meta
      pane does not use gpui-component's `DescriptionList`" has its "today's
      look" sentence corrected; the `PaneWidths` section's "Seen" bullet
      follows the `todo.md` item's new state.
    - `todo.md`: the item `### App: the meta pane breaks words mid-word at its
      160 px minimum` no longer holds a decision box. It is narrowed to (or
      replaced by) one Windows real-device check box: with the meta pane at
      160 px on a file with long values (`Electronic`, `Compressed RAW`,
      `Mechanical`, `Manual`) and a long name, expect no break inside a word,
      wrapped values on their own right-aligned line, and the name broken
      after `_` / `-`; at the default 220 px, look at the rows whose values
      now go under the label and judge the look. References to the old heading
      elsewhere in `todo.md` follow.
    - `CLAUDE.md`'s `src/meta.rs` sentence names the new helper if one is
      added there.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Rows: add `flex_wrap` to the row `div` and let the value grow
      (`flex_grow`, basis left at auto, keeping `min_w_0` and `text_right`),
      with the 8 px gap only between columns (`gap_x_2`), so a wrapped row
      gets no extra vertical gap. This relies on two things read from the
      source, not measured: GPUI's text element measures at its full
      single-line width unless the available width is definite
      (`elements/text.rs`: `wrap_width` is `None` under min-/max-content), so
      the value's flex basis is its whole width and the flex line breaks when
      label + gap + value exceeds the row; and on its own line the value
      shrinks to the row (`min_w_0`) and wraps at UAX #14 break opportunities
      (`LineWrapper::break_indices`, `icu_segmenter`). Do not set `flex_1` on
      the value: its 0 basis would stop the line break.
    - Name: render the segments as children of one `flex flex_wrap` `div`
      with no gap, each segment `min_w_0`, under the heading's
      `FontWeight::SEMIBOLD`. No zero-width space: how the fonts draw U+200B
      on the three platforms is unverified.
    - The pane's content is 132 px wide at 160 (`pl_3` and the 16 px
      `SCROLLBAR_TRACK`); the labels stay `flex_none`.
    - Layout cannot be asserted by the unit tests (the UI tests cover no
      pixels). If the implementer can run the debug build with
      `--features screenshot` and synthetic input ("Driving the window in a
      real-device run on Windows" in `docs/agents/app.md`), a capture at
      160 px may turn the rule to (Measured) and settle the `todo.md` box;
      otherwise leave it (Inferred) and the box open.
    - Files: `crates/app/src/main.rs` (`meta_pane` only),
      `crates/app/src/meta.rs`, `docs/agents/app.md`, `todo.md`.
    - Independent of Step 1; the order is only for review size.

## Trade-offs and risks

- Step 1, the threshold. Kept: the boundary does not move for the first moves
  of a drag (GPUI's 2 px threshold plus one move event), then moves to the
  press-relative position at once. Removing it would mean bypassing
  `resize_handle`'s `on_drag` with own mouse-down / mouse-move listeners,
  which gives up the component's drag state, cursor and payload plumbing for a
  few px; not recommended.
- Step 1, where the origin comes from. Recommended: the press (a mouse-down
  listener on the handle), so the boundary keeps the press's exact offset.
  Alternative: the first `DragMoveEvent` of a drag as the origin. It needs no
  press listener, but the boundary then stays behind the pointer by what the
  pointer travelled before that event (7 px in the measured sequence) for the
  whole drag. It is the fallback only if the handle's listener does not fire.
  A third option, a root-level capture of every left press, works too but
  records presses that have nothing to do with the handles.
- Step 1 changes the meaning of `dragged`'s parameter (or adds a helper), so
  its existing tests are rewritten, not only added to.
- Step 2, the options of the `todo.md` item. Chosen by the user on 2026-11-22:
  the value on its own line under the label (`flex_wrap`), which loses no
  information and keeps `MIN`. Not taken: an ellipsis (hides the value exactly
  where the pane is narrow, and the pane has no tooltip), and a larger `MIN`
  (the label plus the longest word needs about the 220 px default, so the pane
  would barely shrink, and stored widths under the new minimum would be
  clamped).
- Step 2, a visible side effect at every width: a value that does not fit
  whole beside its label now goes under the label, where today it wrapped at
  spaces inside the value column. At 220 px that moves long values (a long
  lens name, a date with a time zone) onto their own line. Flexbox cannot
  express "wrap only when the longest word does not fit" without measuring
  text. The hand check at 220 px is there to judge this.
- Step 2, the file name. Recommended: segments after `_`, `-` and space in a
  wrapping row (lossless, no special characters). Alternatives: leave the
  glyph-level wrap (lossless but the finding stays), a middle ellipsis (one
  line, loses characters), U+200B after the separators (one text element, but
  its rendering is unverified). Whether the extension should also be kept from
  splitting off (`03` / `.arw`) is not an issue with the recommended rule: `.`
  is not a separator.
- Step 2 and the A/B latency gate. The pane is drawn on every page, so by the
  letter of "A same-session A/B gates only work on the page path" this is a
  rendering change. Decided: no A/B. The change is one style flag per row and
  about five more text `div`s for the name, against a measured pane cost of
  about 0.2 ms in `present`; the gate needs the machine idle for 30 to 60
  minutes; and the same document already declined the gate for a pane change
  on cost grounds.
- Both steps leave their real-window behaviour unverified by CI: the flex
  layout and the press listener are checked only by reading the source. The
  `todo.md` boxes each step leaves are the pending hand checks (Windows; macOS
  and Linux stay under the existing boxes of the drag-resizable pane and strip
  items).

## Progress

- (2026-11-22) Step 1 complete
- (2026-11-22) Step 2 complete
