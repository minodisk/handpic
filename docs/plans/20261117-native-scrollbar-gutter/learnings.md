# Learnings: native-scrollbar-gutter

## Step 1

- `Scrollbar::width()` (gpui-base 0.9.1, `#[doc(hidden)] pub const fn`) is
  reachable as `gpui_kit::component::scroll::Scrollbar::width()`, so
  `theme::SCROLLBAR_TRACK` has a test against it. gpui-base also reads a
  theme's `scrollbar` track width (`themed_track_width`, falling back to
  `WIDTH`); none of the vendored files under `crates/native/themes/` sets one
  (they set only the track and thumb backgrounds), so 16 px holds for every
  shipped theme.
- The dialog bodies needed no change, as the plan read: gpui-component
  0.9.1's `Dialog::render` turns the dialog's own padding into `paddings`
  (16 px per edge when unset) and pads the body's
  `v_flex().overflow_y_scrollbar()` with `pl` / `pr` after the call, which
  `Scrollable` keeps on the scrolled content. `shell`'s `px_4()` is one rem,
  and `Root` sets the rem to the theme's `font_size`, which `theme.rs` sets
  to 16 px. A comment in `shell` now says so.
- `theme::SCROLLBAR_TRACK` in `main.rs` resolves to the module even inside
  `tree_row` / `meta_pane`, which bind a local `theme`: a `::` path looks its
  first segment up in the type namespace.

## Deferred issues (todo candidates)

- Pending manual check (Windows first, then macOS and Linux; Neutral dark,
  Neutral light and one vendored theme), Step 1 of this plan, from the
  changes to `tree_row` / `meta_pane` in `crates/native/src/main.rs` and the
  comment in `shell` in `crates/native/src/modal.rs`. Step 1's checkbox was
  ticked on the automated criteria only. Verify:
  1. Open a folder tree tall enough to overflow: no count badge or name lies
     under the thumb at rest, hovered or dragged. Measure the gap between the
     badge's right edge and the thumb (by the source the resting thumb paints
     4-10 px from the pane's edge and the badge now ends 16 px from it; the
     planning screenshot showed the badge covered, which the numbers did not
     explain).
  2. A press on a badge's right edge selects the row (the track does not take
     it).
  3. A tree that does not overflow looks unchanged apart from 4 px more room
     right of the badges.
  4. In the meta pane with a long file's metadata, the bar is at the pane's
     right edge with no value under it, and the notes and errors under the
     body have not moved (still 12 px from the right edge).
  5. A long `Import…` or Move to Trash dialog body has no text under its bar.
  These replace the "tree rows' right 16 px" half of check 4 in
  `docs/plans/_archived/20261117-native-scrollbars-always/learnings.md` and
  that half of its `todo.md` item ("Native: real-device checks of the
  always-visible scrollbars").
