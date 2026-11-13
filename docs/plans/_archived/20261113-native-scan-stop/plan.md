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

# Native: reset the scan status when a scan is stopped, and stop the scan on quit

## Purpose

Two `todo.md` items of the GPUI port (`crates/native`) concern a scan that is
stopped without the view or the process following through:

- `### Native: a failed prepare after a re-listing leaves the stopped scan's status line`:
  `list` takes and stops the previous `Scan`, but `progress` / `analyzing` are
  cleared only where `opened` starts a new scan. When `Scan::prepare` fails
  after a re-listing that interrupted a scan, the status line keeps the
  stopped scan's `scanning` / `analyzing` text and `scanning()` stays true,
  which keeps `focus_scan_soon` arming its timer for a scan that does not exist.
- `### Native: quitting or closing the window does not stop the scan`:
  `Riffle::quit` and `on_window_closed` call `cx.quit()` with no `Scan::stop`,
  so the workers die with the process: a quit mid-pass 2 logs no `scan faces:`
  summary and a write transaction may be cut.

Once done, the status line and `scanning()` always describe the scan that is
actually held, and every quit path (`q`, `--auto`, closing the window) cancels
the scan and waits for it, for a bounded time, so the `scan faces:` line is
logged and no index write is in flight when the process exits.

## Steps

- [x] Step 1: Reset the scan status whenever a scan is stopped, and stop the scan (bounded) on quit and window close
  - Done when:
    - After a re-listing that interrupted a scan, when `Scan::prepare` fails
      (and likewise when the listing itself fails, or there is no index),
      `scanning()` is false and the status line has no `scanning` /
      `analyzing` text. Pinned by a unit test in `crates/native/src/scan.rs`
      on the status model (no GPUI needed).
    - Quitting with `q`, at the end of `--auto`, or by closing the window
      while pass 2 runs logs the backend's
      `scan faces: ... canceled=true ...` line before the process exits, and
      the wait is bounded by a named constant. A unit test covers the bounded
      stop on a real `Scan` (an ended scan reports stopped; an empty holder is
      a no-op). The mid-pass 2 quit itself is a hand check, recorded in
      `learnings.md` with the measured time from quit to the `scan faces:`
      line and to process exit.
    - `docs/agents/native-app.md`: the rule "`--auto`'s quit does not stop the
      scan ..." is replaced, and the quit and scan sections describe the new
      behaviour. `CLAUDE.md`'s `crates/native` `src/scan.rs` description
      mentions the status model and the quit stop.
    - Both `todo.md` items (heading, body and TODO) are removed.
    - `mise run ci` and `mise run native:check` pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/scan.rs` (logic and tests),
      `crates/native/src/main.rs` (wiring only), `docs/agents/native-app.md`,
      `CLAUDE.md`, `todo.md`. No backend or Tauri change is expected.
    - Status reset. Move the two fields `progress` / `analyzing` out of
      `Riffle` into a small pure type in `scan.rs` (for example `Status`),
      with one method per transition that `main.rs` performs inline today:
      scan started with `total` (`progress = Some((0, total))`,
      `analyzing = None`), pass 1 progress, pass 1 done (`progress = None`,
      `analyzing = Some((0, 0))`), pass 2 progress, pass 2 done, a
      `clear` / stopped transition, `scanning()`, and the status-line suffix
      (`"  scanning {done}/{total}"`, else `"  analyzing {done}/{total}"`,
      strings unchanged). `main.rs` calls these from `opened`, `scanned`,
      `scanning` and `status`.
    - Call the clear in `list`, at the point where it takes the previous scan
      (`self.scan.take()`), not only in the failed-`prepare` arm. `opened` has
      three arms that start no scan (listing `Err`, `(Some(Err(e)), _)`, and
      `_`), and clearing at the take covers all of them. See Trade-offs for
      the visible consequence.
    - Bounded stop. `std::thread::JoinHandle` has no timed join. Give the scan
      thread a `std::sync::mpsc::Sender<()>` that it only drops when it ends,
      and keep the `Receiver` in `Scan`. A new `Scan::stop_within(timeout) -> bool`
      sets `cancel`, then calls `recv_timeout`: `Disconnected` means the
      thread ended (also on a panic), so join and return true; `Timeout` logs
      a warning and returns false, leaving the thread to die with the process.
      Keep `Scan::stop` (unbounded) for `list`: a re-listing must not
      reconcile while the old scan still writes.
    - Reaching the scan at quit. The quit work must run in the app-level
      `App::on_app_quit` handler that `Riffle::new` already registers for the
      sidecar writer, not in `Riffle::quit`. `docs/agents/native-app.md` ("The
      writer drains in `App::on_app_quit`") records that closing the window
      can release the view before quit handlers run, that the handler body
      runs synchronously, and that only the returned future is cut at 200 ms.
      One handler there covers `q`, `--auto` and window close alike. So the
      held scan must be shared: replace `scan: Option<Scan>` in `Riffle` with
      a cloneable holder defined in `scan.rs` (for example `Running`, an
      `Arc<Mutex<Option<Scan>>>` with `put`, `take`, the held scan's `id`,
      `focus(paths)` and `stop_within(timeout)`), and clone it into the
      handler. The handler is registered only when a writer exists, which is
      the same condition (an index) under which a scan can exist.
    - Order in the handler: stop the scan first, then the existing judgment
      store drain and writer flush; log the outcome on the existing `quit:`
      line or a second one (for example `scan stopped=<bool> in <ms>ms`).
      `Riffle::quit` keeps printing the summary table and calling `cx.quit()`.
    - Measure how long the stop takes mid-pass 2 before fixing the constant.
      After `cancel`, no new file starts; each of the `faces_threads()`
      workers (an eighth of the cores) finishes its current stage of
      `extract_analysis` (file read, preview decode, YuNet, face mesh, the
      full-size JPEG read and decode for `full_eye_sharp`), and waits on
      `ReadSlots` and `yield_to_pause` poll `cancel`. Then `run_faces_scan`
      commits what finished and logs `scan faces:`. The expectation is well
      under a second on local disks; a slow or network volume could hold one
      read longer, which is what the bound is for. Start from 2 s (the same
      as `sidecar::DRAIN_TIMEOUT`) and adjust from the measurement.
    - Hand check: a folder large enough that pass 2 outlasts the quit (for
      the sizes, see the rule in `docs/agents/native-app.md` about how many
      files keep a run inside pass 2; hardlinked padding works). Put it under
      `D:\Photos\tests\<date>-native-scan-stop\` with a fresh `--index` file
      in that folder. Run `--auto N` and, separately, close the window by PID
      with `CloseMainWindow` (never for every `riffle-native` process; kill
      nothing). Read stderr for `scan faces: ... canceled=true` and the
      `quit:` line. No screenshots. Run `cargo build` in `crates/native`
      first (`native:check` does not build the exe); NASM must be on `PATH`
      on Windows.
    - A quit during pass 1 logs `scan extract: ... canceled=true` and no
      `scan faces:` line, because `run_faces_pass` returns early when `cancel`
      is already set. That is the existing behaviour of a canceled pass 1 and
      is not changed here.
    - Docs: in `docs/agents/native-app.md`, replace the measurement rule
      "`--auto`'s quit does not stop the scan ..." with what now holds (an
      `--auto` quit inside pass 2 cancels it and logs `scan faces:` with
      `canceled=true` and the `done` count; the neighbouring rule that a warm
      run must end its last pass uncanceled still applies). Extend "The
      writer drains in `App::on_app_quit`" and "Pass 2 runs on pass 1's
      thread" with the bounded stop and the shared holder.

## Trade-offs and risks

- **Where to clear the status.** Clearing at `list`'s take (planned, approved)
  fixes all three no-scan arms of `opened`, but the `scanning` / `analyzing`
  text disappears for the duration of the stop, listing and reconcile, until
  the new scan sets it again. Clearing only in the failed-`prepare` arm keeps
  the old text on screen during the re-listing but leaves the same stale state
  when the listing itself fails (the `Err` arm of `opened`).
- **A scan being stopped by a re-listing at the moment of quit.** `list` moves
  the previous scan into a background task that calls `Scan::stop`, so the
  holder is empty then and the quit handler does not wait for it. The process
  can exit while that scan is still winding down, possibly before its
  `scan faces:` line. Left out as rare and more state.
- **Bound versus completeness.** With a timeout, a stop that outlasts it
  (slow or network volume) exits as today: no summary, workers killed
  mid-file. Without one, quit could hang for as long as one read takes. The
  plan takes the bound. The window may stay on screen, unresponsive, for up
  to the scan bound plus the two existing 2 s drains in the worst case.
- **"Index closed cleanly".** The plan guarantees that no scan write is in
  flight at exit when the stop succeeds. It does not add an explicit close or
  WAL checkpoint of the `Index`: the view and the writer still hold `Arc`s to
  it, and the process exit ends them as today. An explicit close would be a
  separate backend change.
- **Refactor size in `main.rs`.** Moving `progress` / `analyzing` into a
  `scan.rs` type and `scan` into a shared holder touches every use site in
  `main.rs` (about a dozen lines). It is what makes the acceptance test
  possible without GPUI and what lets the app-level handler reach the scan.

## Progress

- (2026-11-13) Step 1 complete
