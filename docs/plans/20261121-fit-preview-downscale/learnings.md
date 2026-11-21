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
