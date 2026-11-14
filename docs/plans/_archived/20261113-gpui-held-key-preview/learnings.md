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

## Step 2

- `Held` carries its `seq`; `Zoom::placeholder_fits()` is the one question
  `zoom_paint` asks (no crop held, or the held one is the current page's), and
  a private `holds(viewport)` makes `toggle` / `settled` count only the current
  page's crop. `Arrived::Failed` became `{ error, release }` so the current
  page's failure releases a stale held crop.
- Departed from the plan's wording in one place: `page()` keeps the held crop
  only while zoomed, and a stale arrival while not zoomed is still released.
  Keeping it while not zoomed would let `z` open on another file's crop (with
  no placeholder) until the current one arrives; nothing chases it while not
  zoomed, so there is no flicker to avoid there.
- "Stale held crop, nothing in flight" is unreachable through the public API
  (every stale arrival while zoomed asks for the current page, and that read's
  answer either replaces or releases the stale crop), so the unit test of
  `toggle` / `settled` against a stale crop clears `in_flight` directly.
- Callers of `zoom_page` (`go`, the listing, a moved `refilter`): nothing
  indexes `self.files` with a held crop's page; `send_crop` always uses
  `pager.current()`, and an empty listing makes no request.
- `zoom::send`, `zoom::read`, `Paint::paint` and `peaking.rs` needed no change.

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
- Pending manual check (Step 2, Windows, by the user): in the native app
  (`mise run native:dev`), on a folder whose crop read outlasts the key repeat
  (RAF, ARW), press `z`, then hold `→`. Expected: the 1:1 view flips through
  crops during the hold (skipping some files) instead of staying on the first
  one until release; no crop is ever drawn over another file's scaled preview;
  on release the current file's crop is shown. A single press while zoomed
  goes old crop, new crop; `z` off / on on the same file reuses its crop with
  no new `zoom` line, and a `z` press prints one `zoom` line. The step's
  checkbox was ticked on the automated criteria (`zoom.rs` unit tests,
  `mise run ci`, `mise run native:check`). Basis: Step 2's Done-when; files
  `crates/native/src/zoom.rs`, `crates/native/src/main.rs`.
