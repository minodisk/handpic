# Learnings

## Step 1: stopped before implementing (2026-11-27)

- The measurement folders were reachable (`D:\Photos\2026\2026-09-19`,
  `D:\Photos\2026\2026-11-07`). The numbers are in
  [measurement.md](measurement.md).
- The plan's premise does not hold on these previews: the same face's eye
  landmarks move by up to 0.43 of the box side between the crop detection and
  the 640x448 whole-image search, and up to 0.69 on the 320x224 gate search,
  more than one face's own eye distance (0.04 to 0.4).
- The double counts the IoU rule misses (`2026-09-19/_DSC1899`,
  `2026-11-07/_DSC1543`, `_DSC1524`) are profiles where one detection puts
  the eyes on the ear, and a partial box. Their eye distance (0.35 to 0.57)
  overlaps a real neighbour's (`2026-11-07/_DSC1517`, 0.47).
- Replacing the IoU match would add a double count on 84 (`2026-09-19`) and
  133 (`2026-11-07` slice) AF frames' gate counts at a 0.2 cut, against 1 frame
  double-counted today. The union with a safe cut (0.2) changes no measured
  frame. Neither closes the `todo.md` item, so no code was changed and the
  decision went back to the caller.
- The scratch `handpic-cli eyematch` / `eyeshow` subcommands that measured
  this were not committed.
- Tooling: a bare `python3 -` in Git Bash hung for minutes (as `CLAUDE.md`
  warns); use `mise x -- python`, or `sed` / `awk` for small edits.

## Step 1, second attempt: the overlap over the smaller box, stopped (2026-11-27)

- Measured on `2026-09-19`, the whole of `2026-11-07` (3638 ARW, 3354 AF
  frames with a judged face) and the no-AF DNG folder `2026-02-01`. The
  numbers and the by-eye verdict of every counted pair at or under IoU 0.3
  with an overlap of 0.3 or more are in
  [overlap-measurement.md](overlap-measurement.md).
- The whole `2026-11-07` folder holds far more missed twins than the first
  measurement's slice suggested: about 30 AF frames whose 320x224 gate count
  includes the judged face a second time (a box on the ear, a larger or a
  partial box), not 1 to 3.
- No cut separates them from real neighbours: different people sit at 0.30,
  0.315, 0.325, 0.347, 0.363, 0.406, 0.444, 0.451, 0.470 and 0.532, twins
  from 0.306 to 0.740. Above 0.532 (`2026-11-07/_DSC1970`, a blurred face in
  front of the judged one) every pair is the same face.
- Replace against union: for an IoU above 0.3 the overlap is at least
  0.4615, so at a cut at or under 0.46 the two are the same rule. Above it,
  replacing turns IoU-matched same-face pairs (overlap 0.47 to 0.6 at
  320x224) into double counts: 18 gate frames at 0.55, 42 at 0.6. Any cut
  above the highest neighbour therefore has to be the union.
- The option left for the caller: the union with a cut above 0.532 (0.55 or
  0.6) lowers 11 or 4 gate frames, all the same face, `_DSC1899` and
  `_DSC1524` among them, drops no person and leaves the twins under 0.532
  double-counted, so the `todo.md` item would stay open, narrowed. Per the
  caller's rule no code was changed and Step 1 stays unchecked.
- `2026-11-07/_DSC1543`, named as a twin at 640x448, scores 0.67 there and
  is absent at 320x224: no count takes it today.
- The scratch subcommands (`overlapm`, `overlapshow`) were not committed;
  they are in the session scratchpad as `overlapm.rs.txt` and
  `overlapm-main.diff`. Building only `handpic-cli` in release took seconds
  and the whole `2026-11-07` run about two minutes on 8 threads.

## Step 1: the union shipped (2026-11-27)

- The user chose the partial fix: `allfaces::same_face`, an IoU above
  `NMS_THRESHOLD` or an overlap over the smaller box above
  `SAME_FACE_OVERLAP` (0.55), in both `gate_count` and `candidates`.
  `FACES_VERSION` went 14 to 15; `ALLFACES_VERSION` stayed.
- The `todo.md` item was reworded to the twins left between about 0.3 and
  0.55, not deleted.
- `mise run ci:local` first failed in its test stage on the host's sccache
  config (`%APPDATA%\Mozilla\sccache\config\config`, `dir = "V:\sccache"`,
  an unescaped backslash TOML rejects), outside the repository. Re-run once
  with `RUSTC_WRAPPER=` empty, it passed.

## Deferred issues (todo candidates)

- **Taken up by Step 1 (folded into the reworded `todo.md` item "Core: the
  judged face can be counted twice in the all-faces counts and the gate
  count (all-faces-check)"); do not file again.** **Core: the judged face's
  double count needs a rule other than eye landmarks
  (judged-face-eye-match).** The eye-landmark match the plan
  proposed for `allfaces::gate_count` and `allfaces::candidates` cannot
  separate the same face from a neighbour on the measured AF folders (see
  `measurement.md` in this folder): the missed twins are profiles and partial
  boxes with eye distances of 0.35 to 0.57 of the box side, a real neighbour
  sits at 0.47, and replacing the IoU rule would add a double count on 84 and
  133 frames. Candidate directions, unmeasured: the overlap over the smaller
  box (intersection over the smaller area) instead of IoU, since in every twin
  seen one box lies mostly inside the other; or leaving it, at 1 to
  3 frames in about 580. Files: `crates/core/src/allfaces.rs`
  (`gate_count`, `candidates`).
  The overlap over the smaller box was then measured too (see
  `overlap-measurement.md` in this folder): no cut separates the twins
  (0.306 to 0.740) from different people (up to 0.532), and the whole
  `2026-11-07` folder has about 30 AF frames whose gate count holds the
  judged face twice, not 1 to 3. A union "IoU above 0.3 or overlap above
  0.55 / 0.6" fixes 11 / 4 of them and drops no person; the rest need
  another rule.
