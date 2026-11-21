# Learnings

## Step 1

- `Held` and `OnScreen` now carry `upright`, the source preview's upright size
  (the `orient::rotate_bgra` output size of a full decode). The preview
  canvas's fit (`fit_in`) and the 1:1 placeholder (`zoom::placeholder`) read it
  from `Handpic::screen`, which `present` and `clear_image` set and clear
  together with `Handpic::image`, so the two always pair up.
- `zoom::placeholder` swaps its input for orientations 6 / 8, so it expects
  the upright size (what `image.size(0)` gave before); `upright` keeps that
  exactly, unlike `px`, which is the unrotated decode size the `page` line
  reports.
- `prefetch::Pixels for Held` now reads `image.size(0)`, the decoded pixels.
- The strip keeps `fit_centered`, now a thin wrapper over `fit_in`.
- The first `ci:local` run failed on the new `fit_in` test: `assert_eq!` on
  `Pixels` saw `500.00003px` against `500px`. Compare GPUI pixel results with
  a tolerance.
- `mise run ci:local` on a Rust change took over 180 s here and the Bash tool
  moved it to the background on its own; redirecting the output to a file and
  reading it afterwards kept the second run in the foreground.

## Step 2

- The pure rule is `crates/app/src/fitscale.rs`: `scale_for` takes the
  smallest `n/8` whose decoded size (mozjpeg's `ceil(native * n / 8)`,
  `scaled`) is not below the fit on both axes, the fit rounded to whole
  device pixels and capped at the source's own size; `serves` is the same
  test on an already decoded size, which the growth check reuses. `load`
  takes the canvas (`viewport`, read where `send`, `request_prefetch` and the
  re-read build their closures) and calls `Decompress::scale` before
  `to_colorspace`; `Held.px` stays the stored size and `upright` is now
  `fitscale::turned(stored, orientation)` rather than the rotation's output.
- Design: the growth check hangs off the canvas paint, not the resize call
  sites. The paint already writes `viewport`; when the value changes it asks
  for `fit_resized` through the `cx.defer` it runs anyway. That covers every
  canvas change (window resize, pane / strip drags, panes shown or hidden, a
  scale factor change, which no existing hook named) and guarantees the check
  sees the new `viewport`. Its own timer (`fit_timer`), at the crop re-cut's
  150 ms (`FIT_RESIZE_DEBOUNCE = zoom::CROP_RESIZE_DEBOUNCE`), so a zoom page
  dropping `zoom_timer` does not cancel it.
- Design: the re-read path is narrow, not `present`: `refit` loads the file
  again and `refitted` swaps only `self.image` (old one retired), so no
  `page` line, dwell, `mark_viewed`, `marks.show` or meta pane change.
  `present` and `clear_image` bump a `refit` counter, so a page turn or a
  clear supersedes it (a re-listing through `order`, as `send` does). The
  re-read's `Paused` ends when it lands; it is not held for a paint.
  `show_current` also calls `refit`, which re-reads a page read decoded for
  an older canvas when it is shown; a prefetch that arrives below the canvas
  is released in `prefetched` right after `settle`, so it is read again.
- Design: the ring gives up the entries the settled canvas outgrew through a
  new `Ring::release` (not marked crowded, so `next` reads them again at the
  new scale), then `request_prefetch`.
- The first `--auto` page already decoded at 3/8: `--auto` waits
  `AUTO_SETTLE` (2 s), so the canvas is painted before the first keyed read.
- The first `ci:local` run caught a real bug through the unit test:
  `scale_for` was the first `n` that `serves`, and `serves` is true for
  anything on an unknown canvas, so a read before the first paint would have
  decoded at 1/8. `scale_for` now returns 8 for a zero canvas before the
  search. The `--auto` measurements above were taken before the fix but never
  hit it (every keyed read came after the first paint).
- Python on Windows writes CRLF in text mode; editing `main.rs` through a
  script turned the whole file to CRLF once. Open with `newline=''`.

### Measurements (release, this machine, `--auto 60 --no-prefetch`, temp index, 1600x1000 default window)

Folders `D:\Photos\tests\20261108-gpui-compare\RAF` and `\ARW` (20 files
each), before = `066b48ad` (main), after = this step; one run each, p50 / p95.

| folder | build | decoded | decode | paint | keypressToPixels |
| --- | --- | --- | --- | --- | --- |
| RAF | before | 4416x2944 | 49.14 / 75.43 ms | 3.55 / 8.78 ms | 66.96 / 140.79 ms |
| RAF | after | 1656x1104 (3/8; 1104x736 upright 736x1104 at 2/8 for orientation 6/8) | 33.71 / 42.36 ms | 0.71 / 1.13 ms | 49.69 / 57.84 ms |
| ARW | before | 1616x1080 | 7.02 / 10.21 ms | 0.74 / 1.09 ms | 19.30 / 30.43 ms |
| ARW | after | 1212x810 (6/8) | 7.61 / 10.54 ms | 0.39 / 0.59 ms | 22.87 / 30.51 ms |

- The RAF decode drops about a third at 3/8, not more: the entropy decode
  does not scale with the output. The ARW decode at 6/8 is not faster (no
  SIMD IDCT at 6/8); the 0.6 ms p50 difference is within run-to-run noise
  (ARW `frameWait` moved more than that). No stage regressed beyond noise.
  The in-app `keypressToPixels` does not include the GPU upload after
  `present`, which is Step 3's A/B ruler.
- With prefetch on (after, RAF): 57 of 60 pages `source=prefetch`, all at the
  fit scale, `keypressToPixels` p50 11.38 ms.

## Step 3

- The same-session A/B met the target: RAF `screen` minus `keypressToPixels`
  p50 47.02 ms (A) to 15.09 ms (B), ARW 14.36 / 14.92 ms; RAF `screen` p50
  112.60 to 64.94 ms. See `results.md`. The `todo.md` RAF entry is closed.
- Driving it: the branch's `run-ab.ps1` with its B side swapped for a second
  GPUI build and `summarize.py` reading both builds' stdout, kept in `raw/`.
  The ruler was exported with `git archive 1e97309e tools/latprobe` into the
  scratch folder, so the measurement worktree was not touched.
- A was built in a temporary worktree at `6cf88896` with its own
  `CARGO_TARGET_DIR` in the scratch folder: a cold release build took 3 m 13 s,
  as long as the warm-ish B build in this worktree (3 m 25 s). Both worktree
  and target folder were removed afterwards.
- The first attempt failed on every run with probe exit 2, "Desktop
  Duplication delivered no frame": the display had gone to sleep (a sanity run
  10 minutes earlier had worked). A `SetThreadExecutionState` helper started
  after it did not wake it; a 1 px mouse move and back (`mouse_event`) did.
  The driver now does that before each run.
- Other sessions' `cargo test` / builds discarded 16 of 28 measured attempts;
  `RAF-B2` failed all 4 tries in its slot and was redone after `RAF-B3`.
  The whole session took 40 minutes of runs plus about 7 minutes of builds.
- The Bash tool moves any foreground command past 180 s to the background,
  so waiting on the driver worked as `until [ $(wc -l < runs.txt) -ge N ]`
  loops kept under that. A plain `sleep` is refused.
- Python heredocs through Git Bash mangled backslashes in a Windows path
  (`	` became a tab, `6` an octal escape) when writing `todo.md`;
  building the path with `chr(92)` in a script file fixed it.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the interactive desktop), Steps 2 and 3,
  both ticked on their automated criteria (unit tests, `--auto` runs, the
  Step 3 latency A/B and its `decoded=` sizes). Nothing was looked at by eye.
  Already written into `todo.md` as the entry "App: by-eye checks of the
  scaled fit-view decode (fit-preview-downscale)", because Step 3's Done-when
  asks for it there; do not add it again. It covers: on the RAF compare folder
  with prefetch on, (1) quality at 3/8, 4/8 and 6/8 against a pre-`d3794643`
  build; (2) `z`'s placeholder at the same place and size, then the crop, and
  paging while zoomed; (3) `e` in the fit view and at 1:1, recomputed after a
  re-decode; (4) a held `g`; (5) `f`'s face boxes and AF mark where they were;
  (6) enlarging the window, hiding the side panes and dragging the strip: the
  image sharpens after about 150 ms plus a decode, with no flash, meta pane
  flicker or lost marks. Basis: Step 2 Done-when (quality check, growth
  handling) and Step 3 Done-when (manual check list); files
  `crates/app/src/main.rs` (`refit`, `refitted`, `fit_settled`),
  `crates/app/src/fitscale.rs`, `crates/app/src/peaking.rs`,
  `crates/app/src/zoom.rs`.
