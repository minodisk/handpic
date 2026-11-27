# Learnings

## Step 1

- The watchdog's `on_fire` gets only the phase line (`watchdog_report`, a
  pure function of the label, the elapsed time and the limit), not the log
  lines. The production closure (`abort_with`) writes and flushes that line
  first and only then calls `log_lines`, which takes the `LOG` mutex, so the
  phase reaches stderr even if the hung thread holds that lock. The tests
  check the report through a channel without the log lines.
- `Watchdog::start` takes the first phase's label, so the firing test needs
  no `set` racing a 20 ms limit.
- The cause of the hang was not found while implementing; the `todo.md`
  section now waits for the watchdog's first report from CI.
- `Ui::read` is not instrumented: a hang inside a direct `read` or `ui.cx`
  call is reported as `test body, after <last phase>`, as option A intends.
- The first local `mise run ci:local` failed 7 UI tests on `QUIT_MAX`
  (`the quit handlers took 1.5s` to `4.0s`) while the whole crate's 1184
  tests ran in parallel on the shared host; the UI tests alone passed and
  the second full run passed. The same bound flaked before
  (`docs/plans/_archived/20261122-held-rename-access-denied/learnings.md`,
  `docs/plans/_archived/20261127-macos-ci-bundle-reuse/learnings.md`);
  the watchdog does not touch the quit path beyond setting its phase.
