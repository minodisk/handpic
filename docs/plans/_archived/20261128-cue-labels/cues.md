# Cue labels: the pre-registration

Written on 2026-11-28, before any label of this round exists, and merged
before the user marks anything. The labels are read by a later plan, which
changes nothing here.

## The round

- **The sample**: 900 blind frames of the 2026-10-12 app dump
  (`D:\Photos\tests\2026-10-12-rank-app\features\<folder>.tsv`), drawn by
  `sample.py` in this folder from the 19 folders of `D:\Photos\2026` whose
  every dump frame is still in the library, less the 2,400 frames of the
  three earlier labeling rounds (strict, discard-labels, model-search): a
  population of **16,470** frames.
- **The strata**, on the dump's readings, the first that matches:

  | Stratum | Rule | Population | Drawn | `π` |
  | --- | --- | ---: | ---: | ---: |
  | `pitch45` | `|pitch|` > 45 | 434 | 300 | 0.691 |
  | `closed` | EAR <= 0.137 | 1,068 | 100 | 0.094 |
  | `rest` | everything else | 14,968 | 500 | about 0.033 |

  `pitch45` and `closed` are simple random samples; `rest`, the blind
  control group, is drawn per folder in proportion to the folder's `rest`
  size with a floor of 8 (or the whole cell). `pitch` is positive when the
  face tilts up (`crates/core/src/pose.rs`).
- **The question**: "OK to discard?", as in discard-labels
  ([plan](../20261018-discard-labels/plan.md)), marked the same
  way. The guide is `labeling.md` in this folder.

## Prior knowledge

Stated before the labels, so the readings below are not chosen on them.

- **The prior cue-bin readings** on the earlier discard-question labels,
  weighted by `1/π`: `|pitch|` > 45 is "OK to discard" at 62.3% on 14
  frames (discard-labels, base rate 48.5%) and 29.3% on 19 frames
  (model-search, base rate 42.3%), Kish effective sizes 5.9 and 5.2; EAR <=
  0.137 at 38.5% on 56 and 63.4% on 80, effective sizes 15.8 and 20.1;
  `|yaw|` > 35 at 44.0% and 47.6%, effective sizes 40.8 and 35.8. On the
  strict labels ([bad-photo-labels](../20261017-bad-photo-labels/plan.md)),
  `|pitch|` > 45 read 3.20x the strict base rate on 19 frames, interval
  [0.0, 40.5].
- **Today's app reads fewer poses.** In `2026-09-27-a` (the app's index at
  `faces_extractor` 14), 94 frames had a dump `|pitch|` > 45; 16 still have
  a pitch, with the same value, and 78 have none (the mesh presence floor
  and the untrusted side face mesh came after the dump). One folder, so an
  extrapolation: most of the `pitch45` stratum may be frames today's app
  withholds a pose for.
- **The library was culled after the dump** in eight folders; only the 19
  intact folders are sampled, so the estimates speak for those 19.
- **The model changes** named for a further round were partly run already:
  model-search ran the weight clip and logistic forms without gain, and
  discard-model's fresh round put `LR/T` below `eye_focus`
  ([discard-model](../20261023-discard-model/plan.md)). The
  pick-excluding target was never run.

## Labels

- `keep` = the copy's sidecar carries rating 1; `discard` = viewed and
  unmarked. A Pick, a Reject, a color label or another rating is listed as
  unexpected and counts by the rating-1 mark alone.

## Estimates

- Horvitz-Thompson weighted (`1/π`) to the population of 16,470 frames, the
  unweighted value and the sampled count beside each.
- Intervals: a 1,000-draw stratified bootstrap within `pitch45`, `closed`
  and each (`rest`, folder) cell, seed 0, one count matrix for every
  quantity so paired differences share draws; a folder-cluster bootstrap as
  the sensitivity reading.

## Readings

- **The first number**: the discardable base rate of the population.
- **The primary reading**: the `discard` rate among the frames with a dump
  `|pitch|` > 45, against the rate among all other frames.
- **Secondary readings**, each a firing bin against its complement:
  - on the dump's readings: `pitch` > 45 (up) and `pitch` < -45 (down)
    apart; EAR <= 0.137; `|yaw|` > 35; `eye_focus` < 0.9; no `eye_focus`
    (no judged face); the folder's bottom sharpness decile;
  - on **today's readings of the sampled copies**, taken after labeling and
    without the labels (the app's index of the sample folder, or
    `handpic-cli features` over it): `|pitch|` > `GOOD_MAX_PITCH`; a dump
    `|pitch|` > 45 with no pose today; EAR <= `EYES_CLOSED_EAR`; the Low
    mark (`full_eye_sharp` under `LOW_FULL_EYE_SHARP`); the good-photo
    tier, read in the keep direction;
  - the earlier discard-question labels (discard-labels and model-search)
    restricted to the 19 folders, read beside this round's with their own
    `π`, not pooled into the primary reading.

## Bars

- **Bar 1, a cue catches discardable frames**: the firing bin's weighted
  `discard` rate exceeds its complement's by at least **15 points**, with
  the paired stratified 2.5% quantile above zero and the folder-cluster
  2.5% quantile above zero. The strict round's bar, twice the base rate,
  cannot be met at a base rate near 45%.
- **Bar 2, a cue is usable sorted last on its own**: the bin's rescue rate
  `RR` (its `keep` share) is at most 25% with the stratified 97.5% quantile
  at most 35%, and the bin holds at most 5% of the population's picks
  (`PL`, label-free; a pick is a frame whose `xmp` or `dop` column in the
  dump says pick, the source discard-retarget's `PL (full dump)` used).
  These are the bars the user chose for
  discard-retarget
  ([plan](../20261019-discard-retarget/plan.md)).
- **Multiplicity**: one primary reading; every secondary reading is
  reported with its interval and is not a pass on its own.

## The follow-up rule

1. The primary reading meets bar 1 and the bin of today's `|pitch|` >
   `GOOD_MAX_PITCH` reads at or above its complement (the bin's weighted
   `discard` rate at or above that of all other frames) → the Decision
   recommends a later plan that shows or uses the pitch cue, naming what
   and where.
2. The primary reading meets bar 1 only through the frames with no pose
   today (the `pitch45` frames with no pose today, against all non-`pitch45`
   frames, meet bar 1, and the `pitch45` frames that still have a pose today
   do not) → the Decision names "no pose on a found face" as the cue to read
   next.
3. Otherwise → nothing, and the todo item closes.

## Model variants (secondary, not sized for)

Three forms of the discard model, trained leave-one-folder-out on the
discard-question labels of the 19 folders (this round, discard-labels,
model-search, each with its own `π`):

- `D0-clip`: the weight clip of model-search's `All@32 / HGB / discard`, as
  it was ([model-search](../20261020-model-search/plan.md));
- `D0-lin`: discard-model's `LR/T` form and columns;
- `D0-pick`: the target `discard` and not a pick (a pick as in bar 2's
  `PL`: the dump's `xmp` / `dop` columns).

A variant **passes** when, on this round's labels, a per-folder q5 or q10
cut meets `RR` <= 25% with the 97.5% quantile <= 35%, `PL` <= 5% and a
removed share >= 4.5%, and its weighted AUC exceeds `eye_focus`'s with the
paired 2.5% quantile above zero. A pass names a fresh blind confirmation as
its only follow-up: the earlier winners lost 0.079 and 0.083 AUC on theirs,
and the embedding and ranking code is no longer in the repository.

## Caveats

One photographer, 19 folders, mostly basketball; the strata are cut on
readings of 2026-10-12, which today's app no longer makes on every frame.
