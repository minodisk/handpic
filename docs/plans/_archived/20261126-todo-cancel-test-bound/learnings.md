# Learnings

## Step 1

- Before deleting, the test body at HEAD was checked again: `a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` in `crates/core/src/scan.rs` still waits on `ended.recv_timeout(HANG)` and has no `Instant` / `elapsed` assertion. The `Instant::now()` lines nearby belong to the neighbouring tests (`a_wait_for_no_pause_returns_at_once` and the ones after it), which the plan leaves alone.
- The todo item was eight lines (heading, paragraph, `#### TODO`, checkbox and their blank lines); deleting them left the neighbouring sections' spacing intact.
