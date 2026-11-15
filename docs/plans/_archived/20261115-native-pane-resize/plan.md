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

# Resizable side panes in the native app

## Purpose

In the GPUI app (`crates/native`) the folder tree (left pane) and the meta
pane (right pane) are a fixed 220 px (`PANE_WIDTH` in `src/main.rs`). Long
folder names and long metadata values are cut off, and a user who wants a
larger preview cannot narrow them. After this work each pane's width is
changed by dragging its boundary with the preview, is clamped so that neither
the pane nor the preview collapses, survives a restart, and comes back
unchanged when the pane is hidden and shown again. The Tauri app
(`crates/app`) is frozen and is not touched. The filmstrip's height stays
fixed.

## Steps

- [x] Step 1: Drag-resizable, persisted widths for the tree pane and the meta pane
  - Done when:
    - Dragging the boundary between the tree and the preview, and between
      the preview and the meta pane, changes that pane's width, within its
      minimum and maximum, and the preview takes the rest.
    - The preview keeps at least its minimum width while a pane is dragged
      and when the window is made narrower; widening the window again brings
      the panes back to the widths the user set.
    - The widths are stored in the settings store under `panelWidths` and a
      restart comes up with them. A missing or invalid value, or entry, means
      220.
    - Under `--auto` the panes start at 220 whatever the store holds and
      nothing is written to `panelWidths`.
    - Hiding a pane (`toggleLeft`, `toggleRight`, `toggleSides`) and showing
      it again gives it its last width.
    - While the 1:1 view is on, a width change re-cuts the crop after
      `CROP_RESIZE_DEBOUNCE`, as a window resize does.
    - `PANE_WIDTH` is gone from `main.rs`; the default lives in the new
      module.
    - The new module has unit tests for the setting's parse and
      serialization, the drag's clamping and the drawn widths in a narrow
      window; `settings.rs` has a test that `panelWidths` is read with its
      rule.
    - `CLAUDE.md`'s Layout paragraph names the new module and the
      `panelWidths` key, and `docs/agents/native-app.md` records the design
      and no longer states the panes as a fixed 220 px.
    - What was not observed in the running app (the drag itself, the look of
      the handle, a release outside the window) has a hand-check entry in
      `todo.md`, as the earlier native PRs leave them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - **Not `h_resizable`.** `ResizablePanelGroup` keeps the sizes in its
      own `ResizableState`: a saved width can only go in as a panel's
      initial `.size()`, a hidden pane either stays as an empty panel whose
      neighbour still draws a handle (`.visible(false)`) or, when the child
      is left out, has its size truncated away by `sync_panels_count`, a
      container resize rescales the panels that are not `fixed`, and the
      clamping would sit inside the component, out of reach of a unit test.
      See `gpui-base-0.9.1/src/resizable/mod.rs` and `panel.rs` in the cargo
      registry.
    - **Use the standalone handle**: `gpui_component::resizable::resize_handle`
      (`gpui-component-0.9.1/src/resizable.rs`, over
      `gpui-base-0.9.1/src/resizable/resize_handle.rs`), which exists for an
      edge no group owns. It supplies the grab band, the col-resize cursor
      and the hover / drag appearance; what the drag resizes is the caller's.
      Its API: `resize_handle::<T, E>(id, Axis::Horizontal)`,
      `.inside(HandleEdge::Leading | HandleEdge::Trailing)`,
      `.on_drag(value, |value, _, _, cx| cx.new(..))`, where `T` is the drag
      payload and `E` a `Render` view for the dragged item (an empty one, as
      the component's own crate-private `DragPanel` is).
    - The handle is `absolute` and places itself against its parent, so the
      parent must be `relative`. The preview container in `render`
      (`div().relative().flex_1().min_w_0().h_full()`) already is: add the
      left handle there with `HandleEdge::Leading` while `panels.left`, and
      the right one with `HandleEdge::Trailing` while `panels.right`, after
      the canvas. This keeps the handles off the panes' own scrollbars (the
      component's source has a FIXME about a trailing handle over a
      scrollbar) and leaves `tree_pane` / `meta_pane` with only their width
      changed.
    - Follow the drag with `on_drag_move::<T>` on an element that covers the
      window (the root `div` of `render`, or the main row). In gpui-pre
      0.3.12 it runs in the capture phase for every move while a drag of
      that payload type is active, inside the element or not. The panes are
      flush with the window's edges, so the left width is the pointer's `x`
      and the right width is the window's width less the pointer's `x`; the
      payload says which pane (`panels::Panel::Left` / `Right` in a newtype,
      since GPUI tells drags apart by payload type and `ExternalPaths` is
      already taken by the folder drop).
    - New module `crates/native/src/panewidths.rs`, free of GPUI types, after
      `panels.rs`: a `PaneWidths { left, right }` in logical px with the
      default (220, the former `PANE_WIDTH`), the pane minimum and maximum
      and the preview minimum as constants; `from_setting(Option<&Value>)`
      (a non-object value, or a missing, non-number or non-finite entry,
      means 220; a number out of range is clamped) and `setting()`
      (`{"left": n, "right": n}`); the width a drag gives a pane, from the
      pointer position, the window width and which panes are shown, clamped
      to the pane's range and to what leaves the preview its minimum beside
      the other shown pane; and the widths to draw for a window width, which
      narrow the shown panes without changing the stored ones when the
      window cannot hold them and the preview's minimum. The exact rule for
      a window too narrow for even the minimums is decided in this step and
      pinned by a test.
    - `main.rs`: the view holds the `PaneWidths`; `tree_pane` and
      `meta_pane` take the drawn width instead of `px(PANE_WIDTH)` (both
      keep `flex_none`). A drag move sets the width, calls
      `zoom_resized(cx)` and `cx.notify()`. `zoom_resized` is the one thing
      that does not follow by itself: the canvas's paint sets `viewport`
      from its own bounds and the peaking layer is keyed by the drawn device
      size inside that paint, so both follow the canvas; the strip spans the
      window and its `strip_width` comes from the window's viewport, so it
      is unaffected; but the re-cut is requested only from
      `observe_window_bounds` and `change_panels`. `zoom_resized` replaces
      its timer, so calling it on every move is the debounce.
    - Hidden-and-shown needs no code: the widths are a separate value from
      `Panels`, and `change_panels` does not touch them.
    - `settings.rs`: `Settings` gains the widths, read in `from_values` from
      `values.get("panelWidths")`. `Riffle::new` takes them only when
      `remember` is on, else the default, as `panels` does (around line
      793); the write is guarded by the existing `remember_panels` flag.
    - Persist through `self.settings.writes.set("panelWidths", ..)` when the
      drag ends, not on every move. `on_drop::<T>` on the root `div` is the
      candidate for the end of the drag. Verify during implementation that
      it runs when the button is released over any part of the window, and
      what happens on a release outside the window; if the end cannot be
      seen reliably, write on every move instead (the `Writes` thread
      applies them in order) and record the choice in `learnings.md`.
    - Verify during implementation: that the handle's 1 px line and its
      hover pill read well on the preview's edge with the app's theme
      (`theme.rs`), that the 5 px band does not get in the way of the
      preview's own clicks (the empty-state message, the context menu), and
      that the tree's `uniform_list`, its inline rename input and the meta
      pane's rows lay out at the minimum width.
    - `docs/agents/native-app.md`: add a section next to "The panes are one
      `Panels` value, stored under `panels`" with the rules above (why not
      `h_resizable`, where the handles sit, stored versus drawn widths, the
      `--auto` rule, when the value is written), and update the places that
      give the panes as 220 px (the folder tree section, its inferred canvas
      size, the meta pane section) to say it is the default.
    - `CLAUDE.md`: add `src/panewidths.rs` to the native crate's part of the
      Layout paragraph next to `src/panels.rs`, and `panelWidths` to the
      list of what `src/settings.rs` reads.

## Trade-offs and risks

- **Standalone `resize_handle` instead of `h_resizable`.** The request named
  `h_resizable` / `ResizableState` "if it fits". It does not fit the hidden
  panes, the restore of saved widths or a testable clamp (see the approach),
  so the plan uses the same file's standalone handle and keeps the widths in
  the app. If the handle turns out to misbehave here (its `occlude`, its
  drag, its look), the fallback is a hand-drawn `div` with `on_drag` and
  `cursor_col_resize`; the pure module and everything else stay the same.
- **When the widths are written.** At the drag's end (planned), on every
  drag move, or at quit next to `windowBounds`. The end of the drag is one
  write but depends on `on_drop` being seen; every move is the simplest and
  cannot miss, at the cost of a burst of ordered SQLite writes on the
  settings thread; at quit loses the change on a crash and does not mirror
  `panels`.
- **The limits.** 160 to 480 px for a pane and 240 px for the preview,
  approved by the user as working numbers; the meta pane's fixed label
  column (`native-app.md`, the meta pane section) may call for a larger
  minimum. The implementation checks them by eye and records the final
  values.
- **A narrow window.** The plan keeps the user's widths and only draws
  narrower panes while the window cannot hold them, so widening the window
  restores them.
- **Look.** The handle draws a 1 px line in the theme's border color on the
  preview's edge where there is none today, and takes the outer 5 px of the
  preview for the pointer.
- **`docs/humans/usage.md`.** Left alone, as the recent native-only PRs did.
- **Not included**: a double click on the handle to restore 220, keyboard
  resizing, a resizable filmstrip, and any change to `crates/app`.
- **Verification.** No agent can drag in the running app from CI, so the
  drag, the cursor and the look are covered by `todo.md` hand checks only;
  the unit tests cover the arithmetic and the setting.

## Progress

- Step 1: the tree and meta panes resize by dragging their boundary with the
  preview, and their widths persist in the `panelWidths` setting (the pure
  arithmetic in `crates/native/src/panewidths.rs`, the handle and drag in
  `crates/native/src/main.rs`). Ticked on the automated criteria only (unit
  tests and `mise run ci:local`); the drag, cursor and look were not observed
  in the running app, and the hand checks are listed under `## Deferred
  issues (todo candidates)` in `learnings.md`.

(2026-11-16) Step 1 complete
