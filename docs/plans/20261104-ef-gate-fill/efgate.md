# `eye_focus` gate and fill: a mesh-misfit gate, the no-face fill, and their combination

## Pre-registration of the exploration

Written 2026-11-04, **before the `riffle-cli facechoice` extension exists and
before any nose offset, window score or gated score has been computed or
read**: when this section was written no per-face nose offset, mesh nose
point or window-model score of any frame existed anywhere (the index and both
dumps store the judged face's `eye_focus` and `eye_offset` only, and the
per-face dumps carry neither), no script of this plan existed, and
`D:\Photos\tests\2026-11-04-ef-gate-fill\` held nothing of this plan. It is
merged on its own, before Step 2's CLI change or Step 3's `gatescores.py`
exists; the merge commit of the PR that adds this section, without results,
is the record, as for the archived
[discard-model exploration](../_archived/20261023-discard-model/discardmodel.md)
and [face-choice evaluation](../_archived/20261023-face-choice/facechoice.md)
before it. Nothing in this section changes after a per-face value of this
plan exists; a deviation found while implementing Steps 2–4 is recorded in
`learnings.md` with its reason and does not touch the tuning, selection or
stop rule.

The question: does changing **`eye_focus` itself** with two label-free rules,
a mesh-misfit gate that re-measures the judged face on the YuNet eye window,
and a no-face fill from the whole-image detection, or their combination,
order the blind "OK to discard?" labels better than today's `eye_focus`
(`ef`)? Riffle's AI goal is to suggest frames to discard (sorted last, not
hidden) so the human picks from what remains. This section fixes the
exploration (Steps 2–4) on the three labeled rounds and the stop rule that
decides whether a fresh blind round is drawn; the confirmation's bars are
fixed in Step 5's "Pre-registration of the confirmation", before any fresh
label exists, and their planned values are listed under "What happens
after".

### What was known before this section

This pre-registration is **not blind**. The design was chosen **after
reading `ef`'s and `ef-fill`'s numbers on all three rounds**, and, in
particular, after round 3's **not pre-registered** missing-face subset
reading (`ef-fill` − `ef` AUC +0.277 on the 201 labels without a crop face),
which motivates the fill. So **all three rounds are selection data** here;
any winner is selected on labels that shaped the design, and only the fresh
round of Steps 5–6 decides. Every number below is weighted by its round's
`1/π`, with the stratified bootstrap (1,000 draws, seed 0) in brackets.

| Round | Reading | Value | Source |
| --- | --- | --- | --- |
| 3 | `ef` (missing last) weighted discard AUC | 0.743 [0.679, 0.800] | [discardmodel.md, "Results of the confirmation"](../_archived/20261023-discard-model/discardmodel.md) |
| 3 | `ef-fill` AUC; − `ef` (paired) | 0.744 [0.681, 0.803]; +0.002 [−0.008, +0.012] | same |
| 3 | `LR/T` (the discard-model winner) AUC − `ef` | −0.013 [−0.048, +0.023] | same |
| 3 | Discardable base rate | 60.56% [53.81, 67.29] | same |
| 3 | `ef` `RR` q5 / q10 / q20 | 11.3 / 14.6 / 14.8% | same |
| 3 | `ef` `PL` q5 / q10 / q20 (the six folders' 1,795 picks) | 4.07 / 7.30 / 14.99% | same |
| 3 | Missing-face subset (201 labels): `ef-fill` − `ef` (not pre-registered) | +0.277 [+0.131, +0.407] | same, the subset table |
| 2 | `ef` AUC | 0.785 [0.723, 0.844] | [facechoice.md, "Reproduction checks"](../_archived/20261023-face-choice/facechoice.md); discardmodel.md, "Results of the exploration" |
| 2 | `ef-fill` − `ef` | −0.010 [−0.022, −0.001] | discardmodel.md, "Results of the exploration" |
| 2 | Discardable base rate | 42.31% [35.85, 48.91] | facechoice.md, "Reproduction checks" |
| 1 | `ef` AUC | 0.777 [0.716, 0.841] | same |
| 1 | `ef-fill` − `ef` | −0.011 [−0.028, +0.003] | discardmodel.md, "Results of the exploration" |
| 1 | Discardable base rate | 48.45% [42.03, 55.11] | facechoice.md, "Reproduction checks" |
| - | Missing `ef` share, dump (27 folders) / after the `af-whole` fill | 12.4% / 4.5% | facechoice.md, "Results" (face counts) |
| - | Missing `ef` share, the six fresh folders / after `ef-fill` | 9.00% / 1.18% | [discard-model learnings, Step 4](../_archived/20261023-discard-model/learnings.md) |
| - | `facechoice --check` against the stored `eye_focus` | 0 mismatches on 32,459 + 14,525 frames | [face-choice learnings](../_archived/20261023-face-choice/learnings.md); discard-model learnings, Step 4 |

- **No learned model has beaten `ef`**; the face-choice evaluation found no
  better face rule than the AF-nearest face. This plan therefore changes the
  value `ef` takes on two kinds of frame, not the model over it.
- **The mesh-roll finding**
  ([mesh-roll learnings](../_archived/20261010-mesh-roll/learnings.md),
  summarized in [tract-onnx-inference.md](../../agents/tract-onnx-inference.md)
  and [burst-keep-score plan](../_archived/20261008-burst-keep-score/plan.md)):
  of the frames with `eye_offset` > 0.10, about **27%** have a mesh off the
  face (a real misfit) and about **65%** have a fitted mesh against YuNet eye
  points that are off the eyes (profiles, YuNet eye distance under about
  0.20 of the box). Since `eye_focus` already measures the mesh eyelid
  regions when a mesh exists, gating on the YuNet-vs-mesh **eye**
  disagreement would mostly re-measure faces where the mesh is right and
  YuNet is wrong. The gate therefore targets **mesh misfits** through the
  nose: on a profile YuNet's nose and the mesh's nose tip still agree, while
  a mesh off the face moves its nose with it.
- **Spot counts, label-free** (this plan's investigation, 2026-10-30, from
  the features TSVs' `eye_focus` / `eye_offset`): `2026-10-17` (fresh, 1,675
  frames) 104 without `ef`, 595 with `ef` but no mesh (faces under 60 px),
  220 with `eye_offset` > 0.10 (13.1%); `2026-06-05` (dump, 1,337) 429 / 76
  / 298 (22.3%). With the 27% misfit share, misfits are expected around
  3–6% of the meshed faces.
- **Nothing is known about any nose offset or window score**: no per-face
  nose offset, mesh nose point or `window_ef` exists, so how many faces the
  gate touches and whether the window reads lower or higher than the mesh on
  them is unknown.

### Data, labels and weights

- **Dumps, complete for all 33 folders** (46,984 frames, 6,214 picks:
  4,419 + 1,795): `D:\Photos\tests\2026-10-12-rank-app\` (27 folders;
  `features\<folder>.tsv` with `eye_offset` at full precision, `embed.tsv`,
  `genre_text.tsv`) and `D:\Photos\tests\2026-10-23-discard-model\dump\` (the
  six fresh folders; same layout, header a superset, 4 decimals), both
  loaded through the archived `gate.load` / `gate.build`; `badfilter.load` is
  **not** called (it opens the strict labels). Every frame of the 33 folders
  has a trusted AF point. The source folders `D:\Photos\2026\<folder>` are
  **read-only**.
- **Per-face dump**: Step 2's new
  `D:\Photos\tests\2026-11-04-ef-gate-fill\faces\<folder>.tsv` for all 33
  folders, checked against the old per-face dumps
  (`…\2026-10-23-face-choice\faces\` and
  `…\2026-10-23-discard-model\dump\faces\`) in Step 3. `ef-fill` exists
  already in `scores_model.tsv` / `scores_fresh.tsv` and is checked equal.
- **Round 1** (`D:\Photos\tests\2026-10-18-discard-labels\`): 600 labels,
  211 keep / 389 discard; population the 27-folder dump minus the 1,200
  strict frames, 31,259; strata `b05`, `b10`, `b20` and per-folder
  `control`.
- **Round 2** (`D:\Photos\tests\2026-10-20-model-search\`): 600 labels, 248
  keep / 352 discard; population the dump minus the 1,800 earlier-labeled
  frames, 30,659; `strata.tsv` 31 cells (`b05`, `b10`, `b20`, `g0only`,
  per-folder `control`).
- **Round 3** (`D:\Photos\tests\2026-10-23-discard-model\`): 600 labels, 127
  keep / 473 discard; population the six fresh folders, 14,525; strata
  `b05` / `b10` / `b20` / `efonly` / per-folder `control`, cut by `LR/T` and
  `ef`. Labeled under the "sorted last" wording, rounds 1–2 under "hid".
- **Label**: rating 1 = `keep`, reviewed and unmarked = `discard`, as each
  round's `labels.tsv` holds it; **`discard` is the positive class**. Each
  round keeps its own population, `π` and strata as its harness defined
  them; nothing is pooled.
- **Harness, imported by path, none modified**: `badfilter` (`read_tsv`,
  `strat_counts`, `cluster_counts`, `graded_cut`, `finite_by_folder`,
  `aucs`, `auc_pair`, `quant`, `iv`, `civ`, `Tee`; it brings `rf`, `gate`
  (`load`, `build`) and `bf.sample`, aliased `bs` (`allocate`, `under`,
  `listing`, `sha256`, `cell`)); `discardfilter` (`metrics`, `unweighted`,
  `counts`, `QS`, `PL_BAR`, `REMOVED_BAR`); `facechoiceread` (the
  `load_faces` pattern, `choose`, `auc_draws`, `cut_metrics`, `diff`, the
  rounds 1–2 cell construction); the discard-model plan's `freshread.py`
  (round 3's cells from its `strata.tsv`), `freshsample.py`, `freshlabels.py`,
  `labeling.md` and `freshdump.sh`, with the face-choice `run.sh`. The
  discard-model folder is resolved at run time from either
  `docs/plans/20261023-discard-model/` or its archived location. No script
  of this plan is named `sample.py`.

### The per-face values

From the code at `7085586e`; this plan edits neither `faces.rs`, `eyes.rs`
nor `candidate.rs`, and the values below are computed by Step 2 in
`crates/cli/src/facechoice.rs` from the public types. Coordinates are the
stored (unrotated) image's pixels: `faces::Face` (box `x`, `y`, `width`,
`height`, YuNet's `left_eye`, `right_eye`, `nose`) as the cue maps it, and
`eyes::Mesh.points` ("a face mesh in the stored (unrotated) image's
pixels"). The box's longer side is `max(width, height)`.

- **`nose_offset`** = |`Mesh.points[4]` − `Face.nose`| / box longer side.
  Point 4 is the mesh nose tip, as `pose.rs`'s tests read it.
- **`eye_offset`** = `candidate::mesh_eye_offset(face, &mesh.points)`: the
  centers of `LEFT_EYE_CONTOUR` and `RIGHT_EYE_CONTOUR` (the mean of each
  contour's points), the larger of the two center-to-YuNet-eye distances in
  the pairing (straight or crossed) that keeps it smaller, over the box's
  longer side. It is today's `EyeMeasures.eye_offset` / `Cue::eye_offset` /
  `files.eye_offset`, gated by `rank.rs`'s good-photo rule at
  `MAX_EYE_OFFSET` = 0.1.
- **`max_offset`** = max(`nose_offset`, `eye_offset`).
- **`window_ef`** = `candidate::eye_focus(gray, w, h, face, None).probability`:
  the `Scored::Window` path, the `eye_window` of the face scored with the
  window model (`LOGIT_INTERCEPT`, `LOGIT_LAP`, `LOGIT_EDGE_WIDTH`) on the
  same full-size `luma` as the cue; **probability 0 with `NotCandidate`**
  when the window has no edge width. It is the path a face under
  `eyes::EYES_MIN_FACE` (60 px, `candidate::meshes_face` false) takes today.
- `nose_offset`, `eye_offset` and `max_offset` are `-` without a mesh;
  `window_ef` is on every face.
- A meshed face whose mesh gives no counting eyelid region is already scored
  on the window (`eye_focus` falls back to `Scored::Window` with the mesh
  present), so there `window_ef` equals `ef` and the gate changes nothing.

### The candidates and the references

Every score is "higher keeps"; a missing score sorts last (`-inf` in the
per-folder cut, removed before any finite value, ties by the seed-0 draw;
the folder's lowest finite value minus 1 in the AUC, `bf.finite_by_folder`),
as every earlier read. `ef` on every frame is the per-face dump's `af` value
at full precision (it equals the stored `eye_focus`), as round 3 took it.

- **References, not candidates**: `ef` (today's, missing last) and random
  (seed 0).
- **Gate statistic family** `s` ∈ {`nose`, `max`}: `nose` is `nose_offset`,
  `max` is `max_offset`, of the judged face. A face with `ef` but no mesh has
  no `s` and is **never gated** (it is already on the window).
- **`ef-gate(s, τ)`**: on the frame's judged face (today's `af` face, the
  crop set's `nearest`), `window_ef` where `s > τ`, else `ef`; missing where
  `ef` is missing. A `window_ef` of 0 stays 0, a real low value (Decision 8).
- **`ef-fill`**: `ef`, the missing filled by the whole set's `nearest` face's
  `eye_focus` (`facechoiceread.choose`'s `af-whole` face), the rest last;
  checked equal to the discard-model plan's column.
- **`ef-gate+fill(s, τ)`**: the fill first, then the gate on the face behind
  the value, the filled face included (its `s` from its whole-set row; a
  filled face under 60 px has no mesh and keeps its value, which is already
  a window value).

Nothing else is searched: no other statistic, normalization, re-measure,
fill source or form.

### The `τ` grid rule

Fixed before any per-face value exists; the values come out of Step 3
mechanically, label-free, and are recorded in "The grid as fixed".

- For each family `s`, `τ` takes the values of `s` at the **80th, 90th,
  95th and 97.5th percentiles** over the **meshed `af` faces of all 33
  folders** (the judged face of every frame whose `ef` exists and whose face
  has a mesh), **rounded to 3 decimals**. Four values per family, eight
  (`s`, `τ`) pairs.
- Justification: misfits are expected around 3–6% of meshed faces (the
  mesh-roll finding with the spot counts above), which the 90th–97.5th
  percentiles bracket, with the 80th as a loose end; the same rule for both
  families keeps their gated shares comparable.

### The read (Step 4)

Per round with its own population, `π` and strata; rounds 1 and 2 as
`facechoiceread.py` reads them, round 3 as `freshread.py` does. discard.md's
vocabulary:

- the weighted discard AUC (`roc_auc_score(discard, -score,
  sample_weight=1/π)`) with the stratified bootstrap (1,000 draws, seed 0,
  within the round's cells) and the folder-cluster bootstrap (the round's
  folders with replacement, seed 0, 1,000 draws; it does not decide), and
  the unweighted value;
- `DP` (discard precision), `RR` (rescue rate, 1 − `DP`), `PL` (the share of
  the round's folders' picks inside the cut, label-free) and the removed
  share at q5 / q10 / q20, with the per-folder bottom `floor(q · n / 100)`
  cut (`bf.graded_cut`); a per-folder cut is the same whether taken on the
  round's folders or all 33;
- paired differences against `ef` (**one count matrix per round**, shared by
  every candidate and quantity).

### Tuning, selection and stop rules

- **Tuning (mechanical, rounds 1 and 2)**: over the 8 (`s`, `τ`) pairs, the
  one with the **largest sum over rounds 1 and 2 of `ef-gate(s, τ)`'s
  weighted AUC − `ef`'s** (each round on its own population and `π`); ties
  to `nose`, then to the smaller `τ`. Rounds 1 and 2 are read for this, and
  the readings beside, only. The chosen `(s*, τ*)` is shared by
  `ef-gate` and `ef-gate+fill`.
- **Selection (round 3, primary)**: among `ef-gate(s*, τ*)`, `ef-fill` and
  `ef-gate+fill(s*, τ*)`, those whose **q5 or q10** cut passes the label-free
  conditions on **all 33 folders** (`PL` <= 5% of the 6,214 picks, removed
  >= 4.5% of the 46,984 frames) are eligible; the winner is the eligible
  candidate with the **highest weighted AUC on round 3**, ties by the sum
  of rounds 1 and 2's AUCs. The references are not eligible. If no candidate
  passes the label-free conditions, there is no winner and the plan ends as
  under the stop rule.
- **Stop rule**: the winner goes to the fresh round only if its **AUC − `ef`
  point is above zero on round 3 and on each of rounds 1 and 2**. Otherwise
  Steps 5–6 are skipped and the Decision (Step 7) is "nothing, keep
  `eye_focus`". The rounds 1–2 condition guards the fill, which round 3
  motivated and which read −0.010 / −0.011 there alone.

### Readings beside (they decide nothing)

- Per round, the cut tables at q5 / q10 / q20 (`DP`, `RR`, `PL`, removed,
  `RR` − `ef`) for every candidate and reference.
- The AUCs, and AUC − `ef`, on three subsets per round: **no crop face**
  (frames without `ef`), **gated** at `(s*, τ*)` (frames whose judged face
  has `s* > τ*`), and **the complement** (frames with `ef`, not gated). The
  cut stays the full per-folder cut; only the labeled frames read are
  restricted.
- The **label × gated** cross-tab per round at `(s*, τ*)`, with the share of
  gated faces whose `window_ef` is lower / higher than their `ef`.
- Label-free (Step 3): the distribution of `nose` and `max` over the meshed
  `af` faces (percentiles 50, 80, 90, 95, 97.5, 99); per (`s`, `τ`) the gated
  share of frames and of picks, the gated share by |yaw| band (0–15, 15–30,
  30–60, 60+), the window lower / higher shares; the same for
  **`eye_offset` > 0.10 gating as a reference only** (not a candidate);
  the missing share before and after each rule; per candidate and q the
  removed share and `PL` on all 33 folders with the two label-free verdicts.

### Multiplicity and population

- **Multiplicity**: 8 pairs × 2 rounds for the tuning, 3 candidates × 3 cuts
  on round 3, plus the three subsets and the cross-tabs, read **without
  correction** on labels the design was chosen after (round 3's subset
  reading motivated the fill); hence the mechanical tuning, selection and
  stop rules and the single fresh read.
- **Population caveat**: one photographer, mostly basketball; 33 folders,
  1,800 exploration labels. The gate may touch few frames, so its paired
  AUC difference may be tight and small, and the `RR` / `PL` readings at the
  cut carry the practical meaning.

### The user's decisions (at planning time, 2026-11-02 to 2026-11-04)

1. **Gate statistic: the nose offset, not the existing `eye_offset`.** `nose`
   (the mesh nose tip to YuNet's nose over the box's longer side, not the
   eye distance, which shrinks on profiles), with `max` (max of the nose and
   eye offsets, same normalization) as a second family, chosen with `τ` on
   rounds 1–2. `eye_offset` alone stays a label-free reading. Rejected: the
   existing `eye_offset` as the statistic; an eyes+nose statistic normalized
   by the eye distance.
2. **A gated face is re-measured on the YuNet eye window with the window
   model** (`Scored::Window`), per face, dumped by the CLI extension.
   Rejected: missing / sorted last; a neutral value (the folder's median
   `ef`); a fallback to `af-whole`'s face.
3. **`τ` and the family tuned on rounds 1 and 2; round 3 selects the
   winner.** Rejected: tuning on all three rounds.
4. **Stop rule**: AUC − `ef` point above zero on round 3 and on each of
   rounds 1 and 2. Rejected: round 3 and the sum of rounds 1 and 2.
5. **Round-4 population: the never-labeled frames of the 33 folders**, read
   as fresh frames of known folders (optimistic; the cluster bootstrap read
   as a within-folder check). Rejected: waiting for new shoots, since no
   folder newer than `2026-10-17` exists; deciding after Step 4.
6. **No profile exemption of the gate.** Step 3 prints the gated share by
   |yaw| band, Step 4 the label × gated cross-tab; the Decision may propose
   an exemption. Rejected: gating only when YuNet's eye distance is >= 0.20
   of the box.
7. **The `τ` grid is fixed by the label-free percentile rule above**; its
   values come out of Step 3 mechanically. Rejected: the user fixing the
   values after seeing Step 3's distribution.
8. **A gated face whose window score is 0** (no edge width, `NotCandidate`)
   keeps 0 as a real low value, as a face under `EYES_MIN_FACE` does today.
   Rejected: treating it as missing, sorted last.

### What happens after

No step of this plan changes the app. Step 2 extends `riffle-cli facechoice`
and reruns it on the 33 folders with `--check`; Step 3 builds every
candidate on every frame, fixes the `τ` grid and prints label-free readings
only; Step 4 reads the three rounds and applies the rules above. If the stop
rule sends the winner on, Step 5 pre-registers the confirmation and draws
600 fresh blind frames from the never-labeled frames of the 33 folders
(about 43,984; strata the winner's per-folder `b05` / `b10` / `b20`,
`efonly`, per-folder `control` with a floor of 3; 120 per stratum; seed
20261104), labeled under round 3's "sorted last" guide, and Step 6 reads
them once. The planned bars, which Step 5 fixes before any fresh label
exists (the discard-model plan's five): a cut of the winner at q5 or q10 is
usable when (1) weighted `RR` <= 25% with stratified 97.5% quantile <= 35%;
(2) `PL` <= 5% of the population's picks (label-free); (3) removed >= 4.5% of
the population (label-free); (4) `DP`'s stratified 2.5% quantile above the
base rate; (5) **beats `ef`**: paired AUC − `ef`, stratified 2.5% quantile
above zero (the key bar). q20 is read, not eligible; a pass whose cluster
interval crosses a bar is fragile. Pass → an in-app plan (the gate and/or
fill in `candidate.rs` / the scan with `(s*, τ*)`; a pass on fresh frames of
known folders may first ask for a folder-fresh round when new shoots
exist); close (as the discard-model plan defined it) → more labels;
otherwise → nothing, `eye_focus` stays. Step 7 decides.

## The grid as fixed

Step 3, 2026-11-06, label-free: `gatescores.py` (in this plan folder) on
Step 2's per-face dump of the 33 folders, with no `labels.tsv` and no
sidecar opened (its audit hook lists every file it opened; stdout saved as
`D:\Photos\tests\2026-11-04-ef-gate-fill\gatescores.out.md`, the scores as
`scores_gate.tsv` beside it). Every check passed: `af` reproduces the
stored `eye_focus` (27 folders exactly, the six fresh to the features TSV's
4 decimals) and the old per-face dumps' `af` / `af-whole` exactly; `ef-fill`
equals `scores_model.tsv`'s and `scores_fresh.tsv`'s; the judged face's
`eye_offset` equals the features TSVs'; the mesh columns are `-` exactly on
the faces scored on the window with a box under 60 px; the earlier `PL` and
missing readings of `ef` and `ef-fill` reproduce.

The rule applied: the percentiles of `s` over the **30,658 meshed `af`
faces** of the 33 folders (the judged face of every frame with `ef` whose
face has a mesh; `numpy.percentile`, linear interpolation), rounded to 3
decimals; a face is gated where `s` > `τ`.

| `s` | p50 | p80 | p90 | p95 | p97.5 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `nose` | 0.0723 | 0.1485 | 0.2147 | 0.2862 | 0.3573 | 0.4394 |
| `max` | 0.0869 | 0.1654 | 0.2322 | 0.2999 | 0.3638 | 0.4438 |

| `s` | `τ` (p80) | `τ` (p90) | `τ` (p95) | `τ` (p97.5) |
| --- | ---: | ---: | ---: | ---: |
| `nose` | 0.149 | 0.215 | 0.286 | 0.357 |
| `max` | 0.165 | 0.232 | 0.300 | 0.364 |

These eight (`s`, `τ`) pairs are the tuning grid of Step 4. Label-free
readings beside them (`gatescores.out.md` has the full tables):

| Gate | Gated `af` faces (% of the 46,984 frames) | % of the 6,214 picks | `window_ef` lower / equal / higher than `ef`, % |
| --- | ---: | ---: | ---: |
| `nose` > 0.149 | 6,104 (12.99) | 13.45 | 27.3 / 67.7 / 5.1 |
| `nose` > 0.215 | 3,055 (6.50) | 5.87 | 17.6 / 77.1 / 5.3 |
| `nose` > 0.286 | 1,534 (3.26) | 2.85 | 14.5 / 80.7 / 4.8 |
| `nose` > 0.357 | 770 (1.64) | 1.61 | 15.6 / 79.5 / 4.9 |
| `max` > 0.165 | 6,158 (13.11) | 13.34 | 25.0 / 69.3 / 5.7 |
| `max` > 0.232 | 3,070 (6.53) | 5.87 | 17.3 / 77.1 / 5.6 |
| `max` > 0.300 | 1,532 (3.26) | 2.94 | 15.4 / 78.6 / 6.0 |
| `max` > 0.364 | 766 (1.63) | 1.54 | 16.7 / 77.4 / 5.9 |
| `eye_offset` > 0.10 (reference, not a candidate) | 8,742 (18.61) | 18.80 | 28.3 / 65.9 / 5.8 |

- Most gated faces read the same on the window as their `ef` (equal), so
  the gate changes the value of 19–32% of the faces it touches; no gated
  face has `window_ef` = 0.
- The gated share is flat over the |yaw| bands (0–15, 15–30, 30–60, 60+)
  for `nose` at p95 and p97.5 (5.0 / 5.1 / 4.5 / 4.4% at 0.286) and rises
  toward profiles at p80 (17.1 to 24.2%); `eye_offset` > 0.10 rises from
  18.5% to 44.1% (the mesh-roll finding). The 165 meshed faces without a
  pose are gated at 25–96%.
- Missing shares: `ef` and every `ef-gate` 11.34% of the 33 folders' frames;
  `ef-fill` and every `ef-gate+fill` 2.50%.
- Label-free conditions on the 33 folders: every candidate passes at q5
  (`PL` 3.49–4.33%, removed 4.96%) and none at q10 (`PL` 6.61–8.63%), so all
  17 are eligible; every `ef-gate` removes as many picks at q5 as `ef` (269, `PL`
  4.33%), the fill lowers it to 217–224.
