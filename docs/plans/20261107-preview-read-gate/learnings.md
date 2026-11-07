# Learnings: preview-read-gate

## Step 1

- `READ_SLOTS` cannot be a `const` while it equals `scan_threads()`, which
  calls `available_parallelism`, so Step 1 has a `read_slots()` function next
  to `scan_threads()`. Step 3 replaces it with the constant the plan names.
- `AppReadSlots` implements `Default` in `commands.rs` so `scan_threads()`
  stays private and `main.rs` manages it the way it manages `Scans`.
- `acquire` looks at `cancel` before it looks at the permits, as the plan's
  "returns `None` once `cancel` is set" says. That changes one old assertion:
  `a_file_canceled_mid_pipeline_is_not_delivered` expected a missing file under
  a set flag to be an `Err`, and such a file is now abandoned unread (`None`).
  The test now asserts both cases, the `Err` under an unset flag.
- `free` is unsigned, so a counter inside `ReadSlots` could not show the cap
  being exceeded. The `extract_all` half of proof (a) instead holds the one
  permit and shows that nothing is delivered until it drops, which proves the
  read needs a permit; the same test runs `extract_analysis_all`.

## Deferred issues (todo candidates)

- **`MAX_PAUSE` counts from the start of `acquire`, including the wait for a
  permit.** Implemented as the plan's Implementation approach says ("this call
  has waited `MAX_PAUSE`"). A worker that queued for a permit for over 1 s then
  ignores a live pause. It cannot happen at Step 1's cap of `scan_threads()`,
  but by the plan's 42.8 ms per read the queue at a cap of 1 with 22 workers is
  about 0.9 s, so that cell of Step 2's sweep may read worse than the pause
  deserves. If so, start the clock when the worker first sees the pause and
  reset it when the pause is off. Basis: Step 1's `ReadSlots::acquire`,
  `crates/core/src/scan.rs`.
- **`embed_all`'s slot has no test.** Its per-file closure needs a loaded SigLIP
  model, which the tests do not have, so its acquire is only checked by reading
  the code; `extract_all` and `extract_analysis_all` are both tested. Basis:
  Step 1's proofs (a) and (b), `crates/core/src/scan.rs`.
- **The first post-read cancel check lost its test.**
  `a_file_canceled_mid_pipeline_is_not_delivered` set the flag before the read
  and so reached `extract_unless`'s and `extract_analysis_unless`'s check after
  it. `acquire` now returns `None` first and nothing can set the flag during
  the read without a test hook in `read_preview`. Basis: Step 1's change to
  that test, `crates/core/src/scan.rs`.
