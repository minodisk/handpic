# Learnings

## Step 1

- The 4 px inset went on the `uniform_list` itself (`.px_1()`), not on the
  `relative()` wrapper: `uniform_list` lays its items out in its
  `padded_bounds` (gpui-pre 0.3.12 `uniform_list.rs`), so the row's
  `w_full()` resolves to the padded width, while the wrapper, which carries
  `vertical_scrollbar`, keeps its full size and the scrollbar stays at the
  pane's right edge.
- `tree_name` in `crates/native/src/folderops.rs` already had
  `.flex_1().min_w_0()` on both branches and needed no change.
- The agent could not drive the native app's window, so the layout checks
  are recorded below as pending manual checks; the step's checkbox was
  ticked on the automated criterion (`mise run ci:local`). The
  `native-app.md` rule is marked Inferred for the same reason.

## Deferred issues (todo candidates)

- Pending manual check (Windows, and any platform the native app runs on),
  from Step 1 of this plan (`crates/native/src/main.rs` `tree_pane` /
  `tree_row`): run the native app (`crates/native`), open a folder tree
  holding a folder with a long name (e.g. `2026-10-03-dop-orientation`) at
  several depths, and verify each of the following. The step's checkbox was
  ticked on the automated criteria only.
  1. Long name truncation: a folder name longer than the pane ends in an
     ellipsis inside the pane at every depth; no row content reaches past
     the pane's right edge.
  2. Count badge: expand a folder holding RAW files; its count badge is
     fully visible at the row's right edge, both next to a short name and
     next to a truncated one.
  3. Highlights: the keyboard cursor ring (click in the tree, move with
     arrows), the hover highlight and the open row's highlight span the
     pane less a 4 px inset on both sides, with rounded corners.
  4. Scrollbar: with enough rows to scroll, the vertical scrollbar sits at
     the pane's right edge, and dragging its thumb scrolls the list.
  5. Rename: the inline rename editor (`Rename…` from the folder's
     right-click) fits inside its row; a held rename's pending name (clock
     icon, italic) truncates like any other name.
  6. Scrolling into view and clicks: the reveal on open, the keyboard
     cursor and the rename's row are brought fully into view by
     `scroll_to_item`; a click and a right-click on a row's inset edge
     behave as before.

  Done when all six are confirmed and the folder tree rule in
  `docs/agents/native-app.md` ("a tree row takes `w_full()` ... not yet
  observed in the running app") is changed from Inferred to Measured with the
  observed result, or corrected with the observed behavior if a point fails.
