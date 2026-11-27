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

# Bad-photo labels: a reviewed sample of the library and the first measurement of a bad-photo filter against it

## Purpose

What the user wants from the AI / ranking feature is a **bad-photo filter**:
drop the clearly bad frames, single shots and burst frames alike, and leave a
candidate set the human picks from. The two evaluations so far,
[folder-wide-ranking-eval](../20261015-folder-wide-ranking-eval/plan.md)
(Decision 1) and
[reject-filter-eval](../20261016-reject-filter-eval/plan.md)
(Decision 2, [reject.md](../20261016-reject-filter-eval/reject.md)),
could not answer that question because their labels conflate **bad** with
**not picked**: a culled folder's non-picks are the blurred frames *and* the
second-best frame of a burst and the duplicates, so the removed share was
only an upper bound on "bad removed", and the precision of a filter on "bad"
was unmeasurable. Explicit rejects in the library are about 140 (58 in the
dump, a third of them in one folder), too few to train on or to bar on.

This plan builds the missing labels the cheap way: a **sample folder of
copies** drawn from the 27 dumped folders, which the user reviews frame by
frame in Riffle, marking **only the bad ones** as Reject. Because every frame
in the folder is reviewed, "no reject" means "not bad", so for the first time
both **bad recall** and **precision** (how many not-bad frames a cut drags
along) are measurable, and, through the recorded inclusion probabilities,
reweightable to the library. The plan then measures every existing score,
rule and cue against those labels under a pre-registered metric and bar set,
fits one dedicated bad-vs-not-bad model held out by folder, and ends at a
recorded Decision: does a usable bad-photo filter exist today, which cues
catch bad photos, and is a dedicated model worth a next plan. No app change
happens in this plan.

### What is known (investigated 2026-10-17)

- **Data with stored scores exists only for the dump.** The 27 folders of
  `D:\Photos\tests\2026-10-12-rank-app\` (`features/<folder>.tsv`: capture
  time, `xmp` / `dop` flags, `sharpness`, `state`, `eye_focus`, raw `ear`,
  `eyes_open`, `yaw` / `pitch` / `roll`, `eye_offset`, `edge_gap`;
  `embed.tsv`; `genre_text.tsv`) hold 32,459 frames (2026-06-05 to
  2026-10-03, folders of 100–2,700 frames, 6,969 bursts of which 1,943
  singles, 4,419 picks, 58 explicit rejects). Their held-out scores are in
  `D:\Photos\tests\2026-10-15-folder-wide-eval\scores.tsv` (`W0`, `W0+off`,
  `F0`, `F0+ctx`, `P0` and nine cue variants) and
  `D:\Photos\tests\2026-10-16-reject-filter-eval\scores_reject.tsv` (`R0`,
  `G0`, `rule count`; also `scores_ablate.tsv`). Frames outside the dump
  have no scores; the dump was produced by the ignored test
  `commands::tests::rank_dump_of_real_folders` at 190–230 s per folder, so
  re-dumping more of `D:\Photos\2026` (85 folders) is hours of work for
  variety the dump already has. **This plan samples from the 27 dump folders
  only** (see Trade-offs).
- **The library path of a dump frame** is `D:\photos\2026\<folder>\<file>`
  (`dump.log` lists the dirs), the RAW is `_DSC####.ARW`, sidecars sit beside
  it (`<file>.dop`, `.xmp`) and PhotoLab virtual copies in `Output/`. One ARW
  is about 45 MB; D: has 542 GB free, so 1,200 copies cost about 54 GB.
- **Naming convention** of the archived hand-check sample
  `D:\Photos\tests\2026-10-09-good-mark\` (`samples-manifest-batch2.tsv`):
  `<folder>_<file>` (`2026-06-14__DSC4513.ARW`) with a `source` column
  holding the library path.
- **Where hand-check files go**: `D:\Photos\tests\<YYYY-MM-DD>-<topic>\`
  (the user's convention for files opened by hand in the app; the
  `D:\photos\samples\<FORMAT>\` library is for downloaded format-verification
  samples with a camera-model prefix, which these are not). The data folder
  of this plan is **`D:\Photos\tests\2026-10-17-bad-labels\`**, with the
  copies in its `sample\` subfolder (the folder the user opens in Riffle) and
  the manifests, labels and outputs beside it.
- **How a Reject leaves Riffle.** The reject key (`x` by default,
  `shift+x` for "reject the rest") writes the copy's sidecar through the
  sidecar writer in the user's `sidecarFormat`: XMP `xmpDM:good="False"`
  and / or `.dop` `ShouldProcess = 1` (`crates/core/src/xmp.rs`,
  `dop.rs`). Copies carry no sidecar, so every copy starts unflagged, and
  the labels are read from the sample folder's sidecars; the library's
  sidecars are never read or written by this plan.
- **The sample folder and the app's ranking.** `Index::rank_dirs` lists
  every folder holding a view or a reject, so once labeled the sample folder
  enters `refit_ranking` while `aiEnabled` is on; `rank::pairs` forms pairs
  only inside bursts that hold a pick, and the sample folder has none, so
  the user's weights do not move. The labeling guide still asks for
  `aiEnabled` off during labeling, so the recommended icon, the ranking
  rows of the meta pane and the embedding pass stay out of the way.
- **Harnesses, imported by path, none modified** (the workaround recorded
  in the archived
  [learnings](../20261016-reject-filter-eval/learnings.md), Step
  2: the five archived folders on `sys.path`, `gate` first, then
  `folder_gate`, `ballgate`, `genre_offsets`, `folderwide as fw`, then
  `fw.GENRES = folder_gate.GENRES = <repo>/crates/core/genres/siglip2.json`
  before `folder_gate.extend`). Reused: `gate.load` / `build` / `bursts_of`
  (the frames, the pick label rule, bursts at gap 1000 ms, the genre
  labels), `folder_gate.extend` / `design(d, mask, "full")` (the 46-column
  design `G0` used), `rejectfilter.load_scores` / `load_ear` / `rules` /
  `g0_fold` / `HGB` (import `rejectfilter` by path from
  `docs/plans/_archived/20261016-reject-filter-eval/`; it already resolves
  the archive from under `_archived/`), `fw.boot_folders`, `gate.table`.
  Python: the venv `%TEMP%\embedvenv` (numpy, scikit-learn, scipy, joblib),
  run as `python -I -B` from outside the data directories; check
  `git status` for stray `__pycache__/` afterwards.
- **Out of scope, owned elsewhere (do not touch):** `rank.rs`'s `pairs`,
  `Labeled`, `fit`, the `views` table and its data; the dlg-genre-weights,
  dlg-folder-best and dlg-likely-pick-icon sessions; other in-flight plans
  (not linked). No app code changes; the library `D:\Photos\2026` is read
  only. This plan adds files only under its own folder and writes data only
  to `D:\Photos\tests\2026-10-17-bad-labels\`.

## Steps

- [x] Step 1: `sample.py`, the sample folder of copies with its manifest, and the labeling guide `labeling.md`
  - Done when:
    - `sample.py` in this plan folder (`python -I -B sample.py <app-dump-dir>
      <folder-wide-eval-dir> <reject-filter-eval-dir> <out-dir> [n]`,
      default `n` 1,200) loads the dump through the archived harnesses,
      joins `scores.tsv` and `scores_reject.tsv`, assigns every one of the
      32,459 frames to exactly one **stratum**, draws the sample with
      `numpy.random.default_rng(0)`, writes `manifest.tsv` to `<out-dir>`
      and copies the RAW files (RAW only, no sidecar, `shutil.copy2`) into
      `<out-dir>\sample\`. It refuses to write under `D:\Photos\2026` (an
      assertion on the output path), writes nothing into the library, and
      is reproducible (a rerun with the same arguments draws the same
      frames; existing identical copies are skipped).
    - **Strata** (disjoint, assigned in this priority order so each frame
      is in the first that matches; sizes to be printed by the script):
      1. `closed`: raw EAR <= `EYES_CLOSED_EAR` 0.137 (`rejectfilter.rules`
         reads it from `features/<folder>.tsv`; 2,385 frames in the dump);
      2. `focus_low`: an AF face with `eye_focus` < `GOOD_EYE_FOCUS` 0.9;
      3. `away`: `|yaw|` > 35 or `|pitch|` > 45;
      4. `sharp10`: the folder's bottom 10% by absolute sharpness;
      5. `g0_bottom`: the folder's bottom 15% by held-out `G0`;
      6. `w0_bottom`: the folder's bottom 15% by held-out `W0`;
      7. `rest`: every other frame.
      **Allocation** (option A, cue-heavy, chosen by the user on
      2026-10-17; the final counts are recorded in `learnings.md`):
      `closed` 120, `focus_low` 120, `away` 120, `sharp10` 120, `g0_bottom`
      150, `w0_bottom` 70, `rest` 500, for 1,200. Strata 1–6 are simple
      random samples within the stratum; `rest` is drawn per folder,
      proportional to the folder's `rest` size with a floor of 8 per
      folder, so every folder appears. The **inclusion probability** of a
      frame is `n_drawn / N` of its (stratum, folder) cell for `rest` and
      of its stratum otherwise, and is positive for every frame of the
      dump, so every estimate in Steps 4–5 can be reweighted to the dump.
      No stratification on genre, single / burst or pick status: the script
      prints their coverage (top genre, singles vs burst frames, picks,
      explicit rejects, per folder) and the counts are read in review; an
      under-covered class is a recorded reason to adjust the allocation
      before the user labels.
    - **Order and names** (shuffled, chosen by the user on 2026-10-17).
      The copies are shuffled (one uniform draw per sampled frame, seed 0)
      and named `<nnnn>_<folder>_<file>` (`0001_2026-06-05__DSC0440.ARW`):
      the four-digit draw order so the file-name sort shows a random order
      and the user judges each frame on its own, the `<folder>_<file>` tail
      the good-mark convention so an original maps back by name alone.
    - `manifest.tsv` columns: `name`, `source` (library path), `folder`,
      `file`, `draw` (the shuffled index), `stratum`, `inclusion_prob`,
      `pick` and `reject` (the dump's labels, for consistency checks only),
      `single` (burst of one), `burst_size`, `capture_ms`, `genre` (the
      frame's top-level label from `gate.build`, `Unknown` when none), the
      raw cue values (`sharpness`, `eye_focus`, `ear`, `yaw`, `pitch`,
      `roll`, `tier`), the rule flags (`closed`, `focus_low`, `away`,
      `sharp10`), and the stored held-out scores (`W0`, `W0+off`, `F0`,
      `F0+ctx`, `P0`, `R0`, `G0`, `rule count`). Also `strata.tsv`: per
      stratum (and per folder for `rest`) the population size, the drawn
      count and the inclusion probability.
    - **Checks** printed and recorded in `learnings.md`: each copy's size
      and SHA-256 equal its source's; no `.xmp` / `.dop` exists in
      `sample\`; the count of copies equals the manifest's rows; the
      library folders' listings are unchanged (a listing before and after,
      compared); the sample opens in Riffle (the user or the main agent
      opens `D:\Photos\tests\2026-10-17-bad-labels\sample` once and the
      strip shows 1,200 unflagged frames).
    - `labeling.md` in this plan folder (English; the main agent relays it
      in Japanese) states: the purpose (labels of **bad**, not of "not the
      best"); the definition, **Reject = you would never keep this frame on
      its own** (closed eyes at the moment, missed focus on the subject,
      motion or camera blur, a cut-off or hidden subject, a face turned
      away at the decisive moment, exposure beyond rescue, nothing in the
      frame), and the explicit non-reasons (**a better frame exists**, a
      duplicate, a merely ordinary photo, a frame you would not pick but
      would not mind keeping); the setup (open
      `D:\Photos\tests\2026-10-17-bad-labels\sample`, `aiEnabled` off,
      file-name sort, no filter, let the scan finish first); the key (`x`
      reject, sticky, so `u` (unflag) or undo clears it, not a second `x`
      (corrected in Step 1); `shift+x` must not be used, it rejects
      the rest); the rule to look at the picture, not the `f` mark or the
      meta pane; the pace (1–2 s per frame, about 20–40 minutes for 1,200,
      in as many sittings as wanted, nothing to save); the **stopping
      rule** (every frame viewed once, from `0001` to the last; a frame left
      unflagged counts as not bad, so skip none; when unsure, do not
      reject); and what to say when done ("done", plus anything odd, such
      as frames that would not open).
    - `mise run lint` passes.
  - Implementation approach:
    - Model the manifest on the good-mark one; keep the harness import
      order of `rejectfilter.py` and import `rejectfilter` itself for
      `load_scores`, `load_ear` and `rules` (its `rules` returns the four
      masks and the sharpness-bottom cut the strata need).
    - The bottom-15% cuts of `g0_bottom` / `w0_bottom` use the folder's
      frames with a score (`W0` is `nan` on `2026-07-23`); ties by the
      seed-0 draw as the archived metrics do.
    - Print the stratum table, the coverage tables and the disk use before
      copying; copy last. Copy time is minutes (54 GB).
    - Script and docs only; no Rust, no app change.

- [x] Step 2: Pre-register the metrics, the scores, the dedicated model and the bars in `badfilter.md`, before any label exists
  - Done when:
    - `badfilter.md` in this plan folder has a "Pre-registration" section
      merged **before the user has labeled anything and before any label is
      read** (the merge commit is the record). It states what is already
      known (Decision 2's numbers on the non-pick criterion, the stratum
      sizes of Step 1), that no bad label existed when it was written, and
      that the bars were chosen by the user before labeling.
    - The section fixes, in this order:
      - **Labels and population.** `bad` = the copy carries a Reject in its
        sidecar; `not bad` = reviewed and unflagged (the stopping rule of
        `labeling.md` makes "unflagged" a label). The population is the
        dump's 32,459 frames; every estimate is reported **weighted by the
        inverse inclusion probability** (Horvitz–Thompson, the manifest's
        column), with the unweighted sample value beside it. The dump's
        `pick` / `reject` labels are consistency checks only (the weighted
        share of picks labeled bad is expected near zero; the sampled
        explicit rejects labeled bad is a test-retest reading), never a
        target.
      - **Unit.** Every sampled frame is a sample. No per-folder means
        (about 45 sampled frames per folder); the folder enters as a
        cluster in the bootstrap sensitivity reading.
      - **Cuts.** Every graded score is cut **per folder at a quantile**,
        as the archived label-free `quantile@r` does: remove the bottom
        `q`% of the folder's frames by the score (ties by the seed-0 draw),
        `q` = 5 / 10 / 20 / 30 / 40 (the "keep rate" 95 / 90 / 80 / 70 / 60%); plus
        the one cut Decision 2 named, **`G0` at its label-free `quantile@95`
        share** (the share that kept 95% of the picks). A binary rule is one
        cut. The cut is computed on the full dump (all frames of the
        folder), then read on the sampled frames, so the removed share is
        the library's, not the sample's.
      - **Primary metrics**, per cut, weighted:
        1. **Bad recall `BR@q`**: the share of bad frames removed.
        2. **Not-bad loss `NL@q`**: the share of not-bad frames removed
           (what the human loses).
        3. **Precision `P@q`**: the share of removed frames that are bad.
        And the threshold-free **weighted AUC** of bad vs not-bad and the
        **partial AUC at not-bad loss <= 10%** (`roc_auc_score(bad, -score,
        max_fpr=0.10, sample_weight=1/π)`). **The primary reading is the
        pair (`BR`, `NL`) at the cut whose `NL` is nearest the worth-a-filter
        bar's loss ceiling** (below), with `P` beside it.
      - **Scores and rules** (names as in the archived files): `W0`,
        `W0+off`, `F0`, `F0+ctx`, `P0`, `R0`, `G0`, `rule count`, the
        burst-centered `W0`; the baselines random, absolute sharpness,
        `eye_focus` (missing last), `photoTier` good; the binary rules
        `eyes closed`, `eye focus low`, `face away`, `sharpness bottom 10%`,
        `any of`, `not good`. **Per-cue reading**: the weighted bad rate per
        stratum and per bin of each raw cue (EAR, `eye_focus`, `|yaw|`,
        `|pitch|`, sharpness decile, `tier`), so the "which cues catch bad
        photos" question is answered from the labels, not from a model.
      - **Dedicated model `B0`** (Step 5): `HistGradientBoostingClassifier`
        at `rejectfilter.HGB`'s fixed setting on `folder_gate.design(d,
        mask, "full")`'s 46 columns (NaN restored as `g0_fold` does),
        trained on the sampled frames' bad / not-bad labels with
        `sample_weight = 1/π` rescaled to mean 1 per fold, **leave one
        folder out** over the 27 folders, scored on the held-out folder's
        sampled frames; and `L0`, the same with `sklearn.linear_model.
        LogisticRegression` (`C` 1, class-balanced within the weights) as
        the linear comparison. Both are read on the same cuts as the other
        scores (the per-folder quantile of the held-out score within the
        folder's sampled frames, since `B0` scores sampled frames only; the
        caveat written out). The weight scale is fixed here, in advance, as
        Decision 2 asked.
      - **Intervals.** A 1,000-draw **stratified bootstrap** over the
        sampled frames (resample within each stratum, seed 0, fresh per
        quantity) for every weighted metric and for paired differences
        between scores at the same cut; a **folder-cluster bootstrap**
        (resample the 27 folders) as a sensitivity reading beside it. The
        weighted **bad base rate** with its interval is the first number
        reported.
      - **Bars** (option B, chosen by the user on 2026-10-17 before any
        label; options A (L 10%, R 40%, P 40%) and C (L 2%, R 60%, P 60%)
        listed as rejected):
        - *Worth-a-filter bar*: a score or rule is a **usable bad-photo
          filter** when at some pre-listed cut its weighted `NL` <= **5%**
          and its weighted `BR` >= **50%** and `P` >= **50%**, with the
          bootstrap interval of `BR` above **40%** (R − 10 points).
        - *Dedicated-model bar*: a next plan trains and ships a dedicated
          model when `B0` (or `L0`) exceeds the best existing score's `BR`
          at the cut where both have `NL` <= 5% by >= **+10 points** with
          the paired bootstrap interval above zero, **or** when no existing
          score passes the worth-a-filter bar but `B0` does.
        - *A cue catches bad photos* when the weighted bad rate of its
          firing bin is at least **twice** the base rate with the interval
          above the base rate.
      - **What happens after**: no step changes the app; the Decision names
        the follow-up plan, if any.
    - `mise run lint` passes.
  - Implementation approach:
    - Assumes Step 1 merged (the stratum sizes are quoted). Model the
      section on the archived
      [reject.md](../20261016-reject-filter-eval/reject.md)
      pre-registration and keep its vocabulary. Doc-only PR.

- [x] Step 3: The user labels the sample folder (manual, the plan pauses here), then `labels.py` exports `labels.tsv`
  - Done when:
    - **The user has said "done"** after reviewing every frame of
      `D:\Photos\tests\2026-10-17-bad-labels\sample` in Riffle per
      `labeling.md`. Nothing of this step starts before that; the main
      agent waits and does not read the sample folder's sidecars meanwhile.
    - `labels.py` in this plan folder (`python -I -B labels.py <out-dir>`)
      reads the sidecars beside the copies (`<name>.xmp` / `<stem>.xmp`:
      `xmpDM:good`; `<name>.dop`: `ShouldProcess`; the `.dop` decides when
      both exist, as `gate.load` does), joins them to `manifest.tsv` and
      writes `labels.tsv` (`name`, `label` = `bad` / `not_bad`, `source_sidecar`,
      plus the manifest's columns), and prints: the counts of bad / not bad
      overall and per stratum, the sidecar formats found, any copy whose
      sidecar holds a Pick or a rating (unexpected; listed, not dropped),
      and the dump-label consistency counts (picks labeled bad, sampled
      explicit rejects labeled bad). **No score is read against the labels
      in this step.**
    - `learnings.md` records the user's remarks (frames that would not
      open, doubts about the definition, time taken) and the counts.
    - `mise run lint` passes.
  - Implementation approach:
    - Assumes Steps 1 and 2 merged. Parse the sidecars with the standard
      library (an attribute regex for `xmpDM:good`, a line match for
      `ShouldProcess`); do not import `riffle-core`.
    - If the user reports frames they could not judge (a copy that failed
      to open), the script takes an optional `skip.txt` of names, which
      leave the sample with their reason recorded; their inclusion
      probabilities are not re-normalized (reported as a deviation).

- [x] Step 4: `badfilter.py`: every existing score, rule and cue against the labels, reweighted to the library, with intervals
  - Done when:
    - `badfilter.py` in this plan folder (`python -I -B badfilter.py
      <app-dump-dir> <folder-wide-eval-dir> <reject-filter-eval-dir>
      <out-dir>`) imports the archived harnesses and `rejectfilter` by
      path, loads the dump, the stored scores, the rules and `labels.tsv`,
      computes every pre-registered metric, cut, per-cue reading and
      interval, prints Markdown tables, and saves stdout as
      `badfilter.out.md` in `<out-dir>`.
    - **Reference checks** recorded in `learnings.md`: the stratum sizes
      and inclusion probabilities recomputed from the dump match
      `strata.tsv`; the weighted sample size sums to 32,459; random reads
      `BR@q` near `q` and AUC near 0.5; `G0`'s per-folder `quantile@95` cut
      removes the frame share `reject.md` reported for it.
    - `badfilter.md` gains a "Results" section: the data counts and the
      weighted bad base rate with its interval; the per-stratum bad rates;
      the main table (every score, baseline and rule: `BR` / `NL` / `P` at
      each cut, weighted AUC, partial AUC, with intervals); the `G0`
      `quantile@95` row; the per-cue bins table; the paired differences
      against `G0` and against `W0` at the primary cut; the folder-cluster
      sensitivity table; wall time. A "Reading" paragraph stating the facts
      without deciding.
    - `learnings.md` records deviations and the choices the text left open.
  - Implementation approach:
    - Assumes Step 3 merged. Metric-only work; seconds to run. Build the
      per-folder quantile cuts on the full dump's score vectors (all
      frames), then index the sampled frames by (`folder`, `file`).
    - Weighted metrics as sums of `1/π` over the relevant sets; the
      stratified bootstrap resamples indices within each stratum and
      reuses the same draws across scores so paired differences are read on
      the same frames.

- [x] Step 5: The dedicated model `B0` / `L0` held out by folder, and the Decision
  - Done when:
    - `badmodel.py` in this plan folder (same arguments as `badfilter.py`,
      plus `[jobs]`) imports `badfilter` by path for the metrics, fits `B0`
      and `L0` leave-one-folder-out on the sampled frames exactly as
      pre-registered, writes `scores_bad.tsv` (`name`, `B0`, `L0`) to
      `<out-dir>`, prints their rows on the same cuts beside the best
      existing score, the paired differences with intervals, the `B0`
      feature-importance reading (permutation importance on the held-out
      frames, grouped by the archived `REASONS` groups) and saves stdout as
      `badmodel.out.md`; the tables are pasted into `badfilter.md` under
      "Dedicated model".
    - **The Decision** is written in this plan, against the Step 2 bars,
      with the numbers that decided it:
      1. the weighted bad base rate of the library, and the share of "bad"
         the earlier non-pick criterion overstated (the bad rate among the
         dump's non-picks, from the labels);
      2. **usable filter or not**: the best existing score or rule on the
         primary pair, whether it passes the worth-a-filter bar, what it
         costs (`NL`) and what it removes (`BR`, `P`);
      3. **which cues catch bad photos**: per cue bin, catches or not, with
         the bad rate against the base rate; beside Decision 2's
         "contributes at the bottom" column;
      4. **dedicated model**: `B0` / `L0` against the best existing score,
         the dedicated-model bar, and the recommendation (none; a larger
         labeling round of N frames with the same sampler; or a plan that
         trains and ships a bad-photo model) with the conditions stated.
    - `learnings.md` records run times and deviations; `mise run lint`
      passes.
  - Implementation approach:
    - Assumes Step 4 merged. `B0` is `rejectfilter.g0_fold`'s construction
      with the labels and weights swapped in and the fold restricted to the
      sampled frames; write it beside, do not edit the archived script.
      27 folds of about 1,150 frames each run in seconds.
    - Write the Decision only after both tables exist.

## Trade-offs and risks

- **Where the data lives: `D:\Photos\tests\2026-10-17-bad-labels\`.**
  The user's convention puts files opened by hand in the app under
  `D:\Photos\tests\<date>-<topic>\`; `D:\photos\samples\<FORMAT>\` is the
  downloaded format-verification library with a camera-model prefix and
  manifests of a different shape. The copies sit in `sample\` so the folder
  Riffle opens holds RAW files only.
- **Sample from the 27 dump folders only vs a re-dump.** Only the dump has
  stored scores, cue values and embeddings; a re-dump of other library
  folders costs 3–4 minutes per folder plus the embedding model and would
  add folders of the same photographer and mostly the same genre. The dump
  spans five months, 27 folders, singles and bursts and the genre labels.
  The cost: the Decision speaks of this dump (and this photographer), not
  of the whole library; the reweighting is to the dump's 32,459 frames.
- **Sample size and allocation: decided (2026-10-17), 1,200 frames,
  option A (cue-heavy, 700 in strata 1–6, 500 `rest`).** It measures
  precision on the frames a filter would actually remove. At 1–2 s per
  frame that is 20–40 minutes. If the bad rate is around 15–25%, roughly
  300–450 bad frames are expected, enough for `BR` intervals of about ±5
  points. Rejected: option B (uniform-heavy, 400 / 800), 1,000 frames and
  1,500 frames.
- **Order: decided (2026-10-17), shuffled with a draw-order prefix.** The
  label is absolute ("would never keep"), and at a 4% sampling rate burst
  neighbors are mostly absent anyway, so folder order would give partial
  context that invites "a better frame exists" judgments. The user can
  switch Riffle to capture-time sort at any time to get context without
  renaming. Rejected: folder order (`<folder>_<file>` alone).
- **Bars: decided (2026-10-17), option B** (L 5%, R 50%, P 50%; the
  dedicated-model gain +10 points; the cue factor 2x base rate). Rejected:
  A (L 10%, R 40%, P 40%) and C (L 2%, R 60%, P 60%). The base rate is
  unknown when the bars are set; a very low base rate (under 5%) makes
  every P floor hard and is itself a finding ("the library has few bad
  frames to filter"); the pre-registration says so.
- **Labeling bias from the app's own marks.** The `f` focus mark's color
  and the strip's face icon show the good-photo rule, which is one of the
  cues under evaluation; they cannot be turned off without an app change
  (out of scope). The guide asks the user to judge the picture; the
  per-cue reading will show whether the labels merely track the mark (a bad
  rate in the `tier` bins far above what the other cues explain). With
  `aiEnabled` off the ranking's icon and rows are absent.
- **"Unflagged = not bad" rests on the stopping rule.** A frame skipped by
  accident is a false "not bad". Mitigations: the shuffled order makes
  skipping a block unlikely to go unnoticed; the user's "done" is asked for
  explicitly; Riffle keeps the last viewed file so sittings resume. Not
  mitigated: reading the `views` table (owned by another session) to verify
  dwell; stated as a limit.
- **The sample folder in the app's index.** Opening it writes index rows,
  views and sidecars for the copies; it enters `rank_dirs` but forms no
  pairs (no picks), so the user's ranking weights do not move. After the
  plan the folder stays as data; Clear Cache or the 30-day eviction clears
  its index rows but not its sidecars, which are the labels (and
  `labels.tsv` holds them anyway).
- **Scores are held-out per-folder logits with folder-specific scales**, so
  cuts are per-folder quantiles (as Decision 2's label-free primary), not
  absolute thresholds; a cut on the sampled frames of a folder would be
  noisy, which is why the cuts are computed on the full dump and read on
  the sample. `B0` / `L0` have scores on sampled frames only, so their cuts
  are within the sampled frames; the comparison is stated with that caveat.
- **`B0` on about 1,150 training frames per fold** is a small-data fit;
  the plan treats it as a signal reading, not a shippable model, and the
  Decision's "train a dedicated model" means a next plan with more labels
  (the same sampler, a second round) or a different design, not shipping
  `B0`.
- **Multiplicity.** About 25 score rows, 6 cuts and 20 cue bins are read;
  a lone marginal pass is read as such.
- **Fail path is a valid outcome.** If no score passes and `B0` does not
  either, the artifacts are the labeled sample (reusable by any later
  plan), the sampler, the pre-registered bars, the per-cue reading and a
  recorded "no usable filter yet; here is the base rate and here is what
  the cues do".

## Decision (usable bad-photo filter, cues that catch bad photos, dedicated model)

Written 2026-10-18 at the end of Step 5, after both the Step 4 results and
the dedicated-model tables existed, against the bars pre-registered in Step 2
(option B: worth-a-filter `NL` <= 5%, `BR` >= 50% with its stratified 2.5%
quantile above 40%, `P` >= 50%; dedicated model +10 points of `BR` with the
paired interval above zero, or a pass where no existing score passes; a cue
catches at >= 2x the base rate with the interval above it). Every number is
weighted to the dump's 32,459 frames (`1/π`), held out where it comes from a
model, with the stratified bootstrap 95% interval, and comes from
[badfilter.md](badfilter.md) "Results" and "Dedicated model". No bar was
changed after the labels were read.

**1. Base rate: 5.52% of the dump's frames are bad** [4.15, 7.04]
(folder-cluster [3.71, 7.61]; 80 of 1,200 sampled frames, 6.67%
unweighted). It sits just above the 5% the Pre-registration names as very
low, with its lower bound below it. Among the dump's non-picks the bad rate
is 6.41% [4.79, 8.22], so about **94% of the non-picks the earlier
evaluations counted as negatives are not bad**: the non-pick criterion of
Decisions 1 and 2 overstated "bad" about fifteenfold. No sampled pick is bad
(0 of 167); 4 of the 6 sampled explicit rejects are.

**2. No usable bad-photo filter exists today.** No score, baseline or rule
passes the worth-a-filter bar at any pre-listed cut, on the full-dump cuts
or the within-sample ones. The best existing filter (highest `BR` with `NL`
<= 5%) is **`R0` at q5: it costs `NL` 3.7% [2.5, 5.1] of the not-bad frames
and removes `BR` 19.3% [10.9, 30.0] of the bad ones at `P` 23.1% [13.3,
35.6]**; `G0` at q5 reads 18.9% / 2.9% / 27.3%, absolute sharpness 16.8% /
2.6% / 27.1%. The highest `P` at any full-dump cut is 27.3% (29.9% within-sample, `G0` q5) against the 50% floor;
`BR` reaches 50% only at cuts losing a quarter or more of the not-bad
frames. At this base rate `P` >= 50% needs `BR` >= 17.1 x `NL`, so at `NL`
5% a filter would have to remove about 86% of the bad frames, about four
times the best measured. The low base rate is itself the finding the
Pre-registration named: the dump holds few bad frames to filter, and the
ones it holds are not ranked low enough by any existing score.

**3. Only one cue catches bad photos: the folder's bottom sharpness
decile.** Firing bins against the base rate 5.52%, beside Decision 2's
"contributes at the bottom" column (drop-one on the non-pick criterion, for
`F0+ctx` / `W0` / `G0`):

| Cue (firing bin) | Bad rate % [95%] | x base | Catches | Decision 2, bottom |
| --- | ---: | ---: | --- | --- |
| Sharpness decile 1 | 17.7 [11.1, 25.5] | 3.20 | **yes** (cluster lower bound 10.5%) | no / no / no |
| AF eye focus (`eye_focus` < 0.9) | 8.6 [4.4, 13.3] | 1.55 | no | no / no / no |
| Eyes open (raw EAR <= 0.137) | 9.2 [4.2, 15.0] | 1.66 | no | no / no / no |
| Head pose (`\|yaw\|` > 35) | 5.6 [2.2, 9.3] | 1.02 | no | no / no / no (`\|yaw\|`) |
| Head pose (`\|pitch\|` > 45) | 17.6 [0.0, 40.5] | 3.20 | no (3 of 19 sampled) | no / no / no (`\|pitch\|`) |
| Good photo (`tier` not good) | 6.6 [4.9, 8.4] | 1.19 | no | no / no / no |

So on "bad" labels sharpness becomes the one cue that catches, where it did
not contribute at the bottom on the non-pick criterion; the other "obviously
bad" cues still do not. Outside the pre-registered firing bins (a reading,
not a bar): frames with no AF face read 14.6% [10.8, 19.1] (2.65x), and
`photoTier` good frames 0.2% (1 of 256), so the good-photo rule is a strong
"not bad" signal but its complement is not a bad one. In `B0`, the held-out
ranking rests on Look (AUC drop +0.104), then Sharpness, AF eye focus and
Eyes open (+0.027 to +0.038); Head pose, Burst position and Good photo carry
nothing. A lone pass among about 20 cue bins is read with multiplicity in
mind, but this one is backed by the `sharp10` stratum's own rate (21.7%) and
the folder-cluster interval.

**4. Dedicated model: the bar is not met.** On the within-sample cuts the
best existing score is `G0` at q5 (`BR` 22.8%, `NL` 3.1%, `P` 29.9%). `B0`
at q5 reads `BR` 16.4% / `NL` 3.0% / `P` 24.2%, a gain of **−6.4 points
[−17.5, +2.0]**; `L0` at q5 reads 23.6% / 2.8% / 32.9%, **+0.8 [−12.2,
+12.8]**, both short of +10 with the interval above zero; against `R0` (the
full-dump best, read within-sample) the gains are −1.0 and +6.2 [−6.1,
+18.8]. Neither model passes the worth-a-filter bar, so the second branch
(no existing score passes but the model does) does not hold either.
Threshold-free, `L0` has the highest AUC of any score, 0.786 [0.716, 0.858]
(`B0` 0.764, `G0` 0.731, `eye_focus` 0.746), which is not a bar quantity.

**Recommendation: none** of the pre-listed follow-ups, as a
recommendation only. Not "a plan that trains and ships a bad-photo model":
the dedicated-model bar is not met. Not "a larger labeling round with the
same sampler" for this bar: a second round would narrow the intervals
(about ±10 points of `BR` now, on 80 bad frames, 13 of 27 held-out folders
holding none), but the best point estimates (`P` 33%, `BR` 24% at `NL` <=
5%) sit far from the bar's 50% / 50%, and at a 5.5% base rate the bar asks
for about 86% `BR` at 5% `NL`; more labels of the same kind would not be
expected to close that gap with these columns. The labeled sample, the
sampler and the bars stay as reusable artifacts. No app change follows from
this plan.

Left for the user to decide (no plan is opened for them here):

- whether the goal stays a **removal** filter at these bars, or a later plan
  pre-registers a different use of the labels, such as a cue shown to the
  human rather than a cut (the sharpness bottom decile catches at 3.2x, and
  frames with no AF face read 2.65x), with its own bar;
- whether a second labeling round is wanted for a **different question**
  than this bar (for example the cue readings with more power, where `|pitch|`
  > 45 reads 3.2x on 19 sampled frames), knowing it would not be expected to
  change the answer of 2 and 4.

Answered by the user on 2026-11-26, after the plan was archived: the first
question closes with no removal filter pursued. The cue shown to the human
already exists, the strip's Low mark (the gray frowning face, the judged
face's `full_eye_sharp` under `LOW_FULL_EYE_SHARP`, 0.110, in
`crates/app/src/mark.rs`). It is a different rule from the cues read above,
and its precision against these labels is not measured. The second question
stays open.

## Progress

- (2026-10-17) Plan written; 1,200 frames cue-heavy, shuffled order and bar option B chosen by the user
- (2026-10-17) Step 1 complete except the manual check that the sample opens in Riffle with 1,200 unflagged frames (pending, see `learnings.md` "Deferred issues"; do it before the user starts labeling)
- (2026-10-17) Step 2 complete
