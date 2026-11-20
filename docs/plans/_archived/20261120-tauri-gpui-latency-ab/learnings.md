# Learnings

## Step 1

- The app logs (both the GPUI `Handpic.log` and the Tauri log) stamp UTC, not
  local time. The session ran 2026-11-20 08:19-08:49 local (UTC+9, from the
  files' modification times); the logs read 2026-11-19 23:19:54 to 23:49:13.
  The plan's note that the Tauri log was stamped "23:31 local" was a misreading.
- `runs.txt` recount matched the plan: 25 lines, 4 warm runs, 21 measured
  attempts, 12 used, 9 discarded (ARW 6, RAF 3), all for a load sample over
  15 % after the first. `ARW-A2-try1` and `ARW-B3-try1`, both discarded, also
  had `accumulated>1=1`; only `ARW-B3-try2` among the used runs has it.
- The tables were checked cell by cell against `raw/summary.md` with a short
  script parsing both files' tables by column name, after reordering the
  columns into pipeline order.
- The plan's reading "the 23.20 ms synchronous `draw` runs past the frame" for
  Tauri RAF is written as a reading, not a finding: the data shows only that
  `frame1` / `frame2` sit within 2.30 ms of `keypressToPixels`.
- The GPUI build command used for A was not recorded in the raw folder, so the
  reproduction names it as a release build of `crates/native` at `dc6fa3d7`.
