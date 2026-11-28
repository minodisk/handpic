# Learnings: culling cues

## Step 1

- **Pre-registration commit**: `68fee3f7b7a514646c7ca8315a7cbefd0781bc2c`
  (`docs(plans): pre-register the culling-cues reading`) adds `plan.md`
  alone, before the first run of `culling.py` that printed a reading.
- **Run**: `mise x -- python -I -B culling.py D:/Photos/tests/2026-10-12-rank-app
  D:/Photos/tests/2026-10-17-bad-labels D:/Photos/tests/2026-10-18-discard-labels
  D:/Photos/tests/2026-10-20-model-search D:/Photos/tests/2026-11-28-culling-cues`
  from the scratchpad copy, stdout to `culling.out.md` in the output folder
  (`D:\Photos\tests\2026-11-28-culling-cues\`), one process, about 1 to 2 s
  (the listing was warm). `<confirm-dir>` is the model-search round's
  folder. No `__pycache__/` was left in the repository by the runs.
- **Determinism**: a second run printed byte-identical stdout.
  SHA-256 of the output: `56af9dd9f9f96d7fb3c5654b14e7d9aeb263a5c6442405726b2f645e05618ccf`
  (both runs). SHA-256 of `labels.tsv` (11,196 rows):
  `3eb276dacd849b6452dc1951c961f859202e97711478ac35bb857dc64abc42a1`
  (both runs).
- **Library unchanged**: 44,243 files in the 27 dump folders before and
  after each run; 0 changed, 0 added, 0 removed paths. No library file was
  opened; the reader stats the files (and the 27 folders for their mtime).
- **Culled and left-out folders**: the reader found the same seven culled
  folders as "What is known", each with the same dump frames, on-disk
  count, dump picks and `|pitch|` > 45 count (11,196 frames, 1,388 kept).
  Left out: 19 intact folders and `2026-08-29` (3,451 frames, 0 on disk,
  475 picks), 20 folders, 21,263 frames. No difference.
- **Kept against the dump pick**: all 1,388 kept frames are `Pick`; the
  culled frames are 5 `Pick` (all `2026-09-19`), 57 `Reject`, 93 `None`
  and 9,653 `-`. As known.
- **Bin sizes**: all equal "What is known" (`|pitch|` > 45 242, up 16,
  down 226; EAR <= 0.137 937; `|yaw|` > 35 2,810; `eye_focus` < 0.9 3,062;
  no `eye_focus` 1,294; no `pitch` 3,855). The bottom sharpness decile,
  not counted before, is 1,116. On the pick label, `2026-08-29` has 65
  (7 up, 58 down), as known; the 26 folders with a pick have 749 (185 up,
  564 down), the known 768 / 188 / 580 of all 27 less `2026-06-05`, which
  has no pick.
- **Dropped bootstrap draws**: 0 for every reading but `pitch` > 45 on
  the culled label, 17 of 1,000 (draws holding none of the three folders
  with a pitch-up frame). The `2026-08-29`-alone readings have one cluster,
  so their interval is the point.
- **Sidecar mtimes of the kept frames** (local time): `.dop` only in five
  folders, `2026-07-22` and `2026-07-26` 2026-08-07, `2026-07-30`
  2026-08-04, `2026-08-01` 2026-08-02 to 08-03, `2026-08-22` 2026-08-30 to
  09-01 (and one `.xmp`, 2026-09-22); `.xmp` and `.dop` 2026-09-27 in
  `2026-09-13-b` and 2026-09-26 in `2026-09-19`. "What is known" said
  2026-08-02 to 2026-08-30 for the five folders; this run's upper end is
  2026-09-01 in `2026-08-22`. The difference does not move the conclusion
  (still before the repository's first commit on 2026-09-16). Folder
  mtimes: 2026-11-06 (`2026-08-22`, `2026-08-29`, `2026-09-19`) and
  2026-11-07 (the other five), as known.
- **The reading**: the primary ratio is 0.763 (95% 0.442 to 1.017), bar 1
  is not met, bar 2 is not met, branch 2 of the follow-up rule: the
  `todo.md` section was removed.
- **Tripped up**: a `sed` with `\\|` in its pattern was read as GNU BRE
  alternation and rewrote every `")` in `culling.py`; repaired with a
  Python string replace. Edit Python sources with a Python script or the
  Edit tool, not `sed`, when the pattern holds `|`.
- **Tripped up**: a `|` in a Markdown table cell (`` `|pitch|` ``) splits the
  cell, even in a code span; the reader's `table` escapes it as `\|`.
