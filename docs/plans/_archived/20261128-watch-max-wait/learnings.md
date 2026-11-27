# Learnings

## Step 1

- The timing moved into `watch::burst`, a pure function over the pending
  `Burst` (folder, first event, deadline), so the test steps a synthetic clock
  with no thread. In that test a report and an event fall on the same instant
  (2 s, 4 s, 6 s): the due report is taken first and the event starts the next
  burst at once, so the reports land exactly every `MAX_WAIT`, not
  `MAX_WAIT` plus the event gap the plan's trade-offs mention (that gap shows
  when the next event comes after the deadline).
- `run` also emits a due report before taking an event that `recv_timeout`
  hands over past the deadline (`Option::take_if`), so a queue that never
  runs dry cannot hold the report back; the test models the same order.
- Only `cargo test -p handpic-engine --lib watch` ran locally; no
  `mise run ci:local` and no `crates/app` build, per the user's preference.
- The `docs/agents/app.md` paragraph on `wait_for_idle` after a confirmed
  import and the `Import…` rules' `Seen` line were left to the main agent,
  which runs the device check and writes them from its result.

## Device check (2026-11-28)

- Ran on Windows 11 after a reboot (cold cache), 100 hard-linked ARWs
  (5.2 GB) imported into a folder on V: at 413 MB/s in 12.6 s: the strip
  went 0 -> 19 -> 70 during the run and 100 after it; `resync:` lines came
  about every 2 s, those during a scan `deferred`. See `results.md`.
- The cadence is bound by the scan, not `MAX_WAIT`: a debug build's pass 2
  and pass 3 on the landed files held the next re-listing for about 6 s, and
  the last 30 files showed 14 s after the run ended.
- An MCP `import_folder` preview is pending only after its read ends; poll
  `get_status` for `pending` before the confirming call, or the confirm is
  refused with `a folder operation is under way`.
- The `cargo build` of `crates/app` was killed once for low memory on the
  shared host; it went through at `-j 2` under a build slot.

## Deferred issues (todo candidates)

- None. Step 1's pending device check was run (see above and `results.md`).
