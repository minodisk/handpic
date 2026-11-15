# Learnings

## Step 1

- `frames.tsv` was built from the scratch dump joined with the sidecars'
  `xmp:Rating` (by file stem, `D:\Photos\tests\<folder>`, read only), keeping
  the five rated folders and rating 1-3. 345 rows: 281 held-out, 64 fresh, of
  which 6 fresh frames have no judged face (empty value columns). The script
  divides by the frames with a judged face, which is what reproduces the
  planning numbers (fresh 27 / 18 / 13).
- The 2% rule alone does not pin the value: any threshold in
  (0.1034, 0.1171] puts 3/154 held-out 3-star frames under it. The script
  picks on a 0.01 grid, giving 0.110; 0.115 would put 2/27 fresh 3-star frames
  under it. Recorded in `low-threshold.md`.
- The fresh-folder AUCs were rechecked by rerunning `check0605.py` (same
  output as the plan's figures).
- Writing a Windows path with backslashes into a Python docstring through a
  shell heredoc and `sed` turned `\t` into a tab twice; the docstring is now
  a raw string and the path was fixed with the Edit tool.

## Step 2

- `full_eye_sharp` in `scan.rs` now takes the preview bytes and measures them
  when `has_larger_jpeg` is false; the preview path neither reads nor takes a
  `ReadSlots` slot, so a cancel does not abandon it (the unit test pins
  that). `cue.preview_width` is the stored preview's width (the detection's),
  so the preview path runs `full_eye_sharpness` at scale 1.
- With Step 2's build the full-size values equal `frames.tsv` on all 345
  rated frames, and every frame with a judged face was measured (478 of 500
  in the five rated folders, 46 of them `NotCandidate`).
- The preview-measured value is about half the full-size one (median ratio
  0.44-0.47); 146 of 154 held-out 3-star frames fall under `SOFT_BELOW` and 91
  under `LOW_BELOW` on the preview. See `preview-vs-full.md`.
- Pass 2 cost on the folder `2026-11-07` (3638 ARW, 8 threads, warm):
  measured frames 2949 -> 3354; a `NotCandidate` file about +75 ms; the
  other files unchanged. Recorded in `docs/humans/performance.md`.
- Both measurements used a scratch `riffle-cli` subcommand, not committed;
  the "before" binary was built by putting `HEAD`'s `scan.rs` and
  `fullsharp.rs` back for one release build (`cargo` is reached through
  `mise exec -- cargo`; a bare `cargo` in the Bash tool is not on `PATH`).
- A backslash path written through a Python heredoc (`D:\Photos\2026\...`)
  had `\2026` turned into control bytes; the docs name the folder only.

## Deferred issues (todo candidates)

- **The preview path puts most preview-only frames in Low once Step 3
  lands.** Measured on the labeled ARW frames (both sources available),
  `full_eye_sharp` on the preview is about half the full-size value: 95% of
  held-out 3-star frames would not be Good and 59% would be Low under Step
  3's rule. That hits every RAF, every HDR PQ CR3 and the Sony bodies with no
  larger JPEG. The plan accepted the preview path as uncalibrated, but the
  size of the gap calls for a decision before Step 3 merges (for example
  separate preview thresholds, a scale correction, or Neutral on a
  preview-measured value). Basis: Step 2's comparison,
  `docs/plans/20261115-full-eye-sharp-only/preview-vs-full.md`; code in
  `crates/core/src/scan.rs` (`full_eye_sharp`) and the tier rule Step 3
  changes in `crates/app/ui/src/focus.ts` and `crates/native/src/mark.rs`.
