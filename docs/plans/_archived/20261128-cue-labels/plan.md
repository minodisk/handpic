<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Cue labels: a blind labeling round for the cue readings

## Purpose

The bad-photo-labels Decision
([plan](../20261017-bad-photo-labels/plan.md), "Left for the user
to decide") left one question open: a labeling round that gives the **cue
readings** (head pose, closed eyes and the rest) more power. `|pitch|` > 45
read 3.20x the strict base rate on 19 sampled frames, interval [0.0, 40.5].
The user decided on 2026-11-28 to run that round, over-sampling the frames
with a face pitched strongly up or down.

This plan prepares the round and stops where the user starts labeling: it
fixes the population, the strata and the allocation, pre-registers the
question, the readings and the bars before any label exists, extracts the
sample as shuffled RAW copies, writes the labeling guide, and rewrites the
`todo.md` item into one that waits on the user's labels. Reading the labels
and the Decision belong to a later plan. **No app code changes.**

### Decided with the user (2026-11-28)

- Sample from the folders of the earlier rounds under `D:\Photos\2026`.
- Exclude frames already labeled.
- Over-sample the `|pitch|` > 45 stratum.
- The question is **"OK to discard?"**, as in discard-labels
  ([plan](../20261018-discard-labels/plan.md)), marked the same
  way (rating 1 = keep, unmarked = OK to discard).
- A larger blind random group than the 150 controls of discard-labels,
  which stood for 25,071 frames
  ([discard-retarget Decision](../20261019-discard-retarget/plan.md)).
- About 30-50 minutes of labeling at about 3 s per frame.
- After the investigation below, the user approved this plan as written
  (2026-11-28): the 19 intact folders, the 2,400 excluded frames, the
  300 / 100 / 500 allocation and bar 1's 15 points.

### What is known (investigated 2026-11-28, read-only)

- **The readings.** The only complete readings of these folders are the
  dump of 2026-10-12 (`D:\Photos\tests\2026-10-12-rank-app\features\<folder>.tsv`,
  27 folders, 32,459 frames; columns `folder`, `file`, `capture_time`,
  `subsec`, `xmp`, `dop`, `sharpness`, `state`, `eye_focus`, `ear`,
  `eyes_open`, `yaw`, `pitch`, `roll`, `eye_offset`, `edge_gap`, a missing
  value `-`). `pitch` is positive when the face tilts up
  (`crates/core/src/pose.rs`).
- **The count behind the allocation.** Dump frames with `|pitch|` > 45:
  **768** (188 up, 580 down). Already labeled: 52 (19 strict, 14
  discard-labels, 19 model-search). Unlabeled: 716. Unlabeled and still in
  the library: 456. Unlabeled and in the 19 intact folders (below): **434**
  (158 up, 276 down), in 15 folders, 132 of them in `2026-10-03` and 89 in
  `2026-09-27-a`.
- **The library was culled after the dump.** 19,200 of the dump's 32,459
  frames are still in `D:\Photos\2026`. Eight folders lost frames:
  `2026-07-22` (32 of 189 left), `2026-07-26` (7 of 64), `2026-07-30` (105
  of 936), `2026-08-01` (463 of 3,401), `2026-08-22` (267 of 2,677),
  `2026-08-29` (0 of 3,451), `2026-09-13-b` (232 of 1,795), `2026-09-19`
  (282 of 2,134). What is left there is what the user kept. **19 folders
  are intact** (every dump frame present), 17,812 frames: `2026-06-05`,
  `2026-06-13`, `2026-06-14`, `2026-06-20`, `2026-07-02`, `2026-07-05`,
  `2026-07-11`, `2026-07-18`, `2026-07-21`, `2026-07-23`, `2026-07-24`,
  `2026-07-31`, `2026-08-02`, `2026-08-08`, `2026-08-15`, `2026-08-16`,
  `2026-09-27-a`, `2026-09-27-b`, `2026-10-03`.
- **Frames already labeled: 2,400 of the dump, in three manifests** with no
  overlap, each keyed by `folder` and `file`:
  `D:\Photos\tests\2026-10-17-bad-labels\manifest.tsv` (1,200, strict),
  `D:\Photos\tests\2026-10-18-discard-labels\manifest.tsv` (600) and
  `D:\Photos\tests\2026-10-20-model-search\manifest.tsv` (600, the blind
  confirmation of [model-search](../20261020-model-search/plan.md),
  the same question). 1,342 of them are in the 19 intact folders (682, 349,
  311). The fourth round
  ([discard-model](../20261023-discard-model/plan.md)) drew from
  six other folders.
- **Prior readings of the cue bins** on those labels, weighted by `1/π`,
  read here for sizing and stated as prior knowledge: `|pitch|` > 45 is "OK
  to discard" at 62.3% on 14 frames (discard-labels, base rate 48.5%) and
  29.3% on 19 frames (model-search, base rate 42.3%), Kish effective sizes
  5.9 and 5.2; EAR <= 0.137 at 38.5% on 56 and 63.4% on 80, effective sizes
  15.8 and 20.1; `|yaw|` > 35 at 44.0% and 47.6%, effective sizes 40.8 and
  35.8.
- **Today's app reads fewer poses.** In `2026-09-27-a`, which the app's
  index holds at `faces_extractor` 14 (today's `FACES_VERSION` is 16), 94
  frames had a dump `|pitch|` > 45; 16 still have a pitch, with the same
  value, and 78 have none (the mesh presence floor and the untrusted side
  face mesh came after the dump). One folder, so an extrapolation: most of
  the dump stratum may be frames today's app withholds a pose for.
- **The archived harnesses no longer load.** The archived `sample.py`
  imports `rejectfilter` at module level, which needs scikit-learn, joblib,
  the venv `%TEMP%\embedvenv` (gone) and `crates/core/genres/siglip2.json`
  (removed by remove-ai-features). `mise x -- python` has no numpy.
- **The model changes named for a further round** (clipped `1/π` weights, a
  linear `D0`, a target that keeps picks out of the cut): model-search ran
  the clip (`DP@10` +3.3 [-14.1, +20.8], AUC -0.005) and logistic forms;
  discard-model ran `LR/S` and `LR/T` on 1,200 blind labels and its fresh
  round put `LR/T` below `eye_focus` (AUC -0.013 [-0.048, +0.023]). The
  pick-excluding target was never run.
- **Size on disk**: one ARW is about 45 MB, so 900 copies are about 40 GB;
  `D:` has 436 GB free.

### The design

- **Population**: the dump's frames of the 19 intact folders, minus the
  frames of the three manifests: **16,470** frames. A folder is intact when
  every one of its dump frames is in the library when `sample.py` runs.
- **Strata**, disjoint, on the dump's readings, the first that matches:

  | Stratum | Rule | Population | Drawn | `π` |
  | --- | --- | ---: | ---: | ---: |
  | `pitch45` | `|pitch|` > 45 | 434 | 300 | 0.691 |
  | `closed` | EAR <= 0.137 | 1,068 | 100 | 0.094 |
  | `rest` | everything else | 14,968 | 500 | about 0.033 |

  `pitch45` and `closed` are simple random samples. `rest` is the blind
  control group: drawn per folder in proportion to the folder's `rest` size
  with a floor of 8 (or the whole cell), largest remainder, the scheme of
  the archived `allocate`. **900 frames, about 45 minutes.**
- **Blind**: shuffled, named `<nnnn>_<folder>_<file>`; nothing in the name,
  the order or the folder says which stratum a frame is in.
- **Where the data goes**: `D:\Photos\tests\2026-11-28-cue-labels\`, the
  copies in its `sample\` subfolder, `manifest.tsv`, `strata.tsv` and the
  library listings beside it. Nothing of it enters the repository.

## Steps

- [x] Step 1: The sampler, the pre-registration, the extracted sample, the labeling guide and the `todo.md` item
  - Done when:
    - `sample.py` in this plan folder (`mise x -- python -I -B sample.py
      <app-dump-dir> <strict-dir> <discard-dir> <confirm-dir> <out-dir>
      [--check]`) reads the dump's `features\*.tsv` and the three
      manifests, finds the intact folders, builds the population and the
      strata of "The design", draws with `random.Random(0)` over sorted
      keys, and writes `manifest.tsv` (`name`, `source`, `folder`, `file`,
      `draw`, `stratum`, `inclusion_prob`, then the dump's raw columns
      `capture_time`, `subsec`, `xmp`, `dop`, `sharpness`, `state`,
      `eye_focus`, `ear`, `eyes_open`, `yaw`, `pitch`, `roll`,
      `eye_offset`, `edge_gap`) and `strata.tsv` (`stratum`, `folder`,
      `population`, `drawn`, `inclusion_prob`; one row per folder for
      `rest`). Then it copies the RAW files (no sidecar, `shutil.copy2`)
      into `<out-dir>\sample\`, skipping a copy whose size and mtime
      already match.
    - It refuses an `<out-dir>` under `D:\Photos\2026`, opens every
      library file read-only, and stops before copying when a sampled
      source is missing.
    - **Exclusion**: no sampled frame is in any of the three manifests;
      the script asserts it and prints how many frames of each stratum the
      exclusion removed.
    - **Deterministic**: a second run with `--check` (no copy) writes a
      `manifest.tsv` byte-identical to the first; its SHA-256 is recorded
      in `learnings.md`.
    - **Library unchanged**: the script writes a listing (path, size,
      `mtime_ns`, recursive, all 27 dump folders) to
      `<out-dir>\library.before.tsv` before the first read of a RAW file
      and `library.after.tsv` after the last hash, and reports 0 changed,
      added or removed paths.
    - **Checks printed and recorded in `learnings.md`**: the intact and
      the excluded folders with their counts; the strata table (population,
      drawn, `π`) against the numbers of "The design", with any difference
      explained (the library may have changed since 2026-11-28); the sum of
      `1/π` over the sample equal to the population; every copy's size and
      SHA-256 equal to its source's; no `.xmp` / `.dop` in `sample\`; files
      in `sample\` equal to the manifest's rows; per folder, the sampled
      count by stratum; the up / down split of the sampled `pitch45`
      frames; the disk use and the wall time.
    - `cues.md` in this plan folder holds the "Pre-registration" below,
      and is merged **before the user marks anything**. The main agent
      tells the user to start only after the PR is merged.
    - `labeling.md` in this plan folder (English; the main agent relays it
      in Japanese) is the guide described under "The labeling procedure".
    - `todo.md`: the section "Core: decide whether a second bad-photo
      labeling round for the cue readings is wanted" is replaced by one
      item that waits on the user's labels, as under "The todo item".
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - **Standard library only** (`csv`, `random`, `hashlib`, `shutil`,
      `os`). Do not import the archived `sample.py`: its import chain no
      longer loads ("What is known"). Re-implement the four small pieces
      it needs (`under`, `allocate`, `listing`, `sha256`) and say in a
      comment which archived file each follows. Change nothing under
      `_archived/`.
    - The script takes every data path as an argument and depends on
      nothing relative to its own location, so the plan folder's move
      under `_archived/` does not break it.
    - Print the tables and run every label-free check before copying;
      copy last, in the foreground, resumable. The archived learnings
      record a background copy killed after 482 files.
    - Run the script from the scratchpad with `-I -B` and check `git
      status` for a stray `__pycache__/`.
    - The copy and the hashes are the only load this plan generates; run
      one copy at a time, no parallel hashing.
    - `todo.md` is edited by other sessions: rebase onto `origin/main`
      before the push and, on a conflict, keep the other side for
      everything but this section.
    - Single-PR mode archives this folder in the same PR, so `todo.md`
      names the files at `docs/plans/_archived/20261128-cue-labels/`.
    - For `labeling.md`, take the key names from
      `crates/engine/src/shortcuts.rs` (`rate1`, `toggleStrip`,
      `toggleRight`, `toggleSides`) and confirm in `crates/app` that the
      rating key and the paging keys act while the strip is hidden. If
      they do not, the guide keeps the strip and tells the user not to go
      by its marks, as the earlier guides did.

### Pre-registration (the content of `cues.md`)

Written before any label of this round exists. It states the prior
knowledge of "What is known", including the prior cue-bin readings.

- **Labels.** `keep` = the copy's sidecar carries rating 1; `discard` =
  viewed and unmarked. A Pick, a Reject, a color label or another rating
  is listed as unexpected and counts by the rating-1 mark alone.
- **Estimates.** Horvitz-Thompson weighted (`1/π`) to the population of
  16,470 frames, the unweighted value and the sampled count beside each.
  Intervals: a 1,000-draw stratified bootstrap within `pitch45`, `closed`
  and each (`rest`, folder) cell, seed 0, one count matrix for every
  quantity so paired differences share draws; a folder-cluster bootstrap
  as the sensitivity reading.
- **The first number**: the discardable base rate of the population.
- **The primary reading**: the `discard` rate among the frames with a dump
  `|pitch|` > 45, against the rate among all other frames.
- **Secondary readings**, each a firing bin against its complement:
  - on the dump's readings: `pitch` > 45 (up) and `pitch` < -45 (down)
    apart; EAR <= 0.137; `|yaw|` > 35; `eye_focus` < 0.9; no `eye_focus`
    (no judged face); the folder's bottom sharpness decile;
  - on **today's readings of the sampled copies**, taken after labeling
    and without the labels (the app's index of the sample folder, or
    `handpic-cli features` over it): `|pitch|` > `GOOD_MAX_PITCH`; a dump
    `|pitch|` > 45 with no pose today; EAR <= `EYES_CLOSED_EAR`; the Low
    mark (`full_eye_sharp` under `LOW_FULL_EYE_SHARP`); the good-photo
    tier, read in the keep direction;
  - the earlier discard-question labels (discard-labels and model-search)
    restricted to the 19 folders, read beside this round's with their own
    `π`, not pooled into the primary reading.
- **Bar 1, a cue catches discardable frames**: the firing bin's weighted
  `discard` rate exceeds its complement's by at least **15 points**, with
  the paired stratified 2.5% quantile above zero and the folder-cluster
  2.5% quantile above zero. The strict round's bar, twice the base rate,
  cannot be met at a base rate near 45%.
- **Bar 2, a cue is usable sorted last on its own**: the bin's rescue rate
  `RR` (its `keep` share) is at most 25% with the stratified 97.5%
  quantile at most 35%, and the bin holds at most 5% of the population's
  picks (`PL`, label-free). These are the bars the user chose for
  discard-retarget.
- **Multiplicity**: one primary reading; every secondary reading is
  reported with its interval and is not a pass on its own.
- **The follow-up rule**: (1) the primary reading meets bar 1 and the bin
  of today's `|pitch|` > `GOOD_MAX_PITCH` reads at or above its complement
  → the Decision recommends a later plan that shows or uses the pitch cue,
  naming what and where; (2) the primary reading meets bar 1 only through
  the frames with no pose today → the Decision names "no pose on a found
  face" as the cue to read next; (3) otherwise → nothing, and the todo
  item closes. `cues.md` holds the operational form of each branch (the
  rates compared and the with / without a pose today test) and governs
  where the two differ.
- **Model variants, secondary and not sized for**: three forms of the
  discard model, trained leave-one-folder-out on the discard-question
  labels of the 19 folders (this round, discard-labels, model-search,
  each with its own `π`): `D0-clip` (the weight clip of model-search's
  `All@32 / HGB / discard`, as it was), `D0-lin` (discard-model's `LR/T`
  form and columns), `D0-pick` (the target `discard` and not a pick). A
  variant **passes** when, on this round's labels, a per-folder q5 or q10
  cut meets `RR` <= 25% with the 97.5% quantile <= 35%, `PL` <= 5% and a
  removed share >= 4.5%, and its weighted AUC exceeds `eye_focus`'s with
  the paired 2.5% quantile above zero. A pass names a fresh blind
  confirmation as its only follow-up: the earlier winners lost 0.079 and
  0.083 AUC on theirs, and the embedding and ranking code is no longer in
  the repository.
- **Caveats**: one photographer, 19 folders, mostly basketball; the strata
  are cut on readings of 2026-10-12, which today's app no longer makes on
  every frame.

### The labeling procedure (the content of `labeling.md`)

- **The question**: "OK to discard?": if Handpic hid this frame from what
  you pick from, would that be fine? Not "is it bad", not "would you pick
  it". The wording of discard-labels, kept so the rounds compare.
- **Mark a frame that should not be discarded** (you might pick it, you
  would want to see it first, you would mind it hidden); leave the rest
  unmarked; **when unsure, mark it**.
- The folder mixes kinds of frames and nothing shows which is which.
- **Setup**: open `D:\Photos\tests\2026-11-28-cue-labels\sample`; sort by
  file name; clear every filter; let the scan finish; turn the `f` overlay
  off; **hide the strip and the meta pane**, because the strip's Low and
  closed-eyes marks and the pane's Analysis rows are the cues this round
  measures.
- **Keys**: `1` marks keep (it advances with Auto-advance on), `0` clears;
  no Pick, no Reject, no other rating, no color label.
- **Pace**: about 3 s per frame, 900 frames in about 45 minutes, in as
  many sittings as wanted.
- **Stopping rule**: every frame viewed once, `0001` to `0900`; an
  unmarked frame counts as OK to discard.
- **When done**: say "done", how long it took, and anything odd (frames
  that would not open, doubts, whether the strip stayed hidden).

### The todo item (replaces the section in `todo.md`)

Heading: `### Core: read the cue-labels round once the user has labeled
it`. Background: what the round is (900 blind frames, the three strata
with their counts, the question), where the sample is, the guide and the
pre-registration at their archived paths, the findings of "What is known"
that a reader of the labels needs (19 intact folders, 2,400 excluded,
today's app reading fewer poses), and that it **waits on the user**. TODO
lines: the user labels the folder per `labeling.md` and says "done"; then
a plan that exports the labels (`labels.py`, as the archived ones), reads
today's values of the sampled copies, computes the pre-registered readings
and writes the Decision by the follow-up rule, changing no bar.

## Trade-offs and risks

Decisions this plan made by a default, confirmed by the user on 2026-11-28:

- **Population: the 19 intact folders.** Taking all 27 would add 22
  `pitch45` frames (456) and 1,309 others, but in the eight culled folders
  only the frames the user kept remain, so "OK to discard?" there is
  already answered by the culling and the sample would not be blind to it.
  The cost of the default: the estimates speak for 19 folders, and a
  comparison with the earlier rounds needs their labels restricted to the
  same folders (349 and 311 frames).
- **Exclusion: 2,400, not 1,800.** The 600 frames of model-search's blind
  confirmation carry the same question's labels. Leaving them in would
  re-ask 311 frames of the 19 folders.
- **Strata on the dump's readings.** They are complete, they are the
  readings behind the 3.2x, and they need no build. The alternative is a
  re-dump of the 19 folders with today's `handpic-cli features` first: a
  local release build and a scan of 17,812 frames, after which the stratum
  would be today's cue but, by the one folder measured, about a sixth the
  size (about 70 frames), nearly a census. Today's `features` also has no
  `eye_focus` and no full-size eye score column. With the default, about
  50 of the 300 `pitch45` frames are expected to carry a pitch today, so
  the today's-reading bin stays a secondary reading with a wide interval.
- **Allocation 300 / 100 / 500.** A census of the 434 `pitch45` frames
  (1,000 frames, about 50 minutes) removes that stratum's sampling error
  and raises the expected today's-pitch frames to about 75. The `closed`
  stratum was not asked for; it is there because EAR's effective size was
  16-20 in both earlier rounds. Dropping it gives 100 more controls.
- **Bar 1's 15 points** was proposed by the plan and accepted by the user.
- **The model variants stay as secondary readings.** Two of the three
  were run in model-search and discard-model without gain, and
  discard-model's Decision, confirmed by the user on 2026-10-30, declined
  more labels for a learned form over the same columns. Running them
  again needs a venv with numpy, scikit-learn and joblib and the genres
  file from git history. The alternative is to drop them from `cues.md`.
- **Wording "hid"** follows discard-labels. The fresh round of
  discard-model said "sorted last" and read a higher base rate (60.56%
  against 48.45% and 42.31%), on other folders, so the two could not be
  told apart there.
- **The manifest stays outside the repository**, as in every earlier
  round: it names personal files and each frame's stratum. Its SHA-256 and
  the strata table go into `learnings.md`.

Risks:

- **The library can change between this count and the run.** `sample.py`
  recomputes the intact folders, so the numbers may differ from "The
  design"; `learnings.md` records the difference.
- **"Unmarked = OK to discard" rests on the stopping rule**, as before.
- **Hiding the strip changes the labeling conditions** against the earlier
  rounds, whose guides only asked the user not to go by the marks.
- **The pre-registration and the extraction share one PR.** The record
  that the bars came first is the merge commit and the user starting only
  after it.

## Progress

- 2026-11-28: Step 1 done. The 900-frame sample was extracted to
  `D:\Photos\tests\2026-11-28-cue-labels\`; the numbers matched "The
  design", and the `manifest.tsv` SHA-256 and the checks are in
  `learnings.md`. PR: this branch's PR (link added at merge). Review round 1
  also made the follow-up rule's branch 2 and the source of the picks
  operational in `cues.md`.
