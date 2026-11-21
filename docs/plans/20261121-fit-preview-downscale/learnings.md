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
  `show_current` also calls `refit`, which re-reads a ring entry or a page
  read decoded for an older canvas when it is shown.
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

## Deferred issues (todo candidates)

- Pending manual check (Windows, the interactive desktop; Step 2's automated
  criteria ticked the checkbox): on the RAF compare folder with prefetch on,
  (1) look at the fit view at 3/8, 4/8 and 6/8 (resize the window so the
  `page` line's `decoded=` shows each) and confirm no visible softness or
  aliasing against `main`; (2) enlarge the window, hide the side panes, drag
  the strip down: the image is soft for at most about 150 ms plus a decode,
  then sharpens with no flash, no meta pane flicker and the `f` marks kept;
  (3) `z` shows the placeholder at the same place and size as on `main`
  (softer), then the crop; (4) `e` recomputes after the sharpened image
  arrives. Step 3's manual check list covers the same ground; this records
  that Step 2 did not exercise it by eye. Basis: Step 2 Done-when (quality
  check by eye, growth handling); files `crates/app/src/main.rs`
  (`refit`, `refitted`, `fit_settled`), `crates/app/src/fitscale.rs`.
