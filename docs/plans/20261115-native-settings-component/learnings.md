# Learnings: native-settings-component

## Step 1: the settings as a full-window layer

- `FocusTrapElement::focus_trap(id, handle)` calls `track_focus(handle)` on
  the wrapped element itself (`gpui-base` 0.9.1 `focus_trap.rs`), so the
  box's own `track_focus(&self.focus)` is redundant but harmless (same
  handle). The trap stays registered in `FocusTrapManager` while the view
  lives; `active_focus_trap` only picks it while it contains the focus, so a
  closed settings view does not capture the main view's Tab.
- The layer is a plain (not `deferred`) `absolute` child painted after the
  `Handpic` `div`, so its `occlude()` blocks every hitbox of the main layout
  (`on_mouse_down`, hovers, tooltips, `on_scroll`, `on_drop`). Deferred
  layers of the main view still paint above it; the menus are closed on
  open and the right-click menu closes on any key, `ctrl+,` included.
- The tree pane's `on_mouse_down_out` ignores occlusion, so a press inside
  the settings clears `tree_keys`, as a press inside the old dialog did.
  Kept as it was.
- A drop on the window while the settings are shown should be ignored: the
  root's `on_drop` fires only on its hovered hitbox, which the layer
  occludes. Not checked by hand (see the pending checks below).
- The hand checks below could not be run from this agent session (no way to
  drive the GUI's keys, pointer and an OS drag here); the step's checkbox was
  ticked on the automated criteria.

## Deferred issues (todo candidates)

- Pending manual check (Windows, Step 1 of
  `docs/plans/20261115-native-settings-component/plan.md`, files
  `crates/native/src/settingsview.rs`, `crates/native/src/main.rs`). Run the
  native app with a test `--settings` store and an open folder, then:
  1. `ctrl+,` shows the settings over the whole window, opaque, with the
     title row, the tab list, the controls and the status line.
  2. Typing works in a label name field (Sidecar, XMP format) and in the
     Import pattern field; Enter and blur commit.
  3. Tab / Shift+Tab cycle only through the settings' controls and wrap
     (no stop in the tree, strip or preview underneath).
  4. On Keyboard Shortcuts, `+` captures the next key, `tab` and
     `shift+tab` included.
  5. Pointer presses, hovers (no strip tooltip) and wheel scrolls over the
     settings do not reach the panes underneath.
  6. A folder dropped on the window while the settings are shown opens
     nothing (record what actually happens).
  7. Escape (cancelling a capture first, committing a focused field first)
     and the `×` return to the main view, where Left / Right page at once.
  Expected: all as described. The step's checkbox was ticked on the
  automated criteria (`mise run ci:local`).
