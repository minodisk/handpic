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

# Full-size eye sharpness as the only eye-focus measure

## Purpose

Handpic judges whether the eyes of the judged face are in focus on two paths:
the preview's `eye_focus` (a logistic over the Laplacian variance and the edge
width of the mesh eye regions, `crates/core/src/candidate.rs`) and, only for
its candidates with a larger JPEG, `full_eye_sharp`
(`crates/core/src/fullsharp.rs`). On the user's labels (ILCE-7M5, 281 held-out
frames and 58 fresh ones) `full_eye_sharp` separates "focus plane on the eyes"
better than `eye_focus` and is stable under leave-one-folder-out, while refits
of the Laplacian / edge-width model at full size are not. The user does not
want two paths.

After this plan there is one measure: `full_eye_sharp`, computed for every
frame with a judged face on the largest JPEG the file holds. The Good /
Neutral / Low tiers of the Tauri app and the native app read it alone, and
`eye_focus` is gone from the code, the index, the UI, the MCP answers, the CLI
and the docs (archived plans and `CHANGELOG.md` excepted).

Target rules:

- Measure: `full_eye_sharp` for every frame whose cue judged a face, on the
  full-size embedded JPEG when the file holds one larger than the preview,
  else on the preview itself (a RAF's single JPEG, an HDR PQ CR3 whose full
  image is HEVC, a body with no larger JPEG). Same metric and the same
  thresholds on both; the value is uncalibrated on preview-only files (the
  user accepted this).
- Good: `full_eye_sharp >= SOFT_BELOW` (0.162) and the existing non-sharpness
  cuts (EAR, yaw, pitch, mesh eye offset, edge gap).
- Low: `full_eye_sharp < LOW_BELOW`, the new threshold Step 1 derives: the
  value under which at most 2% of the held-out 3-star frames fall (about
  0.110 at planning time; the user's choice).
- Neutral: everything else, including no judged face and no value.

Evidence and the threshold's derivation live in this folder (`experiment.md`,
`low-threshold.md`, `low_threshold.py` and its per-frame table, all written in
Step 1).

Decisions taken with the user at planning (2026-11-15):

- Low threshold: at most 2% of the held-out 3-star frames under it.
- The personal ranking's `Good photo` feature is set to always 0 (inert), and
  the `eye_focus` column is always the missing case. The prior is not refit.
  The folder-wide ranking ablations
  (`../_archived/20261015-folder-wide-ranking-eval/folderwide.md`) found
  neither `AF eye focus` nor `Good photo` contributes (dropping either moves
  the frame AUC by at most ±0.003, intervals across zero).
- PR #1066 (the native meta pane's stored values, with `AF eye in focus` and
  `Focus candidate` rows) merges first; Step 4 removes those rows.
- The reduced per-frame table (folder, file, rating, `full_eye_sharp`) is
  committed to this folder so the derivation is reproducible from the
  repository.
- The meta pane row `Eyes at full size` is renamed `Eye focus` (Tauri and
  native, docs). Internal names (`candidate.rs`, `Cue`, the filter's
  `candidates` key, the `full_eye_sharp` column) stay.

## Steps

- [x] Step 1: Record the experiment and derive the Low threshold from the labels
  - Done when:
    - `experiment.md` in this folder holds the experiment's numbers: the
      held-out folders and label counts (281 frames: 3 stars 154, 2 stars 39,
      1 star 88; fresh folder 58: 27 / 18 / 13), the AUCs with their CIs
      (3 stars vs rest 0.619 vs 0.694, diff +0.075, CI +0.022..+0.130; fresh
      0.640 vs 0.737, diff +0.097, CI +0.010..+0.200; not-1-star vs 1 star
      0.603 vs 0.681 held-out, diff +0.078, CI +0.019..+0.144; 0.723 vs 0.737
      fresh, diff +0.014, CI across 0), the rejected full-size Laplacian /
      edge-width refits (0.53–0.60 under leave-one-folder-out), the
      relabeling note (the strict 1:1 labels tracked ISO, AUC 0.806; "focus
      plane on the eyes" removed it, about 0.60), the ranking ablation result
      behind the `Good photo` decision, and the statement that the
      preview-only fallback was never measured.
    - A reduced per-frame table in this folder (`frames.tsv`: folder, file,
      rating, `full_eye_sharp`, one row per rated frame of the five rated
      folders) and a small pure-Python script (`low_threshold.py`, standard
      library only) that reads it and prints, per threshold on a grid, the
      frames under it per star for the held-out folders and for the fresh
      folder. The script's header says how the table was produced (a dump of
      a scratch `riffle-cli` subcommand joined with the `xmp:Rating` of the
      sidecars under `D:\Photos\tests\<folder>`, 0 = skipped), and that the
      table, not the external files, is its input.
    - `low-threshold.md` in this folder records the rule (at most 2% of the
      held-out 3-star frames under the threshold), the chosen `LOW_BELOW`, the
      table the script printed, the 3-star share under the threshold
      (held-out and fresh), the 1-star recall (held-out and fresh), the
      2-star share, and the same counts for today's `NotCandidate` Low for
      comparison.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Inputs to build `frames.tsv`, all outside the repository:
      `C:\Users\daisu\AppData\Local\Temp\claude\0b9f3ace4447eb66\f4971480-421f-46bc-8879-8a6a9ee28236\scratchpad\fullsize-eyefocus\dump.tsv`
      (columns `folder`, `file`, `iso`, `face_side`, `eye_focus`, `state`,
      `full_eye_sharp`, ...; `full_eye_sharp` there is measured on every
      judged frame), the `xmp:Rating` of the sidecars under
      `D:\Photos\tests\<folder>`, and the existing reader
      `C:\Users\daisu\AppData\Local\Temp\handpic-eyefocus\check0605.py`, whose
      join can be copied. `results.md` next to the dump holds the
      experiment's full tables for `experiment.md`. These Temp files can
      disappear; build the table first.
    - The table may also carry `eye_focus` and `state` so the script can print
      today's Low for comparison.
    - Held-out folders: `2026-06-14`, `2026-07-18`, `2026-08-01`,
      `2026-08-22` (`-focus-sample`); fresh folder: `2026-06-05-focus-sample`.
      The dump holds four more folders; select by name.
    - The threshold is chosen on the held-out folders; the fresh folder is a
      report-only check.
    - A planning-time run of the same join gave, for frames under the
      threshold: 0.110 → held-out 3 stars 3/154, 2 stars 5/39, 1 star 13/88,
      fresh 3 stars 0/27, 1 star 5/13. Treat it as a cross-check of the
      script, not as the result.
    - Touch nothing outside `docs/plans/20261115-full-eye-sharp-only/`.

- [ ] Step 2: Measure `full_eye_sharp` for every judged face on the largest JPEG
  - Done when:
    - `extract_analysis` fills `full_eye_sharp` for every frame whose cue
      judged a face (`cue.eye_centers` is `Some`), whatever its `eye_focus`:
      from the full JPEG when `has_larger_jpeg`, else from the preview bytes
      the pass already holds.
    - `fullsharp::full_eye_sharpness` measures a JPEG at any scale, the
      preview's own (scale 1) included.
    - `FACES_VERSION` is 11 with its doc comment extended, so pass 2 reruns.
    - Unit tests cover: a judged non-candidate is measured; a file without a
      larger JPEG is measured on its preview; no judged face gives `None`;
      cancel still abandons the file.
    - The preview-measured value is compared with the full-size one on the
      labeled ARW folders and the result (how many 3-star frames fall under
      `SOFT_BELOW` and under Step 1's `LOW_BELOW` when measured on the
      preview) is recorded in this folder. This is a record, not a gate.
    - The pass 2 cost is measured before and after on one real folder
      (frames measured, ms per file) and recorded in
      `docs/humans/performance.md` and its `.ja.md` pair.
    - `docs/agents/tauri-app.md` (the `FACES_VERSION` paragraph) and
      `CLAUDE.md` describe the new measuring rule.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (the comparison reads `LOW_BELOW` from
      `low-threshold.md`).
    - Files: `crates/core/src/scan.rs` (`full_eye_sharp`, `has_larger_jpeg`,
      the `Analysis::full_eye_sharp` and `extract_analysis` doc comments,
      their tests), `crates/core/src/fullsharp.rs`,
      `crates/backend/src/index.rs` (`FACES_VERSION` only; no schema change),
      docs.
    - `full_eye_sharp` in `scan.rs` drops the
      `cue.state == FocusCandidate::Candidate` condition. `has_larger_jpeg`
      stays as the switch between the two sources. The preview path needs no
      read and no `ReadSlots` slot: `extract_analysis_unless` already holds
      the preview bytes, and `reader` already hands an HDR PQ CR3's HEVC
      `PRVW` out as a JPEG.
    - `fullsharp::full_eye_sharpness` returns `None` when the JPEG is not
      more than `MIN_SCALE` (1.5) times the preview width. That guard has to
      go for the preview path. The test
      `full_eye_sharpness(&jpeg, eyes, 100), None, "exactly 1.5x"` changes
      with it.
    - A failed full-size read or decode stays `None`, as today; it does not
      fall back to the preview.
    - The tier rule is not touched in this step. Its side effect: a file
      without a larger JPEG now carries a value, so today's
      `full_eye_sharp >= GOOD_FULL_EYE_SHARP` cut starts applying to it.
    - Cost to call out in the PR and the docs: every judged face now reads
      the full JPEG (about 77–81% of them did), at about +51 ms per measured
      frame with 8 threads and a warm cache (`docs/agents/tauri-app.md`); the
      preview path adds two partial decodes and no read. The cold-disk cost
      is unmeasured.
    - `riffle-cli facechoice` already prints `full_eye_sharp` per face; a
      preview-measured column there is one way to do the comparison.

- [ ] Step 3: Read the Good / Neutral / Low tiers from `full_eye_sharp` alone
  - Done when:
    - `fullsharp.rs` holds `LOW_BELOW` (Step 1's value) next to `SOFT_BELOW`,
      pinned by a Rust test like `the_threshold_is_the_figure_the_ui_cuts_good_at`.
    - Tauri (`crates/app/ui/src/focus.ts`): `afEyeState` is `low` when
      `full_eye_sharp` is under the Low threshold, `good` when it is at or
      above `GOOD_FULL_EYE_SHARP` and the EAR, pose, eye offset and edge gap
      cuts hold, else `neutral`; a `null` value is `neutral`. `photoTier` no
      longer reads `candidate` or `eye_focus`. `GOOD_EYE_FOCUS` and
      `CANDIDATE_PERCENT` are gone, and `AF_EYE_HELP` states the new rule
      from the constants.
    - Native (`crates/native/src/mark.rs`): `photo_tier` and `af_eye_state`
      follow the same rule with the same constants; `filter.rs` and the
      companion give the same states through them.
    - Tests in `focus.test.ts`, `filter.test.ts`, `mark.rs` and `filter.rs`
      cover: Good at the cut and not below it; Low under the Low threshold
      whatever else clears; Neutral between the two; Neutral with no value;
      Neutral without a focus.
    - `README.md` / `README.ja.md` and `docs/humans/usage.md` /
      `usage.ja.md` describe the tiers by the new rule (no 90%, no 77%, no
      "skips the check" for RAF / ORF / HDR PQ CR3) and give the new
      evidence in place of the candidate cue's precision and coverage
      figures. `docs/agents/tauri-app.md` and `docs/agents/native-app.md`
      follow.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged: before it a non-candidate has no value and
      would turn Neutral instead of Low.
    - `focus.ts` and `mark.rs` mirror `SOFT_BELOW` as `GOOD_FULL_EYE_SHARP`
      today; mirror `LOW_BELOW` the same way, with the same kind of pinning
      test on each side.
    - Low used to come from Rust (`candidate: "not_candidate"`); it now
      comes from the value on the consumer's side, in `focus.ts` and
      `mark.rs`. The `candidate` and `eye_focus` fields stay on the types
      until Step 5; this step only stops the tier reading them.
    - Keep the filter's `candidates` key, `data-candidate` and the native
      `Toggle::Candidate` as they are: they name the `Face focus` set, and
      the stored `filter` setting uses the key.
    - `rank.rs` has its own `GOOD_EYE_FOCUS`; leave it for Step 4.
    - The frontend tests do not run under `ci:local`; the PR's checks run
      them.

- [ ] Step 4: Stop reading `eye_focus` in the ranking, the meta pane and the CLI; rename the meta row
  - Done when:
    - `crates/core/src/rank.rs`: `Frame` has no `eye_focus`; the value's
      column is always the missing case; `good_photo` is always false (the
      `Good photo` feature inert); `GOOD_EYE_FOCUS` is gone;
      `ranking/siglip2.json`, `RANK_VERSION`, `COLUMNS` and `TECH` are
      unchanged. A new test ranks and fits frames with no `eye_focus` and
      gets finite scores and places.
    - `Index::rank_rows` (`crates/backend/src/index.rs`) no longer selects
      `eye_focus`, and the debug dump in `crates/app/src/commands.rs` (the
      TSV header near line 3009) no longer prints `state` / `eye_focus`.
    - `crates/app/ui/src/meta.ts` has no `AF eye in focus` row, and its
      `Eyes at full size` row is named `Eye focus`; `meta.test.ts` follows.
    - The native meta pane (`crates/native/src/meta.rs`, from PR #1066) has no
      `AF eye in focus` and no `Focus candidate` row, and names its
      full-size row `Eye focus` if it has one.
    - `riffle-cli`: nothing calls `candidate::eye_focus`, `EyeFocus`,
      `Scored`, `FocusCandidate` or `Cue::eye_focus` / `Cue::state`. The
      `candidates` subcommand, which exists to validate the logistic model,
      is removed with its usage line; `focusbox`, `features` and
      `facechoice` lose their `eye_focus` / `state` / `scored` / window
      columns and keep the rest (`facechoice` keeps the face choice, the
      mesh offsets, the edge gap and `full_eye_sharp`).
    - `docs/humans/usage.md` / `usage.ja.md` (the meta pane rows, the new
      `Eye focus` name), `docs/humans/performance.md` / `.ja.md` and
      `docs/agents/tract-onnx-inference.md` (the mentions of
      `riffle-cli candidates` as a tool to run) and `CLAUDE.md` (the
      `facechoice` description) follow.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 3 is merged, and PR #1066 is merged (the user's choice);
      if #1066 is still open, stop and report rather than editing its branch.
    - Do not rework the ranking and do not refit the prior. `tech()` already
      maps a missing `eye_focus` to NaN (then `prior.tech_mean`) and sets
      indicator column 7; make that the only case.
    - Verify what the `AF eye focus` and `Good photo` entries of `REASONS`
      yield once their columns are constant within a burst (a reason is a
      difference from the burst's mean, so it should be 0), and that the
      meta pane's `Reason` row never names them. Remove an entry if the
      "logits sum to score" property holds without it.
    - The user's stored `ranking.json` was fit with the old columns; check
      that `rank` still runs on it (the background refit replaces it).
    - The performance docs keep their past measurements; only wording that
      tells the reader to run `riffle-cli candidates` changes.

- [ ] Step 5: Remove `eye_focus` from the core, the index and the app types
  - Done when:
    - `crates/core/src/candidate.rs` has no logistic model: `LOGIT_*`,
      `MESH_LOGIT_*`, `CANDIDATE_LOGIT`, `CANDIDATE_WINDOW_MIN`,
      `EYE_REGION_MIN`, `FocusCandidate`, `EyeFocus`, `Scored`, `Eye`,
      `eye_focus`, `mesh_logit`, `scored`, `candidate`,
      `candidate_probability`, `edge_width`, `eye_window`, `eye_measures`
      and the region measures are gone with their tests. `Cue` has no
      `state` and no `eye_focus`. What stays: the face choice
      (`nearest_face`, `focus_cue`, `focus_cue_unless`, `face_cue_unless`,
      `meshes_face`), `luma` (used by `fullsharp.rs`), `mesh_eye_centers`,
      `mesh_eye_offset`, `edge_gap` with `grown_bounds` and
      `EYE_REGION_MARGIN`, the EAR and the pose.
    - The index has no `eye_focus`: `SCHEMA_VERSION` 25 drops
      `files.eye_focus` in place, a new database is created without it, and
      a migration test opens a v24 database and keeps its files, ratings and
      the other pass 2 values. `Focus`, `FaceReady` and `FacesRow` have no
      `eye_focus` and no `candidate`.
    - MCP `get_photo`, `faces-progress` and the folder entries carry neither
      field.
    - Tauri frontend: `MarkFocus`, `applyFaceReady`, the types in `main.ts`
      and `meta.ts` and the tests have neither field.
    - Native: `filter.rs`, `mark.rs`, `overlay.rs` and `meta.rs` have neither
      field and no `FocusCandidate` import.
    - `git grep -i -E "eye_focus|eyeFocus|FocusCandidate|not_candidate|CANDIDATE_LOGIT|GOOD_EYE_FOCUS"`
      finds nothing outside `docs/plans/**` and `CHANGELOG.md`.
    - `CLAUDE.md`, `docs/agents/tauri-app.md`, `docs/agents/native-app.md`,
      `docs/agents/tract-onnx-inference.md` and `todo.md` no longer describe
      `eye_focus` as something the app has.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 2–4 are merged, so nothing outside the scan reads the
      fields any more.
    - Migration precedent in `Index::migrate`:
      `ALTER TABLE folders DROP COLUMN culling_finished` for versions 21–22,
      and the v15 swap of `eye_sharpness`. Older migration blocks that
      `ADD COLUMN eye_focus` (v10–v14, v15) must still end without the
      column: check every version range, and update the older migration
      tests that assert the column exists (`a_v13_...`, `a_v14_...`,
      `a_v15_...`).
    - `FACES_VERSION` needs no further bump if the remaining cue values
      (face, EAR, pose, eye offset, edge gap, eye centers) are unchanged;
      verify that on a real folder and bump only if they move.
    - A face without a mesh (under `eyes::EYES_MIN_FACE`, or a mesh
      failure) used to be scored on the eye window. It now keeps only its
      YuNet eye centers for `full_eye_sharp`; `cue_of` already falls back to
      them.
    - The module keeps its name `candidate.rs` and the type `Cue`; reword
      the module doc and the "focus candidate cue" phrases in `CLAUDE.md`
      and the guides.
    - `docs/agents/tauri-app.md` holds learnings that are history ("A
      learned discard score ... did not beat `eye_focus`", the
      `GOOD_EYE_FOCUS` notes): mark them as superseded by this plan or cut
      them to what still guides work, rather than leaving them reading as
      current.
    - `todo.md` has an item proposing a held-out check of `GOOD_EYE_FOCUS`;
      it is moot after this plan.
    - `crates/native` is a separate workspace that uses `riffle-backend` by
      path, so the field removal and the native changes must be one PR.

## Trade-offs and risks

- **Low catches only clearly soft frames.** At about 0.110, 3/154 held-out
  3-star frames and 13/88 1-star frames (15%) fall under it. Low becomes
  "clearly soft", not "most misses". Today's Low holds 3/88 held-out 1-star
  frames.
- **Good gets looser on one side and stricter on another.** Without the
  0.90 cut, a frame at or above 0.162 is Good whatever its preview score.
  On the held-out labels 29/154 3-star frames (18.8%) are under 0.162 and so
  not Good, and 41/88 1-star frames are at or above it before the other cuts.
  A missing value now means Neutral, where it used to leave a candidate Good.
- **Preview-only files are uncalibrated.** The metric was never measured on
  a preview. On a 1616 px preview an eye crop is a few dozen px upscaled to
  154 px, so scores may sit systematically lower and push the bodies with no
  larger JPEG (α9 II, α7R IV, α7C, α6400, α6600, ZV-E10), RAF and HDR PQ CR3
  toward Low. Step 2 records a comparison; it does not gate. The user
  accepted this for one path.
- **`MIN_SCALE` goes.** A full JPEG at most 1.5x the preview width was not
  measured; under "the largest JPEG" it is.
- **Cost.** Every judged face reads the full JPEG (about 77–81% did), about
  +51 ms per measured frame warm; cold-disk cost is unmeasured. The preview
  path adds two partial decodes and no read, and no HEVC decode of its own.
- **Ranking.** `Good photo` inert and `eye_focus` always missing leave their
  prior weights inert. The ablations measured no contribution from either, so
  no refit; the ranking is Tauri-only and the GPUI port drops AI features
  (#1017).
- **PR #1066.** Merged before Step 4 by the user's choice; Step 4 removes the
  two rows it adds. If it is still open at Step 4, stop and report.
- **Names kept.** `candidate.rs`, `Cue`, the filter's `candidates` key and
  the `full_eye_sharp` column keep their names although "candidate" no longer
  means a model's output and the value can come from a preview. Renaming the
  module or the stored filter key is a wider diff and a settings migration;
  not taken. The user-facing row is renamed `Eye focus`.
- **Photo file names in the repository.** `frames.tsv` carries file names and
  ratings of the user's test photos (no images); archived plans already name
  such frames. The user chose to commit it.
- **One body.** All labels are ILCE-7M5. `SOFT_BELOW` and `LOW_BELOW` are
  applied to every camera.
- **Intermediate states on main.** After Step 2 and before Step 3,
  preview-only files get the Good cut from an uncalibrated value while Low
  still comes from `eye_focus`. Steps 2 and 3 should be merged back to back.
- **Index downgrade.** A v25 index has no `eye_focus`; an older build
  opening it is whatever `Index::open` does with a newer `user_version`
  today. The Tauri app and the native app keep separate index files, and
  both migrate through the backend.

## Progress
