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

# Judged-face twins: box shape and size

## Purpose

`allfaces::same_face` drops the judged face found again in the whole-preview
search by an IoU above `NMS_THRESHOLD` or an overlap over the smaller box
above `SAME_FACE_OVERLAP` (0.55). A second box of the same face (on the ear,
larger, partial) overlapping by about 0.3 to 0.55 is still counted twice,
mostly in the 320x224 gate count. Neither the eye landmarks nor the overlap
separate those twins from different people (see
[measurement.md](../_archived/20261126-judged-face-eye-match/measurement.md)
and
[overlap-measurement.md](../_archived/20261126-judged-face-eye-match/overlap-measurement.md)).

This plan closes the `todo.md` section "Core: the judged face can be counted
twice in the all-faces counts and the gate count (all-faces-check)" one way
or the other. It measures exactly one cheap candidate, the box's shape and
size, and then either adds it to `same_face` or records that the double count
is accepted. The user decided this scope; mesh-based and full-size-image
rules are out of scope.

## Steps

- [x] Step 1: Measure the box shape and size of the remaining twins against
      the different-person pairs, then either extend `same_face` or accept
      the double count, and delete the `todo.md` section
  - Done when:
    - `measurement.md` in this plan folder records, for every counted pair
      of `overlap-measurement.md` with IoU at or under 0.3 and overlap from
      0.3 to 0.55 (the pairs `same_face` misses today): the frame, the search
      (640x448 or 320x224), the overlap, the label taken from that table
      (same face / different person / spurious / unclear), both boxes' width
      and height, the aspect ratio of each, the aspect ratio between them
      (larger / smaller), and the long-side ratio (larger / smaller). It also
      records the method, the number of files read, and the verdict with the
      threshold tried and what it catches and drops.
    - The verdict follows the rule fixed for this plan: a threshold on the
      aspect ratio, the long-side ratio, or either of the two, that matches
      at least 80% of the same-face pairs and no different-person pair, is
      adopted (outcome A); otherwise the double count is accepted
      (outcome B). Spurious and unclear pairs are outside the denominator.
    - Outcome A, the rule is adopted:
      - `same_face` in `crates/core/src/allfaces.rs` also matches a pair by
        the measured threshold, as a union with the two existing rules, only
        for pairs overlapping by at least the lowest overlap the measurement
        labelled (0.3 or higher). The threshold is a named `pub const` with a
        doc comment giving the measured numbers and naming this plan's
        `measurement.md`.
      - A unit test next to
        `a_box_mostly_over_the_judged_face_is_the_judged_face_whatever_its_iou`
        covers a pair the new rule matches and a pair at the same overlap it
        does not.
      - `FACES_VERSION` in `crates/engine/src/index.rs` is bumped with a line
        in its doc comment when any 320x224 or no-AF gate pair changes;
        `ALLFACES_VERSION` is bumped instead when only 640x448 candidate
        pairs change.
      - `docs/agents/shared-crates.md`'s section on matching the judged face
        states the new rule, its numbers and what it still leaves
        double-counted. The `same_face` clause in `CLAUDE.md`'s Layout
        paragraph and the module doc of `allfaces.rs` name the third
        condition.
    - Outcome B, the double count is accepted:
      - No code changes.
      - `docs/agents/shared-crates.md`'s section on matching the judged face
        records that box shape and size do not separate the twins either,
        with the measured numbers, and that the remaining double count
        (overlap about 0.3 to 0.55) is accepted. Its "What is left" bullet no
        longer names an open `todo.md` item.
    - Either outcome: the `todo.md` section "Core: the judged face can be
      counted twice in the all-faces counts and the gate count
      (all-faces-check)" is deleted, and no other `todo.md` section is
      touched.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Data. The pairs are the rows of
      [overlap-measurement.md](../_archived/20261126-judged-face-eye-match/overlap-measurement.md)'s
      table "Every counted pair at or under IoU 0.3 with an overlap of 0.3 or
      more" whose overlap is at or under 0.55: by a count of that table,
      24 same-face pairs, 10 different-person pairs, 3 spurious boxes and
      2 unclear ones, on about 37 files of `D:/photos/2026/2026-11-07`
      (one of `2026-09-19`). Recount them from the measurement; with 24
      twins, 80% is at least 20. The photos are only read, never written.
    - That table lists every counted pair of its region over the three
      measured folders, so a rule confined to overlap 0.3 or more is fully
      evaluated by the named files. Do not re-run the whole folders. Nothing
      under overlap 0.3 was labelled, which is why the new rule keeps an
      overlap floor.
    - Run it once, over the named files only, on a few threads. No repeated
      or whole-folder runs (shared host).
    - Tool. The earlier measurements used a scratch `handpic-cli` subcommand
      that was not committed. Do the same: a scratch subcommand that takes
      the judged face as pass 2 does (`candidate::focus_cue` on an AF frame)
      and compares it with the faces of `faces::decode_whole` +
      `faces::detect_whole_upright` and of `faces::decode_gate` +
      `faces::detect_gate_upright`, printing per pair the IoU, the overlap,
      the score, both boxes' `width` and `height`, and
      `candidate::meshes_face`. `raw_paths` in `crates/cli/src/main.rs`
      accepts single files. Match each printed pair to its label by frame,
      search and overlap value. An earlier scratch version may still exist at
      `C:\Users\daisu\AppData\Local\Temp\claude\492ecb14e79bda8e\ac79541b-6f26-4af3-b811-61510b298b05\scratchpad\overlapm.rs.txt`
      (with `overlapm-main.diff` beside it); it lacks only the box `w` / `h`.
      Keep the scratch code out of the commit.
    - Only pairs a count takes matter: at 320x224 a score of at least
      `GATE_LOW_RES_CONFIDENCE` and a long side within `GATE_MIN_SIZE` to
      `GATE_MAX_SIZE` (0.5 to 2.0) of the judged face's; at 640x448 a score
      of at least `FACE_CONFIDENCE` and `meshes_face`. The size band caps a
      gate pair's long-side ratio at 2, so check whether the size has any
      room to separate before relying on it.
    - Both boxes are in the same stored coordinates, so the aspect ratio
      needs no orientation handling.
    - Outcome A keeps the edit inside `same_face` and its constants and
      test. Another session is editing `candidates` in the same file for
      the `todo.md` section "Core: pass 3 judges faces from 0.8 while the
      gate now counts from 0.6"; do not touch `candidates`, `gate_count`'s
      body or that section.
    - The union can only lower counts. Never replace the IoU rule (see the
      "Replace against union" bullet in `docs/agents/shared-crates.md`).
    - A row pass 2 rewrites returns to `allfaces_extractor` 0, so a
      `FACES_VERSION` bump re-runs pass 3 without an `ALLFACES_VERSION`
      bump, as at 14 to 15. Grep for tests or docs pinning the version
      number before bumping.
    - `docs/humans/**` and `README.md` do not describe `same_face`; no
      translation pair needs an update.

## Trade-offs and risks

- Scratch subcommand or a committed CLI addition. The plan uses a scratch
  subcommand, as both earlier measurements did, so the PR carries no
  measurement code.
- The spurious and unclear pairs are scored on neither side: a spurious box
  dropped is harmless, an unclear pair counts for neither side (approved by
  the user).
- A small labelled set. The different-person side is 10 pairs from one
  folder of one sport. `measurement.md` gives the margin between the
  threshold and the nearest different-person pair; outcome B is the safe
  side when the margin is thin.
- The cost of a `FACES_VERSION` bump. Outcome A re-runs pass 2 on every
  indexed folder of every user after the update, for a correction of roughly
  20 gate counts in the measured data.
- A concurrent edit. The other session may change `candidates` and bump
  `ALLFACES_VERSION` in the same files. Whichever PR merges second rebases;
  if both bump the same constant, the second takes the next number and keeps
  both doc lines.
- A local build. The measurement needs one release build of `handpic-cli`
  on the host, although builds are otherwise left to the PR's checks
  (approved by the user).

## Progress

