<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Make the native judgment debounce test wait on the writer, not a fixed sleep

## Purpose

`crates/native/src/judgment.rs`'s
`keys_inside_the_debounce_are_written_once_with_the_last_value` failed on
`main`'s post-merge CI (run 43425673373, the `native` job, after #997): it
sleeps `DEBOUNCE * 3` and then reads the sidecar, which did not exist yet on
a loaded runner (`NotFound` at `judgment.rs:611`). A rerun passed. The test
must not depend on how fast the writer thread runs.

## Steps

- [x] Step 1: Wait for the writer instead of sleeping
  - Done when:
    - The test keeps its first check (no sidecar while the first key's
      debounce has not run out) and then waits for the write without a
      fixed wall-clock sleep, e.g. `folder.writer.flush(DRAIN_TIMEOUT)` as
      the neighbouring tests do, or a poll with a generous deadline; it still
      asserts the sidecar holds the last value (`xmp:Rating="3"`) and the
      index has no dirty rows. If the test's point (one write for keys
      inside the debounce) needs the debounce itself to elapse rather than a
      flush, keep that meaning and say how in a comment.
    - Other tests in `crates/native` that read a sidecar after a fixed
      `thread::sleep` are made the same way (search for them; list what was
      changed or that none were found).
    - `mise run native:check` and root `mise run ci` pass; the test passes
      repeatedly under load (e.g. run the native tests a few times in a row
      with full parallelism).
  - Implementation approach: test-only change; no runtime code changes. No
    A/B (off the page path, per `docs/agents/native-app.md`'s gate-scope rule).

## Progress

- (2026-11-13) Step 1 complete
