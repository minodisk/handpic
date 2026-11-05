# Learnings: `eye_focus` gate and fill

## Step 1 (pre-registration of the exploration)

- The discard-model plan is already archived
  (`docs/plans/_archived/20261023-discard-model/`), so `efgate.md` links
  there; the scripts of Steps 3–6 still resolve its path at run time.
- Code references checked at `7085586e` before quoting them:
  `candidate::mesh_eye_offset`, `candidate::eye_focus`, `Scored::Window`,
  `candidate::meshes_face`, `eyes::EYES_MIN_FACE` (60.0), `eyes::Mesh.points`
  (stored coordinates), `faces::Face.nose`, and `pose.rs`'s tests reading
  point 4 as the nose tip.
- `eye_focus` falls back to `Scored::Window` even with a mesh when no eyelid
  region counts; on those faces `window_ef` equals `ef`, so the gate cannot
  change them. Stated in `efgate.md` under "The per-face values".
- Sources of the known numbers: round 3 and its subset reading from
  discardmodel.md "Results of the confirmation"; rounds 1–2 `ef-fill` from
  its "Results of the exploration"; rounds 1–2 `ef` and base rates from
  facechoice.md "Reproduction checks"; the fresh missing shares (9.00% /
  1.18%) from the discard-model learnings, Step 4; 6,214 picks = 4,419 +
  1,795.
- Added one sentence the plan implies but does not spell out: if no
  candidate passes the label-free conditions at q5 or q10, there is no
  winner and the plan ends as under the stop rule (as discardmodel.md did).

## Step 2 (the extended per-face dump)

- New columns after `mesh_ms`: `nose` (YuNet), `mesh_left_eye`,
  `mesh_right_eye` (the means of `LEFT_EYE_CONTOUR` / `RIGHT_EYE_CONTOUR`,
  summed in `f32` as `mesh_eye_offset` sums them), `mesh_nose` (point 4),
  `eye_offset` (the cue's own `EyeMeasures.eye_offset`, i.e.
  `mesh_eye_offset`), `nose_offset`, `max_offset`, `window_ef`; 31 columns,
  the no-face row 24 dashes after `face`. `nose_offset` is computed in the
  CLI in `f32` and widened to `f64`, as `mesh_eye_offset` does.
  `max_offset` is `-` unless both offsets exist (they always do with a
  finite mesh; `eye_offset` is `None` only on a non-finite contour point or
  an empty box). `window_ef` is a second `eye_focus(.., None)` call on every
  face, so on an unmeshed face it equals `eye_focus`.
- The `USAGE` line did not need a change (no new argument).
- Frozen CLI: `D:\Photos\tests\2026-11-04-ef-gate-fill\riffle-cli.exe`, the
  release build of this step's code on top of `8020d753`, **SHA-256
  `a1a664c00e9ce38687bf199dee081a77730c61b901bdb9710a795b527ba80c38`**.
- Run: `facesdump.sh <cli> 12 [folder...]` from that folder (2026-07-21
  alone first as a trial, then all 33; the trial's output was kept and
  skipped). Result: 33 `faces\<folder>.tsv`, **46,984 files, 0 errors,
  `--check` 0 mismatches of 46,984**; per folder the distinct files equal
  the RAW count of the folder and the old per-face dump's files; every row
  31 columns; 105,634 crop faces and 210,833 whole faces, 141,337 meshed
  (48.6 ms mean mesh). `listing.before.tsv` and `listing.after.tsv`
  (84,637 files) identical: "library unchanged".
- **Wall times** (12 threads, `--check`): 2026-06-05 136 s, 06-13 86,
  06-14 65, 06-20 96, 07-02 24, 07-05 86, 07-11 67, 07-18 107, 07-21 4,
  07-22 9, 07-23 71, 07-24 7, 07-26 3, 07-30 41, 07-31 13, 08-01 336,
  08-02 6, 08-08 168, 08-15 75, 08-16 137, 08-22 212, 08-29 235, 09-13-b
  167, 09-19 393, 09-27-a 1,657, 09-27-b 248, 10-03 466, 10-11 1,186,
  10-12a 320, 10-12b 516, 10-17 652, 09-13-a 739, 09-27-c 137; **8,465 s
  in all** (about 2 h 21 min) against the plan's 37 min. Not the window
  score: CPU stayed at 15–25% while drive D: read at about 2,400% disk
  time (not this run alone; other processes were active on the machine),
  so the run was I/O-bound; 2026-07-21 alone took 4 s against the
  earlier 1 s.
- Each size / mtime listing of the 33 folders takes about 3 minutes on D:.
- The Bash tool moves a foreground call to the background after about
  180 s here whatever its `timeout`, so the waits were `timeout 175 tail
  -n0 -F logs/wall.log | grep -m1 "facesdump done"` repeated.
- No value of the dump was read against any label; only the column count,
  the file sets and the `--check` totals were looked at.
