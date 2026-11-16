# Learnings: native-tree-no-chevron

## Step 1

- The "open and collapse" intent reaches `reveal` through a field,
  `Riffle::reveal_collapsed`, not a parameter on `open`. `tree_row_clicked`
  collapses the row and sets the field after `refuse_renaming` lets the open
  through, so a refused open collapses nothing. `reveal` takes the field only
  after the `roots_loaded` check, so it survives both `reveal_wanted` and an
  open deferred by the gate (`deferred_open`), and any reveal of another
  folder drops it (compared with `tree::relation`, since `reveal` gets the
  absolute path and the row carries the tree's spelling).
- The loop in `reveal` breaks before the chain's last step when the field
  matched, so the end is neither expanded nor listed; for a root (a chain of
  one) nothing is listed at all, and the final update still sets
  `tree_open` and the cursor.
- `tree_row_clicked` looks the row up by path at click time instead of
  trusting the index captured at render, since the rows may have been
  rebuilt between the render and the click.
- The first `mise run ci:local` failed on clippy: with the chevron gone,
  `tree_row`'s `icon` closure had one use left that the folder icon no longer
  went through, leaving it unused; the closure was removed.
- Verified: `row_click`'s four rules by unit tests in `tree.rs`, the icon
  paths by `icons.rs`'s tests, the build and clippy through
  `mise run ci:local`. Not verified: the drawing and the click wiring in a
  running app (no GPUI window test in `crates/native`); this is the pending
  manual check below.

## Deferred issues (todo candidates)

- Pending manual check (Windows, best; any platform the native app runs on):
  run `mise run native:dev`, open a folder tree, and check that rows draw no
  chevron and no empty slot before the folder icon and the indent still reads
  the depth; a click on a collapsed row that is not open opens and expands
  it; a click on an expanded row that is not open opens it and it stays
  collapsed after the reveal; a click on the open row's icon / indent / count
  toggles it without re-listing; a plain click on the open row's name only
  arms the slow-click rename; a click on an open leaf row does nothing.
  Step 1's checkbox was ticked on the automated criteria only. The check is
  already folded into `todo.md`'s item `### Native: real-device check of the
  folder tree rows drawn as ListItems (native-tree-listitem)` by this step.
  Basis: plan Step 1's Done-when; files `crates/native/src/main.rs`,
  `crates/native/src/tree.rs`.
