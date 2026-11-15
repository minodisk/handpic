# Learnings

## Step 1

- The end of the drag is not seen by `on_drop::<PaneDrag>` on the root
  `div`. In gpui-pre 0.3.12 a drop listener runs on a `MouseUpEvent` only
  while its own hitbox is hovered (`div.rs`, the `drop_listeners` block), and
  the handle's band is `occlude`d, so at most releases (the pointer still on
  the band, which follows it) the root is not hovered. GPUI also clears
  `active_drag` on any mouse up after dispatch (`window.rs`). Instead the
  root registers `capture_any_mouse_up` (hovered) and `on_mouse_up_out`
  (left button, not hovered), which between them see every release the window
  receives; `drag_pane` sets `pane_dragged`, the first release after it
  writes `panelWidths` once. So the plan's "write at the drag's end" holds
  without the every-move fallback.
- `on_drag_move` runs in the capture phase for every move while a drag of
  that payload type is active, whatever the hitbox, so the root `div` follows
  the drag even when the pointer leaves the preview.
- The pointer `x` is taken as the boundary, as the plan says. The band is the
  preview's outer 5 px, so the first move can shift the boundary by up to
  5 px toward where the band was grabbed; not compensated (the handle's
  `on_drag` constructor sees the grab offset but cannot hand it to the
  payload without shared state).
- A stored width is clamped on read, and the dragged one is rounded to whole
  px, so the stored JSON stays integral (`f32` widened to `f64` would
  otherwise store digits like 233.3300018310547).
- The narrow window rule chosen: the shown panes give up the excess in
  proportion to their width over `MIN`; no pane is drawn under `MIN`, so a
  window too narrow for both minimums and `PREVIEW_MIN` takes the rest from
  the preview. Pinned by `panewidths.rs`'s tests.
- The limits stay at the approved working values (160 / 480 / 240); they were
  not checked by eye in the running app.
- `mise run fmt` failed first in this fresh worktree: `node_modules` was
  absent, so `vp fmt` could not load (`Cannot find module ...vite-plus/bin/vp`),
  after both `cargo fmt` runs had already applied. `mise exec -- pnpm install
  --frozen-lockfile` fixed it (plain `pnpm` / `node` are not on the Bash
  `PATH`; they come through mise).

## Deferred issues (todo candidates)

- **Pending manual check** (native pane resize, Windows first, then macOS and
  Linux): the Step 1 checkbox was ticked on the automated criteria (unit
  tests, `mise run ci:local`); nothing below was observed in the running
  app. Basis: Step 1's Done-when "What was not observed in the running app
  ... has a hand-check entry". Files: `crates/native/src/main.rs`
  (`pane_handle`, `drag_pane`, `end_pane_drag`, the root `div` in `render`),
  `crates/native/src/panewidths.rs`. Steps and expected results:
  1. `mise run native:dev` (or `native:watch`) on a folder. Hover the
     preview's left edge beside the tree: the cursor turns col-resize and the
     handle's pill shows; the 1 px line reads well on the theme.
  2. Drag it right and left: the tree follows the pointer between 160 and
     480 px and the preview takes the rest; same for the meta pane's edge.
     Note whether the boundary jumps a few px at the first move.
  3. With both panes wide, narrow the window: the preview keeps 240 px while
     the panes narrow (never under 160); widen it again: the panes come back
     to the dragged widths.
  4. Release the drag over the preview, over a pane, over the strip and
     outside the window; restart: the widths come back each time (for a
     release outside the window, note whether it is written at once or only
     at the next click inside).
  5. `Tab` (`toggleSides`), `Ctrl+Alt+Left` (`toggleLeft`) and
     `Ctrl+Alt+Right` (`toggleRight`) hide and show a pane: it comes back at its last width.
  6. `z` for 1:1, then drag a boundary: the crop is re-cut once the drag
     settles.
  7. The 5 px bands do not get in the way of the empty-state message's click,
     the preview's context menu or the panes' scrollbars; the tree's rows, its
     inline rename input and the meta pane's rows lay out at 160 px.
  8. `--auto`: the panes start at 220 whatever is stored and a drag writes
     nothing to `panelWidths`.
