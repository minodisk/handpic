# Learnings

## Step 1

- The workflow change, the documents and the parent plan's Step 10 rewrite
  were done and checked locally (`mise run lint` for actionlint and lychee,
  `mise run ci`, and the native crate's clippy and tests on Windows). The
  Linux and macOS legs cannot run before the PR exists (`ci.yml` triggers on
  `pull_request`), so the checkbox was ticked on the local checks and the
  three-platform pass is confirmed by the PR's checks. Any failure those legs
  hit is fixed in the PR and recorded here and in `docs/agents/native-app.md`
  as **Hit**.
- Read ahead for non-Windows breakage and found none to fix:
  `crates/native/src` has no `cfg(windows)`-only code (one
  `cfg(not(target_os = "macos"))` test in `keys.rs`, `cfg!` branches in
  `logger.rs` and `tree.rs`); `tree.rs`'s path helpers and `sort.rs`'s file
  name split work on strings, so their `C:\` and UNC test inputs pass
  anywhere; the `datadir.rs` / `logger.rs` path tests build both sides with
  `Path::join`, so `C:/config` is just a relative-looking path on Unix and
  still compares equal. `keys.rs`'s default-keymap test uses keys whose
  defaults are the same on macOS (`ctrl+alt+1` for `red`).
- Foreseeable but unconfirmed: `ilammy/setup-nasm@v1` on the arm64 macOS
  runner. `build.rs` skips the NASM check on arm64, but the run step's
  `mise exec rust@1.98.1 -- nasm -v` still runs on every leg; if the action
  leaves no `nasm` on macOS, guard that line (or the action) with the
  architecture rather than dropping it on x86.
- Cold durations and cache sizes of the two new legs are to be recorded here
  once the PR's first run finishes.

## Deferred issues (todo candidates)

- **Pending check (PR checks): the `native` job's Linux and macOS legs.**
  Basis: Step 1's Done-when ("clippy (`-D warnings`) and `cargo test` pass on
  all three in the PR's checks"); `.github/workflows/ci.yml`'s `native` job.
  Platform: GitHub's `ubuntu-latest` and `macos-latest` runners. Steps: open
  the PR, read the `native (ubuntu-latest)` and `native (macos-latest)` legs
  (`gh run view --log-failed` on failure), fix in `crates/native` (or the
  backend / core when the cause is there), push. Expected: both legs green
  and the `ci` aggregate green. The step's checkbox was ticked on the local
  checks (Windows clippy and tests, `mise run lint`, `mise run ci`).
