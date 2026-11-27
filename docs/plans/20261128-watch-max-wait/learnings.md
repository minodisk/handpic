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

## Deferred issues (todo candidates)

- Pending manual check (Step 1's real-device check, Windows 11 host): with the
  destination date folder open, run an MCP `import_folder` of about 125 RAW
  files from a temp copy or hard links of `D:\Photos\tests` (never written
  under it) on a debug `crates/app` build with the `screenshot` feature,
  temp `--index` / `--settings` / `--log` / `--import-settings`,
  `--mcp-port 47830`. Poll `get_view` / `get_status` about once a second and
  take a few `screenshot`s. Expected: the listed file count rises in several
  steps before the import ends, and `Handpic.log` shows `resync:` lines (some
  `deferred`) during the run; record the `import:` line's throughput. Needs
  the Windows host, a local app build, the test photos and MCP tools with
  screenshots plus the log. Step 1's checkbox was left unticked for it; the
  main agent runs it and then writes the app.md `wait_for_idle` paragraph and
  the `Import…` rules' `Seen` line. Basis: plan Step 1; files
  `crates/engine/src/watch.rs`, `docs/agents/app.md`.
