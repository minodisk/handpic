# Learnings: todo-four-display-fixes

## Step 1

- Item 1: the existing fixture `fixture(6, &[0u8; 512])` does take the
  `catch_unwind` panic arm, not `Ok(Err(..))`: with the payload downcast to
  `String`, the test's message starts with `libjpeg fatal error:`.
- Item 2: `Zoom::read_failed` also clears on `toggle`, so turning `z` off and
  on again does not show the previous attempt's message while the new read
  is in flight.
- Item 3: the engine test writes the scanned file's `files` row through
  `Index::write_batch` with an error row (`Failure::from(..)`), the cheapest
  `files` row the fixture allows, and checks `entry` sees it before counting.
- Item 4: `shared` outside a drag is computed (`shared_height` in `main.rs`:
  the viewport height less the client-side frame's top and bottom padding,
  `TITLE_BAR_HEIGHT` and `STATUS_ROW_HEIGHT`), option (a) of the plan's
  trade-off, so the clamp holds from the first frame.
  Measured on 2026-11-23 with a temporary `canvas` in the wrapper `div` the
  `StripDrag` listener sits on, printing its bounds' height next to
  `shared_height(window)` during `uitest::opens_with_no_folder` on Windows
  (GPUI's test platform): `bounds=936 computed=936` on every frame. Not
  measured on a real window, nor on Linux with client-side decorations,
  where `window_paddings` is non-zero (`menu_max_height` already relies on
  the same arithmetic).
- A cold `cargo test` of `crates/app` took 14 minutes here and a cold
  `handpic-core` test about 7; the Bash tool moves both to the background
  after 180 s.

## Deferred issues (todo candidates)

- Pending manual check (Windows, real machine): recorded in `todo.md` as
  `App: real-device checks of the four display fixes
  (todo-four-display-fixes)` by this step, since the plan asked for that
  item in the same PR: the strip cell and hover text of
  `NIKON_D70_Nikon.nef` / `CGO3P_YUN00007.dng` read the libjpeg reason
  with no path; `z` on them shows `The 1:1 view of this file could not be
  shown.` and paging to a good file shows its crop; with the strip at `MAX`
  a shorter window keeps the preview at 240 px or more, a taller one gives
  the height back and a relaunch keeps the stored height; the rewrite
  dialog's third row reads `N files with no judgment on record (skipped)`.
  The step's checkbox was ticked on the automated criteria. Basis: plan
  Step 1. Files: `crates/core/src/scan.rs`, `crates/app/src/zoom.rs`,
  `crates/app/src/empty.rs`, `crates/app/src/sidecars.rs`,
  `crates/app/src/stripheight.rs`, `crates/app/src/main.rs`.
- `shared_height` on Linux with client-side decorations and on macOS is
  unmeasured against the wrapper's bounds (see Step 1 above); a mismatch
  would leave the preview a few px off `PREVIEW_MIN` in a short window.
  Basis: Step 1's measurement ran on GPUI's test platform only. File:
  `crates/app/src/main.rs` (`shared_height`).
  Change: print the `StripDrag` wrapper's bounds height next to
  `shared_height(window)` on a real Linux window with client-side
  decorations and on macOS. Done when: the two are measured equal or
  `shared_height` is corrected, and the "unmeasured" sentence in
  `docs/agents/app.md`'s strip-height section is replaced by the result.
  Suggested heading: `App: measure shared_height against the wrapper's
  bounds on Linux (client-side decorations) and macOS`.
