# Learnings

- The debounce test polls (10 s deadline) for the sidecar to exist and the
  index to have no dirty row instead of calling `writer.flush`: a flush writes
  the pending entry at once, so the write would no longer be the debounce's
  own. This mirrors `eventually` in `crates/backend/src/sidecar.rs`'s tests.
- No other `crates/native` test reads a sidecar after a fixed `thread::sleep`;
  the only other sleep (`hold.rs`) times a cancel, not a write.
- `cargo` is not on the Git Bash PATH here; `mise exec rust@1.98.1 -- cargo
  test` in `crates/native` runs the native tests directly (five runs in a row
  passed).
