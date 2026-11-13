# Learnings: native-scan-stop

## Step 1

- `scan::Status` holds pass 1's and pass 2's `(done, total)` and the status
  line's suffix; `list` calls `clear` where it takes the previous scan, so every
  arm of `opened` that starts no scan (listing `Err`, failed `prepare`, no
  index) leaves `scanning()` false. Pinned by
  `a_stop_leaves_no_scan_on_the_status_line`.
- `scan::Running` (`Arc<Mutex<Option<Scan>>>`) replaces `Option<Scan>` in the
  view; its clone lives in the app-level `on_app_quit` handler, which stops the
  scan before the judgment store drain and the writer flush. The bounded stop
  waits on an `mpsc` channel whose only sender the scan thread drops when it
  ends (`recv_timeout` -> `Disconnected`), then joins.
- Bound: `STOP_TIMEOUT` = 2 s (as `DRAIN_TIMEOUT`), kept after the measurement
  below; it is there for slow or network volumes, not for the local case.
- Hand check (release build, Windows, 24 cores so 3 pass 2 threads), on
  `D:\Photos\tests\20261113-native-scan-stop\photos` (3,360 ARW, 6 hardlinks of
  each of the 560 ARW samples) with a fresh `--index` in that folder each run:
  - Window close by PID (`CloseMainWindow`) 3 s into pass 2, twice:
    `scan faces: ... done=264 / 272 ... canceled=true`, `quit: scan
    stopped=true in 10ms / 6ms`; quit to `scan faces:` line 25 / 29 ms, quit
    to process exit 140 / 53 ms (measured from the PowerShell side by polling
    the stderr file, so a few ms coarse).
  - `--auto 10`: `scan faces: ... done=813 ... canceled=true`,
    `quit: scan stopped=true in 7ms`; from the summary on stdout to the
    `scan faces:` line 28 ms, to process exit 47 ms.
  - `q` goes through the same `cx.quit()` and app-level handler as `--auto`;
    not pressed by hand separately.
- Tripped up: a first `--auto 60` run on an index left from the close run
  quit only after pass 2 had finished on its own (60 presses at
  `AUTO_INTERVAL` ~1 s plus `AUTO_SETTLE` = 62 s, pass 2's remaining 3,066
  files took 41 s), so it logged `canceled=false` and `quit: scan
  stopped=true in 0ms`. Size the presses below pass 2's length.
- `cargo` is not on `PATH` in the Bash tool; run it through
  `mise exec rust@1.98.1 -- cargo ...` in `crates/native`.
