# Learnings: preview-latency-breakdown

## Step 1

- `requestPreview` used to drop the keypress mark of a key that found a
  request in flight, so a held key would have logged no `wait` at all and
  scenario 2 of Step 2 could never fire R1. The mark is now kept in
  `queuedKeypressAt` for the re-request that serves that key. When requests
  are issued is unchanged.
- The plan's "`preview_payload_header_encodes_kind_one_and_orientation`" is
  called `payload_header_encodes_kind_and_orientation` in `commands.rs`; that
  test now covers `PREVIEW_KIND_JPEG_V2` (tag 6: 3 and 4 were the old crop
  kinds, 5 the current one).
- The third pass has no flag of its own: `embedPassScanId` is set at
  `faces-done` and cleared at `embed-done`, which the backend emits even when
  it skips the pass, so `scan=3` covers only a running pass.
- A switch no key timed gets `wait` measured from `switchAt` so its stacked
  bar still reaches its height; the `page` line omits `wait` for it.
- Compare also reads the `preview` payload (`loadCompare`), so its kind check
  moved to V2 together with `requestPreview`'s.

## Step 2

- Measured by the user; the numbers and the Decision are in `results.md`.
- Scenario 1's and 2's folder and body were not captured with the logs, and
  the `page` line carries no path, so `results.md` says so rather than naming
  one. Scenario 1 got 20 keyed presses of 30 planned, scenario 4 got 16 of 20.
- Pass 1 is hard to hit by hand: two attempts finished it before any key
  landed in it, and the third gave 3 keyed lines. A re-run needs a folder with
  far more files to do.
- RAF idle (n=11) mixed-size p95 is 113.0 ms by nearest rank, the method every
  other figure here uses. The figure 103.5 ms in the hand-off notes for this
  step is the second-largest sample, so `results.md` uses 113.0. The target of
  150 ms is met either way.
- R5's letter would close the native question at 8.5 ms, but
  `keypressToPixels` ends at the canvas draw, so the user keeps it open for a
  separate GPUI spike. `results.md` records both.

## Step 4

- `requestPreview` gained an early return for `shown.seq === seq`: a ring hit
  draws while an older real request may still be in flight, and that request's
  stale-reply path would otherwise re-request and draw the same page twice.
- `plan` drops every non-neighbour, and the file about to be shown is no
  longer a neighbour, so `show()` must `take` before `plan` or it closes the
  bitmap it is about to draw. `prefetch.ts` says so on `plan`.
- The prefetch is deliberately not in the `inFlight` flag: a page turn issues
  its own request alongside one. The cost is a duplicate read when a key lands
  on a neighbour whose prefetch has not returned; the reply is then closed
  because the page is no longer a neighbour of itself.
- A prefetch's worker reply is told from a page's by an echoed `prefetch` flag,
  not by `seq`, so the two id spaces cannot collide.
- A re-listing clears the ring (`resync`), because it is the only signal that
  a file's bytes may have changed under the same path; a filter or sort change
  only `retain`s.
- `refused` holds failed reads until forgotten. `crowded` holds the files evicted
  for space and lasts only while they stay wanted (`plan` drops them once the
  page has moved past), so two neighbours that cannot share the cap (24 MP JPEGs
  of a view-only folder against 32 MP) cost no discarded read per page turn,
  and a file is retried when the position comes back to it. A RAF is 13 MP of
  the 32, so RAW folders never reach this.
- Prefetch only while no scan pass runs (the plan's default): the measurement
  gave no argument against it. Pass 1 and 2 reads are not slowed by it.

## Deferred issues (todo candidates)

- **Pending manual check**: the stacked latency bar and its legend in the
  overlay. Platform: Windows (the running app). Steps: open a folder, press
  `d`, page with `→`. Expected: each red bar is stacked in seven shades of
  red, the legend names `wait` to `draw` after `Latency` and does not collide
  with the MB/s / ms text at the right on a normal window width. The step's
  checkbox was ticked on the automated criteria; drawing a canvas was not
  exercised here. Basis: Step 1's overlay Done-when, `drawPerf` in
  `crates/app/ui/src/main.ts`.

- **Pending manual check**: Step 4's scenario 1 and 5 re-run for `results.md`
  (the last open Done-when of Step 4; the checkbox was ticked on the automated
  criteria). Platform: Windows, a `mise run tauri:release:devtools` build,
  `Timing logs` on, overlay on (`d`). Steps: (1) in a scanned folder of
  1616x1080 previews with the status line idle, press `→` about once a second,
  30 times; (2) in `D:\photos\samples\RAF` after its scan is idle, press `→`
  20 times; read `%LOCALAPPDATA%\com.minodisk.riffle\logs\Riffle.log` between
  the two. Split the `page` lines by `source=prefetch` / `source=read` and
  write the hit rate and the hit / miss p50 / p95 of `keypressToPixels` side by
  side in `results.md`. Expected: after the first press every press is
  `source=prefetch` with `read`, `ipc`, `post`, `worker` and `decode` at
  0.0ms and `keypressToPixels` of a few ms (the `draw`, about 2 ms at
  1616x1080 and up to about 15 ms on a 4416x2944 RAF); no stale image after a
  rename, delete or folder switch. Basis: Step 4's Done-when, `drawHeld` in
  `crates/app/ui/src/main.ts`.
