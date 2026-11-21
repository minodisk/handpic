# Learnings: app-icon-handpic

## Step 1

- resvg-py 0.5.0's `svg_to_bytes` takes `svg_string`, `skip_system_fonts`,
  `font_files` and `resources_dir` (the names the plan guessed) and returns
  the PNG as a list of ints, hence `bytes(...)`. Its PNG is 8-bit RGBA,
  non-interlaced, like the Chrome render. The family name in the TTF is
  `Barlow Condensed` / `Bold`, and `font-weight="700"` selects it.
- Font measurement against `main`'s render (luminance-weighted extents,
  windows wider than the text): DIN Condensed's title cap height was 19.35 px
  and `12`'s 19.18 px. Barlow Condensed Bold reproduces them at
  `font-size` 26.8 (19.36 / 19.16 px), so `TEXT_FS = 26.8` with `FS = 26`
  kept for the layout; the arrow, the baselines and `12A`'s place did not
  move (`12A`'s size scales by the same factor, 19.3).
- Barlow is about 13% wider than DIN Condensed at equal cap height, not the
  6% the plan quoted (`60FPS` 60.1 px against 53.2 at a tracking of 0.4,
  `CULLER` 73.6 against 65.6). Matching the old width needs a tracking of
  about -1.1, which renders the title cramped (letters nearly touching, the
  word spaces closed up). The title uses `TRACK = -0.4` instead, a judgment
  call: its letter gaps look like the old ones, and the title is about 12%
  wider than at -1.1 (404–620 px, well inside the body's corners). The
  plan's Done-when asks only for the cap height to match; the width was an
  approach. One constant changes it if the user prefers the exact width.
- The numerals match the old width at a tracking near 0 (`12` 18.8 px both,
  `12A` 22.9 against 23.1 at 0.1), so they get their own `NUM_TRACK = 0.1`
  rather than the title's tracking; one shared value could not match both.
- resvg against Chrome outside the title band and the sticker's area: the
  photo's mean per-channel difference is about 1/255, with 0.07% of its
  pixels over 32 (resvg's downscale of `photo.png` is a touch sharper at 4x
  zoom, invisible at 1x). The hole edges differ on under 1% of their band's
  pixels (anti-aliasing). No visible change, so no fallback to Chrome.
- The legacy `il32` / `is32` entries decode in Pillow (which reads them for
  the 32 and 16 px @1x sizes, no `icp5` / `icp4` present) to exactly the
  pixels of the ICO's 32 and 16 px PNGs. The literal-only RLE makes `il32`
  3104 bytes against `tauri icon`'s 2068.
- Pillow 13 lists ICNS sizes as `(w, h, scale)`; loading one is
  `im.size = (w, h); im.load(scale=s)`, not `im.size = (w, h, s)`.
- `todo.md`'s "Docs: consider a guide for verifying Pillow pixel edits"
  pitfalls did not recur: the diff used `getbbox(alpha_only=False)` and
  per-pixel statistics from the start, and every measurement window was
  wider than the text. (Here the alpha channel differs too, along the
  body's anti-aliased outline, so the default `getbbox()` would not have
  silently passed anyway.)
- `git mv tools/macos/app-icon tools/app-icon` left no `tools/macos/`.

## Deferred issues (todo candidates)

- Close `todo.md`'s "Docs: consider a guide for verifying Pillow pixel
  edits": its own rule drops it when the two pitfalls do not recur, and they
  did not here (see Step 1). Resolved in this wrap-up by deleting the item;
  no `docs/agents/verifying-pixel-edits.md` is written.
