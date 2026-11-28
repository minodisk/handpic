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

# Culling cues: the cue readings against the user's own culling

## Purpose

The cue-labels plan ([plan](../20261128-cue-labels/plan.md),
[pre-registration](../20261128-cue-labels/cues.md)) prepared a
900-frame blind "OK to discard?" round to give the cue readings (head pose,
closed eyes and the rest) more power. The user decided on 2026-11-28 not to
label it and to read the cues against their real culling instead: eight
folders of the 2026-10-12 dump lost frames since, and what is left there is
what the user kept.

This plan reads Handpic's cue readings of the dump against that label
(kept = still on disk, culled = gone), with bars fixed here before any
reading is computed, writes the Decision, and replaces the `todo.md` item
that waits on the user's labels. **No app code changes, no builds, the
library is only listed.**

The user approved this plan on 2026-11-28 with the seven culled folders as
the primary population (the defaults under "Trade-offs and risks").

### What is known (investigated 2026-11-28, read-only, no reading computed)

Only label-free bin counts, per-folder kept and pick counts and file times
were looked at. No bin was crossed with the label.

- **The dump.** `D:\Photos\tests\2026-10-12-rank-app\features\<folder>.tsv`,
  27 folders, 32,459 frames; columns `folder`, `file`, `capture_time`,
  `subsec`, `xmp`, `dop`, `sharpness`, `state`, `eye_focus`, `ear`,
  `eyes_open`, `yaw`, `pitch`, `roll`, `eye_offset`, `edge_gap`, a missing
  value `-`. `pitch` is positive when the face tilts up. The `xmp` and
  `dop` columns hold `-`, `Pick`, `Reject` or `None`.
- **The culled folders.** Seven folders lost some frames and kept some:

  | Folder | Dump frames | On disk | Dump picks | `|pitch|` > 45 |
  | --- | ---: | ---: | ---: | ---: |
  | `2026-07-22` | 189 | 32 | 32 | 18 |
  | `2026-07-26` | 64 | 7 | 7 | 1 |
  | `2026-07-30` | 936 | 105 | 105 | 34 |
  | `2026-08-01` | 3,401 | 463 | 463 | 84 |
  | `2026-08-22` | 2,677 | 267 | 267 | 41 |
  | `2026-09-13-b` | 1,795 | 232 | 232 | 13 |
  | `2026-09-19` | 2,134 | 282 | 287 | 51 |
  | all seven | 11,196 | 1,388 | 1,393 | 242 |

  The kept share is 12.4%, the culled share 87.6%.
- **Kept is the dump's pick.** A pick is a frame whose `xmp` or `dop`
  column says `Pick`. All 1,388 frames on disk are dump picks; no frame on
  disk is a non-pick; 5 dump picks are gone, all in `2026-09-19`. The
  culling deleted what was not picked, and the picks existed at the dump.
- **When the picks were made.** The sidecars of the kept frames were last
  written 2026-08-02 to 2026-08-30 in five folders (`.dop` only; one file
  of `2026-08-22` on 2026-09-22) and on 2026-09-26 / 27 in `2026-09-13-b`
  and `2026-09-19` (`.xmp` and `.dop`). The repository's first commit is
  2026-09-16, and the head-pose and closed-eyes plans are dated
  2026-10-07. The folders' own mtimes, probably the deletion, are
  2026-11-06 and 2026-11-07. An mtime is the last write, so this is
  evidence, not proof.
- **`2026-08-29` was not culled.** All 3,451 of its frames are gone; the
  folder holds only `Output\` with 475 JPEGs, and the dump has 475 picks
  for it. Its file names recur in other folders (`_DSCnnnn`), so a name
  search says nothing.
- **The pick label exists beyond the seven folders.** 18 of the 19 intact
  folders have dump picks (`2026-06-05` has none); all 27 folders hold
  4,232 picks and 768 frames with `|pitch|` > 45 (188 up, 580 down).
- **Label-free bin sizes in the seven folders**: `|pitch|` > 45: 242 (16
  up, 226 down); EAR <= 0.137: 937; `|yaw|` > 35: 2,810; `eye_focus` <
  0.9: 3,062; no `eye_focus`: 1,294; no `pitch`: 3,855. `2026-08-29` has
  65 frames with `|pitch|` > 45 (7 up, 58 down).
- **Earlier labels in the seven folders**: 409 strict
  (`D:\Photos\tests\2026-10-17-bad-labels\labels.tsv`), 189 discard-labels
  (`D:\Photos\tests\2026-10-18-discard-labels\labels.tsv`) and 242
  model-search (`D:\Photos\tests\2026-10-20-model-search\labels.tsv`), each
  keyed by `folder` and `file` with a `label` and an `inclusion_prob`
  column.
- **Prior knowledge of rules against picks.** reject-filter-eval
  ([reject.md](../20261016-reject-filter-eval/reject.md), "Rule
  points", 25 folders) read `face away` (`|yaw|` > 35 or `|pitch|` > 45)
  as keeping 75.1% of the picks while removing 23.5% of the non-picks,
  `eyes closed` 92.8% / 11.3%, `eye focus low` 87.1% / 19.8% and
  `sharpness bottom 10%` 93.4% / 10.3%. Pitch on its own was never read
  against the picks. The prior readings on the discard question are in
  cues.md.
- **The bars of cues.md do not carry over.** At a culled rate of 87.6% a
  bin cannot exceed its complement by 15 points (it would need about
  102.5%), and a rescue share of at most 25% is met by any random bin.
- **Today's readings cannot be taken on culled frames**: the files are
  gone. The 900 copies of the cue-labels round are all from intact
  folders, and their `manifest.tsv` carries the dump's `xmp` and `dop`
  columns.

### Pre-registration

Written on 2026-11-28 with the knowledge above and before any bin is
crossed with a label. The commit that adds this file precedes the first
run of the reader that prints a reading.

- **Population**: the dump frames of the folders that have at least one
  dump frame on disk and at least one gone, as the reader finds them
  (expected: the seven above, 11,196 frames). A folder with no dump frame
  on disk is not culled and is left out.
- **Label**: `kept` = the dump frame's file name is in its library folder
  (case-insensitive), `culled` = it is not.
- **Estimates**: a census, unweighted, no `π`. Intervals: a 1,000-draw
  folder-cluster bootstrap (folders drawn with replacement, seed 0, one
  draw matrix for every reading), 2.5% and 97.5% quantiles. A draw whose
  bin is empty or whose complement has no kept frame is dropped and
  counted. Beside it: the per-folder breakdown and the leave-one-folder-out
  range of each ratio.
- **The first number**: the kept share of the population.
- **The primary reading**: the frames with a dump `|pitch|` > 45 against
  all other frames: kept share of each, their ratio (bin over complement),
  and the culled rates with their difference in points.
- **Secondary readings**, each a firing bin against its complement, same
  quantities:
  - on the culled label: `pitch` > 45 and `pitch` < -45 apart; EAR <=
    0.137; `|yaw|` > 35; `eye_focus` < 0.9; no `eye_focus`; the folder's
    bottom sharpness decile (over the folder's dump frames);
  - on the **pick label** (a dump pick against the rest), the same bins:
    `2026-08-29` alone, and every dump folder with at least one pick;
  - the label's own check: kept against the dump's pick, `Reject` and
    `None` values, per folder (not a cue reading);
  - the earlier labels of the population's frames (strict `bad`,
    discard-labels and model-search `discard`) against kept and culled,
    counts and the `1/π` weighted shares, read beside the culling and not
    pooled into it.
- **Bar 1, a cue catches culled frames**: the bin's kept share is at most
  **half** its complement's (ratio <= 0.5) and the folder-cluster 97.5%
  quantile of the ratio is below 1. This replaces the 15 points of
  cues.md, which cannot be met here; it is the rare-event form the strict
  round used (twice the base rate), read on the kept side.
- **Bar 2, a cue is usable sorted last on its own**: bar 1 is met, the
  bin's kept share is at most 25% with the folder-cluster 97.5% quantile
  at most 35%, and the bin holds at most 5% of the population's kept
  frames. The 25% and 35% of cues.md are kept as written and do not
  discriminate at this base rate, which is why bar 1 is required.
- **Multiplicity**: one primary reading; every secondary reading is
  reported with its interval and is not a pass on its own.
- **The follow-up rule**:
  1. The primary reading meets bar 1 → the Decision recommends a later
     plan that reads today's pose cue (`|pitch|` > `GOOD_MAX_PITCH`, and a
     dump `|pitch|` > 45 with no pose today) against the dump's picks on
     files that still exist, the 900 cue-labels copies first, before any
     app change. The `todo.md` section becomes that item.
  2. Otherwise → nothing, and the `todo.md` section is removed.
- **Caveats**: the label is "not picked", stricter than "OK to discard";
  the picks were made before the dump and, by the file times, before the
  pose cue existed; whether the user saw a strip mark while picking the
  two September folders is unknown (the user was asked and did not
  answer); one photographer, seven folders, mostly basketball; the
  readings are the dump's, which today's app no longer makes on every
  frame; seven clusters make the bootstrap coarse.

## Steps

- [x] Step 1: The reader, the readings, the Decision and the `todo.md` section
  - Done when:
    - `culling.py` in this plan folder (`mise x -- python -I -B culling.py
      <app-dump-dir> <strict-dir> <discard-dir> <confirm-dir> <out-dir>`)
      reads the dump's `features\*.tsv` and the three `labels.tsv`, lists
      the library, builds the label, writes `labels.tsv` (`folder`,
      `file`, `kept`, then the dump's columns) into `<out-dir>` and prints
      every table of "Pre-registration" as Markdown on stdout.
    - It opens no library file: it only lists the 27 dump folders
      (recursive path, size, `mtime_ns`). It refuses an `<out-dir>` under
      `D:\Photos\2026`.
    - **Library unchanged**: it writes `library.before.tsv` before and
      `library.after.tsv` after, and reports 0 changed, added or removed
      paths.
    - **Deterministic**: a second run prints byte-identical output; the
      SHA-256 of `labels.tsv` and of the output is in `learnings.md`.
    - **Checks printed and recorded in `learnings.md`**: the culled and
      the left-out folders with their counts against "What is known", any
      difference explained; kept against dump pick per folder; the bin
      sizes against "What is known"; the dropped bootstrap draws per
      reading; the sidecar mtime range of the kept frames and each
      folder's mtime, from the listing.
    - `findings.md` in this plan folder holds the tables, each bar's
      verdict for the primary reading, the caveats, and the **Decision**
      by the follow-up rule, changing no bar. The Decision also says that
      the 900 copies at `D:\Photos\tests\2026-11-28-cue-labels\` are kept
      (not deleted) and that the blind round stays possible, its
      pre-registration untouched.
    - `todo.md`: the section "### Core: read the cue-labels round once the
      user has labeled it" is removed (branch 2) or replaced by the
      follow-up item of branch 1, which names the copies, the manifest's
      pick columns and `findings.md` at its archived path.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Commit this `plan.md` first, before the first run that prints a
      reading, and record that commit's hash in `learnings.md`.
    - **Standard library only** (`csv`, `random`, `os`, `hashlib`, `sys`).
      Re-implement `under`, `listing`, `write_listing`, `changed` and
      `read_tsv` after `sample.py` of the cue-labels plan
      (`../20261128-cue-labels/sample.py`), naming the source in
      a comment. Import nothing from `_archived/` and change nothing
      there.
    - Every data path is an argument; nothing depends on the script's own
      location.
    - A pick is `xmp == "Pick" or dop == "Pick"`; a missing value is `-`.
    - Data goes to `D:\Photos\tests\2026-11-28-culling-cues\`; `labels.tsv`
      and the listings name personal files and stay out of the repository.
    - Run the script from the scratchpad copy with `-I -B` and check `git
      status` for a stray `__pycache__/`. Write Windows paths into files
      from a script file, not a Git Bash heredoc (the cue-labels learnings
      record lost backslashes).
    - The run is one listing of about 44,000 files and arithmetic on
      32,459 rows: one process, no parallelism.
    - `todo.md` is edited by other sessions: rebase onto `origin/main`
      before the push and, on a conflict, keep the other side for
      everything but this section.
    - Single-PR mode archives this folder in the same PR, so `todo.md`
      names files at `docs/plans/_archived/20261128-culling-cues/`.

## Trade-offs and risks

Defaults this plan took, approved by the user on 2026-11-28.

- **The primary population is the seven culled folders.** Because kept is
  the dump's pick, the same label exists in `2026-08-29` and in 18 intact
  folders: 26 folders and about three times the pitch bin, with 26
  clusters instead of 7. Making that the primary reading gives a tighter
  interval, but it is no longer the user's culling, and an intact folder
  may not be picked through. Default: the culling is primary, the pick
  label over all folders is the first secondary reading.
- **Bar 1 at a ratio of 0.5.** The 15 points of cues.md, at its base rate
  near 45%, meant a keep share falling from about 55% to 40%, a ratio of
  about 0.73. A bar of 0.73 is the closer translation and easier to meet;
  0.5 is the strict round's rare-event form and the default here.
- **Bar 2 keeps numbers that do not discriminate here.** The alternative
  is to drop bar 2 from this reading.
- **`2026-08-29` is left out of the primary reading** and read on the pick
  label. Including it in the primary would label all 3,451 frames culled.
- **The reading may add little.** reject-filter-eval already read
  `face away` against the picks without lift. What is new is pitch on its
  own. The alternative is to close the `todo.md` item on the facts of
  "What is known" without a script.
- **The Decision's branch 1 follow-up needs the 900 copies** (39.7 GB).
  Default: keep them.

Risks:

- **Seven clusters.** `2026-08-01` holds 84 of the 242 bin frames; the
  leave-one-folder-out range is there to show how far one folder carries
  the reading. Pitch up has 16 frames.
- **The library can change between this count and the run.** The reader
  recomputes the culled folders; `learnings.md` records any difference.
- **The pre-registration and the reading share one PR.** The record that
  the bars came first is the commit order.

## Progress

- 2026-11-28: Step 1 done. The reader ran over the library's culled folders;
  the primary reading is a ratio of 0.763, so bar 1 is not met (and bar 2
  with it). By the follow-up rule that is branch 2: nothing follows, and the
  `todo.md` section for the cue-labels reading was removed. PR: this
  branch's PR (link added at merge). Review round 1 limited the "no
  secondary reading" sentence in `findings.md` to the culled label.
