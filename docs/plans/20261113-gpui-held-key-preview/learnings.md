# Learnings: GPUI held-key preview

## Step 1

- `Pager::finish(index, ok, shown) -> Finished` now carries the decision:
  `Current`, or `Stale { draw, next }`. `loaded()` derives `shown` from
  `self.shown`, counting only `Shown::Read` (a shown failure is `None`, as the
  plan says), and hands a drawn stale result to a new `stale()`; the status
  line's `format!` moved to `status_line()` for both arms.
- A stale `Ok` that is not drawn (the current file's image already on screen)
  now goes through `retire` instead of being dropped; its `Paused` guard drops
  with it, releasing the scan's pause as before.
- Verified by reading, not assumed: every change of `pager.current()` within
  one `order` goes through `go` or a moved `refilter`, both of which call
  `zoom_page` (bumping `seq`), and a listing or `refilter` bumps `order`, which
  drops any read still out. So `zoom.preview()` is `None` for each stale frame.
  Fit peaking keys its layer by `(source.id, size)` in `Paint::paint`, so a
  stale frame asks for its own edges.
- `send()`, `go()`, `refilter`, `overlay.rs`, `zoom.rs`, `peaking.rs` and
  `hold.rs` needed no change.
- `docs/humans/usage.md` mentions holding an arrow key only for the Tauri
  app's viewed rule (the native dwell is unchanged), and `README.md` says
  nothing about it, so neither was touched.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, Windows, by the user): open a folder in the
  native app (`mise run native:dev`), ideally RAF files whose preview load
  outlasts the key repeat, and hold `→` in the fit view. Expected: the
  preview flips through frames during the hold (skipping some files) instead
  of staying on the first one until release; on release the current file is
  shown with its `f` overlay (AF mark, face boxes, mesh), its meta pane and
  a `page` line; no frame ever shows another file's marks. The step's checkbox
  was ticked on the automated criteria (`pager.rs` unit tests,
  `mise run ci`, `mise run native:check`). Basis: Step 1's Done-when;
  files `crates/native/src/pager.rs`, `crates/native/src/main.rs`.
