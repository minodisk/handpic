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

# Fix the UI tests' timeout on macOS CI

## Purpose

Five of the six `uitest` tests (every one that opens a folder) time out in
`Ui::open_folder`'s `wait_idle` on the macOS `native` CI job on most runs
(PR #1166 run 44301956697; PR #1171 run 44323630264, attempts 1 and 2), while
Ubuntu and Windows pass every time. A red macOS job on unrelated PRs makes
the UI regression tests a cost instead of a safety net. Once this is done the
macOS job passes them reliably, and a future timeout prints enough of the
app's own log to be diagnosed from the CI output alone.

What the investigation established (from the code and the CI logs):

- The panic state `scan Some(("analyzing", 0, 0)), running false` does not
  mean the scan ended. `running` is `gate.in_flight()` (a trash or folder
  run). `("analyzing", 0, 0)` is `scan::Status::extracted()`: the current
  scan's `Done` (pass 1) arrived and neither a `FacesProgress` nor
  `FacesDone` has. `run_faces_scan` reports after its first file, so pass 2
  had not finished one file when `WAIT` (10 s) ran out.
- A re-listing is ruled out: `list()` calls `scan_status.clear()` and then
  `started()`, which would read `scanning`, and `scanned()` drops the events
  of any scan but the held one. `dir`, `screen` and `current` are all the
  canonical `/private/var/...` path. No path was found that leaves the status
  at "analyzing" after the scan thread ended (every exit sends `FacesDone`),
  so no app status bug is known.
- Timing: in the failing run the other ~1050 tests were done within 1 s, and
  the UI tests then ran alone and still exceeded 10 s each. In a passing
  macOS run the same tests took 2.3 s to about 8.7 s; Ubuntu and Windows take
  about 3 s to 6 s; a local Windows run takes 1.4 s for a three-file folder.
  The macOS result is bimodal, which points at scheduling.
- Leading hypothesis, not confirmed: pass 2's worker runs at
  `Priority::Lowest`, which is `QOS_CLASS_BACKGROUND` on macOS
  (`crates/core/src/scan.rs`, `lower_current_priority`), a class whose CPU
  and disk I/O macOS throttles. Its first file builds the YuNet tract plan
  (`faces::detector`) and runs an inference, unoptimized under the dev
  profile. Pass 1 runs at `QOS_CLASS_UTILITY` and finishes. Linux nice 10
  and Windows `THREAD_PRIORITY_LOWEST` do not throttle an idle core.
- Not ruled out: pass 2 stuck rather than slow (`faces_todo`'s index lock,
  `ReadSlots::acquire`, the file read). `PaintHold` is ruled out: its waits
  are bounded by `MAX_PAUSE` and a held pause would still show
  `analyzing 1/N`.
- A harness defect makes one timeout cascade: `Ui`'s `Drop` returns early
  when panicking, so the app never quits and the scan thread keeps running
  uncancelled next to the tests that start after it.

## Steps

- [ ] Step 1: Make the folder-opening UI tests pass reliably on macOS CI
  - Done when:
    - The cause is recorded in `learnings.md` with its evidence: the log
      lines of a failing macOS run of this PR that show whether pass 2 was
      slow (it ended, and after how long) or stuck (and where).
    - The fix addresses that cause, and the macOS `native` job passes the
      `uitest` tests on at least 3 consecutive runs of the PR's final commit
      (re-run the job; record the run ids in `learnings.md`).
    - The Ubuntu and Windows `native` jobs still pass.
    - If the evidence shows an app bug (for instance a scan that ended
      without clearing its status, or a worker stuck for good), a unit test
      covers it; if it shows only a slow, throttled pass 2, no app unit test
      is added and `learnings.md` says so.
    - A UI test that times out prints the app's log lines for its folder in
      the test's failure output.
    - When a `Ui` drops while panicking, the view's scan is cancelled, so a
      timed-out test does not leave its pass 2 worker running next to the
      later tests.
    - `docs/agents/native-app.md`'s UI test sections ("The main view opens on
      GPUI's test platform", "The UI tests drive the view with keys over
      generated DNGs") describe the harness as it now is (the wait, the log
      on a timeout, the rule that came out of the cause).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/uitest.rs` for the diagnostics and the
      harness side of the fix; `docs/agents/native-app.md`. Which other files
      change depends on the fix chosen below. This PR does not touch
      `todo.md` (see "Trade-offs and risks" for the order with PR #1171).
    - Diagnostics first, as the PR's first commits, since no macOS host
      exists:
      - No logger is installed under `cargo test` (`logger::install` is
        called from `main` only), so the scan's timing lines are lost: `scan
        list:`, `scan reconcile:`, `scan sidecars:`, pass 1's summary, `scan
        faces first:`, `scan faces: ... in Nms`, and the "ran at normal
        priority" warning. Install a small test-only `log::Log` in
        `uitest.rs` (once per process, behind a `Once` / `OnceLock`) that
        stamps each line with the elapsed time and lets a timed-out
        `wait_until` print the lines for its folder (they carry `dir=<path>`).
        Writing each line with `eprintln!` is the simplest form; libtest's
        capture is inherited by threads spawned from the test thread.
      - Tell slow from stuck: on a timeout, before panicking, keep waiting
        for a bounded extra time (for instance 60 s to 120 s; mind the
        job's 30-minute limit and the 5 tests) and put in the panic message
        whether and after how long the view became idle.
      - To separate the priority from the unoptimized work, time the same
        detection at normal priority on the test thread (for instance
        `handpic_backend::ondemand::read_faces` on one fixture) and print it
        beside pass 2's time. `handpic-core` has no `log` dependency, so
        there are no per-stage lines inside pass 2 to rely on.
      - Push, then re-run the macOS `native` job until it fails (it failed 3
        of 4 runs so far) and read the output. Remove what was only for the
        hunt before the final commit; the test logger and the idle time in
        the timeout message stay.
    - Then fix on that evidence:
      - Pass 2 confirmed slow under `QOS_CLASS_BACKGROUND` (decided): option
        A below, the UI tests' scans at normal priority through a narrow
        seam.
      - Pass 2 stuck: an app fix with a unit test; update this plan first.
      - Do not raise `WAIT` alone without evidence that the scan finishes in
        a bounded time.
    - Fix the cascade whatever the cause: when `Ui` drops while panicking,
      cancel or stop the view's scan (the view's `scan` is a `scan::Running`
      with `cancel` and `stop_within`). Keep the early return from the quit
      assertions.
    - Constraints:
      - Non-test code must not use a test-only GPUI API (the rule in
        `docs/agents/native-app.md`).
      - `handpic-backend` and `handpic-core` are shared with the root
        workspace and compiled twice (`docs/agents/shared-crates.md`); a
        signature change there must build in both and keep the CLI's
        benchmarks at `Priority::Normal`.
      - The macOS job took 22 min cold against a 30-minute limit, so measure
        the job's duration if a dev-profile `opt-level` is added.

## Trade-offs and risks

- The cause is a hypothesis until the diagnostics run on macOS CI. If they
  show pass 2 stuck rather than slow, the fix is an app fix with a unit test
  and the options below do not apply; update the plan then.
- Fix options if pass 2 is confirmed slow under `QOS_CLASS_BACKGROUND`;
  A is chosen.
  - A (chosen). Run the UI tests' scans at normal priority through a narrow
    seam: the passes' priority becomes a parameter of `index::run_scan` /
    `index::run_faces_scan` (or of `Scan::start`) that the app passes as
    today and the harness overrides. Removes the throttling from the tests.
    Cost: a signature change in the shared backend crate, and the tests no
    longer schedule exactly as the app does (they test wiring, not
    scheduling).
  - B. Raise `WAIT` to a hang guard (for instance 60 s). No app change.
    Cost: no known upper bound under throttling, and a bad run takes
    minutes.
  - C. Optimize the analysis crates in `crates/native/Cargo.toml`'s
    `[profile.dev.package]` (the tract crates, `ndarray`, `handpic-core`).
    Helps only if the throttle is on CPU; a longer cold build on a job near
    its limit.
  - D. Change the app: pass 2 at `QOS_CLASS_UTILITY` on macOS. Changes the
    paging latency the priority was chosen for, which needs measuring on a
    Mac; out of this plan's reach.
- Product question raised by the finding: if pass 2 of a 3 KB file takes
  more than 10 s on an idle macOS machine at `QOS_CLASS_BACKGROUND`, users
  may see "analyzing 0/N" for long on macOS. This plan does not change that;
  if the diagnostics confirm it, record it in `learnings.md` as a deferred
  issue with the measured times (it becomes a todo item at the wrap-up)
  rather than change the QoS here.
- Order with PR #1171 (`chore/archive-native-qa`), which adds the todo item
  "### Native: the uitest tests timed out once on macOS CI waiting for the
  listing (native-qa)" and whose own macOS check is one of the failing runs
  (decided): land this fix first, then rebase #1171 on main, drop that item
  there, and let its macOS job run green on the fix.
- The test logger stays (about 20 lines in `uitest.rs`) so every later UI
  test failure is readable from CI.

## Progress
