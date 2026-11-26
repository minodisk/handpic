# Learnings

## Step 1

### The lock change

- `mise x -- cargo update -p notify --precise 8.2.2` at the root, on base
  commit `739a17ee`, printed exactly the planned changes: `notify` 8.2.0 ->
  8.2.2, `fsevent-sys` 4.1.0 removed, `objc2-core-services` 0.3.2 added,
  `windows-sys` 0.60.2, `windows-targets` 0.53.5 and the eight `windows_*`
  0.53.1 crates removed. Nothing else changed in `Cargo.lock`, and
  `notify-types` stays at 2.1.0. The new `notify` checksum
  (`c41a04a5f59df9322cc657286dd2d79f90816119b22c5f847607c850110205ab`) is the
  same as the one in `crates/app/Cargo.lock`, which was not touched.
- `mise x -- cargo tree -p handpic-engine -i notify` reports `notify v8.2.2`
  for the test binary built against the new lock
  (`mise x -- cargo test -p handpic-engine --no-run`, 3 min 02 s from a cold
  `target/`, binary `target/debug/deps/handpic_engine-83ee0e087bdd5624.exe`).
- When you edit a file through `python` on Windows, text mode writes CRLF.
  Open it with `newline=""`, or convert back to LF afterwards.

### The stress run (stopped before it finished)

- The load was 24 `multiprocessing` spinner processes (`load.py` in the
  session scratchpad), one per logical core. CPU `% Processor Time` was 100 %
  while it ran. Each iteration ran the built binary with one filter, e.g.
  `handpic_engine-83ee0e087bdd5624.exe treewatch::`, and a failure was a
  non-zero exit code. Unrelated `handpic_engine-c5006f554f` test binaries
  from another session were also running on the host. No Handpic app
  instance was running.
- First attempt (not counted): the load was started as a separate background
  command and died after about 10 minutes (exit 1 at 23:53:50). So the
  `treewatch::` 300 / 0 failures result it printed cannot be shown to have
  run under load throughout. It was aborted, and the driver was rewritten to
  start the load itself and to check, before each iteration, that all 24
  spinners were alive and heartbeating.
- Second attempt, on 2026-11-26 / 27:
  - `treewatch::`: **300 iterations, 0 failures**, all 300 with the full
    24-process load alive (23:56:21 to 00:16:43, about 4 s per iteration
    under load).
  - `watch::`: started at 00:17:26. The coordinator stopped it at 00:19:11
    because the full-core load was starving other sessions on the machine.
    The driver keeps its iteration count only in memory, so the exact count
    was lost. From the timing it was about 25 iterations, **0 failures** (no
    failure output was saved).
  - `rename::`: **not run** (0 iterations).
- The user then dropped the local stress run entirely, and no load or loop
  was run again. The plan's Done-when was reworded to match: verification
  rests on the PR's three-platform `test` job. Step 1 was ticked on the
  remaining criteria (the lock, the guides and the `datefolder.rs`
  comment). `mise run ci:local` was not run locally: the user's standing rule
  is to skip local `ci:local` and cargo builds and let the PR's GitHub checks
  verify, and the host was contended.
- Lesson: a full-core load for about an hour is not acceptable on this
  shared host while other sessions run. Agree on a quiet window first, or
  use a load that scales down.
