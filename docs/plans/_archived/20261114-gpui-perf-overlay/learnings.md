# Learnings

## Step 1

- `scan::Event` is the view's one channel's message type, so the sampler's
  route is a new `Event::Perf` variant in `crates/native/src/scan.rs`, not in
  `main.rs`.
- The unkeyed pages needed less than the plan feared: every selection change
  (a key, a strip click, an open, a refilter, a re-listing) goes through
  `request(at, ..)`, while the chained `send` after a stale result does not,
  so stamping `switched = at` in `request` is the selection-change mark. The
  `pending` slot became `Option<(Page, bool)>`, the flag saying whether the
  page was keyed; an unkeyed one is built only while the overlay is on, so the
  off path prints and records exactly what it did before.
- Editing files with Python's text mode on Windows wrote CRLF line endings
  into LF files (`main.rs`, `scan.rs`, the parent plan). Use
  `open(..., newline='')`, or strip the `\r` afterwards, and check with
  `git ls-files --eol`.
- clippy's `type_complexity` fires on `[(&str, fn(&Stages) -> Duration); 8]`;
  a `type Stage` alias, as `timing.rs`'s `Column`, clears it.
- The model's tests check the insertion prune on the deque directly, because
  `view(now)` prunes again and would hide whether the add pruned.

## Deferred issues (todo candidates)

- Pending manual check (the overlay's look and motion, Windows, release
  build of `crates/native`; the step's checkbox was ticked on the automated
  criteria, `mise run native:check` and the model / layout unit tests): the
  agent could not see the window. Steps: open a folder with
  `mise run native:dev -- <folder>`, press `d`. Expected: a 120 px band along
  the bottom of the preview area, `#171717` at 0.85 alpha, grid lines at 0,
  50 and 100 %; the `Handpic CPU` / `System CPU` (blue, solid 2 px / dashed
  1 px) and `Handpic disk` / `System disk` (green) lines move every 500 ms
  with the newest point at the right edge; the one-line legend at 11 px with
  each line's swatch, `Latency` and the eight native stage names fading, and
  `<n> MB/s  <n> ms` at the right; one red stacked bar per preview switch for
  an arrow key, a strip click and a folder open from the tree; the band sits
  over the fit view and the 1:1 view (`z`) without moving them, and a strip
  click or key works as before. Press `d` again: the band disappears, and on
  reopening it starts empty. With the overlay off, a held-key run's `page`
  lines and the `summary` table look as before the change. Basis: Done-when
  of Step 1 in `docs/plans/20261114-gpui-perf-overlay/plan.md`; files
  `crates/native/src/perfgraph.rs`, `crates/native/src/main.rs`.
