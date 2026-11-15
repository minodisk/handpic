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

- The first commit measured a file without a larger JPEG on its preview.
  `cue.preview_width` is the stored preview's width (the detection's), so
  that ran `full_eye_sharpness` at scale 1 with `MIN_SCALE` removed.
- User decision (2026-11-15), after the comparison below: a file without a
  larger JPEG is not measured and stays Neutral. The follow-up commit drops
  the preview path and restores `MIN_SCALE`, because `has_larger_jpeg` only
  tells a full JPEG apart from the preview (no full, HEVC, same offset), not
  how much larger it is; a full JPEG at most 1.5x the preview is near the
  preview's scale, where the full-size thresholds do not hold.
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
- The cost run used the first commit's build (preview path, no `MIN_SCALE`).
  It holds for the final rule: every file of that folder has a full JPEG
  well over 1.5x the preview, so neither difference touched it.

## Step 3

- The mirrors are named `LOW_FULL_EYE_SHARP` (`focus.ts`, the native
  `mark.rs`) next to `GOOD_FULL_EYE_SHARP`. `fullsharp.rs` pins `LOW_BELOW`
  to 0.110 and `focus.test.ts` pins both figures; the native `mark.rs` test
  compares its two constants with `riffle_core::fullsharp`'s directly, since
  the native crate depends on `riffle-core`.
- Test fixtures that only meant "a focus candidate" had `full_eye_sharp:
  null` (the Tauri `filter.test.ts` and native `filter.rs` `candidate()`);
  under the new rule a missing value is Neutral, so they now carry 0.3. The
  named-frame fixtures (`_DSC2638` and the rest) keep their `eye_focus`
  values, which the tier no longer reads; Step 5 removes the field.
- The native overlay test `only_the_latest_read_is_kept` made its Low answer
  with `FocusCandidate::NotCandidate`; it now sets `full_eye_sharp` under the
  Low threshold.
- `CLAUDE.md` described Low as Rust's `not_candidate` in three places
  (`mark.rs`, `focus.ts`, `filter.ts`); updated with the rule.
- The frontend tests (`mise exec -- pnpm exec vp test run`, 813) pass
  locally; `ci:local` does not run them.

## Plan change (2026-11-15, user)

- The Tauri app (`crates/app`, `crates/app/ui`) is frozen: only the GPUI
  native app grows. No Tauri improvement (its meta row keeps the name
  `Eyes at full size`); the only Tauri edits allowed are the mechanical ones
  that keep it building once Step 5 removes `eye_focus` / `candidate`, and
  they belong to Step 5.
- PR #1066 (native meta pane) is fixed in its own branch by another session:
  it drops its `AF eye in focus` / `Focus candidate` rows and renames
  `Eyes at full size` to `Eye focus` before merging. Step 4 no longer
  touches `crates/native/src/meta.rs` or waits for #1066; Step 5 waits for
  it (removing the fields first would break its branch).
- `plan.md` was rewritten accordingly: the decisions list, Step 4 (ranking
  and CLI only), Step 5 (the Tauri compile-only edits, the #1066
  precondition) and the trade-offs on #1066 and the names.

## Step 4

- `rank.rs`: `tech()` writes the constants NaN (then `prior.tech_mean[1]`),
  0 and 1 into columns 1, 6 and 7; `good_photo`, `logit` and the
  good-photo cut constants are gone. Those columns are the same for every
  frame, so `row[j] - mean[j]` is 0 and their reasons were always 0: the
  `AF eye focus` and `Good photo` entries of `REASONS` and the `(7, 1)`
  `MISSING` entry are removed. The new test checks that each score still
  equals the frame's dot product minus the burst's mean, with weight 5 on
  the inert columns, under both a fit and the prior.
- A stored `ranking.json` holds `COLUMNS` weights fit with the old columns;
  nothing about its shape changed, so `rank` reads it as before and the
  weights on the inert columns simply cancel within a burst.
- The plan's `focusbox` is the CLI's `faces` subcommand (the one that
  printed the cue's state and probability and drew the scored region);
  `focusbox` itself never read `eye_focus`. `faces` keeps the nearest face
  line and drops the scored region's rectangle.
- `facechoice` now takes the EAR, pose, eye offset and edge gap straight from
  the mesh (`eyes::more_closed_ear`, `Mesh::pose`,
  `candidate::mesh_eye_offset`, `candidate::edge_gap`), not through
  `EyeMeasures`, which Step 5 removes. Its `--check` compares the rule's
  `full_sharp` with `scan::extract_analysis(..).full_eye_sharp`; not run on
  real files in this step.
- The `features_line` test builds `Cue` with `..Cue::default()` so it no
  longer names `state` / `eye_focus`.
- A heredoc through the Bash tool turned `\t` into `\t`; Python edit
  scripts with backslashes were written to a file with the Write tool.

## Deferred issues (todo candidates)

- `todo.md` still names `riffle-cli candidates` as a command to run (around
  lines 173 and 192, a held-out check of the candidate cue). Step 4 removed
  the subcommand (`crates/cli/src/main.rs`); the items are moot or need
  rewording onto `facechoice` / `features`. Reflect at wrap-up; `todo.md`
  is not edited in a step.
