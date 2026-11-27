<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Measure the luma-reuse score for files with a trusted AF point, and adopt it only if the burst ranking holds

## Purpose

Close the `todo.md` section "Core: a file with a trusted AF point still
decodes its preview twice in pass 2".

Pass 2 (`scan::extract_analysis`) decodes the preview of a file with a
trusted AF point twice: full-size RGB for the face cue (`af_cue`), then
grayscale for the sharpness score (`score` -> `sharpness::score_preview`).
Since `FACES_VERSION` 13 a faced file with no AF point scores from the BT.601
luma of the cue's RGB decode instead
([no-af-decode.md](../_archived/20261123-todo-four-closeouts/no-af-decode.md)).
Doing the same for a file with a trusted AF point would save the grayscale
decode, about 4-5 ms on a 1616x1080 ARW preview. On ARW, CR3, NEF, RAF and
ORF previews that luma differs from mozjpeg's Y channel on about half the
pixels (score 0.1-1.1% off), so the within-burst ranking has to be checked on
real Sony bursts before the score changes.

This plan measures the saving and the ranking, decides by the rule below, and
either ships the variant with a `FACES_VERSION` bump or records "measured,
not adopted". Either way the measurements land in this folder and the
`todo.md` section is removed.

Decision rule (decided by the user before this plan; repeat it in the PR
body):

- Adopt (bump `FACES_VERSION`, ship the variant) only if the luma-reuse
  score's within-burst Spearman rank correlation against the current score is
  about as high as the precedent no-AF variant's was when it was adopted, and
  the top-ranked frame of each burst is almost unchanged.
- If the result is unclear, do not adopt: record "measured, not adopted". The
  todo body allows that outcome, and the saving is only about 4-5 ms per
  file.
- The numeric reading of "about as high" is Trade-offs 1 (decided: the
  recommended bar).

Decisions taken at approval (2026-11-28), all the recommended options below:
Trade-offs 1 the recommended bar; Trade-offs 2 (a), decide on Sony alone and
state the limit; Trade-offs 4, drop the measurement code in both outcomes and
keep its diff in this folder; Trade-offs 5, a middle result is "not adopted".

Rules for the step:

- Single-PR mode: the measurement, the write-up, the decision, the adoption
  (or nothing in core) and the `todo.md` deletion go in one PR.
- Remove only this plan's `todo.md` section. Other sessions edit other
  sections concurrently: touch no other line of `todo.md`.
- Measurement limits. The host is short of memory and shared with many
  sessions, and `CLAUDE.md` forbids generated CPU load: one thread, one
  process at a time, no repeated loops over the folders, a few hundred frames
  in total, the photo folders read only (write tables to the scratchpad or
  this folder, never beside the photos). If the result seems to need a larger
  scale, stop and report to the caller instead of scaling up.
- Run Python through `mise x -- python` (a bare `python` hangs in Git Bash on
  this host). Keep scripts to the standard library.
- The app is not launched.

## Steps

- [x] Step 1: Measure the luma-reuse score on Sony bursts with a trusted AF point, decide, and close the todo item
  - Done when:
    - Both scores were computed for every file of the measured Sony ARW
      folders that has a trusted AF point: the current score
      (`sharpness::score_preview` of the preview with `trusted_focus` and the
      `eye_af_frame` frame, as `scan::score` calls it) and the luma-reuse
      score (`sharpness::score_gray` on `candidate::luma` of
      `decode::decode_rgb` of the same preview, same focus and frame). Each
      file also has the time of the part each one costs: the grayscale decode
      plus window against the luma pass plus window. One thread, release
      build, warm page cache
    - The current-score column was confirmed equal to what pass 2 stores
      (`scan::extract_analysis(..).sharpness`) on a handful of files
    - `luma-reuse.md` in this folder records:
      - the date, host, build and folders, with the file, trusted-AF and
        face-tracked (`eye_af_frame`) frame counts
      - the per-file saving (median, mean, p10, p90) and the preview size
      - the within-burst result from `burst_spearman.py` in its `one` mode,
        over the trusted-AF frames and again split into face-tracked and
        other AF frames: bursts used, Spearman median / p10 / minimum, the
        share of bursts with the same top frame, the lowest bursts by file
        name
      - for every burst whose top frame differs, the two frames' scores under
        both variants (a near-tie or not)
      - the relative score difference over all measured frames (median, max)
      - the decision with the rule and the Trade-offs 1 bar it was made
        against
      - the exact commands run
    - A copy of `burst_spearman.py` is in this folder (the precedent keeps
      the script beside the plan that used it), unchanged unless a change was
      needed, in which case `luma-reuse.md` says what changed
    - If adopted:
      - in `scan::extract_analysis`, a file with a trusted AF point whose RGB
        decode succeeded is decoded once in pass 2, its score taken on the
        luma of the cue's decode with the same focus point and AF frame as
        before
      - a file whose RGB decode fails still gets a score through
        `score_preview`'s own decode, and a cue that errors or panics after
        the decode still leaves the file its score
      - the score of a file with no trusted AF point is bit-for-bit what it
        was
      - unit tests in `crates/core/src/scan.rs` cover the new path, with and
        without an eye-AF frame, and the decode-failure fallback
      - `FACES_VERSION` in `crates/engine/src/index.rs` is the then-current
        value plus one (17 unless another change landed first), its doc
        comment extended with the number, the reason and the measured
        Spearman and top-frame share
      - the module docs of `crates/core/src/sharpness.rs` and the doc
        comments in `scan.rs` say what is true now
      - `docs/humans/performance.md` ("Which pass carries which cost": the
        `Sharpness score (grayscale decode + window, ~3ms)` row and the
        paragraph after the table; the RAF paragraph "Pass 2 still decodes
        the 4416x2944 preview in full twice") and `docs/agents/shared-crates.md`
        (the `FACES_VERSION` 13 note) are updated, with the same change in
        `docs/humans/performance.ja.md`
      - the measurement column and the adopted core agree: the luma-reuse
        score of the measurement equals `extract_analysis`'s `sharpness` on
        the files spot-checked above
    - If not adopted:
      - `crates/core` and `crates/engine` are unchanged
      - `luma-reuse.md` gives the reason
      - one sentence under "Which pass carries which cost" in
        `docs/humans/performance.md` and `docs/humans/performance.ja.md` says
        the AF path's second decode was measured and kept, with a link to
        this plan's archived `luma-reuse.md` path
    - The measurement-only CLI code is not in the merged tree (Trade-offs 4);
      its diff is kept in this folder (`lumascore.patch`, or quoted in
      `luma-reuse.md` if a lint objects to the file)
    - `todo.md` no longer holds the section "Core: a file with a trusted AF
      point still decodes its preview twice in pass 2" (heading, Background
      and TODO), and nothing else in `todo.md` changed
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Measure before changing core. Everything the variant needs is already
      public (`candidate::luma`, `sharpness::score_gray`,
      `sharpness::score_preview`, `sharpness::trusted_focus`,
      `sharpness::eye_af_frame`, `decode::decode_rgb`,
      `reader::read_preview`), so the measurement is CLI-only code in
      `crates/cli/src/main.rs`, as Step 2 of
      [the precedent plan](../_archived/20261123-todo-four-closeouts/plan.md)
      did for `eye_mid`.
    - Recommended form: a small single-threaded subcommand (working name
      `lumascore <dir|file>...`, over `raw_paths`) rather than new `features`
      columns. `features` also runs the cue, the meshes, the eyes path and
      the full-JPEG read (about 0.1-0.3 s per file and several ONNX models in
      memory); the two scores need only one preview read and two decodes per
      file and no model, which suits the shared host. Print a tab-separated
      table with at least `folder`, `file`, `capture_time`, `subsec`
      (`shot.capture_time`, `shot.subsec`, as `eyes_features` reads them),
      `af` (`af` / `noaf`), `eye_af` (`1` / `0`), `sharpness`, `sharp_luma`,
      `gray_ms`, `luma_ms`; `-` for the values of a `noaf` file. Those column
      names let `burst_spearman.py` run unchanged:
      `mise x -- python burst_spearman.py one t.tsv sharpness sharp_luma af=af`,
      then with `eye_af=1` and with `eye_af=0` added. Bursts are formed over
      every row, so print the `noaf` rows too.
    - Timing: `gray_ms` times `score_preview(&jpeg, focus, frame, &[])` (the
      decode pass 2 would drop); `luma_ms` times `luma(..)` plus
      `score_gray(..)` on the already decoded RGB (what replaces it). The RGB
      decode itself is the cue's and is not timed. The saving per file is
      `gray_ms - luma_ms`. Because both are timed in one process on the same
      file, no old/new binary alternation is needed. `analysis_ms` of
      `features` is too noisy for a 4-5 ms difference (the precedent's
      per-file saving had p10 -58 ms, p90 +121 ms from the full-JPEG read).
      Read each folder once untimed first so the page cache is warm.
    - Folders: `D:\photos\2026\2026-09-19` (282 α7 V ARW; 25 face-tracked
      bursts qualified there in
      [burst-rank.md](../_archived/20261123-todo-four-closeouts/burst-rank.md))
      first. Add `D:\photos\2026\2026-09-13-b` (232) only if fewer than about
      30 bursts qualify. Do not go beyond those two without asking
      (`2026-09-13-a` has 622 files). Build with
      `cargo build --release -p handpic-cli`.
    - The script: `burst_spearman.py` in
      `../_archived/20261123-todo-four-closeouts/`. It groups per folder in
      capture order with a 1000 ms gap, uses bursts with at least 3 frames
      carrying both values, average ranks for ties.
    - Core change if adopted, in `crates/core/src/scan.rs`: `af_cue` already
      returns the full-size RGB next to the cue, and `cue_and_gate`'s AF
      branch discards it (`let (cue, _) = af_cue(..)`). Score from it there
      and return the result as the fourth `CueAndGate` value;
      `extract_analysis_unless` already falls back with
      `scored.or_else(|| score(..))`, which keeps the decode-failure case.
      `score_rgb` passes `None` for focus and frame today; the AF path needs
      the focus point and `eye_af_frame(shot)`'s frame, so `cue_and_gate`
      needs the frame (or the shot) handed in. Keep the no-AF call's
      arguments as they are so its scores do not move. `af_cue` returns the
      RGB even when the cue came back unknown through `guarded`, so the score
      survives a mesh panic.
    - `candidate::focus_cue_unless` and the test-only `scan::cue`, named by
      the todo, need no change: pass 2 goes through `af_cue` and
      `focus_cue_rgb_unless`.
    - `scan::extract_face_counts` recomputes the cue but takes no score, and
      `ondemand` does not score either; neither changes.
    - `laplacian_variance` reads only the window's pixels, so a luma of just
      the window would give the same score as the full-frame luma. Measure
      the full-frame `luma` first (it is what exists). Only if it eats a
      visible share of the saving, and the variant is adopted, consider a
      window-only luma; the ranking result covers both because the values
      are identical.
    - The variant applies to every format with a trusted AF point, not only
      ARW (Trade-offs 2).
    - Before bumping, re-read `FACES_VERSION` on `main` (16 today) and take
      the next number; rebase if another bump landed.
    - Keep the raw tables (or their summary) with the plan before the
      scratchpad goes; the precedent's `learnings.md` records losing sight of
      raw output once.

## Trade-offs and risks

1. **What "about as high as the precedent" means in numbers.** The
   precedent's adoption has two readings:
   - (a) Its written bar (Trade-offs 3 of the precedent plan): median
     per-burst Spearman at least 0.95, no burst under 0.8 (and a saving of at
     least about 5 ms).
   - (b) Its measured result: median, p10 and minimum all 1.000 and the same
     top frame in 26 of 26 bursts. That result was trivial, because on the
     Leica DNG previews measured the two lumas were identical on every pixel.
     Here the pixels do differ, so an exact 1.000 everywhere is not expected
     even when nothing meaningful changed.

   Decided bar, between the two: adopt when the median is at least 0.99, no
   burst is under 0.8, the top frame is the same in at least 95% of bursts,
   and in every burst where it differs the two frames' scores are a near-tie
   (within about 1%, the size of the luma-vs-Y difference). Anything else is
   "unclear" and means not adopted.

2. **The ranking check covers Sony only, the change covers every format.** A
   trusted AF point exists on RAF, CR3, NEF, ORF and DNG files too, and all
   would score from the RGB luma after adoption. On a 4416x2944 RAF preview
   the grayscale decode is about 50 ms (`docs/humans/performance.md`), so the
   saving there is much larger than on ARW, while the luma-vs-Y score
   difference measured there was the largest (GX680 RAF, 1.1%). Decided: (a)
   decide on the Sony bursts alone, as the todo and the decision rule say, and
   state the limit in `luma-reuse.md` and the `FACES_VERSION` comment. Timing a
   handful of non-Sony sample files (no ranking) would be cheap and would show
   the real saving, but is not required by the rule.

3. **The cost of adopting.** A `FACES_VERSION` bump re-runs pass 2 (0.1-0.3 s
   per file) on every indexed folder at its next open, to save about 4-5 ms
   per Sony file from then on. The saving is under the precedent's own
   "at least about 5 ms" threshold on ARW. The decision rule given for this
   plan has no saving threshold, so the plan follows the ranking alone;
   `luma-reuse.md` records the measured saving so the cost is on record.
   Adopting without a bump is not offered: rows scored before and after
   would mix two slightly different scores within a burst of a partly
   scanned folder.

4. **The measurement code.** Decided: drop it in both outcomes. If adopted,
   the two columns are equal by construction; if not, it measures a variant
   that was turned down. The method survives in `luma-reuse.md`, with the diff
   of the subcommand kept in this folder, so the run can be reproduced, and
   `CLAUDE.md` needs no change.

5. **A middle result.** For example a high median with one or two bursts
   whose top frame moves between frames that are not near-ties. Unclear means
   not adopted. List those bursts by file name in `luma-reuse.md`.

6. **Noise on a shared host.** The per-file times are a few milliseconds and
   other sessions run on the machine. Report medians over the files, state
   the conditions, and do not answer noise by adding load or repeating the
   run many times; a second single pass is the most that fits the
   constraints.

## Progress
