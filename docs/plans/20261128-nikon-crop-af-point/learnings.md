# Learnings: Nikon crop-mode AF point mapping

## Step 1

- The sample check matched the plan's expected focus lines on all 14 NEFs:
  Z6III DX `3984 2656 1991 1328` (7810, 7814, 7818), 1:1 `4032 4032 2015
  2016`, 16:9 `6048 3400 3023 1700`, FX `6048 4032 3023 2016`; Z50II 1:1
  `3712 3712 1864 1868` (7765, 7766, 7767), 16:9 `5568 3128 2789 1573`,
  uncropped `5568 3712 2789 1868`; Z5II `focus: -` on all three.
  `focusbox` drew the box at the JPEG's center on each crop. Because the AF
  area size is passed through unscaled while the frame shrinks to the crop,
  the box is drawn proportionally larger on a crop preview than before (for
  example 222x245 in a 3984-wide frame), which is the physically right size.
- `EXTRACTOR_VERSION` alone is enough: `write_batch` writes a re-extracted
  row with `faces_extractor = 0`, so pass 2 re-scores it and `FACES_VERSION`
  stays.
- Wrapping `cargo test` in `timeout 590` killed the linker (`link.exe`
  exit 143) on a cold `target/`; the compile itself was fine. Let the Bash
  tool's own timeout bound the run instead.
- `mise run ci:local` was stopped by the harness while it compiled the
  engine's dependencies, because the host ran low on memory (other sessions
  share it); it was not re-run. Verified locally: `cargo test -p
  handpic-core --lib nef` (15 passed, the three new tests included),
  `cargo build -p handpic-cli` and `mise run fmt`. The engine change is the
  `EXTRACTOR_VERSION` constant and its doc comment; clippy, the engine
  tests and the Markdown link check are left to the PR's GitHub checks.
