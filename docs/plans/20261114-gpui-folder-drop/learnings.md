# Learnings: GPUI folder drop

## Step 1

- The empty-state port was not on `origin/main` at implementation time
  (`origin/main` at `6f1e7936`, no native module holding the no-folder text),
  so the no-folder text was left untouched; the PR body says so, for the
  empty-state PR to restore the "Drop a folder onto the window" clause.
- GPUI applies a `drag_over` / `group_drag_over` style only to an element
  that has a hitbox, and `group_drag_over_styles` alone does not make one
  (`Interactivity::should_insert_hitbox` lists `drag_over_styles`, not the
  group ones). The outline overlay carries both: `drag_over` for the hitbox,
  `group_drag_over` on the root's group so a drag over the strip lights it
  too, as Tauri's `body.dragging` does.
- A `border_3` on the main area itself would shrink the preview canvas while
  dragging (a viewport change, a 1:1 crop re-cut); an absolute overlay with a
  transparent border keeps the layout, like the CSS `outline-offset: -3px`.
- `FileDropEvent::Exited` clears `active_drag` and refreshes the window, and
  the drop listener takes it, so the outline clears on leave and drop without
  view state.
- `cargo` is not on the Git Bash `PATH` here; `~/.cargo/bin` has to be
  prepended for a direct `cargo check` / `cargo test` in `crates/native`.

- This worktree had no `node_modules`, so `mise run fmt` failed at its
  `vp fmt` line until `mise exec -- pnpm install --frozen-lockfile` (`pnpm`
  is not on the Git Bash `PATH` either). `mise run ci:local` passed twice.

## Deferred issues (todo candidates)

- Pending manual check (the user's; Step 1's checkbox was ticked on the
  automated criteria): the folder drop in the running native app on Windows.
  Based on Step 1's last Done-when line; code in `crates/native/src/main.rs`
  (`drop_paths`, `drop_outline`) and `crates/native/src/dropped.rs`.
  Steps, with `mise run native:dev` running:
  1. Drag a folder from Explorer over the window: the main area (tree,
     preview, meta pane) shows a 3px `#737373` outline, also while the
     pointer is over the strip or the status row. Drop it: the folder opens
     and is revealed in the tree, the outline goes.
  2. Drop one file (a RAW, then a non-RAW such as a `.txt`): its parent
     folder opens; the file is not selected.
  3. Drop two folders at once: nothing opens and the status line reads
     `Drop a single folder or RAW file.`
  4. Open the settings modal (`ctrl+,`) and drop a folder over it: nothing
     opens (the occluded layer swallows it).
  5. Drag a folder in and back out without dropping: the outline clears.
  Record the results in this file and turn the "Not verified in the running
  app" bullet of "A folder dropped onto the window opens, a later drop
  winning" in `docs/agents/native-app.md` into a Measured one.
