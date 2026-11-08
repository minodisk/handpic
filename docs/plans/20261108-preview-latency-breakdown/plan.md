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

# Break the preview latency into stages, measure it on Windows, and fix what the measurement names

## Purpose

Fast preview display is the single most important property of Riffle. The
time from the key that turns the page to the preview on screen has never been
measured on real hardware: `todo.md` ("App: unmeasured end-to-end per-page
latency") still says so, and the only numbers are the Rust-side read
(`docs/humans/performance.md`, "Per-page preview read") and two eyeballed
overlay readings (20-100 ms during pass 2, 100-200 ms under an external CPU
hog; `docs/plans/_archived/20261011-performance-graph-overlay/results.md`).

The path today, verified in code: a key calls `move()` → `show()` →
`requestPreview()` synchronously (`crates/app/ui/src/main.ts`, one request in
flight, a stale response re-requests the latest file) → the `preview` command
(`crates/app/src/commands.rs`: `spawn_blocking`, `ReadSlots::pause()` around
`reader::read_preview`, which reads the 1 MiB prefix plus a ranged read when
the JPEG lies past it, and `hevc::to_jpeg` for an HDR PQ CR3) →
`tauri::ipc::Response` of an 8-byte header plus the JPEG → the main thread
parses the header and **copies** the JPEG with `payload.slice` → the worker
(`crates/app/ui/src/worker.ts`) runs `createImageBitmap` and transfers the
bitmap back → `draw()` scales and rotates it onto the canvas 2D context and
draws the focus / face marks. No preview is cached and no neighbour is
prefetched.

The existing instrumentation lumps those stages into two numbers (`page
invoke=… decode=…`: `invoke` is read + IPC, `decode` is worker queue +
`createImageBitmap` + transfer; the draw is untimed, the main-thread copy is
untimed, and the backend sends no timing). The perf overlay (`d`) shows one bar
per switch with no breakdown.

So this plan does three things, in order: give each stage its own number,
visible in the running app and in `Riffle.log`, at no cost while the overlay
and `Timing logs` are off; measure the realistic cases on the user's Windows
machine with a small hand procedure; and then, by written decision rules,
pick which of the conditional steps to run, including "none", and whether
native rendering is even on the table. The disk-side fix (the scan's read cap
and pause) is owned by `../20261107-preview-read-gate/plan.md` and is not
redone here.

Stages (the names used by the log line, the overlay legend and `results.md`):

| Stage | Measured as | Where |
|-------|-------------|-------|
| `wait` | keypress → the `preview` invoke is actually sent (the time a key spent behind an in-flight request; 0 for a lone press) | frontend |
| `read` | `read_preview` inside the `spawn_blocking` closure, pause included (the disk queue, the HEVC decode of an HDR PQ CR3) | backend, carried in the payload header |
| `ipc` | invoke round trip minus `read` (the blocking-pool hop, the `Response` copy, WebView2's fetch) | derived |
| `post` | invoke resolved → `worker.postMessage` (header parse, `previewPixelLimit`, the `slice` copy) | frontend |
| `worker` | postMessage → the worker's `message` handler fires, plus the bitmap transfer back (two queue hops) | derived: frontend span minus `decode` |
| `decode` | `createImageBitmap` inside the worker | worker, carried in its reply |
| `draw` | the `message` handler from its success path to the end of `draw()` (the bitmap swap, `setStatus()`'s meta-pane rebuild, canvas resize, `drawImage`, the marks) | frontend |

`keypressToPixels` = the sum; `total` (from the request) = the sum without
`wait`, as today.

## Steps

- [x] Step 1: Time every stage of the preview path and show the breakdown in the overlay and the `page` line
  - Done when:
    - `preview` returns a `PREVIEW_KIND_JPEG_V2` payload whose header is
      still 8 bytes with the formerly reserved `u32` at offset 4 now
      `read_us`, the microseconds the `spawn_blocking` closure spent in
      `read_preview` (pause acquisition included). `THUMBNAIL_KIND_JPEG_V1`
      is unchanged (the strip keeps reading V1, with the word zero).
      `payload`'s doc table and the unit test next to
      `preview_payload_header_encodes_kind_one_and_orientation` cover the
      new kind; the frontend rejects V1 for a preview as it rejects any
      unknown kind today.
    - The worker's reply carries `decodeMs`, the `performance.now()` span
      around `createImageBitmap` measured inside the worker (a duration,
      never an absolute time: a worker's clock has its own origin).
    - `main.ts` records, per `seq`, the marks needed for the seven stages
      above and logs one line
      `page wait=… read=… ipc=… post=… worker=… decode=… draw=… total=… keypressToPixels=… jpegKB=… px=WxH scan=idle|1|2|3`
      through `debugLog` (so only while `Timing logs` is on). `scan` is the
      pass running at the moment of the request (pass 1 while
      `scanRunning`, 2 while the faces pass is in progress, 3 while the embed
      pass is, else `idle`, using the flags `main.ts` already keeps for the
      status line), `jpegKB` the payload size, `px` the decoded bitmap size.
      `wait` and `keypressToPixels` are omitted for a switch no key timed,
      as today.
    - `PerfModel`'s `LatencyPoint` gains an optional `stages` record of the
      seven numbers; `addLatency` still records nothing while `off` and
      `stop()` still drops everything. The overlay draws each latency bar
      stacked by stage, bottom to top in path order, in colours derived
      from the existing `--chart-latency` token (lighter / darker steps or
      alpha, no new palette), and the legend names the stages. A point
      without `stages` draws as the single red bar it is today.
    - `perf.test.ts` covers: a point with stages keeps them; `latencyMax`
      uses the total, not a stage; `stop()` drops staged points. A unit test
      for the small pure helper that turns the marks into the seven stages
      (put it in `perf.ts`, e.g. `stagesOf(marks)`, so `main.ts` only
      collects marks) pins the arithmetic, including `ipc = invoke − read`
      clamped at 0 and `worker = span − decode` clamped at 0.
    - `docs/humans/usage.md` ("Performance graph") and `usage.ja.md`
      describe the stacked bar and name the stages in one sentence each;
      `docs/agents/app-log.md` ("Timing lines") lists the new `page` line
      fields; `docs/humans/performance.md` "Measuring on your own folder"
      updates the `page` line's description (and `performance.ja.md`).
    - Nothing runs while both the overlay and `Timing logs` are off beyond
      a handful of `performance.now()` calls and one `u32` in the header.
      `mise run ci` passes.
  - Implementation approach:
    - Follow `focus_crop` / `CropTiming` for the backend timing (an
      `Instant` around `read_preview` inside the closure, `as_micros() as
      u32`) and the `zoom crop=…` line for deriving `ipc`.
    - In the worker, measure `decodeMs` around the `createImageBitmap`
      promise, not around the whole handler; post `{ seq, bitmap, decodeMs }`.
    - Keep the one-in-flight design and the `seq` checks exactly as they
      are; this step must not change when requests are issued.
    - Do not fix the `slice` copy here even if it is obviously wasteful;
      Step 2 decides from numbers. Record its cost as `post`.
    - The stacked bar only needs a loop over the stage array inside the
      existing `for (const point of latency)` in `drawPerf`.
- [x] Step 2: Measure on Windows, write the numbers and the decision
  - Done when:
    - `results.md` in this folder holds, per scenario below, n and p50 /
      p95 of `keypressToPixels` and of each of the seven stages, computed
      from the `page` lines of `Riffle.log` (an ad-hoc script in the
      scratch directory, not committed), with the conditions written next
      to every table as `docs/agents/tauri-app.md` "Write a performance
      number with its measurement conditions" demands: build, folder, file
      count, body / preview size, drive, which scan pass was running, warm
      or cold, and how coldness was established.
    - Scenarios (each is one short action by the user; the agent prepares
      the build, the folders and the log reading):
      1. **Idle, warm, single presses**: a folder the app scanned before,
         with its scan finished (status line idle), the overlay on (`d`),
         `Timing logs` on; press `→` about once a second, 30 times.
      2. **Idle, warm, key held**: same folder; hold `→` for about 5 s.
         This is the only scenario where `wait` is expected to be nonzero.
      3. **Cold, during pass 1**: an unopened folder under `D:\Photos\2026`
         on D: (the SATA SSD; a folder the read-gate plan's Step 2 has not
         used either); open it and immediately press `→` about once a
         second for 30 s while the status line still shows the first pass.
      4. **Warm, during pass 2 / 3**: in the folder of 3, once the status
         line reads "analyzing", press `→` about once a second, 20 times.
      5. **Large previews, idle, warm**: `D:\photos\samples\RAF`
         (4416x2944 / 4000x3000, 1.3-5.5 MiB, past the prefix), 20 presses
         after its scan finished; if `D:\photos\samples\CR3` holds an HDR
         PQ file (an `hevc::to_jpeg` candidate), 10 presses across it and
         its neighbours too.
      6. **Cold, idle** (optional, needs admin): after 3 and 4 finish and
         the app is closed, empty the standby list with Sysinternals RAMMap,
         reopen the same folder, and press `→` 20 times once the deferred
         rescan's status line is idle. Skipped if RAMMap is not at hand;
         `results.md` then says so.
      The total is under 15 minutes of the user's time. Scenarios 1-4 run
      in the `mise run tauri:release:devtools` build (the `Timing logs`
      item shows only there or in debug), never the debug build.
    - `results.md` ends with a "Decision" section that applies the rules
      below to the numbers, says which of Steps 3-5 run (or none), and
      states the native-rendering verdict with the two numbers it rests on.
      This `plan.md` is updated in the same PR: the conditional steps that
      will not run are struck through with a one-line reason; the ones that
      will run keep their checkbox.
    - `docs/humans/performance.md` "Per-page preview read" (and
      `performance.ja.md`) gains the idle-warm table (scenario 1 and 5) and
      a sentence on the pass-1 case, replacing "it has not been measured
      yet", and `todo.md`'s "App: unmeasured end-to-end per-page latency"
      item is ticked with a pointer to the section.
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 1 is merged. The agent builds the devtools release,
      tells the user the six actions in order, and reads
      `%LOCALAPPDATA%\com.minodisk.riffle\logs\Riffle.log` after each
      (the 1 MB cap deletes the file when passed, so read between
      scenarios, not at the end).
    - Targets (approved by the user): idle warm on a ~1616x1080 preview,
      `keypressToPixels` p50 ≤ 50 ms and p95 ≤ 100 ms; large previews (RAF)
      p95 ≤ 150 ms; during pass 1 p95 ≤ 250 ms (the read-gate plan's figure).
    - Decision rules, applied in this order, each on the scenario it names:
      - **R0, nothing to do here**: every target is met in scenarios 1, 2,
        4 and 5, and scenario 3's p95 is explained by `read` (the disk
        queue the read-gate plan is fixing) → no conditional step runs;
        the plan ends with Step 2.
      - **R1, key held waits behind stale requests** (scenario 2): `wait`
        p50 ≥ 20 ms or ≥ 25 % of `keypressToPixels` → Step 3.
      - **R2, the hop is the cost** (scenarios 1 and 5): `ipc + post +
        worker` p50 ≥ 15 ms or ≥ 30 % of `total` → Step 4 (prefetch hides
        every stage but `draw` for the neighbour already in the ring; the
        `post` copy is removed in Step 4 as well by posting the buffer with
        an offset). If `post` alone is ≥ 5 ms on RAF, Step 4 is on
        regardless.
      - **R3, the decode is the cost** (scenario 5, and 1 if it applies):
        `decode` p50 ≥ 30 ms on RAF or ≥ 15 ms on a 1616x1080 preview →
        Step 5 (and Step 4 if not already chosen: a decoded neighbour costs
        no decode on the page turn).
      - **R4, the draw is the cost**: `draw` p95 ≥ 16 ms → add to whichever
        step runs a sub-item that measures `drawImage` without the marks
        (`f` off) and, if the marks are the cost, moves them to their own
        overlay canvas; if `drawImage` itself is, tries a
        `bitmaprenderer` context with the worker pre-scaling to the canvas
        size. Not a separate step: it is one change in `draw()` and is
        written into Step 4's or 5's Done-when at that time.
      - **R5, native rendering** (the stay-on-Tauri criterion): a native
        surface or GPUI can remove `ipc + post + worker + draw` (the decode
        stays: it is the same libjpeg either side) and replace it with a
        blit of about 1 ms. So native rendering is considered **only if**,
        after the in-Tauri steps chosen above have run (or R2-R4 chose
        none because those stages were already small), scenario 1's p95 of
        `ipc + post + worker + draw` still exceeds 50 ms while `read` p95
        is under 10 ms. If that sum is under 20 ms at p95 at any point,
        write in `results.md` that a rewrite cannot win more than that and
        close the question. If R5 fires, the next action is a separate
        plan whose first step is a spike (a bare native window drawing the
        same decoded preview on a key, measured the same way), not a step
        of this plan.
      - Every rule that fires is quoted with its number in `results.md`'s
        Decision; a rule that does not fire is listed with the number that
        kept it off.
- [ ] ~~Step 3 (conditional on R1): Let a newer page turn supersede an in-flight preview request~~ not run: R1 did not fire (`wait` p50 1.3 ms in scenario 2)
  - Done when:
    - Holding a paging key no longer waits a whole round trip for a
      preview that will be dropped: either (a) the backend takes a request
      id, stores the latest per the pattern of `eyes_of` (`docs/agents/tauri-app.md`
      "An on-demand per-file command takes its request id from the frontend
      and stores the latest") and returns early with `superseded` when a
      newer one arrived before its read started, or (b) the frontend allows
      two requests in flight and drops the older response. The step picks
      one by measuring scenario 2 again with each (a branch build each),
      and writes both numbers to `learnings.md`.
    - The existing invariant holds: a response for a `seq` that is no
      longer current never draws, and `inFlight` / `requestPreview`'s
      re-request semantics are either kept or replaced by a tested
      equivalent (a unit test on a small pure module if the logic leaves
      `main.ts`).
    - Scenario 2's `wait` p50 drops below the R1 threshold in a re-run
      recorded in `results.md`. `mise run ci` passes.
  - Implementation approach:
    - Assumes Steps 1-2 merged. Keep the `pageTimings` / `seq` bookkeeping
      working so the breakdown stays valid for the chosen design.
- [x] Step 4 (conditional on R2 or R3): Prefetch and keep the neighbours' decoded previews
  - Runs by the user's decision despite R0, because fast display is the app's top priority; see "Decision" in `results.md`. Expected gain: no `read`, `ipc` or `decode` on a hit.
  - Done when:
    - A new `crates/app/ui/src/prefetch.ts` holds a pure ring model: given
      the current index, the paging direction and the file list, which
      indices to hold (±1 by default, the next in the paging direction
      first), capped by total pixels (`PREFETCH_PIXEL_CAP`, a constant
      with a doc comment giving the arithmetic: a 1616x1080 bitmap is
      7 MB, a RAF one 52 MB) rather than by count; eviction order; and
      invalidation on a file list change, a rename, a delete and a folder
      switch. `prefetch.test.ts` covers those.
    - `main.ts` issues a prefetch only when no preview request is in
      flight and the current preview has drawn, through the same `preview`
      command and the same worker, and a page turn to a held index draws
      from the ring with no invoke; its `page` line then reads
      `source=prefetch` with `read=0 ipc=0 post=0 worker=0 decode=0`, so
      `results.md` can tell hits from misses.
    - The `post` copy is gone for every preview: the whole payload buffer
      is transferred to the worker with the header length as an offset
      (`new Blob([new Uint8Array(buffer, PREVIEW_HEADER_LEN)])`).
    - `ImageBitmap`s evicted from the ring are `close()`d; the WebKitGTK
      pixel limit (`PREVIEW_PIXEL_LIMIT`) still applies to prefetched
      bitmaps.
    - Scenario 1 and 5 re-run in `results.md` with the hit rate and the
      hit / miss p50 / p95 side by side; `docs/humans/usage.md` and
      `usage.ja.md` say the neighbours are read ahead; `CLAUDE.md`'s
      layout paragraph names `prefetch.ts`. `mise run ci` passes.
  - Implementation approach:
    - Assumes Steps 1-2 merged (and 3 if it ran: the ring must respect
      the chosen in-flight design).
    - A prefetch read pauses the scan exactly as a preview read does
      (same command). Whether to prefetch while pass 1 runs is decided
      from scenario 3's numbers and written in this step's PR: the default
      is to prefetch only while no scan pass is running, so the thumbnail
      fill is not slowed by reads the user may never look at.
    - A ring hit must still run `draw()` and the marks path; the
      `orientations` map entry comes from the cached header.
- [ ] ~~Step 5 (conditional on R3): Cut the decode of large previews~~ not run: R0 fired and the user chose prefetch only
  - Done when:
    - For a preview over a pixel threshold the step sets from scenario 5
      (e.g. 6 MP), the path decodes to about the viewport size instead of
      full size. Two candidates are measured on `D:\photos\samples\RAF`
      before one is kept, both numbers in `learnings.md`: (a) the worker
      passes `resizeWidth` / `resizeHeight` to `createImageBitmap` (the
      existing Linux path, `fitWithin` in `decode.ts`) for every platform
      above the threshold, if WebView2 decodes faster with it; (b) the
      backend decodes DCT-scaled (`riffle_core::decode`'s scaled decode,
      as the whole-image face search does) and ships RGBA like
      `focus_crop`, trading IPC bytes for decode time per
      `docs/agents/tauri-app.md` "Measure before choosing a JPEG payload
      over raw pixels" (do not re-encode a JPEG: measured slower than the
      decode it follows).
    - The 1:1 zoom (`focus_crop`) and Compare are unaffected; a preview
      under the threshold takes the unchanged path.
    - Scenario 5 re-run in `results.md` meets the RAF target or the step
      records how far off it stays and why. `docs/humans/performance.md`
      (and `.ja.md`) gets the before / after row. `mise run ci` passes.
  - Implementation approach:
    - Assumes Steps 1-2 merged, and 4 if it ran (a prefetched large
      preview takes the same reduced decode).
    - Measure candidate (a) first: it is a few lines and needs no new
      payload kind. Candidate (b) needs a new kind and a header with the
      size, following `crop_payload`.

## Trade-offs and risks

- **Stacked bar vs numbers in the overlay.** The plan stacks the existing bar
  by stage (readable while paging, no new UI surface) and leaves exact
  numbers to the `page` line, which needs a devtools / debug build. The
  alternative, printing p50 / p95 per stage in the overlay header, would
  give numbers in a plain release build but is a second display to keep
  tidy; it can be added later if the user wants numbers without a dev
  build.
- **Header word reuse vs a longer header.** `read_us` goes into the 8-byte
  header's reserved word under a new kind tag, so the thumbnail payload and
  `strip.ts` are untouched. If a second backend number is later wanted (the
  blocking-pool wait before the closure runs), the kind bumps again with a
  longer header, as `focus_crop` did twice.
- **The targets.** 50 / 100 ms idle warm, 150 ms RAF, 250 ms during pass 1.
  A 60 Hz frame is 16.7 ms; the zoom path once used a 50 ms budget.
  Tightening below the sum of `read + decode` for a warm 1616x1080 preview
  (about 5-10 ms per `performance.md`) is unreachable in any architecture.
- **Native rendering.** The ceiling of a native rewrite is `ipc + post +
  worker + draw` minus a blit; the decode stays. Against that ceiling
  stands a port of about 15k lines of TypeScript and 48 test files to a
  pre-1.0 toolkit. R5 makes the comparison with two measured numbers rather
  than a feeling; the plan does not commit to any native work and sends a
  positive R5 to a new plan beginning with a spike.
- **Prefetch and the scan.** A prefetch read is a preview read to the disk
  and to `ReadSlots`, so during pass 1 it slows the thumbnail fill, which
  is also a top priority. Step 4 defaults to prefetching only when no pass
  runs and lets scenario 3's numbers argue otherwise. A ring of decoded
  bitmaps also holds memory (7 MB per ARW preview, 52 MB per RAF), hence
  a pixel cap, not a count.
- **Measurement noise.** One machine, one SATA SSD, one hand on the keys.
  Step 2 records conditions with every table, keeps the debug app out of
  the runs (devtools release build), and does not average a run whose
  coldness is in doubt. NVMe (C:) is not measured unless a RAW folder
  exists there; `results.md` says so.
- **Conflicts.** The read-gate plan's Steps 2-3 change `READ_SLOTS` and
  `scan.rs`; this plan touches neither. The full-size-eye-check plan
  (branch `docs/full-size-eye-check-plan`) changes pass 2, `candidate.rs`
  and `focus.ts`; this plan touches none of them. Both may land in any order.
- **Step 1 widens the `page` line.** Any script or note that parsed the old
  `page invoke=… decode=…` form (none found in the repo) would break; the
  docs that describe it are updated in the same PR.

## Progress

- (2026-11-08) Step 1 complete
- (2026-11-08) Step 2 complete
