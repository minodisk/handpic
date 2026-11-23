# Learnings

## Step 1: the quit waits for a retired scan

- Shape chosen: `scan::Running` now holds the held scan and a list of
  `Retired` entries (id, cancel flag, the `ended` receiver).
  `Running::retire` splits the held `Scan`: the `Retired` part stays in the
  holder, the `Stopping` part (id, cancel flag, `JoinHandle`, a `Running`
  clone) goes to the re-listing's background task, whose `Stopping::stop`
  joins and then drops the entry by id. `Scan::stop` had no other caller and
  was removed.
- `Running::stop_within` takes both out under the lock and waits outside it,
  so the background `Stopping::stop`'s removal never blocks on the quit's
  wait. It sets every retired scan's cancel flag before waiting on any, and
  one deadline covers the held and retired waits.
- The quit line is now `quit: scan stopped=<bool> retired=<none|bool> in
  <ms>ms`; `stopped=` keeps its meaning (the held scan, true with none held).
- The holder test fakes the scan thread (a thread blocked on a release
  channel, built from `Scan`'s private fields in the child test module) so
  the wait is observable without a real-time bound: the only timing in it is
  a 100 ms "not returned yet" check, which a loaded machine can only make
  pass, not fail. The test first waits until the quit's stop has taken the
  retired entry out of the holder, otherwise the background stop could drop
  the entry first and the quit would see none.
- First full `mise run ci:local` (CARGO_BUILD_JOBS=4) built and ran all
  1103 app tests. Only `scan::tests::a_bounded_stop_reports_an_ended_scan_stopped`
  failed (`held: false`: a real scan did not end within the 2 s
  `STOP_TIMEOUT` while other sessions were building), the flake `todo.md`
  already lists. Its held path (`Scan::stop_within`) is unchanged by this
  step. The two new holder tests passed. A re-run of the scan tests alone
  was killed for low memory.
