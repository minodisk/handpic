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

# `eye_focus` gate and fill: a mesh-misfit gate re-measured on the eye window, the no-face fill, and their combination, confirmed on a fresh blind round

## Purpose

Riffle's AI goal is to **suggest frames to discard** (sorted last, not
hidden) so the human picks from what remains; the label is "OK to discard?".
The bar to beat is today's `eye_focus` cue (`ef`: the in-focus probability of
the eyes of the face nearest the AF point, missing sorted last). The
discard-model plan's learned `LR/T` did not beat it on a fresh blind round
(AUC 0.729 against `ef` 0.743, paired −0.013 [−0.048, +0.023]; discardmodel.md,
"Results of the confirmation", in `docs/plans/20261023-discard-model/`, which
may move under `docs/plans/_archived/`), and the face-choice evaluation found
no better face rule than the AF-nearest face
([facechoice.md, Decision](../_archived/20261023-face-choice/facechoice.md)).

This plan changes **`eye_focus` itself**, not a model over it, with two
label-free rules that have not been tested and their combination:

1. **Mesh-misfit gate (`ef-gate`)**: on the judged face, when the face
   mesh's nose tip (and, in the second statistic family, its eyes too)
   sits far from YuNet's landmark past a threshold `τ`, the mesh is taken
   as a misfit and the face is **re-measured on the YuNet eye window with
   the window model** (`candidate::eye_focus`'s `Scored::Window` path, the
   one a face under `EYES_MIN_FACE` takes today). Not missing, not neutral.
2. **No-face fill (`ef-fill`)**: a frame whose 480 px crop holds no face
   (`ef` missing) takes the whole-image AF-nearest face's `eye_focus`
   (`af-whole`; the discard-model plan's `ef-fill`).
3. **Both (`ef-gate+fill`)**: fill, then gate the filled face too.

The exploration tunes the statistic and `τ` on rounds 1 and 2 and picks one
winner mechanically on round 3; the confirmation reads it once on a **new
fresh blind round** the user labels, with the bars pre-registered before any
score is read. **Offline evaluation only; no app code change.** The only
code is a `riffle-cli facechoice` extension (Step 2) that dumps the per-face
values the rules need.

## Decisions (by the user, at planning time, 2026-11-02 to 2026-11-04)

1. **Gate statistic: the nose offset, not the existing `eye_offset`.**
   `eye_focus` is already measured on the mesh eyelid regions when a mesh
   exists, so YuNet-vs-mesh eye disagreement mostly flags frames where
   YuNet's eye points are off and the mesh is right (the mesh-roll finding:
   about 65% of `eye_offset` > 0.10 are profiles with a fitted mesh); gating
   those hurts. The gate targets mesh misfits: **`nose`** = the distance
   between the mesh nose tip (point 4) and YuNet's nose landmark, divided
   by the face box's longer side (not the eye distance, which shrinks on
   profiles). A second family, **`max`** = max(nose offset, eye offset),
   same normalization. The family and `τ` are chosen together on rounds
   1–2. This needs the CLI extension and a rerun on all 33 folders.
   `eye_offset` alone stays a label-free reading in Step 3, not a
   candidate. (Rejected: the existing `eye_offset` as the statistic; an
   eyes+nose statistic normalized by the eye distance.)
2. **A gated face is re-measured on the YuNet eye window with the window
   model** (`Scored::Window`), per face, dumped by the CLI extension.
   (Rejected: missing / sorted last; a neutral value (the folder's median
   `ef`); a fallback to `af-whole`'s face.)
3. **`τ` and the family tuned on rounds 1 and 2; round 3 selects the
   winner.** (Rejected: tuning on all three rounds.)
4. **Stop rule**: AUC − `ef` point above zero on round 3 and on each of
   rounds 1 and 2. (Rejected: round 3 and the sum of rounds 1 and 2.)
5. **Round-4 population: the never-labeled frames of the 33 folders.** The
   Decision states it is fresh frames of known folders (optimistic; the
   cluster bootstrap read as a within-folder check). (Rejected: waiting
   for new shoots, since no folder newer than `2026-10-17` exists; deciding
   after Step 4.)
6. **No profile exemption of the gate.** Step 3 prints, label-free, the
   gated share by |yaw| band; Step 4 reads the label × gated cross-tab; the
   Decision may propose an exemption. (Rejected: gating only when YuNet's
   eye distance is >= 0.20 of the box.)
7. **The `τ` grid is fixed by a label-free percentile rule** pre-registered
   in Step 1 (below); its values come out of Step 3 mechanically.
   (Rejected: the user fixing the values after seeing Step 3's
   distribution.)
8. **A gated face whose window score is 0** (no edge width, `NotCandidate`)
   keeps 0 as a real low value, as a face under `EYES_MIN_FACE` does today.
   (Rejected: treating it as missing, sorted last.)

### What is known (investigated 2026-10-30)

- **Today's code.** `candidate::mesh_eye_offset`
  (`crates/core/src/candidate.rs`): the larger of the two distances between
  a mesh eyelid-contour center (mean of `LEFT_EYE_CONTOUR` /
  `RIGHT_EYE_CONTOUR`) and YuNet's eye landmark, in the pairing that keeps
  it smaller, over the box's longer side; stored as `EyeMeasures.eye_offset`
  / `Cue::eye_offset` / `files.eye_offset`, printed by `riffle-cli
  features`, and gated by `rank.rs`'s good-photo rule at `MAX_EYE_OFFSET =
  0.1`. **No nose offset exists**: `faces::Face` has `nose` (YuNet), the
  mesh nose tip is point 4 (as `pose.rs`'s tests read it), `eyes::Mesh.points`
  are in stored coordinates like the face. `candidate::eye_focus(gray, w,
  h, face, None)` is the window-model score of a face (`Scored::Window`,
  probability 0 and `NotCandidate` without an edge width); with a mesh it
  scores the sharper counting eyelid region, else the window. The
  `facechoice` dump (`crates/cli/src/facechoice.rs`, header
  `folder … left_eye right_eye nearest eye_focus state scored ear yaw pitch
  roll mesh_ms`) carries neither the nose, the mesh points, the eye offset
  nor the window score, so it is extended and rerun.
- **Prior finding on `eye_offset`** (the mesh-roll plan, recorded in
  `_archived/20261008-burst-keep-score/plan.md` and
  `docs/agents/tract-onnx-inference.md`): of frames with `eye_offset` >
  0.10, about 27% real mesh misfits, about 65% fitted meshes with YuNet eye
  points off the eyes (profiles, YuNet eye distance under about 0.20 of the
  box). Spot counts, label-free: `2026-10-17` (fresh, 1,675 frames) 104
  without `ef`, 595 with `ef` but no mesh (faces under 60 px), 220 with
  `eye_offset` > 0.10 (13.1%); `2026-06-05` (dump, 1,337) 429 / 76 / 298
  (22.3%). So misfits are expected around 3–6% of the meshed faces, which
  the `τ` grid rule below brackets.
- **Labels, three rounds, all exploration data.** Round 1
  (`D:\Photos\tests\2026-10-18-discard-labels\`, 600, 211 keep / 389
  discard, population the 27-folder dump minus the 1,200 strict frames =
  31,259), round 2 (`D:\Photos\tests\2026-10-20-model-search\`, 600, 248 /
  352, population 30,659, `strata.tsv` 31 cells), round 3
  (`D:\Photos\tests\2026-10-23-discard-model\`, 600, 127 / 473, population
  the six fresh folders = 14,525, strata `b05` / `b10` / `b20` / `efonly` /
  per-folder `control`, cut by `LR/T` and `ef`; labeled under the "sorted
  last" wording, rounds 1–2 under "hid"). Round 3 was read once (for
  `LR/T`) and its **not pre-registered** subset reading (`ef-fill` − `ef`
  AUC +0.277 [+0.131, +0.407] on the 201 labels without a crop face)
  motivates idea 2, so round 3 is selection data here. Each round keeps its
  own population, `π` and strata.
- **Known numbers to reproduce** (weighted by the round's `1/π`,
  stratified bootstrap 1,000 draws seed 0): round 3 `ef` AUC 0.743 [0.679,
  0.800], `ef-fill` 0.744 [0.681, 0.803], `ef-fill` − `ef` +0.002 [−0.008,
  +0.012], base rate 60.56% [53.81, 67.29], `ef` `RR` 11.3 / 14.6 / 14.8%
  and `PL` 4.07 / 7.30 / 14.99% at q5 / q10 / q20, missing-face subset
  `ef-fill` − `ef` +0.277; round 2 `ef` 0.785 [0.723, 0.844], `ef-fill`
  −0.010 [−0.022, −0.001], base rate 42.31%; round 1 `ef` 0.777 [0.716,
  0.841], `ef-fill` −0.011 [−0.028, +0.003], base rate 48.45%. Missing `ef`
  share 12.4% on the dump (4.5% after the `af-whole` fill), 9.00% on the
  fresh six (1.18% after). `facechoice --check` reproduced the stored
  `eye_focus` on all 32,459 + 14,525 frames (0 mismatches).
- **Dumps, complete for all 33 folders**: `D:\Photos\tests\2026-10-12-rank-app\`
  (27 folders; `features\<folder>.tsv` with `eye_offset` at full precision,
  `embed.tsv`, `genre_text.tsv`) and `D:\Photos\tests\2026-10-23-discard-model\dump\`
  (six folders; same layout, header a superset, 4 decimals); the old
  per-face dumps `…\2026-10-23-face-choice\faces\` and
  `…\2026-10-23-discard-model\dump\faces\` (checked against the new dump
  in Step 3); `ef-fill` already in `scores_model.tsv` / `scores_fresh.tsv`.
  Every frame of the 33 folders has a trusted AF point. Source folders:
  `D:\Photos\2026\<folder>` (the rank-app `dump.log` and the discard-model
  `freshdump.sh` list them), **read-only**.
- **Compute reference**: `facechoice --check` took 1,571 s for the 27
  folders and 667 s for the six on 12 threads (about 37 min); the window
  score per face adds a Laplacian and an edge width over one small window,
  negligible.
- **No folder of `D:\Photos\2026` is newer than `2026-10-17`** and every
  folder with picks is in the 33; the never-labeled frames of the 33
  folders number about 43,984 (46,984 minus 1,200 strict and 1,800
  discard-labeled).
- **Harness, imported by path, none modified**: `badfilter` (`read_tsv`,
  `strat_counts`, `cluster_counts`, `graded_cut`, `finite_by_folder`,
  `aucs`, `auc_pair`, `quant`, `iv`, `civ`, `Tee`; brings `rf`, `gate`
  (`load`, `build`), `bf.sample` = `bs` (`allocate`, `under`, `listing`,
  `sha256`, `cell`)), `discardfilter` (`metrics`, `unweighted`, `counts`,
  `QS`, `PL_BAR`, `REMOVED_BAR`), `facechoiceread` (`load_faces` pattern,
  `choose`, `auc_draws`, `cut_metrics`, `diff`, the rounds 1–2 cell
  construction), and the discard-model plan's `freshread.py` (round 3's
  cells from its `strata.tsv`), `freshsample.py` (`rank_share`, strata,
  the copy with SHA-256 checks, the refusal to write under
  `D:\Photos\2026`), `freshlabels.py`, `labeling.md`, `freshdump.sh` /
  face-choice `run.sh` (the per-folder resumable background run). **Do not
  call `badfilter.load`** (it opens the strict labels). **Name no script
  `sample.py`.** `gate.load` needs `embed.tsv`; both dumps have it. The
  discard-model folder may be archived mid-plan: resolve its path at run
  time from either location.
- **Python**: `D:\Photos\tests\.venv-eval\Scripts\python.exe -I -B` on
  script files only, never heredocs or `-c`; run from the scratchpad; kill
  a hung process by PID only; each tool call under 10 minutes, long runs
  in the background with a bounded `timeout … tail -F | grep -m1` wait;
  `cargo` through `mise exec -- cargo …`; check `git status` for stray
  `__pycache__/`.
- **Where the data goes**: `D:\Photos\tests\2026-11-04-ef-gate-fill\`
  (`faces\<folder>.tsv` the new per-face dump, `logs\`, a frozen copy of
  the release `riffle-cli.exe`, `scores_gate.tsv`, `*.out.md`,
  `manifest.tsv`, `strata.tsv`, `sample\`, `labels.tsv`).
- **Out of scope (do not touch)**: `rank.rs`'s `pairs`, `Labeled`, `fit`;
  the `views` table and its data; `docs/plans/20261023-discard-model/`
  (read only); `faces.rs`, `eyes.rs`, `candidate.rs` and the scan are not
  edited; no app code changes.

### The candidates (pre-registered in Step 1, dumped in Step 2, scored in Step 3, read in Step 4)

Every score is "higher keeps"; a missing score sorts last (`-inf` in the
cut, the folder's lowest finite value minus 1 in the AUC). `ef` on every
frame is the per-face dump's `af` value (full precision; equals the stored
`eye_focus`), as round 3 took it.

- **References, not candidates**: `ef`, random (seed 0).
- **Per-face values** (Step 2's dump, per face of both sets): `nose_offset`
  = |mesh point 4 − YuNet `nose`| / box longer side; `eye_offset` =
  `mesh_eye_offset`; `max_offset` = max of the two; `window_ef` =
  `candidate::eye_focus(…, None).probability`; all `-` without a mesh
  except `window_ef`.
- **Gate statistic family** `s` ∈ {`nose`, `max`}; a face with `ef` but no
  mesh has no `s` and is never gated (it is already on the window).
- **`ef-gate(s, τ)`**: on the frame's judged face (today's `af` face),
  `window_ef` where `s > τ`, else `ef`; missing where `ef` is missing. A
  `window_ef` of 0 stays 0 (Decision 8).
- **`ef-fill`**: `ef`, the missing filled by the whole set's `nearest`
  face's `eye_focus` (`facechoiceread.choose`), the rest last; checked
  equal to the discard-model plan's column.
- **`ef-gate+fill(s, τ)`**: the fill first, then the gate on the face
  behind the value, the filled face included (its `s` from the whole-set
  row; a filled face under 60 px has no mesh and keeps its window value).
- **`τ` grid, fixed by a label-free rule before any label is read**: for
  each family, the values of `s` at the **80th, 90th, 95th and 97.5th
  percentiles** over the meshed `af` faces of all 33 folders, rounded to 3
  decimals (Step 3 prints the distribution and the grid; Step 1
  pre-registers the rule, not the values). Justification: misfits are
  expected around 3–6% of meshed faces (the mesh-roll finding), which the
  90th–97.5th bracket, with the 80th as a loose end; the same rule for
  both families keeps the gated shares comparable.
- **Tuning (mechanical, rounds 1 and 2)**: over the 8 (`s`, `τ`) pairs,
  the one with the largest sum over rounds 1 and 2 of `ef-gate(s, τ)`'s
  weighted AUC − `ef`'s (each round on its own population and `π`); ties
  to `nose` then the smaller `τ`. Rounds 1 and 2 are read for this only;
  `(s*, τ*)` is shared by both gate families.
- **Selection (round 3, primary)**: among `ef-gate(s*, τ*)`, `ef-fill`,
  `ef-gate+fill(s*, τ*)` whose q5 or q10 cut passes the label-free
  conditions on all 33 folders (`PL` <= 5% of the 6,214 picks, removed >=
  4.5%), the one with the highest weighted AUC on round 3; ties by the sum
  of rounds 1 and 2. **Stop rule**: the winner goes to the fresh round
  only if its AUC − `ef` point is above zero on round 3 **and** on each of
  rounds 1 and 2; otherwise Steps 5–6 are skipped and the Decision is
  "nothing, keep `eye_focus`".
- **Readings beside** (decide nothing): per round the cut tables at q5 /
  q10 / q20 (`DP`, `RR`, `PL`, removed, `RR` − `ef`); the AUCs on three
  subsets: no crop face, gated at `(s*, τ*)`, the complement; the label ×
  gated cross-tab and, label-free, the gated share by |yaw| band (Step 3)
  and `eye_offset`-only gating's shares for reference.
- **Multiplicity**: 8 pairs × 2 rounds for the tuning, 3 candidates × 3
  cuts on round 3, plus the subsets, read without correction on labels the
  design was chosen after (round 3's subset reading motivated the fill);
  hence the mechanical rules and the single fresh read. One photographer,
  mostly basketball.

### The confirmation (pre-registered in Step 5, before any fresh label)

- **Population**: the never-labeled frames of the 33 folders (every dump
  frame not in the three rounds' manifests nor the strict sample's
  `manifest.tsv`, about 43,984). **Strata**: the winner's per-folder bottom
  bands `b05` / `b10` / `b20` on the population, `efonly` (`ef`'s bottom
  20% outside the winner's), per-folder `control` with a floor of 3 over
  the 33 folders; **600 frames, 120 per stratum**; `π` = drawn / cell
  population; one `default_rng(20261104)` generator in the order `b05`,
  `b10`, `b20`, `efonly`, the `control` cells by folder name, then the
  shuffle. Copies `<nnnn>_<folder>_<file>` in `sample\`, no sidecar,
  SHA-256 checked, the sampler refusing to write under `D:\Photos\2026`.
- **Labeling**: round 3's guide word for word (the "sorted last"
  question), rating 1 = keep, unmarked = discard, `aiEnabled` off, **never
  Pick / Reject / color label**; nothing reads a sample sidecar before the
  user says "done".
- **Scores read once**: the winner (a label-free rule, no refit), `ef`,
  `ef-fill`, random; the fresh `1/π`; the stratified bootstrap and the
  33-folder cluster bootstrap (read as within-folder); the base rate.
- **Bars** (the discard-model plan's five): a cut of the winner at q5 or
  q10 is usable when (1) weighted `RR` <= 25% with stratified 97.5%
  quantile <= 35%; (2) `PL` <= 5% of the population's picks (label-free);
  (3) removes >= 4.5% of the population (label-free); (4) `DP`'s
  stratified 2.5% quantile above the base rate; (5) **beats `ef`**: paired
  AUC − `ef`, stratified 2.5% quantile above zero (the key bar). q20 read,
  not eligible; a pass whose cluster interval crosses a bar is fragile.
- **Follow-up rule**: pass → an in-app plan (the gate and/or fill in
  `candidate.rs` / the scan with `(s*, τ*)`, discard candidates sorted
  last at the best cut's depth; a pass on fresh frames of known folders
  may first ask for a folder-fresh round when new shoots exist); close (as
  the discard-model plan defined it) → more labels; otherwise → nothing,
  `eye_focus` stays.

## Steps

- [x] Step 1: Pre-register the exploration in `efgate.md`, before the CLI extension and any per-face value exist
  - Done when:
    - `efgate.md` in this plan folder has a "Pre-registration of the
      exploration" section merged **before the CLI extension exists and
      before any nose offset, window score or gated score has been
      computed or read**; the merge commit is the record. It states what
      is known (the numbers above with sources; that round 3's not
      pre-registered subset reading motivated the fill, so all three rounds
      are selection data; the mesh-roll finding and why the gate targets
      misfits), the per-face values and their definitions with code
      references, the candidates, the `τ` grid rule, the tuning, selection
      and stop rules, the readings beside, the multiplicity statement, the
      eight user decisions with the rejected options, and the planned bars.
    - `mise run lint` passes.
  - Implementation approach:
    - Doc-only PR; model it on discardmodel.md's "Pre-registration of the
      exploration" and facechoice.md's "Pre-registration".

- [x] Step 2: Extend `riffle-cli facechoice` with the per-face nose, mesh landmarks, offsets and window score; rerun on the 33 folders with `--check`
  - Done when:
    - `crates/cli/src/facechoice.rs` appends to every face row, after
      `mesh_ms`: `nose` (YuNet, `x,y`), `mesh_left_eye`, `mesh_right_eye`
      (the contour centers as `mesh_eye_offset` takes them), `mesh_nose`
      (point 4), all in stored coordinates, `eye_offset`, `nose_offset`,
      `max_offset` (box-long-side normalized), and `window_ef`
      (`candidate::eye_focus(gray, w, h, face, None).probability`); `-`
      for the mesh columns without a mesh; `window_ef` on every face. The
      no-face row grows by the same number of `-` columns. Everything else
      (the `af` rule, `--check`, `nearest`, the earlier columns) is
      unchanged. Only `facechoice.rs` and its one usage line change in
      `crates/cli`; `faces.rs`, `eyes.rs`, `candidate.rs` are not edited
      (the nose offset is computed in the CLI from `Mesh.points[4]` and
      `Face.nose`).
    - Unit tests cover the new columns (a meshed face with every value, an
      unmeshed face with `-` and a `window_ef`, the header column count),
      and `cargo test -p riffle-cli` and `mise run lint` pass. CLAUDE.md's
      `crates/cli` sentence names the extended dump.
    - The dump exists as `D:\Photos\tests\2026-11-04-ef-gate-fill\faces\<folder>.tsv`
      for all 33 folders, run from a frozen copy of the release
      `riffle-cli.exe` (SHA-256 in `learnings.md`) by a `facesdump.sh` in
      this plan folder (per folder, resumable, background, stderr in
      `logs\`, the library's size / mtime listing before and after
      identical); `--check` reports **0 mismatches** on every folder, 0
      errors, file counts equal to the folders' RAW counts; `learnings.md`
      records the wall times. **No value is read against any label.**
  - Implementation approach:
    - Assumes Step 1 merged. Follow the face-choice `run.sh` and the
      discard-model `freshdump.sh`; wait with a bounded `timeout 580 tail
      -F | grep -m1`, never a sleep; about 37 min in all on 12 threads,
      chunked so no tool call exceeds 10 min.

- [x] Step 3: `gatescores.py`, every candidate on every frame of the 33 folders, the `τ` grid fixed, label-free readings only
  - Done when:
    - `gatescores.py` in this plan folder (`python -I -B gatescores.py
      <app-dump-dir> <discard-model-dir> <gate-dir>`) loads both dumps
      through `gate.load` / `gate.build`, the new per-face dump, builds
      `ef`, `ef-fill`, and `ef-gate` / `ef-gate+fill` for both families and
      the four `τ` each, and writes `scores_gate.tsv` (`folder`, `file`,
      `ef`, `nose`, `max`, `window_ef` of the judged face, one column per
      candidate × `s` × `τ`, full `repr` precision, `nan` where missing)
      into `<gate-dir>`, stdout saved as `gatescores.out.md`.
    - It prints **label-free readings only**: the checks (`af` reproduces
      the stored `eye_focus` and the old per-face dumps' `af` / `af-whole`
      on every frame; `ef-fill` equals `scores_model.tsv`'s and
      `scores_fresh.tsv`'s; `eye_offset` equals the features TSVs' within
      their precision; a mesh column is `-` exactly where `scored` is
      `window` and the box is under 60 px); the **distribution** of `nose`
      and `max` over the meshed `af` faces (percentiles 50, 80, 90, 95,
      97.5, 99) and the **grid** the rule fixes; per (`s`, `τ`) the gated
      share of frames and of picks, the gated share by |yaw| band (0–15,
      15–30, 30–60, 60+), the share of gated faces whose `window_ef` is
      lower / higher than their `ef`, and the same shares for
      `eye_offset` > 0.10 as the reference; the missing share before and
      after each rule; per candidate and q the removed share and `PL` on
      all 33 folders with the two label-free verdicts. An audit hook
      records the files opened (no `labels.tsv` anywhere).
    - `efgate.md` gains "The grid as fixed" (label-free) with the values;
      `learnings.md` records the wall time; `mise run lint` passes.
  - Implementation approach:
    - Assumes Step 2 merged and its dump complete. Import `badfilter`,
      `discardfilter`, `facechoiceread` by path; extend `load_faces`'s
      record with the new columns in this script. Seconds to a minute.

- [ ] Step 4: `gateread.py`, the exploration read of the three rounds, `(s*, τ*)` and the winner by the pre-registered rules
  - Done when:
    - `gateread.py` in this plan folder (the Step 3 arguments plus
      `<strict-labels-dir> <discard-labels-dir> <model-search-dir>`, the
      strict folder for `manifest.tsv` only) reads rounds 1 and 2 as
      `facechoiceread.py` does and round 3 as `freshread.py` does, prints
      Markdown tables saved as `gateread.out.md` and **refuses to run when
      that file exists**: the checks (the three rounds' `ef` rows, base
      rates and `ef-fill` differences reproduce; strata recompute), the
      tuning table (8 pairs × rounds 1 and 2, AUC − `ef` each, the sum,
      `(s*, τ*)` named), round 3's AUC table (every candidate and
      reference, stratified, cluster, unweighted, paired AUC − `ef`), the
      cut tables per round, the three subsets with the label × gated
      cross-tab, the selection rule applied with the winner named and the
      stop rule's verdict.
    - `efgate.md` gains a "Results of the exploration" section with the
      tables and a "Reading" paragraph stating the facts (where each rule's
      gain or loss sits; whether gated frames are discardable and whether
      they are profiles) without deciding. If the stop rule says no fresh
      round, Steps 5–6 are marked skipped here and Step 7 follows.
    - `learnings.md` records deviations; `mise run lint` passes.
  - Implementation approach:
    - Assumes Step 3 merged. Reuse `facechoiceread`'s `auc_draws`,
      `cut_metrics`, `diff` and both cell constructions; one count matrix
      per round so differences are paired. Dry-run on synthetic labels in
      the scratchpad first. About two minutes.

- [ ] Step 5: Pre-register the confirmation and draw the fresh blind sample; the plan pauses for the user's labeling
  - Done when:
    - `efgate.md` has a "Pre-registration of the confirmation" section
      merged **before any fresh label exists**: the frozen winner (with
      `(s*, τ*)`), the population (never-labeled frames of the 33
      folders), the strata, allocation, seed 20261104, bars, follow-up rule
      and multiplicity statement exactly as "The confirmation" above; an
      "As drawn" paragraph (label-free) added after the draw.
    - `gatesample.py` in this plan folder takes the winner, `ef` and
      `ef-fill` from `scores_gate.tsv` (written as `scores_fresh4.tsv` on
      the population), excludes the labeled frames by the four manifests,
      cuts the strata, draws, writes `manifest.tsv` and `strata.tsv` into
      `D:\Photos\tests\2026-11-04-ef-gate-fill\`, prints the tables and
      checks (sum of `1/π` equals the population; the winner's and `ef`'s
      `PL` and removed shares; the `efonly` overlap; no drawn frame in any
      earlier manifest), copies the sample with SHA-256 checks (`--no-copy`
      first, then the copy in the background), refuses to write under
      `D:\Photos\2026`, and compares the 33 source folders' size / mtime
      listing before and after.
    - `labeling.md` in this plan folder: round 3's guide with the new path
      and folder count, the same question, rating 1 = keep only,
      `aiEnabled` off, never Pick / Reject / color label.
    - `mise run lint` passes. **The plan pauses here until the user says
      "done"**; nothing reads a sample sidecar before that.
  - Implementation approach:
    - Assumes Step 4 merged with a "fresh round" verdict. Reuse
      `freshsample.py`'s draw and copy pattern by path (no refit). About 26
      GB, 15 minutes for the copy.

- [ ] Step 6: The confirmatory read, once
  - Done when:
    - After "done", `gatelabels.py` (the `freshlabels.py` recipe) writes
      `labels.tsv`; `learnings.md` records the session length and anything
      odd the user reported.
    - `gatefresh.py` reads **once** (refuses when `gatefresh.out.md`
      exists): the winner, `ef`, `ef-fill` and random at q5 / q10 / q20
      with the fresh `π`, the stratified and 33-folder cluster bootstraps,
      the base rate, the bars with a verdict per cut and the follow-up
      branch, plus the three subsets as a reading; every computation
      before anything is printed, label-free checks first; a dry run on
      synthetic labels in the scratchpad before the real read.
    - `efgate.md` gains "Results of the confirmation" with the tables, the
      verdict, the exploration numbers of the winner (rounds 1–3) beside
      the confirmation's so the shrinkage is visible, and a "Reading".
    - `learnings.md` records deviations; `mise run lint` passes.
  - Implementation approach:
    - Assumes Step 5 merged and the labeling done. Reuse Step 4's functions
      and `freshread.py`'s cell construction.

- [ ] Step 7: The Decision
  - Done when:
    - A "Decision" section in this plan, written against the Step 5 bars
      (or the stop rule if Step 4 ended the plan): whether the gated /
      filled `ef` beats today's `ef` on the fresh blind confirmation, at
      what depth and `RR` / `PL`, how far the exploration numbers shrank,
      where the gain or loss sat (no-face, gated, complement), whether the
      gate flagged misfits or profiles (and a profile exemption proposed
      if so), the follow-up by the pre-registered rule (an in-app plan
      naming the rule, `s*`, `τ*` and the window re-measure; more labels;
      or nothing), with the caveats weighed: the population is fresh frames
      of known folders, round 3 motivated the fill, one photographer; the
      `todo.md` item "Core: improve on `eye_focus` for the discard
      question" updated with the outcome.
    - `learnings.md` records the compute and labeling time; `mise run
      lint` passes.
  - Implementation approach:
    - Doc-only PR; do not move a bar.

## Trade-offs and risks

The choices above were decided by the user (see "Decisions").

Risks:

- **The gate may touch few frames** (misfits around 3–6% of meshed faces);
  the paired AUC difference is then tight and small, and the `RR` / `PL`
  readings at the cut carry the practical meaning.
- **The window re-measure may be worse than the misfit mesh**; the
  label-free "window lower / higher than `ef`" shares in Step 3 and the
  cross-tab in Step 4 show it before any decision.
- **Round 3 motivated the fill**; the stop rule's rounds 1–2 condition
  (`ef-fill` alone was −0.010 / −0.011 there) and the fresh read protect.
- **Population not folder-fresh**: a pass is "fresh frames of known
  folders"; the follow-up may ask for a folder-fresh round when new shoots
  exist.
- **Labeling wording**: round 4 uses round 3's "sorted last" guide, so
  rounds 3 and 4 compare; rounds 1–2 ("hid") had lower base rates.
- **The discard-model folder may be archived mid-plan**; its path is
  resolved at run time.

## Progress

- (2026-11-04) Plan written
- (2026-11-05) Step 1 complete
- (2026-11-06) Step 2 complete
