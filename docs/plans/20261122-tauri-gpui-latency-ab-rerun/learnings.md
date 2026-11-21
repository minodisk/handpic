# Learnings

## The measurement session

- The `RAF-B3` slot failed all 4 tries of the main pass: three for load
  spikes over the 15 % gate, one (`RAF-B3-try4`) for probe exit 2, the window
  losing the foreground to another process before press 4. The driver moved
  on with the slot empty, and the slot was redone after the main pass with
  `raw/redo-raf-b3.ps1` (`RAF-B3-try5` passed).
- At the user's request `RAF-A3` was redone too (`raw/redo-raf-a3.ps1`,
  `RAF-A3-try2`), although `RAF-A3-try1` had passed, so the third RAF pair
  sits together in time; try2 is substituted for try1 in every median.
- `summarize.py` and `endpoints.py` take every non-warm `bad=[]` line of
  `runs.txt`, so the substitution needed a scratch copy of `runs.txt` with
  the `RAF-A3-try1` line marked superseded; `raw/summary.md` and
  `raw/endpoints.txt` come from that copy, while `raw/runs.txt` is the
  original log.
- The plan gave the session as ending at 00:14 local; the app logs' UTC
  stamps put the last run's end at 15:11:15 UTC, 00:11 local, which
  `results.md` uses.

## Step 1

- The per-press CSVs of the 12 used runs (in the measurement worktree's
  `out\20261121`) were checked for the `results.md` claims: `first_change_ms`
  equals `settled_ms` on every press, only `ARW-B2-try1` has a press (press
  2) with `max_accumulated` 2, and the `frames` ranges per app and folder.
- The `decoded=` counts (RAF 51 x `1656x1104` and 9 x `1104x736`, ARW 57 x
  `1212x810` and 3 x `808x540`) hold in each of the six used GPUI runs'
  `.stdout.txt`.
