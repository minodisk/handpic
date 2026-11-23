<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# The burst "best" sharpness mark: measure it against the good-photo mark, then reword, re-base, align or drop it

## Purpose

`relativeSharpness` in `crates/app/ui/src/sharpness.ts` marks the frame
holding its burst's maximum `sharpness` score (and, for a single, the maximum
among up to two singles on each side). The mark is shown in two places: the
strip's left-edge bar turns the pick color (`strip.ts` `paintSharpness`,
`.cell span.sharpness.best` in `crates/app/ui/style.css`), and Compare
labels the highest-scoring frame `BEST` with a green outline (`main.ts`,
the Compare canvas draw; `compare.ts` `comparisonCandidates` also pairs a
lone selected file with its burst's highest-scoring frame). The docs sell
it as "the sharpest frame of a run" (`docs/humans/usage.md` "Sharpness
cue", `README.md` "Sharpness cue" and the Compare bullet, `cameras.md`
"AF frame size and face tracking").

`burst-keep-score` Step 2 (`docs/plans/_archived/20261008-burst-keep-score/results.md`,
Reading) measured it on the user's picks: within a burst the mark ranks picks
only slightly above random (pairwise AUC 0.577 against 0.495; the first
frame of a burst 0.653), and the hand check found sharp frames at 0.11-0.67
of their burst's maximum because the AF-region score follows the texture
under the AF point (a shirt print, a ball), not the subject's focus. The todo
item (`todo.md`, "App: the burst "best" sharpness mark ranks the user's
picks near random") lists: reword what the mark means, measure it on the face
only, or drop it.

**The reference is the good-photo mark, not the picks (the user,
2026-10-10).** Picks within a burst are timing-driven (Decision 1 of
`burst-keep-score`), so they are a weak within-burst label. The good-photo
mark (`focus.ts` `photoTier`: `eye_focus >= 0.90`, eyes open, facing the
camera, mesh-fit and edge exclusions) was validated on 180 frames the user
rated (1 = no subject, 2 = likely rejected, 3+ = pass; 54 of 58 good frames
pass, no 1-star), so it says which frames are not misses. The question this
plan answers is therefore: **does the burst's "best" mark point at a good
frame when the burst has one, and does it avoid misses?** The picks stay as
a secondary check.

This plan measures first, with no UI change until the user approves a
Decision, then implements the chosen option and brings the docs in line.

Out of scope: the good-photo rule's thresholds, the closed-eyes / head-pose
/ cue constants (`candidate.rs`, `eyes.rs`, `pose.rs`, `focus.ts`
constants), and `BURST_GAP_MS`.

## Steps

- [ ] Step 1: Re-dump the data set with a current `riffle-cli features` and measure the current mark and its alternatives against the good-photo mark within bursts; record the Decision
  - Done when: `dump.sh`, `measure.py` and `results.md` are in this plan
    folder; `results.md` holds, per candidate measure, the good-agreement
    metrics below (primary), the pick metrics (secondary) and the stars
    guard, on the ARW block (the DNG blocks have no AF point, so no good
    frames: report the mark's behavior there only), and a "Reading" that
    says which option the numbers support; the "Decision" section of this
    plan is written as "Proposed" with the option it recommends and the
    user has approved one of (a) / (b) / (c) / (d) (recorded in Progress).
    No UI change in this step.
  - Implementation approach:
    - Data: the 36 folders of
      `docs/plans/_archived/20261008-burst-keep-score/dump.sh` (copy the
      loop; output to `D:\Photos\tests\2026-10-10-burst-best-mark\dump\`,
      a release `riffle-cli` built from `main`, 24 threads). The current
      `features` header has `sharpness`, the mesh `eye_focus`, `state`,
      `cue_side`, `ear`, `yaw` / `pitch` / `roll`, `eye_offset` and
      `edge_gap`, so every value `photoTier` reads is in the dump. Bursts
      follow `burst.ts` `groupBursts` at 1000 ms; labels (for the
      secondary pick metrics) follow `metrics.py` of the archived plan;
      reuse its `load` / `auc` code. Port `photoTier` / `clears` from
      `focus.ts` with its current constants (a frame missing any value is
      not good) and check the port on the 180 rated frames against
      `D:\Photos\tests\2026-10-09-good-mark\step7\` (the tier must match).
    - Candidate "best" rules, each marking the frame(s) with the burst's
      maximum as `relativeSharpness` does (ties share the mark):
      1. `sharpness` (the current mark),
      2. `eye_focus` where the frame has a cue face; frames without one
         are never best,
      3. `eye_focus` where any frame of the burst has a cue face, else
         `sharpness`,
      4. the good tier itself: the burst's good frames, then by
         `sharpness` among them (one frame) — option (d)'s rule,
      5. baselines: first frame, random (seed 0).
    - Primary metrics (good agreement), over bursts of 2+ frames, split
      into bursts with at least one good frame and bursts with none:
      - with a good frame: the share of bursts whose best frame is good
        (hit), and the share whose best frame is not good although one is
        (miss);
      - with no good frame: where the best lands (the share on a frame with
        no cue face, on a candidate that is not good, on a not-candidate);
      - the tie rate at the maximum per rule (`eye_focus` saturates near
        1); if ties blur a rule, add `eye_lap` / `eye_edge` columns to
        `features` (the scored eye's `EyeFocus.lap` / `edge_width`,
        `features_line` and its test change), re-dump and measure the raw
        Laplacian as rule 2b; that CLI change ships in this step's PR;
      - per folder as well as pooled.
    - Secondary (picks): the within-burst pairwise AUC and the top-1 hit
      on the picks per rule, comparable with `burst-keep-score`'s 0.577 /
      0.653 (rule 1 should reproduce 0.577 within about 0.01, which
      validates the port).
    - Stars guard: of the 180 rated frames in
      `D:\photos\samples\ARW\good-mark-2026-10-09\` (scores in
      `D:\Photos\tests\2026-10-09-good-mark\samples-scored.tsv` and
      `samples-scored-batch2.tsv`, `name` = `<folder>_<file>`), the mean
      stars and the 1- / 2-star counts of the rated frames each rule marks
      best against those it does not.
    - Write the "Decision" section below (Proposed) with the four options,
      the numbers each rests on, and the recommendation; the user approves.
      Record what was learned in `learnings.md`.

- [ ] Step 2: Implement the approved Decision in the frontend
  - Done when: the strip bar and Compare show what the Decision says;
    `sharpness.test.ts`, `compare.test.ts` and `focus.test.ts` (as touched)
    pass; `mise run ci` passes; a manual check on
    `D:\photos\2026\2026-09-19` after pass 2 confirms the strip and
    Compare (listed in `learnings.md` as pending if not run).
  - Implementation approach (only the approved option is done):
    - (a) Reword: no logic change. Code comments in `sharpness.ts`,
      `strip.ts` (`paintSharpness`) and the Compare draw say the mark is
      the highest AF-region score, not the sharpest subject; rename
      Compare's `BEST` label (e.g. `SHARPEST`) so it does not claim more
      than the measure; the docs change is Step 3.
    - (b) Re-base on the face measure: keep `relativeSharpness` as the
      per-burst maximum logic and change what `main.ts` `applySharpness`
      feeds it (`score:`), Compare's `best` and `comparisonCandidates` to
      the chosen rule (2 or 3, with the fallback exactly as measured).
      `eye_focus` is already in every entry's focus, so no backend change;
      the raw eye Laplacian would need a new index column and an
      `EXTRACTOR_VERSION` bump (a separate backend PR before this step;
      split into 2a / 2b then).
    - (d) Align with good: the best frame is chosen among the burst's good
      frames (by `sharpness` among them, or the rule the Decision names);
      a burst with no good frame shows no best mark, or falls back as the
      Decision says. `relativeSharpness` takes the tier (it already gets
      the entries); the bar's ratio display stays unless the Decision
      drops it.
    - (c) Drop: remove the `best` field of `RelativeSharpness` and the
      `.best` class (the bar keeps showing the ratio unless the Decision
      drops the bar too), and remove Compare's `BEST` label and green
      outline. What Compare pairs a lone file with (`comparisonCandidates`)
      is part of the Decision: the burst's good frame, the burst's first
      frame, the previous frame, or the highest-scoring frame without the
      `BEST` label.
    - A `companion.ts` / `mcp.rs` change only if the Decision changes what
      `set_view` Compare pairs with (`mcp.rs` "its burst's sharpest
      frame").

- [ ] Step 3: Bring the user docs, `README`, `CLAUDE.md` and the todo in line
  - Done when: `docs/humans/usage.md` + `usage.ja.md` ("Sharpness cue",
    "Compare", the `v` key row, `get_view` / `set_view` rows if touched),
    `README.md` + `README.ja.md` ("Sharpness cue" bullet and the Compare
    bullet), `docs/humans/cameras.md` + `cameras.ja.md` ("AF frame size and
    face tracking") say what the mark now is and, in one sentence, what it
    was measured on (Step 1's good-agreement numbers); `CLAUDE.md`'s
    `sharpness.ts` / Compare mentions match; the todo item is removed from
    `todo.md`; `mise run ci` (lychee included) passes.
  - Implementation approach:
    - Each English / Japanese pair changes in the same PR.
    - Can be the same PR as Step 2 if the Step 2 diff is small.

## Trade-offs and risks

- **Good is a set, the mark is one frame.** A burst can hold several good
  frames; "hit" counts the best landing on any of them. Good covers about
  20% of faced AF frames, so many bursts have none; the no-good split keeps
  those from diluting the hit rate.
- **Overlap with the good icon.** Under (d) the best mark sits on a frame
  that already carries the strip's good icon; the mark then adds only "which
  of the good frames", which the user may not need. That is the case for
  (c): if the good icon already answers the question, the best mark can go.
- **`eye_focus` saturates.** Ties at the top are likely under rules 2 / 3;
  Step 1 reports the tie rate and measures the raw eye Laplacian if needed
  (a new index column for (b), a backend PR).
- **Frames without a face.** Rules 2 and 4 leave face-free bursts unmarked;
  rule 3 falls back to the texture score there, which keeps the known
  failure where it is worst. The Decision must pick one.
- **Option (c) changes Compare's one-file behavior.** The Decision must name
  what a lone file is paired with.
- **Re-dump cost and drift.** About 9 minutes at 24 threads over ~106k
  files; sidecars may have drifted since 2026-10-08, so rule 1's AUC may
  move slightly from 0.577.
- **Stars are frame level only.** Only 3 bursts hold two rated frames, so
  the stars guard checks that the marked frames are not misses, not that
  they are better than their neighbors.

## Decision

(Written at the end of Step 1: Proposed, then Approved by the user.)

## Progress

- (none yet)
