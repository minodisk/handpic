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

# Gate count from 0.6 on frames without an AF point

## Purpose

The all-faces check runs pass 3 only on frames whose gate count
(`allfaces::gate_count`, stored by pass 2 in `faces_group`) reaches the group
threshold, 5 by default. On a frame without an AF point the gate counts faces
from `sharpness::FACE_CONFIDENCE` (0.8), so the 7-person posed group
`2026-02-01/L1005161.DNG` counts 4 (three of its faces score 0.64 to 0.75) and
never reaches the gate: the default passes no posed group photo of the
measured folders.

The user decided (not open for this plan): the default `groupMinFaces` stays
5, the path without an AF point counts from 0.6 like the AF path
(`allfaces::GATE_LOW_RES_CONFIDENCE`), and `FACES_VERSION` is bumped so pass 2
re-runs. Once done, a posed group on a body without an AF point (the Leica
DNGs) reaches the default gate, and the todo.md item "Core: the default group
threshold N = 5 passes no posed group photo of the measured folders
(all-faces-check)" can be closed.

Background: [the all-faces-check plan](../20261123-all-faces-check/plan.md)
and its [measurement.md](../20261123-all-faces-check/measurement.md),
which names the measured folders and how they were measured.

## Steps

- [x] Step 1: Count the gate from 0.6 on the path without an AF point, measured before and after
  - Done when:
    - Before any behaviour change is committed, the before / after measurement
      below is done and recorded in `measurement.md` in this plan folder, and
      the stop condition was checked. **If the stop condition is met, nothing
      is committed and the implementer reports back to the caller with the
      numbers and the list of newly passing frames** (the user is consulted).
    - `2026-02-01/L1005161.DNG` has a gate count of 5 or more after the change.
    - On a frame without an AF point, pass 2's gate count counts a face of the
      judged face's size scoring 0.6 or above; a unit test shows it (a face at
      `GATE_LOW_RES_CONFIDENCE` and one between 0.6 and 0.8 count, one at
      `GATE_LOW_RES_CONFIDENCE.next_down()` does not, on the AF-less path's
      rule).
    - `FACES_VERSION` in `crates/engine/src/index.rs` is bumped by one, with
      its doc comment saying what the new version changed.
    - Every passage describing the gate's floors states one floor for both
      paths: `CLAUDE.md`, `docs/humans/performance.md` and
      `docs/humans/performance.ja.md` (kept in sync), and the code doc
      comments listed below.
    - `measurement.md` holds the before / after table and `learnings.md` what
      was learned; the PR body carries the table.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **No new detection is needed and the change has no run-time cost.** The
      AF-less path's whole-image search (`faces::detect_whole_upright` at
      `faces::WHOLE_INPUT`, 640x448) already returns every face from
      `faces::CATCH_CONFIDENCE` = `SCORE_THRESHOLD` = 0.6
      (`detect_scaled_upright`'s filter). The 0.8 floor is only the argument
      at the call site in `crates/core/src/scan.rs`, `cue_and_gate`'s `None`
      arm: `allfaces::gate_count(f, cue.face.as_ref(), FACE_CONFIDENCE)`. The
      change is that argument becoming `allfaces::GATE_LOW_RES_CONFIDENCE`.
      Keep the input at `WHOLE_INPUT`; do not move the AF-less path to
      `GATE_INPUT`. The archived measurement already predicts the result: "On
      `L1005161.DNG` a 0.6 floor on the 640x448 path would count all 7 faces."
    - Leave everything else on `FACE_CONFIDENCE`: the cue's choice of the
      judged face, the sharpness score's fallback faces, and
      `allfaces::candidates` (pass 3). `scan::extract_face_counts` does not
      call `gate_count`, so `ALLFACES_VERSION` is not bumped; the
      `FACES_VERSION` bump re-runs pass 2, which clears each row's all-faces
      counts, so pass 3 re-runs on the rows at the gate anyway.
    - Order of work, the measurement first:
      1. Build the release CLI from the branch point (no change yet):
         `cargo build --release -p handpic-cli`. Run
         `handpic-cli features <folder> [threads]` on each measurement folder
         and keep the TSVs in a scratch folder outside the repository. The
         gate count is a column of `features` (see `features_line` in
         `crates/cli/src/main.rs`); `allfaces` and `facechoice` do not print
         it. Folders, from the archived `measurement.md`: without an AF point
         `2026-02-01` (146 DNG), `2026-09-05` (411 DNG), `2026-10-31`
         (954 DNG); with one `2026-09-19` (282 ARW), the first 300 ARW of
         `2026-11-07`, the first 100 of `2026-10-03`. The folders are on the
         user's machine; if one cannot be found, say which in the report and
         in `measurement.md` and measure the rest.
      2. Make the call-site change in the working tree only, rebuild, re-run
         the same commands.
      3. Compare with a script (run Python through `mise x -- python` on the
         Windows host): per folder, the frames at gate count N >= 3 / 4 / 5
         before and after, and the names of the frames newly at N >= 5. The
         three AF folders must show 0 differing gate counts and every other
         `features` column must be unchanged on all folders (the change
         touches only the gate count); a difference there is a bug, not a
         finding. Known "before" for `2026-02-01`: 9 / 3 / 0.
      4. Look at every frame newly at N >= 5 (the preview, or
         `facechoice`'s per-face dump for the boxes and scores) and class it:
         a group (5 or more people at about the judged face's distance) or
         not (background rows, spectators, passers-by, false detections).
         Record both counts per folder.
      5. Stop condition (decided): stop when, in any no-AF folder, the frames
         newly at N >= 5 that are not groups outnumber the groups gained
         there, or exceed 2% of that folder's frames (3 of 146, 8 of 411,
         19 of 954). If met, commit nothing and report.
    - `gate_count` keeps its `floor` parameter and `GATE_LOW_RES_CONFIDENCE`
      keeps its name (decided: the smallest diff); its doc comment says it is
      now the floor of both searches.
    - The test: the existing
      `scan::tests::without_an_af_point_the_gate_counts_over_the_whole_image_search`
      runs a blank JPEG, which finds no face and so cannot show the floor.
      Pull the `None` arm's count into a small pure function in `scan.rs`
      taking the faces and the cue, and test it with synthetic faces.
    - Doc comments to update with the code, all of which state the two
      floors today: the module doc of `crates/core/src/allfaces.rs`,
      `GATE_LOW_RES_CONFIDENCE`'s own comment, `gate_count`'s comment,
      `faces::GATE_INPUT`'s comment in `crates/core/src/faces.rs`,
      `cue_and_gate`'s and `Analysis::gate_count`'s comments in
      `crates/core/src/scan.rs`, and `FACES_VERSION`'s comment in
      `crates/engine/src/index.rs` (add a sentence for the new version, in the
      style of the earlier ones).
    - Documentation: `CLAUDE.md`'s `src/allfaces.rs` passage ("searched at
      `faces::GATE_INPUT` from `GATE_LOW_RES_CONFIDENCE` on an AF frame and
      over the cue's own whole-image search from `FACE_CONFIDENCE` without
      one") becomes one floor on both searches;
      `docs/humans/performance.md`'s "All-faces check (Windows 11)" section
      says "the 640x448 path keeps 0.8", which becomes false, and
      `docs/humans/performance.ja.md` carries the same sentence in Japanese
      and must change in the same PR. No guide under `docs/agents/` mentions
      the gate's floors (searched), so none changes unless the
      implementation finds one.
    - No app code changes: the meta pane's `Similar faces` row, the strip's
      closed-eyes mark and the filter read the stored gate count through
      `group::shown`.
    - The full re-run of pass 2 on every indexed folder after the bump is the
      decided consequence; say so in the PR body.

## Trade-offs and risks

- **Pass 3 still judges from 0.8.** `allfaces::candidates` keeps
  `FACE_CONFIDENCE`, so `L1005161.DNG` will pass the gate with 5 or more and
  pass 3 will judge at most its 4 faces at 0.8 or above: `Similar faces` can
  exceed `Faces judged` by more than before. Lowering the candidates' floor is
  a different change and out of scope; if the measurement shows it matters,
  record it in `learnings.md` for a todo.md proposal.
- **More pass 3 work on bodies without an AF point.** Every frame that newly
  reaches the gate pays `extract_face_counts`. The gate's own cost does not
  change.
- **The local CLI build.** The measurement needs a release `handpic-cli`
  built twice on this host. The folders are local, so this cannot move to CI;
  only `-p handpic-cli` is built.
- **The name `GATE_LOW_RES_CONFIDENCE`** no longer describes where it is used
  (the AF-less search is 640x448); kept for the smallest diff.
- **Background rows.** The same size-band effect as the open todo.md item on
  `2026-11-07/_DSC1611.ARW` can appear on no-AF frames at the lower floor; the
  classification in the measurement is where it would show.

## Progress

- (2026-11-27) Step 1 complete
