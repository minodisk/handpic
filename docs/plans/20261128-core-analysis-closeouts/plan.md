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

# Core analysis closeouts: windowed luma, the all-faces unknown answer, non-face candidates

## Purpose

Close out three `crates/core` analysis items of `todo.md` (find each by its
heading):

1. "Core: the AF path's sharpness luma covers the whole frame though the
   score reads only its window": pass 2 computes `candidate::luma` over the
   whole RGB decode, though `sharpness::laplacian_variance` reads only the
   score's window. A window-only luma gives the same score for less work.
2. "Core / CLI: `allfaces` answers unknown on a frame with no judged face
   but a 0.6 to 0.8 face (candidates-floor)": `handpic-cli allfaces` prints
   `- - -` on such a frame of a folder without AF points, where it printed
   `0 0 0` before the candidates-floor change. The answer has to be decided
   and documented.
3. "Core: non-faces among pass 3's candidates on AF frames
   (candidates-floor)": at the 0.6 floor the 640x448 whole-image search of
   an AF frame counts a jersey number, a ball, a hand or an ear as a face.
   A size band relative to the judged face or a higher floor on the AF path
   is to be measured and one of them adopted, with an `ALLFACES_VERSION`
   bump.

Out of scope: "Core: the time cost of pass 3's 0.6 floor is unmeasured on an
idle host" (it needs repeated runs with the user's go-ahead).

Rules for every measurement in this plan (from `CLAUDE.md`): single runs
only, one thread, no generated CPU load, no repeated loops. The host is
shared, so no timing from these runs is quoted as a cost unless it is
labeled a single run on a shared host. The user's task asks for these CLI
runs, so Steps 1 and 3 build `handpic-cli` locally
(`cargo build --release -p handpic-cli`, the CLI crate only).

Test material, all present on this host on 2026-11-28 (read only):

- `D:\photos\2026\2026-11-07` (3853 ARW; the item uses the first 300 in
  sorted order, the set of
  [the candidates-floor measurement](../_archived/20261128-candidates-floor/measurement.md)).
- `D:\photos\2026\2026-02-01` (146 Leica M11-P DNG, no AF point).
- `D:\photos\2026\2026-09-19` (282 trusted-AF Sony ARW, 1616x1080 previews,
  the set of
  [the luma-reuse measurement](../_archived/20261128-af-luma-reuse/luma-reuse.md)).
- `D:\Photos\samples\RAF\` (public RAF samples, larger previews).

## Decisions

Taken on 2026-11-28 with the recommended options (Trade-offs 1, 3 and 4):

- Step 2: Option A, zero counts (`0 0 0`) on a no-AF frame with no judged
  face.
- Step 3: the new rule applies to frames with a trusted AF point only.
- Step 3: a size band, if chosen, applies no band on a frame with no judged
  face.

## Steps

- [ ] Step 1: Compute the sharpness luma over the score's window only
  - Done when:
    - `scan::score_rgb` computes the luma of only the window the score
      reads on the three window paths (the eye-AF frame window, the
      `WINDOW` window around the AF point, the eye window of the chosen
      face); the tile fallback (`sharpness::tile_max`) still takes the
      full-frame luma.
    - A unit test asserts exact `f64` equality between the windowed score
      and `score_gray` over the full-frame `candidate::luma`, on an image
      larger than the window so the window is a strict sub-rectangle, for
      each of the three window paths and for a window clamped at an image
      edge.
    - One CLI run over a handful of real files shows bit-identical scores
      (compared as `f64::to_bits`) between pass 2's stored score and the
      full-frame-luma reference, and the file list, the two bit patterns
      per file and the verdict are recorded in `learnings.md` (or a
      `luma-window.md` beside it).
    - `FACES_VERSION` is unchanged (17).
    - `docs/agents/shared-crates.md` (the `FACES_VERSION` 13/17 paragraph)
      says in one sentence that the luma is windowed, and
      `docs/humans/performance.md` and `docs/humans/performance.ja.md` no
      longer describe a full-frame luma where they describe the current
      behavior.
    - The item is deleted from `todo.md`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/core/src/scan.rs` (`score_rgb`, and the test
      `without_an_af_point_the_largest_confident_face_is_judged`, which
      compares `score_rgb` with the full-frame reference on a 64x48 image),
      `crates/core/src/sharpness.rs` (`score_gray`, `window_at`,
      `frame_side`, `eye_window`), `crates/core/src/candidate.rs` (`luma`).
    - `score_gray` picks its window inside itself. The window choice has to
      become reachable from `score_rgb` without duplicating the rule: for
      example a function in `sharpness.rs` that returns the window (or
      "tiles" for the fallback) from `(w, h, focus, frame, faces)`, which
      `score_gray` then also uses. Keep `score_gray`'s signature and its
      `w < 3 || h < 3` error, since `score_preview` and the CLI use it.
    - Why bit-identity holds: `laplacian_variance` reads only pixels inside
      the window (it iterates the window's interior, whose neighbors lie in
      the window) and accumulates in integers, so a luma buffer of just the
      window, scored with a window at origin of the same size, gives the
      same sums.
    - `candidate::luma` has other callers (`fullsharp.rs`, tests); do not
      change what it returns.
    - On the no-AF path an RGB decode exists only when the cue judged a
      face at or above `FACE_CONFIDENCE`, so `chosen_face` finds one and
      the tile path is not expected there; keep the full-frame fallback
      anyway instead of asserting.
    - The real-file check: one release build of the CLI
      (`cargo build --release -p handpic-cli`) with an uncommitted scratch
      subcommand, adapted from
      [lumascore.patch](../_archived/20261128-af-luma-reuse/lumascore.patch),
      that prints for each file `scan::extract_analysis(..).sharpness` and
      the reference `score_gray(&luma(decode_rgb(preview)), ..)` as
      `to_bits`. `handpic-cli features` prints the score to 4 decimals, so
      it cannot show bit-identity by itself. One run, one thread.
    - Files for that run: about 6 ARW of `D:\photos\2026\2026-09-19`
      covering both `eye_af=1` and `eye_af=0` (the luma-reuse spot check
      used `_DSC1796`, `_DSC1797`, `_DSC1802`, `_DSC1806`, `_DSC1807`,
      `_DSC1813`), one RAF of `D:\Photos\samples\RAF\` that has a trusted AF
      point (the larger preview the item asks for; pick one whose
      `features` row says `af`), and one faced DNG of
      `D:\photos\2026\2026-02-01` for the eye-window path. On Leica
      previews the RGB luma equals the Y channel, so the DNG row would also
      match if the score took the grayscale path by mistake; the ARW and
      RAF rows are the real evidence.
    - Docs: the todo item's estimate is "about 3 ms more per ARW file". Do
      not write a new saving as measured unless the same single run timed
      it, and then label it a single run on a shared host. Places that
      quote the full-frame luma: `docs/agents/shared-crates.md` around
      "Since `FACES_VERSION` 17", `docs/humans/performance.md` (the cost
      table row "Sharpness score (BT.601 luma of the cue's RGB decode +
      window, ~3ms ...)" and the "full-frame luma and window that replace
      it 3.1ms" paragraph, which is a dated measurement: add that the luma
      is windowed since, do not rewrite the measured figures), and the
      matching paragraph of `docs/humans/performance.ja.md`. Keep the
      English and Japanese files in sync in the same PR.
- [ ] Step 2: Decide and implement the all-faces answer on a frame with no judged face
  - Done when:
    - `scan::extract_face_counts` gives counts of zero (`0 0 0` in
      `handpic-cli allfaces`) on a frame of a folder without AF points
      whose whole-image detection has no face at `FACE_CONFIDENCE` but one
      at 0.6 to 0.8 (Decisions; Trade-offs 1, Option A).
    - A failed whole-image detection or a failed preview decode that a
      measurement needed still answers unknown (`Ok(None)`).
    - A unit test in `crates/core/src/scan.rs` covers the decided case and
      the decode-failure case.
    - The doc comments of `extract_face_counts`, `other_faces` and
      `whole_image_cue_rgb` state the rule, and
      `docs/agents/shared-crates.md` records the decision and its reason;
      `CLAUDE.md`'s `scan::extract_face_counts` clause is corrected if the
      rule changes what it says.
    - The item is deleted from `todo.md`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/core/src/scan.rs` (`extract_face_counts_unless`,
      `other_faces`, `whole_image_cue_rgb`, the test
      `the_other_faces_need_a_detection_and_a_decode_only_when_there_is_one`),
      `crates/cli/src/main.rs` (`allfaces_line`'s doc comment, if its
      wording on `-` changes), `docs/agents/shared-crates.md`.
    - Where the unknown comes from: `whole_image_cue_rgb` returns
      `(Cue::unknown(), None)` both when `eyes::judged_face` finds no face
      (no decode was attempted) and when the decode failed. `other_faces`
      then answers `None` whenever `allfaces::candidates` is non-empty and
      there is no RGB. The two cases must be told apart: the first is the
      one decided here, the second stays unknown.
    - The AF path is different and is not part of the item: `af_cue`
      decodes before it looks for a face, so an AF frame with no judged
      face has an RGB decode and its candidates are measured. Do not change
      it in this step; Step 3 owns the candidates of AF frames.
    - No `ALLFACES_VERSION` bump: the gate count of a frame without a
      judged face is 0 (`allfaces::gate_count`), `group::MIN` is 2, so
      `Index::allfaces_todo` never selects such a row and no stored count
      changes. Say so in the PR.
    - No real-photo run is required. If one is wanted, a single
      `handpic-cli allfaces 'D:\photos\2026\2026-02-01' 1` shows the 14
      frames; record it in `learnings.md`.
- [ ] Step 3: Drop the non-face candidates on AF frames and bump `ALLFACES_VERSION`
  - Done when:
    - A `measurement.md` in this plan folder records, for the first 300
      ARW of `D:\photos\2026\2026-11-07`: every candidate scoring 0.6 to
      0.8 with its score, its box long side over the judged face's and
      whether `allfaces::measured` accepted it; what each option (a size
      band, a higher AF-path floor) drops among the 6 non-faces, the 1
      undecidable face and the 6 real faces the candidates-floor
      measurement named; the per-frame count changes of the chosen option;
      and the decision with its reason.
    - The chosen option is implemented in `allfaces::candidates` (and its
      caller), with unit tests of the new rule.
    - `ALLFACES_VERSION` is 3, with a doc line saying what 3 changed.
    - One confirming run of the final build over the same 300 files
      matches the predicted counts, recorded in `measurement.md`.
    - `CLAUDE.md`'s description of `allfaces` `candidates`,
      `allfaces.rs`'s module doc, `docs/agents/shared-crates.md` ("Pass 3's
      candidates from 0.6 ..."), `docs/humans/performance.md` /
      `performance.ja.md` (the "entry point's other faces count from the
      same 0.6" paragraph) and `docs/humans/usage.md` / `usage.ja.md` (the
      `Faces judged` description near "score of 0.6 or more, at most
      eight") describe the new rule.
    - The item is deleted from `todo.md`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged: its rule for a frame with no judged face
      stays as decided there.
    - Files: `crates/core/src/allfaces.rs` (`candidates`, the constants'
      docs, the module doc, the tests), `crates/core/src/scan.rs`
      (`other_faces`, which calls `candidates`, and
      `extract_face_counts_unless`, which knows whether the frame has a
      trusted AF point), `crates/engine/src/index.rs` (`ALLFACES_VERSION`
      and its doc), the docs listed above.
    - The measurement, as few runs as possible: one release CLI build
      (`cargo build --release -p handpic-cli`) of the current behavior
      with an uncommitted, environment-gated scratch dump in
      `allfaces::measure_unless` or `scan::other_faces` (the
      candidates-floor measurement did the same), one line per candidate:
      file, box, score, the judged face's long side, accepted or not. One
      run of `handpic-cli allfaces <the 300 files> 1`. Both options can
      then be evaluated offline from that one dump, because candidates are
      independent of each other and no frame reached `MAX_FACES` (at most
      7 of 8). Then one run of the final build to confirm. No timing is
      taken from these runs.
    - The faces to check are named, with scores, in
      [the candidates-floor measurement](../_archived/20261128-candidates-floor/measurement.md):
      non-faces `_DSC1544` 0.687, `_DSC1627` 0.674, `_DSC1644` 0.695,
      `_DSC1653` 0.686, `_DSC1771` 0.603, `_DSC1772` 0.752; undecidable
      `_DSC1501` 0.747; real `_DSC1514` 0.692, `_DSC1676` 0.750,
      `_DSC1677` 0.790, `_DSC1685` 0.796, `_DSC1751` 0.775, `_DSC1754`
      0.751.
    - What is already known about the floor option from those scores: a
      floor of 0.70 on the AF path drops 4 of the 6 non-faces and 1 real
      face; 0.76 drops all 6 non-faces and the undecidable one but also 3
      of the 6 real faces; 0.80 is the old behavior on AF frames. The band
      option's result is unknown until the dump gives the box ratios.
    - Decision rule: take the option that drops more of the 6 non-faces
      while losing fewer of the 6 real faces; on a tie take the size band
      (`GATE_MIN_SIZE` to `GATE_MAX_SIZE` of the judged face's long side,
      the gate's own rule and constants, no new number). Record the
      comparison table either way.
    - Scope the rule to frames with a trusted AF point (Decisions):
      the no-AF set's added faces were all real, and
      `docs/agents/shared-crates.md` says the lever is the AF path.
      `candidates` then needs to be told which rule applies (as
      `gate_count` takes its `floor`).
    - A size band needs a judged face. On a frame without one no band
      applies (Decisions; Trade-offs 4).
    - The bump makes pass 3 re-run on every row at the gate on the next
      scan of a folder; no schema change.

## Trade-offs and risks

1. Step 2, the answer on a no-AF frame with no judged face but a 0.6 to 0.8
   face. Decided: Option A.
   - Option A (taken): answer zero counts (`0 0 0`) by not taking
     candidates when the whole-image search judged no face. It restores the
     answer from before the candidates-floor change, costs nothing, matches
     the gate (0 without a judged face) and the module's framing of the
     other faces as "besides the face the cue judges". Consequences: a real
     0.6 to 0.8 face on such a frame is not counted, and the no-AF path
     differs from the AF path, where a frame with no judged face still
     measures its candidates because it has a decode.
   - Option B: decode the preview and measure the candidates. Symmetric
     with the AF path and counts real faces, but adds a full-size decode
     and up to 8 meshes per such frame for an answer only the CLI shows,
     and makes `judged` 1 or more on a frame whose gate count is 0.
   - Option C: keep `- - -` and only document it. No code change, but
     "unknown" then means both "a read or decode failed" and "nothing was
     attempted", which the CLI's reader cannot tell apart.
   - In every option the app is unaffected: pass 3 never runs at gate 0.
2. Step 3, which option wins is not known at planning time; the step
   decides it by the rule written there. If neither option drops most of
   the non-faces without losing real faces (for example the band keeps the
   jersey numbers because they are face-sized), the fallback is the 0.80
   floor on AF frames, the behavior before candidates-floor on that path;
   it loses the 6 real added faces (1 at gate 5 or more). The plan should
   then be updated before implementing.
3. Step 3, AF path only or both paths. Decided: AF only. It keeps the
   no-AF groups' gain, where all 10 added faces were real, and needs no
   second measurement. A band on both paths is the simpler rule (one rule,
   as the gate has) but may drop real members of posed Leica groups; it
   would need one more single run on `D:\photos\2026\2026-02-01`.
4. Step 3, a size band on a frame with no judged face. Decided: no band
   applies (candidates stay as today; only the CLI sees such frames, at
   gate 0), since it changes nothing the item did not ask for. The
   alternative, no candidate taken, would be uniform with Step 2's Option A
   but changes the AF path's CLI answer on frames whose AF point is on no
   face.
5. Steps 1 and 3 need a local release build of `handpic-cli` and single CLI
   runs on a shared host. That is the CLI crate only, not the root
   workspace build that has run this host out of memory, but it is a local
   cargo build all the same; the 300-file run takes minutes on one thread.
   The scratch dump and subcommand are never committed.
6. Step 1 changes no score, so it bumps no version. If the real-file check
   shows any differing bit pattern, stop: the change is wrong, not a
   candidate for a `FACES_VERSION` bump.
7. Step 3 changes counts users may have seen (`Faces judged`, `Anyone
   soft`) only on frames where a dropped candidate was counted, all under
   the default gate of 5 in the measured set.

## Progress
