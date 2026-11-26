# Learnings

## Step 1

- Truncating `todo.md` by line count (`head -n`) is easy to get off by one;
  check the result with `git diff` that only the target section went.
- The stale `faces_of` / `eyes_of` names also remain in doc comments of
  `crates/engine/src/ondemand.rs` and `crates/cli/src/main.rs`, and in
  `docs/agents/shared-crates.md` and `docs/agents/tract-onnx-inference.md`;
  left alone as the plan scopes this to `CLAUDE.md`.

## Deferred issues (todo candidates)

- Stale `faces_of` / `eyes_of` caller names outside `CLAUDE.md`. Basis: this
  step's plan ("Trade-offs and risks") kept the fix docs-only to `CLAUDE.md`.
  The real callers of `ondemand::read_faces` / `read_eyes` are
  `overlay::detect` / `overlay::judge` in `crates/app/src/overlay.rs`. Files:
  `crates/engine/src/ondemand.rs`, `crates/cli/src/main.rs`,
  `docs/agents/shared-crates.md`, `docs/agents/tract-onnx-inference.md`.
