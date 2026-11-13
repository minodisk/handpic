# Learnings: GPUI prefetch ring

## Step 1

- `prefetch::Pixels` clashes with `gpui::Pixels` in `main.rs`'s imports, so
  `main.rs` implements it as `impl prefetch::Pixels for Held` without importing
  the trait.
- `Ring::forget` is used by its own tests, so a bare
  `#[expect(dead_code, ...)]` is unfulfilled under `cargo test` (`-D
  unfulfilled-lint-expectations` via clippy's `-D warnings`). It needs the
  `#[cfg_attr(not(test), expect(dead_code, reason = ...))]` form `settings.rs`
  uses.
- `Loaded` now wraps a `Held` (image, `jpeg_kb`, `px`, `orientation`,
  `Metadata`, focus) plus the read's timings and `Paused`, so a prefetch result
  hands the ring only the `Held` and its `Paused` ends when the result lands.
  The view keeps the shown image's other facts in `OnScreen` (path, `keepable`
  and the rest), and its `Metadata` stays in `shown`; `present` rebuilds a
  `Held` from both when the image is replaced, for `Ring::keep`.
- `hold.rs`, `zoom.rs`, `overlay.rs` and `peaking.rs` needed no change: a hit
  calls the same `marks.show`, `zoom.shown` and `analyze_current` as a read,
  and the peaking layer is keyed by the image id.
- Verified by reading:
  - `retired` is drained only in the preview canvas's paint (its deferred
    callback), so `give_up` calls `cx.notify()` whenever the ring gave
    something up; otherwise an eviction on `settle` with nothing else to draw
    would hold the image (up to 52 MB for a RAF) until the next frame.
  - Retiring an image that was never painted is what a stale `Ok` that is not
    drawn already did (`cx.drop_image` on an image with no texture).
  - Under a held key the pager stays in flight, so `request_prefetch` returns
    at its first check and at most the prefetch started before the hold is out;
    a hit during the hold sets `Pager::hit` only until the next `go`, so the
    number of page reads per key sequence does not grow.
- `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped` failed once in
  four `cargo test` runs of the native workspace with no change near
  `scan.rs` (a timing-bound `stop_within` on a loaded machine); it passed on
  the reruns.

### Measurement (release, `--auto 60`, warm index)

The A/B ran on Windows, release build of this step's commit, the 20-file
copies `D:\Photos\tests\20261108-gpui-compare\ARW` (1616x1080) and `...\RAF`
(4416x2944). Indexes, settings store and logs were under
`D:\Photos\tests\20261114-gpui-prefetch\`. One warm run per folder
(`--auto 5`, scan ended `canceled=false`, discarded) came first. Then A
(prefetch) and B (`--no-prefetch`) alternated, three runs each per folder.
Every timed run's scan was already done (`scan faces: ... canceled=false` in
all 12). The script waited up to 300 s for two load samples at or under 15 %
with no `rustc` running before each run. Other sessions kept the machine
busy anyway: ARW runs saw 63-100 % load samples mid-run, RAF runs 9-65 %.
Read the ARW numbers as noisy.

Median of the three runs' p50 / p95, in ms:

| folder | side | `keypressToPixels` | `read` | `decode` | `frameWait` | `paint` | `present` | hits |
| ------ | ---- | ------------------ | ------ | -------- | ----------- | ------- | --------- | ---- |
| ARW | A (prefetch) | 12.82 / 20.10 | 0.00 / 0.86 | 0.00 / 7.58 | 10.31 / 17.56 | 0.92 / 1.66 | 0.58 / 1.45 | 57/60 each |
| ARW | B (none) | 23.40 / 34.05 | 1.45 / 2.98 | 7.84 / 12.88 | 10.45 / 16.84 | 0.91 / 1.67 | 0.47 / 1.03 | 0 |
| RAF | A (prefetch) | 14.13 / 56.27 | 0.00 / 1.65 | 0.00 / 45.16 | 8.73 / 16.76 | 3.68 / 5.33 | 0.41 / 0.75 | 57/60 each |
| RAF | B (none) | 66.15 / 78.95 | 2.32 / 5.35 | 46.63 / 54.49 | 10.64 / 17.33 | 3.52 / 4.57 | 0.34 / 0.50 | 0 |

- Every A run hit 57 of 60 presses. The 3 misses are the wraps from the last
  file to the first (the faithful port prefetches no neighbour across the
  wrap), and they set the p95, which stays at a read's latency.
- A hit leaves `frameWait + paint + present`. RAF's p50 `keypressToPixels`
  falls from about 66 to 14 ms, ARW's from about 23 to 13 ms.
- `paint` did not grow on a hit, though a prefetched image's first paint
  uploads its texture. `present`'s p50 rose about 0.1 ms on both folders.
  The neighbour's decode and orient overlapping the current page's present
  did not show, so the prefetch still starts when the current result is
  shown, not from the paint's deferred callback.
- Memory, the process's peak working set / peak private bytes polled every
  200 ms, in MB:
  - RAF: A 332 / 735, 335 / 825, 349 / 835; B 224 / 534, 239 / 527,
    226 / 489. With prefetch that is about +110 MB working set and +300 MB
    private bytes.
  - ARW: A 121-127 / 215-217; B 104-110 / 195-197, about +17 / +20 MB. Two
    1616x1080 neighbours are 14 MB of BGRA.
- Two 4416x2944 RAF neighbours are 26 MP, 104 MB of BGRA, under the cap's
  128 MB. The working-set growth matches that. The private-bytes growth is
  about three times larger. It covers the decode and orient buffers of the
  prefetch in flight, as well as the GPU textures that kept entries still
  hold (an entry painted before it went back to the ring keeps its texture,
  because only a retired image gets `cx.drop_image`). Whether to release a
  kept entry's texture stays open (Deferred issues).

## Deferred issues (todo candidates)

- **Pending manual check (Windows):** with the release build on a folder
  under `D:\Photos\tests\`, wait for the scan to end, then press `→`
  onto the next file. It should show at once (its `page` line says
  `source=prefetch`). Open another folder from the tree, and separately
  change the filter so the current file's neighbour is hidden. Neither
  should ever show another folder's image or a hidden file's image. Step 1's
  checkbox was ticked on the automated criteria (unit tests, the `--auto`
  A/B above). Basis: plan Step 1's Done-when; `crates/native/src/main.rs`
  (`take_held`, `clear_ring`, the `retain` in `refilter`).
- **Kept ring entries keep their GPU texture.** RAF private bytes grow about
  300 MB with prefetch on, against the cap's 128 MB of BGRA. An image that
  was painted and then handed back through `Ring::keep` holds its texture
  until it is retired. Releasing the texture on `keep` (and uploading it again
  on the next hit) would trade memory for `paint` on a hit. Basis: this
  step's memory measurement; `crates/native/src/main.rs` (`present`),
  `crates/native/src/prefetch.rs`.
- **The wrap is always a miss.** `neighbours` stops at the ends of the list
  (the faithful port of `prefetch.ts`), but the native `Pager` wraps on
  `Next` / `Previous`. So the press from the last file to the first pays a
  full read, and that sets `--auto`'s p95. Making `neighbours` wrap when the
  pager does would make it a hit. Basis: plan Trade-offs; this step's
  measurement; `crates/native/src/prefetch.rs`.
- **Flaky native test.** `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped`
  failed once in four `cargo test` runs on a loaded machine, with no change
  near it, and passed on reruns. Basis: this step's `native:check` runs;
  `crates/native/src/scan.rs`.
