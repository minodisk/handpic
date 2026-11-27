<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# UI test harness watchdog

## Purpose

CI's `app (ubuntu-latest)` job hung in `uitest::opens_with_no_folder` (run 45091844682 attempt 1, job 137424185183): libtest logged `has been running for over 60 seconds` at 01:17:55 and the job's 30-minute limit cancelled it at 01:32:39 with no other output. The rerun passed. Every wait the harness `crates/app/src/uitest.rs` owns is bounded (`wait_until` panics at `WAIT`, 10 s; `remove_within` ends at `WAIT`) and the quit handlers in `crates/app/src/main.rs` and `crates/app/src/mcp.rs` all wait with a timeout, so the hang was inside a GPUI call (`TestAppContext::single`, `open_main`, `run_until_parked`, `read_with`, `painted_quads`, the quit, the context's drop) or on a lock, and the log says nothing about which.

This work adds a watchdog thread to each `Ui`. When one harness phase runs past 60 s it writes the phase, the elapsed time and that `Ui`'s log lines straight to stderr and aborts the process. The next hang then costs about a minute instead of the job limit, and its log names the phase, which is what finding the cause needs.

## Steps

- [x] Step 1: Add the per-`Ui` watchdog to the UI test harness and rewrite the `todo.md` section
  - Done when:
    - Each `Ui` starts a watchdog thread in `Ui::open_with` and stops it when the `Ui` is dropped, on the panicking drop path too.
    - The watchdog's phase (a label and its start `Instant`) is set at each stage of `open_with` (`TestAppContext::single`, the index and settings, `open_main`, the scan priorities update, the first `run_until_parked`), in `wait_until`, in `update`, and in the drop's quit and folder removal.
    - A phase older than 60 s makes the watchdog write the phase label, the elapsed time and that `Ui` folder's `log_lines` with `std::io::stderr().write_all` and then call `std::process::abort()`.
    - A test with an injected short limit shows the watchdog firing with a report that names the phase, without aborting the test process, and a watchdog dropped before its limit not firing. If this turns out to need more than a small seam (see the approach), drop the test and say so in `learnings.md`.
    - The `todo.md` section "App: `uitest::opens_with_no_folder` hung on `app (ubuntu-latest)`" holds only the remaining work: find and fix the cause from the watchdog's next report. If the cause is found and fixed in this PR instead, the section is removed.
    - `docs/agents/app.md`'s harness entry and `CLAUDE.md`'s `src/uitest.rs` description mention the watchdog, each briefly.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the `app` jobs on all three platforms run the UI tests).
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/uitest.rs` (the watchdog and its test), `todo.md`, `docs/agents/app.md` ("The main view opens on GPUI's test platform"), `docs/agents/github-actions-workflows.md` ("A job that hits its timeout", one sentence: a UI test now aborts with the watchdog's report, which names the phase), `CLAUDE.md` (the `Ui` parenthetical of `src/uitest.rs`, a clause). Do not touch the bodies of the other UI tests, here or in `crates/app/src/folderops.rs`.
    - Write the report with `std::io::stderr().write_all`, not `eprintln!`: libtest's output capture is inherited by spawned threads and is lost on an abort. Write and flush the phase line first and only then gather `log_lines`, so the phase still reaches the log if the `LOG` mutex is the lock the hung thread holds.
    - Shape that makes the test possible without an abort: the watchdog takes its limit and what to do on firing as arguments (for example `Watchdog::start(dir_name, limit, on_fire)` with `on_fire: impl FnOnce(String) + Send + 'static`). `open_with` passes 60 s (a named constant next to `WAIT`, with a comment saying why: a normal phase takes seconds and it is well above `WAIT`) and a closure that writes to stderr and aborts; the test passes a few tens of milliseconds and a channel sender. Keep the report text a pure function of the label, the elapsed time and the log lines.
    - Stop signal: a channel whose sender the `Ui` owns works without a flag or a join. The thread waits with `recv_timeout` for the time left in the current phase, re-reads the phase when that expires (it may have been replaced meanwhile) and ends on `Disconnected`. This needs no poll interval, fires promptly under a short injected limit, and lets the "does not fire" test end on `Disconnected` (the thread drops the closure and its sender on exit) rather than on a sleep.
    - The phase is shared state (`Arc<Mutex<..>>`), so it can be set through `&self`.
    - Field order matters: Rust drops fields in declaration order after `Drop::drop` returns, and `Ui`'s `cx` (the `VisualTestContext`) is dropped there. Declare the watchdog field last and set a final phase (for example "dropping the test context") at the end of `Drop::drop`, on the early `thread::panicking()` return too, so a hang in the context's own drop is reported as well.
    - `wait_until`: set the phase once on entry (label including `what`), not per loop turn. `WAIT` already bounds the loop at 10 s, so a phase over 60 s there means a call inside it hung.
    - Between harness calls (option A, chosen by the user): on return from `open_with`, `wait_until` and `update` the phase becomes a "test body, after <last phase>" label with a fresh start, so a hang in a direct `ui.cx` call is caught and reported as such.
    - `todo.md`: other sessions edit the file. Rebase onto main right before editing, change only this section, and keep its heading. In the Background keep the run and job ids and the pointer to `docs/agents/github-actions-workflows.md`, add that the harness now aborts with the phase after 60 s, and that what was ruled out is in this plan (name it by its archived path, `docs/plans/_archived/20261128-uitest-watchdog/plan.md`, in backticks). The TODO item becomes: when the watchdog next fires in CI, read the phase and the log lines from the job log, find the cause and fix it.
    - If the cause does become clear while implementing, fix it in this PR and remove the `todo.md` section instead. Do not try to reproduce the hang by generating load locally (`CLAUDE.md` forbids it).

## Trade-offs and risks

- **The cause was not found at planning time.** Checked: the quit handlers (`main.rs` around lines 800, 869 and 942, `mcp.rs` around line 150) wait with `stop_within`, `drain`, `flush` or `recv_timeout`, all with timeouts; `wait_until` and `remove_within` are bounded. Unbounded waits that exist but are not shown to be on this test's path: `scan::Eviction::wait` (`thread.join()`, called from the listing in `main.rs` around line 3340; this test opens no folder, and the test itself only polls `is_finished`), `scan::Stopping::stop` (`thread.join()`, `scan.rs` around line 460), and the settings `Writer`'s drop (`thread.join()`, `settings.rs` around line 110), which runs when the last `Writes` handle goes, possibly inside the `TestAppContext`'s drop. That last one is the reason the plan covers the context's drop with a phase. None of these is evidence, only where to look once a report names a phase.
- **What the phase is between harness calls.** `Ui.cx` is public and the tests call `ui.cx.simulate_keystrokes`, `ui.cx.update`, `ui.cx.run_until_parked` and `ui.read` directly. Option A (chosen): on return from each instrumented call the phase becomes "test body, after X" with a fresh clock. Consequence: a test whose own code legitimately runs more than 60 s between two harness calls would be aborted; none should, since each wait is bounded by `WAIT`.
- **An abort ends the whole test binary.** The other tests running in parallel in that process lose their results and their captured output, and cargo reports a signal rather than a failed test. This is accepted: the alternative is the 30-minute cancel with no output at all. A panic from the watchdog thread would not help, since it cannot unwind the hung test thread.
- **60 s on a slow runner.** A phase that is merely slow (cold cache, many tests in parallel) and passes 60 s would abort a run that would otherwise have passed. No measured phase is near that: `wait_until` is capped at 10 s and the quit is asserted under 1 s. If CI shows a false abort, raise the constant rather than removing the watchdog, and record the phase and its time in `learnings.md`.
- **The firing test depends on real time.** It can only be late, never early, so load makes it slower, not flaky, provided its own wait for the report is generous (seconds, against a limit of tens of milliseconds).

## Progress

- 2026-11-28: Step 1 done (Watchdog in crates/app/src/uitest.rs with its two tests, todo.md section narrowed to the remaining work, docs/agents/app.md, github-actions-workflows.md, CLAUDE.md)
