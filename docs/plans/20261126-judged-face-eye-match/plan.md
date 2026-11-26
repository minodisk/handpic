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

# Match the judged face by its overlap over the smaller box

The folder and the branch are named `judged-face-eye-match` after the first
approach, a match by eye landmarks. That approach was measured and rejected;
the names stay, the rule this plan ships is a different one (see Purpose).

## Purpose

On a frame with an AF point the cue's judged face comes from the crop
detection around the AF point, and the other faces from a whole-preview
detection. `allfaces::gate_count` and `allfaces::candidates` drop the
whole-preview face that is the judged face found again by an IoU above
`faces::NMS_THRESHOLD` (0.3). When the two boxes of the same face overlap at
or under 0.3, that face is counted twice, in the gate count and in the judged
count alike (1 of 258 AF frames on `2026-09-19` at 640x448, see
[the all-faces-check measurement](../_archived/20261123-all-faces-check/measurement.md)).

The first attempt matched the two detections by their eye landmarks. It was
measured and rejected: the same face's eye landmarks move by up to 0.43 of the
box side between the crop detection and the 640x448 search and up to 0.69 on
the 320x224 gate search, more than a face's own eye distance, and the twins
the IoU rule misses sit at the same distance as a real neighbour. The record
is [measurement.md](measurement.md) and [learnings.md](learnings.md) in this
folder; both stay as they are.

In every missed twin seen there, one box lies mostly inside the other (a
smaller partial box, or a profile boxed tighter by one detection). IoU
punishes that size difference; the intersection over the smaller box's area
(intersection / min(area a, area b)) does not. Known data points: the
double-counted twin `2026-09-19/_DSC1899` is at about 0.59, the real
neighbour `2026-11-07/_DSC1517` at about 0.36. This plan replaces the IoU
match with that overlap under a measured cut, and closes the `todo.md` item
"Core: the judged face can be counted twice in the all-faces counts and the
gate count (all-faces-check)". Background:
[the all-faces-check plan](../_archived/20261123-all-faces-check/plan.md).

## Steps

- [x] Step 1: Match the judged face against the whole-preview faces by the
      intersection over the smaller box in `gate_count` and `candidates`
  - Done when:
    - The overlap (intersection / min of the two box areas) between the
      judged face and every whole-preview face is measured on
      `D:\Photos\2026\2026-09-19` and `D:\Photos\2026\2026-11-07`, at both
      `faces::WHOLE_INPUT` (640x448, what `candidates` sees) and
      `faces::GATE_INPUT` (320x224, what `gate_count` sees on an AF frame),
      and recorded in a new file in this folder (`overlap-measurement.md`;
      `measurement.md` stays the record of the rejected approach): the
      distribution on the pairs the IoU rule matched, the value of every
      pair between the lowest twin and the highest neighbour, the chosen cut,
      and for each candidate cut the number of frames whose gate count or
      candidate list changes, in either direction, against today's rule.
    - Every frame whose gate count or candidate list changes at the chosen
      cut was looked at by eye (the two boxes drawn on the preview) and is
      listed in that file with what it is: the same face, a neighbour, a
      spurious box.
    - **If no cut separates the twins from the neighbours, the step stops
      there**: no code change is shipped, the numbers go into this folder and
      `learnings.md`, and the result is reported back to the caller, as the
      first attempt did.
    - **Decided after the measurement (2026-11-27, the user's choice):** no
      cut separates twins (0.306 to 0.740) from different people (0.301 to
      0.532), so the rule ships as the union "IoU above `NMS_THRESHOLD` or
      overlap above 0.55", a partial fix (11 gate frames on `2026-11-07`
      lowered, no different person dropped; about 20 twins between 0.3 and
      0.532 stay double-counted). This replaces the replace-only bullet
      below and the todo deletion: the `todo.md` item is reworded to the
      remaining cases with the measurement, not deleted.
    - `allfaces::gate_count` and `allfaces::candidates` decide "this
      whole-preview face is the judged face found again" by that union, the
      cut held in one named constant whose doc comment names the
      measurement.
    - A test in `crates/core/src/allfaces.rs` builds a judged face and a
      whole-preview box of the same face whose IoU with it is at or under
      `NMS_THRESHOLD`, asserts that IoU, and asserts that `gate_count` gives
      1 and `candidates` leaves the face out. A neighbouring face whose
      overlap is under the cut is still counted and kept (the same test, or
      the existing ones).
    - `FACES_VERSION` in `crates/engine/src/index.rs` is bumped, with a line
      for the new value in its doc comment. `ALLFACES_VERSION` is not bumped
      (see Trade-offs and risks).
    - The `todo.md` item "Core: the judged face can be counted twice in the
      all-faces counts and the gate count (all-faces-check)" is deleted, the
      whole section with its TODO list.
    - The doc comments of `allfaces.rs` (the module doc's "does not overlap
      the judged face (an IoU above `faces::NMS_THRESHOLD`)", `gate_count`'s
      and `candidates`' own) and `CLAUDE.md`'s description of `candidates`
      ("not overlapping the judged face") describe the new match.
    - The "Deferred issues" entry of `learnings.md` in this folder is marked
      as taken up by this step, so the wrap-up does not file it in `todo.md`
      again.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/core/src/allfaces.rs`, `crates/engine/src/index.rs`
      (`FACES_VERSION`, its doc comment), `todo.md` (repository root),
      `CLAUDE.md`, this folder. No human-facing doc (`README.md`,
      `docs/humans/usage.md`) describes the IoU match, so none of those or
      their `.ja.md` pairs change.
    - Both functions hold the same one-line filter today
      (`iou(f, judged) <= NMS_THRESHOLD`, lines 107 and 123). Replace that
      one line in each with a call to one private predicate in `allfaces.rs`
      over two `Face`s, and touch no other line of `gate_count`:
      `docs/plans/20261126-gate-floor-no-af` (in flight, another session,
      `dlg-noaf-floor`) changes `gate_count`'s score floor on the no-AF path
      and `FACES_VERSION` at the same time. The signatures of `gate_count`
      and `candidates` stay, so `crates/core/src/scan.rs` does not change.
      `faces::iou` and `faces.rs` stay as they are (`nms` uses `iou`).
    - Measure first, then write the code. The scratch CLI of the first
      attempt is not committed and sits in the session scratchpad
      (`C:\Users\daisu\AppData\Local\Temp\claude\492ecb14e79bda8e\ac79541b-6f26-4af3-b811-61510b298b05\scratchpad\`:
      `eyematch.rs.txt`, the subcommand; `eyematch-main.diff`, its wiring
      into `crates/cli/src/main.rs`; `slice.txt`, the 300-file slice of
      `2026-11-07`; `em.tsv` / `em2.tsv`, its output). Its TSVs hold the IoU
      and the eye distances but no box coordinates, so the new metric cannot
      be derived from them: add the overlap as a column and run it again. Do
      not commit the subcommand. A scratchpad can be cleared; if it is gone,
      rebuild the subcommand from `measurement.md`'s description (the judged
      face from `candidate::focus_cue`, against every face of the
      whole-preview search at both inputs).
    - The first measurement took the first 300 ARW of `2026-11-07`. Measure
      the whole folder if the time allows, and say in the record which it
      was.
    - Frames to check by name: `2026-09-19/_DSC1899` (twin, both inputs),
      `2026-11-07/_DSC1543` (twin, 640x448), `2026-11-07/_DSC1524` (twin, a
      smaller partial box, 320x224), `2026-11-07/_DSC1517` (a different
      person partly behind the judged one, 320x224) and `2026-09-19/_DSC3290`
      (a spurious box half on the judged head's hair, 320x224; dropping it
      would be an improvement, not a miss).
    - The cut is the one thing this step decides. What is known: the overlap
      is never below the IoU of the same pair, so a cut at or under 0.3 could
      only drop more faces than today; the two known points (0.36 neighbour,
      0.59 twin) put the cut above 0.3, and then pairs of similar-sized boxes
      with an IoU a little above 0.3 fall under it (two equal boxes at IoU
      0.31 overlap by 0.47) and are counted where they were dropped. Measure
      how many of the IoU-matched pairs fall under each candidate cut; they
      are the frames that would gain a double count (see Trade-offs and
      risks).
    - Frames without an AF point can change too, unlike under the first
      approach: there the judged face is one of the whole-image faces, NMS
      keeps the others at or under 0.3 IoU with it, but a small box mostly
      inside the judged one passes NMS and exceeds the cut. Such a face was
      counted and would now be dropped. `gate_count`'s size band keeps boxes
      under half the judged long side out already; `candidates` has no band.
      The two measured folders are AF folders; if a no-AF folder is reachable
      (the all-faces-check measurement used `2026-02-01`, Leica DNG), count
      the frames that change there as well, and say in the record when none
      was.
    - Tests: with the existing `face(x, side, score)` helper, a box twice the
      side centered on the judged one has an IoU of 0.25, an overlap of 1.0
      and a long side still inside the gate's band (`GATE_MAX_SIZE` is
      inclusive), so under the old rule it counted 2. The existing `near`
      fixture (`face(680.0, 100.0, ..)` against `face(600.0, 100.0, ..)`)
      overlaps by 0.2 and stays a neighbour under any cut above that. The
      existing tests assert `iou(..)` against `NMS_THRESHOLD` to document
      their fixtures; once the production code no longer uses `iou` and
      `NMS_THRESHOLD`, move that import into the test module and keep the new
      test's IoU assertion, which is what shows the old rule missed it.
    - `FACES_VERSION` is 14 on `main` (checked at 739a17ee). If the
      gate-floor change merges first, rebase and take the next number,
      keeping both lines of the doc comment's history.

## Trade-offs and risks

- **Replace the IoU match, or add the overlap to it.** The user chose to
  replace. With a cut above 0.3 that is not a pure widening: similar-sized
  boxes with an IoU just above 0.3 (an overlap of about 0.46 to 0.5) would no
  longer match and would be counted twice. The alternative, "the same face
  when the IoU is above 0.3 or the overlap is above the cut", can only lower
  counts. Recommended: replace when the measurement shows no IoU-matched pair
  under the chosen cut, or only pairs that are in fact two people; fall back
  to the union when it shows same-face pairs there. The decision is the
  caller's once the numbers are in.
- **The margin is two data points wide so far.** One twin at 0.59 and one
  neighbour at 0.36 are known; the other twins (`_DSC1543`, `_DSC1524`) and
  the 320x224 values are unmeasured. If a twin falls under a neighbour, no
  cut exists and the step stops without shipping, leaving the `todo.md` item
  open (to be reworded with what was learned, at the caller's call).
- **A face in front of or behind the judged one.** A smaller face whose box
  lies mostly inside the judged box (a person standing behind, a child held
  in front) has a high overlap and is dropped as the judged face. On an AF
  frame the IoU rule dropped it only above 0.3; on a no-AF frame NMS let it
  through. `_DSC1517` at 0.36 is this kind of frame and the reason the cut
  cannot go low. Whether such faces exist above the cut on the measured
  folders is part of what the by-eye check answers.
- **`FACES_VERSION` collides with the gate-floor change.** Both go from 14
  to 15 on `main`. The second to merge rebases to 16. Two bumps mean two
  pass 2 re-runs for a user who updates between them; merging the two close
  together, before a release, keeps it to back-to-back re-runs.
- **The bump re-runs pass 2 everywhere.** Every indexed folder is analyzed
  again on its next open, for a change that moves a handful of frames in
  about 580 measured. That is the cost of keeping stored gate counts honest;
  the alternative, leaving old rows as they are, would let the stored and
  the recomputed gate count of the same file disagree.
- **`ALLFACES_VERSION` is left alone.** Its doc comment asks for a bump on
  any change to the counts, and the counts do change. But the `FACES_VERSION`
  bump re-runs pass 2 on every row, and pass 2's write clears the row's
  counts and sets `allfaces_extractor` to 0, so pass 3 re-runs on every row
  at the gate either way. Bumping both would be harmless and redundant.
- **The names no longer describe the work.** The folder and the branch stay
  `judged-face-eye-match`; renaming would break the auto memory entry and the
  links from `learnings.md`. The title and Purpose say what ships.

## Progress

- (2026-11-27) First approach (eye landmarks) measured and rejected, no code
  changed; see `measurement.md` and `learnings.md`. Plan rewritten for the
  overlap over the smaller box.
- (2026-11-27) Overlap measured (`overlap-measurement.md`): no separating
  cut. The user chose the partial fix, the union at 0.55, with the `todo.md`
  item reworded rather than deleted.
