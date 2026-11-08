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

## Deferred issues (todo candidates)

- **Pending manual check**: the stacked latency bar and its legend in the
  overlay. Platform: Windows (the running app). Steps: open a folder, press
  `d`, page with `→`. Expected: each red bar is stacked in seven shades of
  red, the legend names `wait` to `draw` after `Latency` and does not collide
  with the MB/s / ms text at the right on a normal window width. The step's
  checkbox was ticked on the automated criteria; drawing a canvas was not
  exercised here. Basis: Step 1's overlay Done-when, `drawPerf` in
  `crates/app/ui/src/main.ts`.
