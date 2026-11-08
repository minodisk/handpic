# Learnings: focus peaking

## Step 1

- **Sync vs worker: worker.** The pure `peaking()` (Rec. 601 luma into a
  `Float32Array`, `|gx| + |gy|`) measured in Node / V8 on this machine (vitest,
  random pixels, three runs each): 3840x2160 (8.3 M px) 57-65 ms, 2560x1440
  33-35 ms, 1920x1080 15-18 ms. A full-window fit view on a 4K / 2x display is
  well past one frame, so the computation runs in
  `crates/app/ui/src/peakingworker.ts`; the overlay appears a moment after the
  image. The main thread still rasterizes the bitmap into a canvas
  (`willReadFrequently`) and reads it back with `getImageData`; that part is
  logged as `peaking WxH raster=` and the worker's part as
  `peaking WxH compute=` under `Timing logs`. The in-app numbers on a real
  4K display were not measured here (see the pending check below).
- **Magnitude `|gx| + |gy|`** instead of `hypot`, to spare a square root per
  pixel; the threshold is defined on that sum.
- **Threshold `PEAKING_THRESHOLD = 160`**, a starting value, not tuned by eye:
  on a hard step edge `|gx|` is 4x the luminance step, so 160 lights a hard
  step of 40 levels (of 255) and leaves sensor noise of a few levels dark. No
  image tooling (PIL) was available in the eval venv to compute lit-pixel
  shares on `D:\photos\samples\`, so tuning is left to the hands-on check.
- **Color** `PEAKING_COLOR = [255, 32, 32]` (red): the focus mark uses green /
  cyan / white / gray and the face mesh magenta, so red stays distinct.
- **One code path for both views.** Rather than keeping the crop's raw
  `ImageData` (another 30 MB per crop on a 4K viewport) the crop's overlay is
  built like the fit view's: the bitmap is rasterized at its own size (1:1, so
  no resampling) and read back. A per-view cache (`peakingLayers.fit` /
  `.crop`) is keyed by the source `ImageBitmap`'s identity and the device-pixel
  size, which makes staleness impossible without hooks at every place `shown`
  or `crop` is replaced: a layer is drawn only when its source is the bitmap
  being drawn. The worker holds one request at a time (`peakingBusy`); a reply
  whose tag no layer carries any more is dropped, and every reply redraws, which
  sends whichever layer is waiting (so a window drag does not queue one Sobel
  per resize event).
- **`shortcuts.rs` test fallout.** The test
  `a_stale_default_in_an_old_store_drops_only_that_key` used `e` as an
  arbitrary user key for `next`; with `e` now `peaking`'s default the stored
  `e` is dropped as taken. The test now uses `n`, which keeps its intent.
- `vp` is not on PATH in Git Bash; run it as
  `mise exec -- node ./node_modules/vite-plus/bin/vp test run <file>` after
  `pnpm install --frozen-lockfile` (as `mise.toml`'s `test` task does).

## Deferred issues (todo candidates)

- **Pending manual check (user): focus peaking in the running app.** Step 1's
  checkbox was ticked on the automated criteria (unit tests, `mise run ci`);
  these need a hands-on run on Windows (and ideally macOS / Linux):
  1. Open a folder of RAW files, press `e`: red edge highlights appear over
     the fit view a moment after the image and sit exactly on its edges for
     orientations 1, 3, 6 and 8 (use portrait shots rotated both ways), at
     several window sizes and at 100 % / 150 % / 200 % display scaling.
     Press `e` again: they vanish completely.
  2. Press `z` with peaking on: the 1:1 crop gets its own overlay aligned with
     its pixels; the scaled placeholder shown while the crop loads has none.
  3. Hold `ArrowRight` through a folder with peaking on: no overlay from an
     earlier file ever shows over a later one; the last file gets its overlay.
  4. `v` (Compare) shows no peaking.
  5. Settings > Shortcuts lists `Focus peaking` on `e` and it can be rebound.
  6. Tune `PEAKING_THRESHOLD` (`crates/app/ui/src/peaking.ts`) by eye on
     `D:\photos\samples\`: a sharp eye lit, smooth skin and bokeh dark, shadow
     noise mostly dark; note whether the fit view and the 1:1 crop want
     different values.
  7. With `Timing logs` on, read the `peaking WxH raster=` lines on a 4K / 2x
     display at full window: if the main-thread rasterize + `getImageData`
     costs well over a frame, consider moving the readback off the main
     thread (`OffscreenCanvas` in the worker, which the `safari13` build
     target does not guarantee).
- **A stored `e` binding is lost on upgrade.** A user who had bound `e` to
  another action in the Shortcuts tab loses that key to `peaking`'s new
  default (the keymap's existing rule, as seen in the
  `a_stale_default_in_an_old_store_drops_only_that_key` test in
  `crates/app/src/shortcuts.rs`). Same as every earlier new default; mention
  in the release notes if it matters.
