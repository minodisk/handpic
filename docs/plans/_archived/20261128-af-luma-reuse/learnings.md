# Learnings

## Step 1

- The measurement ran as a small single-threaded `handpic-cli lumascore`
  subcommand (one preview read and two decodes per file, no ONNX model), as
  the plan recommended; 282 files took about 18 s. Its diff is
  `lumascore.patch`, and the subcommand was removed from the tree before the
  commit.
- 50 bursts qualified in `2026-09-19` alone (the precedent's `features` run
  counted 25 face-tracked ones; here the non-face-tracked AF frames count too
  and the filter is only `af=af`), so `2026-09-13-b` was not needed.
- The ranking held perfectly (Spearman 1.000 in all 50 bursts) although the
  score moved a median 1.4%, up to 22% on flat windows: the luma score sits a
  roughly constant +3 above the Y-channel score, which only looks large where
  the score itself is small.
- The full-frame `candidate::luma` costs about half the grayscale decode it
  replaces on a 1616x1080 preview (3.1 of 6.4 ms), so the saving is a median
  3.2 ms, under the todo's 4-5 ms estimate.
- The first `cargo build --release -p handpic-cli` died in `serde_derive`
  with a bare exit code 1 (likely memory on the shared host); a retry with
  `-j 2` built cleanly in about 5 minutes.
- A stray bare `python -` at the start of a shell command hung waiting
  on stdin as `CLAUDE.md` warns, and was killed by its PID. Use
  `mise x -- python` only.
- A link to this plan's future `_archived/` path would fail lychee until the
  wrap-up moves the folder, so `performance.md` names it as a backticked path.

- `mise run ci:local` was skipped after its second run (killed for low memory, its fmt, lychee and core / engine clippy steps having passed) under the host rule that local cargo builds, tests and `ci:local` are skipped for memory pressure; only `mise run fmt` ran, and the PR's GitHub Actions are the verification, including the core and engine unit tests and the new `an_af_frame_is_scored_on_the_luma_of_the_cue_decode`.

## Deferred issues (todo candidates)

- Core: the AF path's sharpness luma covers the whole frame though the score
  reads only its window. Basis: Step 1's measurement (`luma-reuse.md`,
  "Saving per file"): the full-frame `candidate::luma` takes a median 3.1 ms
  of the 6.4 ms grayscale decode it replaced on a 1616x1080 ARW preview, and
  much more on a 4416x2944 RAF preview. `sharpness::laplacian_variance` reads
  only the window's pixels, so a luma of just the score's window (paths 0
  and 1, and path 2's eye window) would give bit-identical scores, needing no
  ranking check and no `FACES_VERSION` bump, for about 3 ms more per ARW
  file. Files: `crates/core/src/scan.rs` (`score_rgb`),
  `crates/core/src/sharpness.rs` (`score_gray`, `window_at`),
  `crates/core/src/candidate.rs` (`luma`).
- Core: the luma-reuse score's ranking on non-Sony AF bursts is unchecked.
  Basis: plan Trade-offs 2, decided (a); `FACES_VERSION` 17 applies the
  luma reuse to RAF, CR3, NEF, ORF and DNG files with a trusted AF point,
  but only Sony bursts were measured (`luma-reuse.md`, "Limits"). When real
  bursts of another make are on the host, run `lumascore.patch`'s
  subcommand and `burst_spearman.py one` on them. Files:
  `docs/plans/20261128-af-luma-reuse/luma-reuse.md`, `crates/core/src/scan.rs`.
