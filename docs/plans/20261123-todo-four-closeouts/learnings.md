# Learnings

## Step 1: Trim the trailing spaces of the displayed lens name

- The lens is trimmed at display in `crates/engine/src/exif.rs` (`trim_end`,
  then a now-empty lens dropped), so `EXTRACTOR_VERSION` stays 13 and the
  index keeps the string as read.
- No other consumer shows the index's raw `lens`: `crates/app/src/meta.rs`
  (the `Lens` row) and `crates/app/src/exif.rs` (`Group::Lens`) both read it
  through `exif()`, and `crates/engine/src/mcp.rs`'s `lens` is only the
  filter argument, matched against those labels.
- A bare `python` in Git Bash hangs on this host, as the plan warns; use
  `mise x -- python`.

## Step 2: Eye-midpoint score in `features` and the burst ranking check

- `features` gained `eye_af` and `eye_mid` after `eyes_ms`, computed by
  `eye_mid_features` outside both timers. It reads the preview a second time
  for the shot and the score; that cost is untimed, so `analysis_ms` and
  `eyes_ms` keep their meaning.
- The cue's face is guarded on `FACE_CONFIDENCE` in the CLI, because
  `score_preview` falls to the sharpest tile below it.
- Three Sony folders (2026-09-19, 2026-09-13-a, 2026-09-13-b) gave 69
  qualifying bursts: median Spearman 0.500, same top frame 62%, under the
  Trade-offs 2 bar. Per the approved option (a), the scoring stays; see
  `burst-rank.md`.
- In this Git Bash, backslash escapes in a `sed` pattern or a Python heredoc
  (`\t` inside Rust string literals) did not match reliably; the Edit tool
  did.

## Deferred issues (todo candidates)

- Core: the within-burst sharpness ranking of face-tracked Sony frames differs
  between the AF-frame window (today's stored `sharpness`) and the eye-midpoint
  window it replaced: over 69 bursts of 2026-09-19 / 2026-09-13-a / -b, median
  per-burst Spearman 0.500, p10 -0.420, same top frame in 62% (bar: 0.9 and
  80%). Kept as is per Trade-offs 2 (a); decide whether the score that ranks
  frames within a burst should return to the eye-midpoint window (with a
  `FACES_VERSION` bump) or whether the burst ranking should read
  `full_eye_sharp` instead. Basis: Step 2's check,
  `docs/plans/20261123-todo-four-closeouts/burst-rank.md`. Files:
  `crates/core/src/sharpness.rs` (`score`, `eye_af_frame`),
  `crates/engine/src/index.rs` (`FACES_VERSION`).
