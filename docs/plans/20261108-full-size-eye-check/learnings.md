# Learnings

## Step 1

- The synthetic threshold test passes at 100 px on purpose. A 5x5 box blur is
  a 5 px ramp, and only the upscale to 154 px spreads it far enough to fall
  under `SOFT_BELOW`: sharp 0.528, blurred 0.150 at 100 px. Back-of-envelope, the
  same blur on a crop of 154 px or more keeps a slope of about 0.2 of the
  contrast, over the threshold, so changing the crop size in that test can
  break it without the metric being wrong.
- No JPEG size helper existed in `riffle-core`, and K has to be known before
  the crop is asked for, so `partial::jpeg_width` reads the header alone.
  `decode_region`'s panic guard became the shared `guarded`.
- `Cue::preview_width` comes from `Detection::width`, documented as the stored
  preview size; the whole-image path passes its detection through the same
  `cue_of`.
- `Landmarks::of` in `facechoice` now returns `Option`, so a non-finite mesh
  point dumps `-` where it used to dump NaN.
- An eye's square clamped at the image edge is resized as a non-square cut,
  where the prototype's PIL `crop` padded it with black. Good frames have
  `edge_gap >= 0.02`, which covers the eye region but not the whole
  `0.6 * d` square, so a few frames near the edge may differ from the
  prototype; Step 2's rank check is where that would show.
