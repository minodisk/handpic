# Learnings

## Step 1

- The plan's hand-back of the tree's keyboard (a `capture_any_mouse_down` on
  a wrapper `div` around the `PopupMenu` entity) would never fire: GPUI's
  `capture_any_mouse_down` tests `hitbox.is_hovered`, a div inserts its
  hitbox before its children's, and `PopupMenu`'s root is `.occlude()`d
  (`HitboxBehavior::BlockMouse`), so the hit test stops at the menu and the
  wrapper is never hovered (`gpui-pre` 0.3.12 `window.rs`'s `hit_test`). The
  wrapper instead holds an absolute `canvas` whose paint registers a
  capture-phase `window.on_mouse_event` that tests the press against the
  canvas's bounds, which are the wrapper's.
- Key dispatch order, read in `gpui-pre` 0.3.12 `Window::dispatch_key_event`:
  a matching binding's action runs first and stops there when handled, and
  only then do the key-down listeners run. So while the menu holds the focus,
  `escape` / `up` / `down` / `enter` are the menu's and never reach
  `Riffle::key_down`; `left` / `right` propagate from a top-level menu
  (`cx.propagate()` in `select_left` / `select_right`) and every other key
  bubbles to the root `div`'s `on_key_down`, since `defer_draw` keeps the
  parent dispatch node. Not run: part of the manual check below.
- `Entity<PopupMenu>::focus_handle` needs the `Focusable` trait in scope.
- `POPUP_PRIORITY` is reachable as `gpui_kit::base::POPUP_PRIORITY`.
- `cx.subscribe` (not `subscribe_in`) is enough for the `DismissEvent`: the
  handler only clears the field and notifies, and the dismiss itself already
  returns the focus to the `action_context`.

## Deferred issues (todo candidates)

- Pending manual check (Windows, `crates/native`), from Step 1's Done-when of
  `docs/plans/20261116-native-context-menu/plan.md`; the step's checkbox was
  ticked on the automated criteria (`mise run ci:local`). Steps and expected
  results, in `riffle-native` on a folder with RAW files:
  - Right-click a root and a non-root folder in the tree, once with the tree
    holding the keyboard (left-click in the pane first) and once without: a
    `PopupMenu` opens at the pointer with the folder groups (the root without
    `Rename…` and `Expand All` / `Collapse All`), separators between groups;
    the folder does not open and the tree's keyboard state does not change.
  - Right-click a strip cell inside a multi-selection and one outside it, a
    gap and the empty area, and a cell in a view-only (JPEG-only) folder: the
    selection rule holds, the judgment groups show their shortcuts as `Kbd`
    text and the current flag / stars / label checked, the gap and empty area
    open nothing, the view-only menu has no judgment groups.
  - Right-click a cell at the window's right edge: the menu flips to the
    pointer's left; on the strip near the bottom it opens above the pointer.
  - Shrink the window: the strip's menu scrolls within the window less the
    title bar and 40 px.
  - Run `Rename…` (ends with the inline `Input` focused), `Copy Path`, a
    judgment (applies to the targets) and `Move to Trash…` (its dialog opens).
  - With a menu open: Escape closes it only; a keymap key (e.g. `p`) closes
    it and acts; Left / Right close it and page the strip; Up / Down move the
    highlight and Enter runs it; a press outside closes it; a second
    right-click elsewhere replaces it. After each close the keymap works
    again (the focus is back on the view), and a press on the menu while the
    tree had the keyboard hands it back.
