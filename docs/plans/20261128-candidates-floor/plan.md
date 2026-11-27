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

# Pass 3 judges faces from 0.6, like the gate

## Purpose

The gate count (`allfaces::gate_count`, the meta pane's `Similar faces`)
counts faces from `allfaces::GATE_LOW_RES_CONFIDENCE` (0.6) on both of its
searches since
[the gate-floor-no-af plan](../_archived/20261126-gate-floor-no-af/plan.md),
but pass 3's candidates (`allfaces::candidates`) are still taken from
`sharpness::FACE_CONFIDENCE` (0.8). So `Similar faces` exceeds
`Faces judged` (`2026-02-01/L1005161.DNG`: 7 against 4; three of its faces
score 0.64 to 0.75), which reads to the user as a count mismatch.

Decided (not open for this plan): lower the candidates' floor to
`GATE_LOW_RES_CONFIDENCE` and bump `ALLFACES_VERSION` so pass 3 re-runs.
The decision is conditional on a measurement made before anything is
committed: how much pass 3's time grows, and how many false closed-eyes
readings (read closed, actually open) the added low-score faces bring. If
the false readings rise notably, the direction switches: the floor stays at
0.8, the reason is recorded in the docs and the todo.md item is rewritten.

Either way this closes the todo.md section "Core: pass 3 judges faces from
0.8 while the gate now counts from 0.6 (gate-floor-no-af)" (or leaves only
the remaining work in it).

## Steps

- [x] Step 1: Measure the 0.6 floor for pass 3's candidates, then land it (or record why not)
  - Done when:
    - Before anything is committed, the before / after measurement below is
      done once and recorded in `measurement.md` in this plan folder (the
      precedent is
      [the gate-floor-no-af measurement](../_archived/20261126-gate-floor-no-af/measurement.md)),
      and the stop condition was checked against it.
    - `measurement.md` holds, per measured set: the frames, the frames at
      the default gate (gate count 5 or more), the faces judged before and
      after, the added faces, the added faces read closed, how many of
      those are actually open (false closed), actually closed, or not
      decidable from the crop, the added non-faces (false detections that
      were counted), the change of the closed and soft counts per frame
      (frames whose closed count goes from 0 to 1 or more, split true /
      false), the per-file `allfaces` time before and after, and
      `L1005161.DNG` / `L1005172.DNG` / `L1005173.DNG` row by row (gate
      count, judged before, judged after).
    - **If the stop condition is not met (the floor is lowered):**
      - `allfaces::candidates` takes faces scoring at least
        `GATE_LOW_RES_CONFIDENCE`; a unit test shows the floor (a face at
        `GATE_LOW_RES_CONFIDENCE` and one between 0.6 and 0.8 are
        candidates, one at `GATE_LOW_RES_CONFIDENCE.next_down()` is not).
      - `ALLFACES_VERSION` in `crates/engine/src/index.rs` is 2, its doc
        comment saying what version 2 changed, in the style of
        `FACES_VERSION`'s per-version sentences. `FACES_VERSION` is not
        bumped.
      - Every passage stating the candidates' floor says 0.6: the code doc
        comments and the documents listed in the implementation approach,
        each English / Japanese pair changed together.
      - The todo.md section is deleted.
    - **If the stop condition is met (the floor stays):**
      - No behaviour change and no `ALLFACES_VERSION` bump are committed.
      - The reason and the numbers are recorded in
        `docs/agents/shared-crates.md` (a new Measured entry next to
        "Matching the judged face against the other faces", with a
        `Source:` line naming this plan's `measurement.md` at its archived
        path in backticks) and in one sentence of `candidates`' doc
        comment in `crates/core/src/allfaces.rs`.
      - The todo.md section is rewritten to only what is left (the
        mismatch stays; what would have to change for the floor to come
        down), no longer saying "Decide whether".
    - No other todo.md section is touched, in particular not "Core: the
      judged face can be counted twice in the all-faces counts and the gate
      count (all-faces-check)", which another session handles.
    - The PR body carries the rationale given for this plan and the
      measurement's numbers, in both outcomes.
    - `learnings.md` holds what was learned.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - **The code change is one line plus its fallout**, all in
      `crates/core/src/allfaces.rs`: `candidates`' filter
      `f.score >= FACE_CONFIDENCE` becomes
      `f.score >= GATE_LOW_RES_CONFIDENCE`. Its only caller is
      `scan::other_faces` (`crates/core/src/scan.rs`), reached only from
      `extract_face_counts_unless`; pass 2 does not call it, which is why
      only `ALLFACES_VERSION` moves. No new detection runs:
      `faces::detect_whole_upright` at `faces::WHOLE_INPUT` already returns
      every face from `faces::CATCH_CONFIDENCE` = 0.6, so the added cost is
      one `eyes::mesh_of` per added face and its two eye crops in the one
      full-JPEG pass.
    - Fallout in the same file: after the change `FACE_CONFIDENCE` is used
      only by the tests there (the `gate_count` tests), so move its `use`
      into the `tests` module to keep the build warning-free. Update the
      test `a_candidate_is_confident_large_enough_and_not_the_judged_face`
      (`shy` at `GATE_LOW_RES_CONFIDENCE.next_down()`, `floor` at
      `GATE_LOW_RES_CONFIDENCE`, plus a face between 0.6 and 0.8). Update
      the module doc ("A candidate scores at least
      `sharpness::FACE_CONFIDENCE`"), `candidates`' doc comment,
      `GATE_LOW_RES_CONFIDENCE`'s comment (it is now also the candidates'
      floor; keep the name, as the gate-floor-no-af plan decided), and
      `MAX_FACES`' comment ("No frame of the measured folders had more than
      4 candidates"), which the measurement must re-check: state the new
      maximum, and how many frames hit the cap.
    - Leave on `FACE_CONFIDENCE`: the cue's choice of the judged face
      (`eyes::judged_face`), the sharpness score's fallback faces, and
      everything in `crates/cli`.
    - `ALLFACES_VERSION` (`crates/engine/src/index.rs`, currently 1): the
      engine's tests compare against the constant, not a literal, so they
      need no change. `allfaces_todo` picks every row at the gate whose
      `allfaces_extractor` is not the version, so pass 3 re-runs on those
      rows on the next open of each folder; say so in the PR body.
    - **Order of work, the measurement first** (nothing committed until the
      stop condition is checked):
      1. Build the release CLI from the branch point:
         `cargo build --release -p handpic-cli` (only this package; never
         the full workspace). Copy the binary to a scratch folder outside
         the repository as the "before" build. The measurement needs local
         builds; the folders are on the user's machine and cannot move to
         CI.
      2. Make the one-line change in the working tree, rebuild, keep the
         "after" binary.
      3. Sets: `D:/photos/2026/2026-02-01` (146 M11-P DNG, no AF point; it
         holds `L1005161` / `L1005172` / `L1005173.DNG`) and the first 300
         ARW by name of `D:/photos/2026/2026-11-07` (α7 V, a team sport,
         trusted AF point, 37 frames at gate count 5 or more in the
         archived measurement). The second set matters because the two
         paths differ: on an AF frame the gate searches at
         `faces::GATE_INPUT` (320x224) while pass 3 searches at
         `WHOLE_INPUT` (640x448), where faces score higher, so the 0.6
         floor can add spectators and background faces there. If a folder
         is missing, say which in `measurement.md` and the report, and
         measure the rest.
      4. Run each of these **once** per set and build, output redirected to
         the scratch folder: `handpic-cli allfaces <files> 1` (one thread,
         as `docs/humans/performance.md`'s all-faces section measures it;
         about 0.1 to 0.2 s per file). For the gate counts, one
         `handpic-cli features <files> <few threads>` run per set with
         either build (the gate column does not depend on this change).
         No repeated runs, no loops, no high thread counts: other sessions
         share the host (`CLAUDE.md`, Development). The photos are only
         read; every output goes to the scratch folder.
      5. Compare with a script (`mise x -- python` on the Windows host):
         per file the judged / closed / soft deltas and the ms delta; the
         totals over all frames and over the frames at gate count 5 or
         more (the frames pass 3 actually runs on by default). A frame
         whose `judged` went down is expected only where `MAX_FACES`
         truncation dropped a 0.8 face for a larger low-score one; list
         such frames, they are a finding. Because each run is single, also
         give the time growth as a count: added meshes per gate frame
         (noise-free), next to the measured ms.
      6. Identify the added faces one by one. `allfaces` prints only
         counts, and `facechoice`'s dump has each whole-image face's box,
         score, EAR and eye offset but not the mesh presence
         `allfaces::measured` gates on, nor the cue. Decide between, and
         keep it minimal: (a) an uncommitted scratch change to the CLI that
         prints, per file, each candidate between 0.6 and 0.8 that
         `measured` accepts (box, score, EAR, closed by
         `eyes::EYES_CLOSED_EAR`) and writes an upright crop of it to the
         scratch folder (the precedent for an uncommitted scratch
         subcommand is the stage profile in
         `docs/humans/performance.md`); or (b) a small committed addition
         to an existing dump. Existing helpers that may be enough for part
         of it: `handpic-cli eyecrops <files> <out-dir>` writes the face
         and eye crops of every face of the scan's detection, which on a
         frame without an AF point is the whole-image search (so it covers
         `2026-02-01`, not the AF set's whole-image faces), and
         `handpic-cli faces <file> <out.png>` draws the boxes and scores on
         the upright preview. Prefer (a) unless the dump is worth keeping.
      7. Look at the crop of every added face read closed and class it:
         actually closed, actually open (false closed), not decidable
         (too small, turned away, covered), or not a face. Sample the
         added faces read open too if they are few enough to look at, to
         count non-faces. Record the file and face of every false closed.
      8. Stop condition: the direction switches when the false closed
         readings are notable, taken as half or more of the added faces
         misjudged (see "Trade-offs and risks" for the readings of this
         rule; record every number so either reading can be applied).
         If met: revert the working-tree change and follow the "floor
         stays" branch of "Done when".
    - Documents stating the candidates' floor today (lowered direction;
      found by search, re-search during implementation):
      - `CLAUDE.md`, the `src/allfaces.rs` passage: "`candidates`: the
        other whole-image faces at `FACE_CONFIDENCE` and `EYES_MIN_FACE` or
        more".
      - `README.md` ("faces of 60 px or more and a detection score of 0.8
        or more, at most eight") and the same sentence in `README.ja.md`.
      - `docs/humans/usage.md`, two places ("a face the detector scores
        under 0.8", and "a detection score of 0.8 or more, at most eight,
        whose face mesh fits") and the same two in
        `docs/humans/usage.ja.md`.
      - `docs/humans/performance.md`, the "All-faces check (Windows 11)"
        section: add the measured cost and false-closed numbers of the
        lower floor next to the gate floor's paragraph, and the same in
        `docs/humans/performance.ja.md`. Its timing tables are dated
        measurements; add to them, do not rewrite them.
      - No guide under `docs/agents/` states the candidates' floor
        (searched: `shared-crates.md` names `candidates` only for
        `same_face`; `app.md` and `tract-onnx-inference.md` do not mention
        it). Add a short Measured entry to `docs/agents/shared-crates.md`
        only if the measurement taught something an agent must know before
        touching the floor again (in the "floor stays" branch it is
        required).
    - No app code changes: the meta pane's rows, the strip's closed-eyes
      mark and the `Anyone closed` / `Anyone soft` filter items read the
      stored counts through `group::shown`.
    - If the app is launched for a look at the result, use dedicated
      `--index`, `--settings` and `--log` paths and `--mcp-port 47891`.

## Trade-offs and risks

- **How the stop rule is read.** The given rule is "half or more of the
  added faces misjudged". Read literally (false closed over all added
  faces) it is hard to reach, since most faces are open and read open.
  Two stricter readings exist: the precision of the closed readings among
  the added faces (false closed over added faces read closed), and the
  frames that newly get the closed-eyes mark falsely against those that
  get it rightly. The plan applies the given rule and has the implementer
  record all three, so the caller can switch on a stricter reading from
  the numbers without a re-run. Consequence of the literal reading: the
  floor can land with most of its new closed marks false.
- **`Faces judged` will still not equal `Similar faces`.** The two count
  different things even on one floor: the gate keeps faces within
  `GATE_MIN_SIZE` to `GATE_MAX_SIZE` of the judged face's size, on an AF
  frame from the 320x224 search; pass 3 takes every face of 60 px or more
  from the 640x448 search, at most `MAX_FACES`, and counts only those
  whose mesh is trusted and fits. After the change `Faces judged` can be
  below the gate count (a face under 60 px or with an untrusted mesh) or
  above it (faces outside the size band). The change removes the 0.8 / 0.6
  gap, not every difference; the PR body and the docs should not promise
  equality. `L1005161.DNG` reaching 7 is expected, not guaranteed: the
  measurement shows it.
- **Spectators on AF frames.** Pass 3 has no size band, so at 0.6 a
  team-sport frame can gain background faces of 60 px or more. They add
  to the closed and soft counts (a background face is often soft), so
  `Anyone soft` and the closed-eyes mark can fire on people outside the
  group. The second measured set is there to show this. A size band on the
  candidates would be a different change, out of scope.
- **The `MAX_FACES` cap.** Candidates are sorted largest first and cut at
  8 with the judged face, so a large low-score face can now push out a
  smaller 0.8 face. The measurement lists frames whose judged count fell
  and frames at the cap.
- **Timing from single runs.** Each measurement runs once (no load on the
  shared host), so the ms delta carries run-to-run noise;
  `docs/humans/performance.md` took the fastest of three. The added mesh
  count per gate frame is recorded beside it as the noise-free figure. A
  caller who wants a tighter time figure has to allow repeated runs.
- **Pass 3 re-runs everywhere.** The `ALLFACES_VERSION` bump re-runs pass
  3 on every row at the gate of every indexed folder on its next open;
  the counts stored at version 1 are replaced as it goes. This is the
  decided consequence.
- **Measurement tooling: scratch or committed.** An uncommitted scratch
  dump keeps the PR to the floor change but leaves the per-face numbers
  unreproducible from the repository; a committed dump option makes the PR
  larger. The plan prefers the scratch one.
- **Docs in the "floor stays" outcome.** The human docs then stay as they
  are (they already say 0.8); only the agents' guide, the code comment
  and todo.md change.

## Progress
