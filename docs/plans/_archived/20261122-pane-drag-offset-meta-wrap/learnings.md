# Learnings

## Step 1

- The press is recorded by a left-button `on_mouse_down` attached to the
  `ResizeHandle` itself (`pane_handle` / `strip_handle` now take `cx` and
  return `ResizeHandle<..>`). By gpui-base 0.9.1's `resize_handle.rs`, the
  handle's `InteractiveElement` forwards to the band `Div`'s
  `Interactivity`, which base lays out keeping every listener but `id`,
  `group`, the hitbox behavior and `on_drag`; base's own press listener is a
  separate `window.on_mouse_event` that does not stop propagation. So the
  press listener fires by the source; it was not seen in a running window,
  and the fallback (the first `DragMoveEvent` as origin) was not needed.
- `panewidths::dragged` now takes the wanted width instead of the pointer's
  `x`, with `panewidths::wanted(pane, start, press, x)` computing it;
  `stripheight::dragged(wanted, shared)` kept its signature, with
  `stripheight::wanted(start, press, y)` beside it. The strip's
  `on_drag_move` no longer reads `e.bounds.bottom()`, only the height for
  the `PREVIEW_MIN` clamp.
- `drag_pane` / `drag_strip` return early without a recorded press (a drag
  cannot start without a press on the handle, so this is only a guard of the
  `Option`).
- Tooling: a `python - <<'EOF'` heredoc with triple-quoted strings tripped the
  Bash tool's parser ("unexpected EOF while looking for matching `'`"); a
  script file in the scratchpad run with `mise x -- python <file>` worked.
  Locating a `todo.md` heading with `s.index('### App: ...')` matched an
  inline backticked reference to it first; anchor on a leading newline.

## Step 2

- The rows are `flex_wrap` with `gap_x_2` and the value `flex_grow(1.)` /
  `min_w_0` / `text_right` (basis auto), as the plan said; the name heading
  is a `flex flex_wrap` row of `meta::name_segments`, which is
  `str::split_inclusive(['_', '-', ' '])` (no empty segment for an empty
  name or a trailing separator, by `split_inclusive`'s contract).
- The layout was not seen in a running window (no screenshot run), so the
  new `docs/agents/app.md` rule stays (Inferred) and the `todo.md` box open.
- GPUI's `flex_grow` takes the factor (`flex_grow(1.)`), unlike the
  argument-free tailwind-style helpers beside it; the first `ci:local` run
  failed on that.

## Deferred issues (todo candidates)

- Pending manual check (Windows real device): press the tree's boundary, the
  meta pane's boundary and the strip's top edge off the line's own column or
  row (e.g. 3 px into the preview from the tree's line at 220), drag each in
  both directions. Expect the first 3 to 5 px to move nothing, then the line
  to keep the press's offset from the pointer (pressed at 223, line at 227
  for a pointer at 230), and a pointer back at the press to put the line back
  where it started. Recorded in `todo.md` as `### App: real-device check that
  a pane or strip drag keeps the press's offset from the boundary`. Step 1's
  checkbox was ticked on the automated criteria (unit tests in
  `crates/app/src/panewidths.rs` and `crates/app/src/stripheight.rs`, `mise
  run ci:local`). Basis: Step 1's "Trade-offs and risks" (the press listener
  is checked only by reading the source). Files: `crates/app/src/main.rs`
  (`pane_handle`, `strip_handle`, `press_pane`, `press_strip`).

- Pending manual check (Windows real device): with the meta pane at 160 px
  on a file with long values (`Electronic`, `Compressed RAW`, `Mechanical`,
  `Manual`) and a long name (`ILCE-1M2_sony_a1_ii_03.arw`), expect no break
  inside a word, each value that does not fit beside its label on its own
  right-aligned line under it, and the name broken after `_` / `-`; at the
  default 220 px, judge the look of the rows whose values now go under the
  label. Recorded in `todo.md` as `### App: real-device check that the meta
  pane keeps words whole at its 160 px minimum`. Step 2's checkbox was ticked
  on the automated criteria (`meta.rs` unit tests of `name_segments`, `mise
  run ci:local`). Basis: Step 2's "Trade-offs and risks" (the flex layout is
  checked only by reading GPUI's source). Files: `crates/app/src/main.rs`
  (`meta_pane`), `crates/app/src/meta.rs` (`name_segments`).
