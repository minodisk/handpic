<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Cap the scan's concurrent file reads and pause them while a preview read is in flight

## Purpose

Riffle is a culling app: the thumbnails and the preview of the focused
thumbnail must win over every other piece of work. The user reports the
preview sometimes appears late. Measured on a release build
(D:\Photos\2026\2026-06-14, 1520 ARW, D: a SATA SSD CT4000BX500, i7-13700,
24 logical cores, 32 GB), a preview read on a normal-priority thread took a
median 0.8 ms idle but **1323 ms (p95 1864 ms) while pass 1 ran** on 22
threads, 2.6 ms during pass 2 and 1.2 ms during pass 3 (the OS cache holds
the bytes by then). The delay is the disk queue, not CPU: the scan workers
lower only their CPU priority (`set_current_priority` in
`crates/core/src/scan.rs`); `THREAD_MODE_BACKGROUND` I/O priority was tried
and was inconclusive (noisy, throughput sometimes collapsed), so it is not
used.

Two things make the queue long. Each pass-1 worker reads the first 1 MiB of
its file (`HEAD_LIMIT` in `crates/core/src/reader.rs`) and issues its next
read as soon as it is done, so the disk never drains; and 22 workers keep 22
reads queued at once. Pass 1 ran 42.8 ms/file, about 23 MiB/s on cold data,
so those 22 queued reads take about 0.9 s to drain, which is most of the
1.3 s. A pause that starts when the preview is requested therefore does not
help the first preview: it still waits behind everything already queued.
The fix is two guarantees, implemented by one primitive:

1. **A cap on how many scan workers are inside the file read at once**,
   separate from the thread count, so decoding, thumbnail encoding and the
   analysis models keep running on `scan_threads()` threads while at most
   `READ_SLOTS` of them are at the disk. By simple arithmetic at 42.8 ms
   per queued read, a cap of 4 leaves about 170 ms ahead of the first
   preview (an estimate: the disk's throughput at a shallower queue, and so
   the pass's ms/file, is unmeasured, which is what Step 2 measures).
2. **A pause while a preview read is in flight**, so a worker whose read
   finishes does not queue a new one ahead of the preview's.

This plan is the first of three fixes the user was offered, and only this
one. Prefetching the neighbours and not waiting on a stale preview request
are follow-ups and are not planned here. Nor are the cold reads with no
scan (330-570 ms per 1 MiB on D: while the debug app was also reading) and
the frontend's own time (IPC, JS, worker decode), which were not measured.

Design settled here (the reasoning is in "Trade-offs and risks"):

- One process-wide `riffle_core::scan::ReadSlots`: `cap` permits plus a
  pause count. A scan worker calls `acquire(cancel)` right before
  `read_preview` and holds the returned RAII `ReadSlot` for the read only.
  `preview` calls `pause()` and holds the returned RAII `Paused` for its
  `read_preview` only. `acquire` waits while the permits are all taken or
  the pause count is nonzero, and returns `None` once `cancel` is set, so
  the worker abandons the file at that stage exactly as `extract_unless`
  already does at its other stages.
- The acquire sits inside `extract_unless`, `extract_analysis_unless` and
  `embed_all`'s per-file closure, around their `read_preview`, not in
  `for_each_path` before `take_next`: the cap must bound the read alone, so
  that the CPU work after it runs uncapped, and the read is the first stage
  of each of those functions.
- A worker waits for the pause at most `MAX_PAUSE` (1 s), then takes a
  permit when one is free, so a stuck or leaked pause slows the scan to one
  file per worker per `MAX_PAUSE` at worst and never stalls it. The cap
  itself is released by every slot's drop, including a panic unwind, so it
  cannot stall either. Between two consecutive preview reads (the preview
  path is single-flight, `requestPreview`'s `inFlight`) the pause is off, so
  sustained paging does not starve the scan.
- Every pass's reads go through the slots: pass 1 reads cold, passes 2 and
  3 re-read the same prefix from the cache, but a resumed scan reads cold
  too, and a cached read holds a permit for about a millisecond.
- The value of `READ_SLOTS` is picked by Step 2's cold sweep (1 / 2 / 4 / 8
  / 22), not guessed, because it also sets how fast the thumbnails fill,
  which is a top priority too. Until then the app passes the thread count,
  which changes nothing but the pause.
- Only `preview` pauses. `metadata`, `faces_of`, `eyes_of` and `focus_crop`
  read the same prefix right after, normally from the cache, and are left
  as they are (a follow-up if measured late). No frontend change.

## Steps

- [ ] Step 1: Add `ReadSlots` (cap + pause) to the scan's reads and pause from `preview`
  - Done when:
    - `riffle_core::scan::ReadSlots` exists: `new(cap)`, `acquire(&self,
      cancel: &AtomicBool) -> Option<ReadSlot>`, `pause(&self) -> Paused`;
      both guards release on drop (a panic unwind included). `acquire`
      waits while no permit is free or a pause is on, returns `None` once
      `cancel` is set, and stops honouring the pause after `MAX_PAUSE`.
    - `extract_unless`, `extract_analysis_unless` and `embed_all`'s per-file
      closure acquire a slot around `read_preview` only (and
      `extract_unless`'s fallback `read_metadata`), abandoning the file
      (`None`) when `acquire` returns `None`. `extract` and
      `extract_analysis` (the public one-file functions) and the CLI's
      `scan` benchmark keep an uncapped, never-paused `ReadSlots`, so
      `docs/humans/performance.md`'s numbers stay comparable.
    - `preview` in `crates/app/src/commands.rs` holds `pause()` for exactly
      the span of its `read_preview` call, and the scan `start_scan` spawns
      acquires from that same handle, with `cap = scan_threads()` for now.
    - Unit tests in `crates/core/src/scan.rs` prove: (a) with threads > cap,
      the number of workers inside the read never exceeds the cap and every
      index is delivered exactly once (an `extract_all` run on fixture files
      with cap 1 and 2 threads, plus a direct test of `acquire` with a
      counter of live slots); (b) while a pause is held, no worker enters
      the read; they enter once it is dropped; (c) a pause dropped by a
      panic (`catch_unwind` around a closure that panics while holding it)
      releases them; (d) a cancel set while workers wait on the pause or on
      a permit ends `extract_all` promptly (join well under a second) with
      every file delivered at most once; (e) with the pause held throughout,
      `extract_all` still finishes, bounded by `MAX_PAUSE` per file per
      worker; (f) a slot dropped by a panic frees its permit.
    - `mise run ci` passes.
  - Implementation approach:
    - `ReadSlots` is `Arc<(Mutex<State>, Condvar)>` with `State { free:
      usize, paused: usize }`, the lock taken with `unwrap_or_else(|e|
      e.into_inner())` like `ScanFocus::hot`. `acquire` loops on
      `wait_timeout` with a short tick (around 50 ms) because the cancel
      flag is a plain `AtomicBool` with no notifier; it leaves with `None`
      when `canceled(cancel)`, and takes a permit when `free > 0` and
      (`paused == 0` or this call has waited `MAX_PAUSE`). Guards decrement
      / increment and `notify_all`. Keep the tests fast by letting the
      maximum pause be a parameter the tests shorten (a `with_max_pause`
      constructor or a field), not a sleep of a second per test.
    - Plumbing: `for_each_path`'s `per_file` becomes `Fn(&Path,
      &AtomicBool, &ReadSlots)` and the three `*_unless` / closure call
      sites take the slots; the handle reaches `for_each_path` as a field of
      `ScanFocus` (`ScanFocus::default()` keeps an uncapped, never-paused
      handle, so the CLI, `run_scan` / `run_faces_scan` / `run_embed_scan`
      and the existing tests keep their signatures; `start_scan` builds the
      focus with the app's handle). `ScanFocus`'s doc comment widens to "how
      the view steers a running scan: the paths it takes first and the read
      slots its reads go through". The alternative, a separate `&ReadSlots`
      parameter through the three `*_all` functions, the index passes,
      `RunningScan`, the CLI and every test, is the purer type and a much
      larger diff; see Trade-offs.
    - App side: `commands::AppReadSlots(ReadSlots)` managed in
      `crates/app/src/main.rs` next to `Scans`, built with a `READ_SLOTS`
      constant next to `scan_threads()` that Step 1 sets to
      `scan_threads()` (Step 3 fixes it). `preview` does `let _paused =
      app.state::<AppReadSlots>().0.pause();` inside its `spawn_blocking`
      closure immediately before `read_preview`, so the guard drops when
      the closure returns, Ok or Err. `preview` must not take the `Scans`
      lock (held for seconds by `clear_index`, the trash commands and the
      eviction's `VACUUM`), which is why the handle is app state rather
      than a field of `ScansState`.
    - The current file heads the focus list, so a worker may read the file
      the preview is reading; both read the same 1 MiB prefix and the
      second is served from the cache, or both hit the disk once for at
      most 1 MiB. Not handled.
- [ ] Step 2: Measure the cap on a cold pass 1 and pick its value
  - Done when:
    - A new ignored test in `crates/app/src/commands.rs`
      (`preview_during_pass_1_on_a_real_folder`, next to
      `embed_pass_on_a_real_folder`, sharing its `summarize` helper) runs
      pass 1 only on `RIFFLE_SCAN_DIR` with the app's thread count and
      priority and a `ReadSlots` of cap `RIFFLE_READ_SLOTS`, with a probe
      thread that every 250 ms does what `preview` does (pauses around
      `read_preview`, then decodes outside the pause, on a normal-priority
      thread) and records the read time and the read + decode time
      separately; `RIFFLE_PROBE_PAUSE=0` runs the probe without pausing.
      Before the pass it reads one file the probe never touches and prints
      that read's time as the coldness check, refuses to run while a
      `riffle` process is alive (so the app's own reads cannot pollute the
      numbers), and prints: the cold-check ms, the pass's wall time and
      ms/file, and p50 / p95 of the probe's read and read + decode. It
      needs no embedding model. Run as
      `RIFFLE_SCAN_DIR=D:\Photos\2026\<unopened folder> RIFFLE_READ_SLOTS=<cap> mise exec -- cargo test --release -p riffle-app preview_during_pass_1_on_a_real_folder -- --ignored --nocapture`.
    - `learnings.md` holds the sweep table: caps 1 / 2 / 4 / 8 / 22 (22 is
      the uncapped baseline), each with and without the pause for at least
      the baseline and the chosen cap, at least two runs per cell, every run
      on cold data: the app closed, the same folder each run with the
      standby list emptied between runs (Sysinternals RAMMap, Empty Standby
      List, needs admin) or a different unopened folder under
      `D:\Photos\2026` of similar size, and the printed cold check at least
      100 ms (a cold 1 MiB read on D: took 330-570 ms; a cached one about
      1 ms). Columns: cap, pause on/off, cold-check ms, pass ms/file, probe
      read p50 / p95, read + decode p50 / p95.
    - `learnings.md` states the picked cap and the rule it followed: among
      the caps whose probe read p95 during pass 1 is at or under 250 ms
      (roughly the idle read + decode + frontend path, so paging during the
      scan feels like paging at idle), the one with the lowest pass ms/file;
      if the lowest-ms/file cap among those costs more than about 20 % of
      the uncapped throughput, note the next larger cap's wait and leave the
      final choice to the user with both rows; if no cap meets 250 ms,
      report the knee of the wait-versus-throughput curve and ask the user.
    - NVMe: if a RAW folder exists on C: (NVMe), one run at the baseline and
      at the chosen cap there, cold, recorded in the same table; if none
      exists, `learnings.md` says the cap is untested on NVMe and what it
      is expected to cost there (a shallower queue costs more on a drive
      that needs deep queues for throughput, which the user's C: may show).
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 1 is merged. The test builds its own `ScanFocus` with a
      `ReadSlots::new(cap)` from the env; `RIFFLE_READ_SLOTS` is read only
      by this test, never by the app, and there is no user setting.
    - Do not change `embed_pass_on_a_real_folder`'s meaning (it measures
      passes 2 and 3 with read + decode together); extract `summarize` into
      a shared test helper.
    - Why not reuse one warm run: the cache is the whole point; a run whose
      cold check prints about 1 ms is discarded.
    - The CPU part of pass 1 is small (a 1 MiB read, a preview decode and a
      thumbnail encode, tens of ms of CPU), so at 23 MiB/s cold the pass is
      disk-bound and a cap may cost little throughput; the sweep decides.
- [ ] Step 3: Fix the cap, document it, and hand-check the app
  - Done when:
    - `READ_SLOTS` in `crates/app/src/commands.rs` is the cap Step 2 picked,
      with a doc comment carrying the measured numbers the way
      `scan_threads()` and `embed_threads()` do.
    - A hand check, recorded in `learnings.md`: open an unopened folder
      under `D:\Photos\2026` in the running release app and page with the
      arrow keys during its first scan; the preview follows the keys without
      the multi-second lag, and the thumbnails keep filling while paging;
      note anything odd (the strip pausing, a scan that seems to stall).
    - `CLAUDE.md`'s layout paragraph (`crates/core` `scan.rs`, `crates/app`
      `commands.rs` `preview`) mentions the read slots, and
      `docs/agents/tauri-app.md` gains a short section after "The scan
      pool's priority levels, and `set_scan_focus` drops what it cannot
      apply" stating that the scan's reads are capped and yield to the
      preview's at the disk, the sweep's numbers, `MAX_PAUSE`, why cancel is
      polled inside the wait, and that the cap was measured on a SATA SSD
      (NVMe result or "untested"). The follow-ups (prefetch, stale requests,
      the other read commands, NVMe if untested) go to `todo.md`.
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 2 is merged. May be the same PR as Step 2 if the caller
      prefers; nothing here changes Step 2's test.

## Trade-offs and risks

- **One primitive or two.** The cap and the pause share one wait (a
  worker waits for a free permit and no pause), one guard type per side,
  one condvar and one cancel poll, and `MAX_PAUSE` applies only to the
  pause half. Two mechanisms (a semaphore in the read functions and a gate
  before `take_next`) would double the plumbing and put the pause where it
  cannot see the read. One primitive is honest here.
- **Why the earlier "pause before `take_next`" placement is dropped.** The
  cap must bound the read, which lives inside `extract_unless` /
  `extract_analysis_unless` / the embed closure via `read_preview`; a gate
  in `for_each_path` cannot see it. Putting both there means the `per_file`
  signature grows by one argument; the alternative of wrapping
  `read_preview` itself in `reader.rs` would gate every caller (the CLI, the
  one-file `extract`), which is wrong.
- **Slots as a field of `ScanFocus` vs a separate parameter.** The field
  keeps the diff to the `per_file` signature and the three read sites; a
  separate `&ReadSlots` parameter through `extract_all` /
  `extract_analysis_all` / `embed_all`, `run_scan` / `run_faces_scan` /
  `run_embed_scan`, `RunningScan`, the CLI and every test is the purer type
  and a far larger diff. The field was chosen.
- **Step count.** Three steps so the value is measured before it is fixed
  (Step 1 is behaviour-neutral on throughput, Step 2 measures, Step 3
  fixes).
- **The cap's cost.** A shallower queue can lower a drive's throughput and
  so slow the thumbnail fill, which is also a top priority; that is why the
  value comes from the sweep and the rule weighs p95 wait against ms/file,
  with the user deciding when the two conflict. On the user's SATA SSD at
  23 MiB/s cold the pass looks disk-bound with headroom, so the expected
  cost is small; on a fast NVMe (C:) a cap of 4 may leave throughput on
  the table, measured only if a RAW folder is available there, else noted
  as untested. The cap stays a constant, not a per-drive setting.
- **`MAX_PAUSE` value.** 1 s. Smaller (250 ms) bounds a leaked pause
  tighter but may release workers under a genuinely slow cold preview read
  (330-570 ms measured with no scan); larger (5 s) lets a leaked or hung
  pause visibly stall the strip. It only matters in failure cases; the
  normal case releases on the guard's drop.
- **The residual wait.** Even capped, the preview waits for up to
  `READ_SLOTS` in-flight reads (about 170 ms at a cap of 4 by the 42.8 ms
  arithmetic, an estimate); the pause then keeps new reads from queuing
  ahead of it. Going lower means a smaller cap (the sweep shows the price)
  or a smaller first read, which is out of scope.
- **Which commands pause.** Only `preview`. `metadata` is requested right
  after for the same file and reads the same cached prefix; `faces_of` /
  `eyes_of` / `focus_crop` likewise. Adding them is one line each if the
  pause spans the read only (never a model run or a decode), but each is
  another pause per page; decide after Step 2 if the meta pane is seen late.
- **Cancel latency.** Cancel is polled every wait tick (about 50 ms)
  inside `acquire`, because the cancel flag has no notifier and
  `ScansState::cancel` has no handle on the slots; a cancel while waiting
  costs at most one tick. A worker that returns `None` from `acquire`
  abandons its file, which the existing cancel semantics already allow
  ("files already running stop at their next stage").
- **Noise.** The earlier A/B at 4 workers was unusable because the debug
  app was reading and the cache mix varied; Step 2's test refuses to run
  with the app alive, confirms coldness per run and repeats runs. A run
  whose cold check reads warm is discarded, not averaged in.

## Progress

- (none yet)
