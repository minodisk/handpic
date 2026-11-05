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

## Step 3 (`gatescores.py`, the scores and the grid)

- Run: `python -I -B gatescores.py D:\Photos\tests\2026-10-12-rank-app
  D:\Photos\tests\2026-10-23-discard-model D:\Photos\tests\2026-11-04-ef-gate-fill`
  from the scratchpad. Every check passed; `scores_gate.tsv` (46,984 rows,
  22 columns: `folder`, `file`, `ef`, `nose`, `max`, `window_ef`, `ef-fill`,
  8 `ef-gate(s,τ)`, 8 `ef-gate+fill(s,τ)`, `τ` printed to 3 decimals in the
  column name) and `gatescores.out.md` in the gate folder.
- **Wall time**: 18 s to 235 s for the same run (load 10–15 s, the scores
  and cuts under 1 s; the rest is the imports and the 18 MB write on drive
  D:, which other processes kept busy, as in Step 2). The final run took
  114 s.
- The three-argument usage the plan fixes has no place for the 27 folders'
  old per-face dump, so the script takes it as the sibling
  `2026-10-23-face-choice\faces\` of `<discard-model-dir>` (stated in its
  docstring); the six folders' old dump is `<discard-model-dir>\dump\faces\`.
- The first run stopped on a check of my own expectation, not on the data:
  the plan's "12.4% on the dump (4.5% after the `af-whole` fill)" is the
  `af-whole` rule's own missing share; `ef-fill` (`ef` first, then
  `af-whole`) leaves 3.1% (1,002 frames), as `discardfit.out.md` already
  printed. The check now compares the three counts (4,022 / 1,446 / 1,002)
  with that output; no pre-registered rule is touched.
- `load_faces` reads only the dump's own folders' files of the new per-face
  dump (each dump's call first read all 33 and counted the other dump's
  rows as extras).
- `folder_gate.extend` is not called: it only adds columns, and the picks
  (`gate.build`'s `pick`) and the row order the cuts use do not depend on
  it; `ef`'s `PL` on the 27 folders reproduces facechoiceread's 4.44 / 9.14
  / 15.59 without it.
- Readings worth carrying into Step 4 (label-free): 67.7–80.7% of the gated
  faces have `window_ef` exactly equal to `ef` (presumably faces already
  scored on the window because no mesh eyelid region counted), so a gate
  changes the value of only 19–32% of the faces it touches; at q5 every
  `ef-gate` removes the same number of picks as `ef` (269); the 165 meshed
  `af` faces without a pose are gated at 25–96%.
- The audit hook records every `open` event; the report lists the data
  files by directory (the venv, the Python installation and `docs/plans`
  left out) and the run fails if any `labels.tsv` or `.xmp` / `.dop` was
  opened. None was.
- Process slip: a stray `python3 -` (no script, waiting on stdin) was
  started by mistake while grepping; it was killed by its PID and touched
  no data.

## Step 4 (`gateread.py`, the exploration read)

- Run once: `python -I -B gateread.py D:\Photos\tests\2026-10-12-rank-app
  D:\Photos\tests\2026-10-23-discard-model D:\Photos\tests\2026-11-04-ef-gate-fill
  D:\Photos\tests\2026-10-17-bad-labels D:\Photos\tests\2026-10-18-discard-labels
  D:\Photos\tests\2026-10-20-model-search` from the scratchpad. All 85
  checks passed (every known number of the three rounds reproduced to the
  printed digit); output `gateread.out.md` in the gate folder. **Wall time
  239 s** (load 9 s; the bootstraps of 19 scores on rounds 1–2 and 5 scores
  × 4 subsets on all three rounds the rest).
- Outcome by the pre-registered rules: `(s*, τ*)` = (`nose`, 0.215) (every
  pair's rounds 1 + 2 sum negative, this one the least), the winner
  `ef-gate+fill(nose, 0.215)` (round 3 AUC 0.7482), the stop rule says no
  fresh round (round 1 −0.0130, round 2 −0.0109). Steps 5–6 are marked
  skipped in plan.md in the dropped-step style of the archived
  ball-game-ranking-features plan (`- [ ] ~~Step N: …~~ skipped (…)`).
- Deviation: the plan's argument list has no place for round 1's and
  round 2's `G0` bands (`scores_reject.tsv`), which the strata recompute
  needs; the script takes it from `2026-10-16-reject-filter-eval\`, the
  sibling of `<discard-model-dir>` (stated in its docstring), as Step 3 did
  for the face-choice dump.
- Deviation: the candidates are read from Step 3's `scores_gate.tsv`, not
  rebuilt; a label-free check recomputes the 8 `ef-gate` columns from its
  `ef`, `s` and `window_ef` columns and the grid, checks `ef-gate+fill`
  against `ef-gate` where `ef` exists, and compares the judged face of
  Step 2's per-face dump (loaded through `gatescores.load_faces`, imported
  by path) with the file.
- Addition beside the pre-registered readings: the label × gated cross-tab
  also splits the gated and complement labels by the judged face's |yaw|
  band, since the Reading has to say whether gated frames are profiles
  (Step 3's yaw-band reading is label-free only). It decides nothing.
- Round 3's `manifest.tsv` stores `eye_focus` at 6 significant digits, so
  that reproduction check uses 5.1e-5 (rounds 1–2 kept 1e-6).
- The stop-rule points print with 6 decimals: in the dry run `ef-gate`
  read +0.0000 at 4 decimals on round 3 but was +0.000011, a real (tiny)
  change, which "above zero" counts.
- Dry run: `scratchpad\dry\` held copies of the round folders' manifests,
  strata and score files with synthetic `labels.tsv` (drawn from each
  manifest, discard more likely at low `eye_focus`), copies of
  `scores_gate.tsv` and `scores_reject.tsv`, and directory junctions
  (`_winapi.CreateJunction`) to the six-folder `dump\` and the per-face
  `faces\`; a driver set the script's `DRY_RUN` flag (label reproduction
  failures printed, not fatal) and an audit hook that refused any
  `labels.tsv` outside the scratch folder. The label-free checks all
  passed there before the real read, and the refusal on an existing
  `gateread.out.md` was exercised.
- Process slip, again: an empty `python3 -` heredoc was started by mistake
  (no code, no data); killed by its PID. Multi-line text into a file goes
  through the Edit / Write tools here: a heredoc holding apostrophes broke
  the Bash tool's quoting.

## Step 7 (the Decision)

- Written against the stop rule (Step 4 ended the plan; Steps 5-6 skipped),
  from efgate.md, `gateread.out.md` and (for the label-free band shares)
  `gatescores.out.md` only; nothing was re-read against
  labels and no bar or rule moved.
- **Compute time** of the plan: the per-face dump 8,465 s (Step 2, I/O-bound
  on drive D:), `gatescores.py` 114 s (Step 3, final run), `gateread.py`
  239 s (Step 4, once), about 2 h 27 min in all; this step ran nothing.
  **Labeling time: none** (no fresh round was drawn).
- The todo item the plan names, "Core: improve on `eye_focus` for the
  discard question", no longer exists: the discard-model wrap-up replaced
  it with "Core: decide whether `ef` / `ef-fill` at per-folder q5, sorted
  last, is worth shipping", which pointed at this plan as in progress. That
  item now carries the outcome. The `G0` item mentions only the
  discard-model finding, not this plan, so it is unchanged.
- The gated / complement discard shares from the brief were given as
  "rounds 1/2/3"; efgate.md's cross-tab lists them as rounds 3 / 2 / 1
  (97.5% is round 3's gated share), and the Decision and todo.md follow the
  table.
