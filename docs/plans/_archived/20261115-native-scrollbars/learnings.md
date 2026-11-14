# Learnings

## Step 1

- The strip's handle lives in its own module, `crates/native/src/stripbar.rs`
  (`StripBar`), since `strip.rs` imports no gpui type. `Strip` gained a public
  `content_width` (`len * CELL_PITCH`), which `max_offset` now uses.
- The handle's `offset()` reports the model's synced offset, not a pending
  request. During a drag the bar compares `offset()` with the new drag offset
  and calls `set_offset` again on every move until the next render syncs it;
  that only overwrites the pending request with the newest value, so it is
  harmless. `drag_offset` computes the scrolled axis from the pointer alone.
- The tree's bar sits on a `relative()`, `size_full()` wrapper of the
  `uniform_list` inside the pane's `py_1` padding rather than on the pane
  itself. On the pane, the bar's viewport would be 8 px taller than the
  list's while `UniformListScrollHandle::content_size` is the list's viewport
  plus its max offset, so the bar would show only past 8 px of overflow and
  its thumb would be off by the padding. A press on the shown bar still stops
  before the pane's `on_mouse_down`, as the plan's Trade-off 3 says.
- The meta pane's `overflow_y_scrollbar()` compiled on the `Stateful<Div>`
  body (`id("meta")`) as is; `overflow_y_scroll()` came off the body.
- `cargo` is not on the Git Bash `PATH` here; `~/.cargo/bin` has to be
  prepended for a direct `cargo test` (the mise tasks find it).
- `mise run fmt` failed in this fresh worktree on `vp fmt` (no
  `node_modules`); `mise exec -- pnpm install --frozen-lockfile` fixed it.

## Deferred issues (todo candidates)

- **Pending manual check (Step 1, the three scrollbars)**, on Windows (and
  ideally macOS / Linux) with `mise run native:dev` on a folder of a few
  hundred files and a deep folder tree. The step's checkbox was ticked on
  the automated criteria (`stripbar.rs`'s unit tests, `mise run ci:local`);
  the agent has no screen. Source: the plan's Step 1 Done-when, the
  implementation in `crates/native/src/main.rs` (`tree_pane`, `meta_pane`,
  `render`) and `crates/native/src/stripbar.rs`. Verify:
  1. The strip, the tree and the meta pane each scrolled by the wheel: the
     bar shows and fades out about 2 s after the scroll stops.
  2. Within the time a bar is shown, dragging its thumb and clicking its
     track scroll the area, on each of the three.
  3. On the strip: after dragging the thumb to the far end of a long folder,
     the thumbnails there appear; the bar shows with the thumb following the
     arrow keys (the current file scrolled into view), a refilter and a
     window resize; a press on the shown bar does not change the current
     file or the selection, and with the bar faded out a press there selects
     the cell as before. The cells' tooltips still show on hover.
  4. On the tree: a row click, a right-click and the inline rename on a row
     next to the bar behave as before.
  5. No bar with short content: a short folder's strip, a short tree, a
     short meta pane.
  6. The settings modal's lists and the sort, filter and right-click menus'
     scrollbars behave as before.
