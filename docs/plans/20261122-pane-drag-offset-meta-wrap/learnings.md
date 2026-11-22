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
