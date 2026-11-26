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

# Align the root lock's `notify` with the version the app ships

## Purpose

The engine's `treewatch`, `watch` and `rename` tests run in CI on the root `Cargo.lock`, which resolves `notify` 8.2.0, while the shipped app resolves `notify` 8.2.2 through `crates/app/Cargo.lock`. The folder rename depends on a notify-internal ordering on Windows (`configure` queuing behind `Unwatch`, the `watch::settle` barrier), so the tests that guard it should run on the shipped version. The user decided to move the root lock to 8.2.2 (rather than pin the app lock to 8.2.0). Background: [the measurement plan](../_archived/20261123-todo-measurements/plan.md) (Trade-offs, "Item 2: which lock is right") and `docs/agents/shared-crates.md` ("A `notify` watch on Windows pins the watched folder's ancestors, not the folder itself").

Once done, CI's three-platform `test` job exercises the engine's watcher and rename tests on the `notify` version the app ships, and the guides no longer describe two different versions.

## Steps

- [x] Step 1: Bump `notify` to 8.2.2 in the root `Cargo.lock`, stress the engine's watcher and rename tests on it, and fix the version wording in the guides
  - Done when:
    - The root `Cargo.lock` has `notify` 8.2.2 with the same checksum as the entry in `crates/app/Cargo.lock`, and `crates/app/Cargo.lock` is unchanged (it already has 8.2.2).
    - The root `Cargo.lock` diff holds only what the `notify` bump itself causes: `notify` 8.2.0 -> 8.2.2, `fsevent-sys` 4.1.0 removed, `objc2-core-services` 0.3.2 added, and `windows-sys` 0.60.2 with `windows-targets` 0.53.5 and its eight `windows_*` 0.53.1 crates removed. No other package changes version (`notify-types` stays at 2.1.0). Any difference from this list is explained in `learnings.md`.
    - The local stress run was dropped by the user (it starved other sessions on the host); learnings.md records the partial iterations and failures completed before it was stopped, and verification rests on the PR's three-platform `test` job.
    - `docs/agents/shared-crates.md` ("A `notify` watch on Windows pins the watched folder's ancestors, not the folder itself") no longer says the root lock stays at 8.2.0 or that aligning the locks is still open: it states that both locks resolve `notify` 8.2.2 since this change, so CI's engine tests run on the shipped version, (the 2026-11-23 scratch-binary re-check stays as the record it is). The dropped stress run is recorded in `learnings.md` only.
    - `docs/agents/app.md`'s rule "`crates/app/Cargo.lock` resolves its own versions (`notify` 8.2.2 against 8.2.0 in the root lock)" no longer names 8.2.0 as the root lock's version.
    - The doc comment of `mod watch_release` in `crates/app/src/datefolder.rs` no longer names 8.2.0 as the root lock's version (comment only).
    - No source change in `crates/engine` (the `settle` doc comment in `crates/engine/src/watch.rs` speaks of "notify 8.2" and stays true), no change to `crates/engine/Cargo.toml`, and no change to `todo.md` (the item is removed in the wrap-up phase).
    - Local `mise run ci:local` was skipped by the user's standing rule (see `learnings.md`); verification rests on the PR's GitHub checks passing (the `test` job on all three platforms, which is what covers 8.2.2's new macOS backend dependencies).
  - Implementation approach:
    - The lock change: at the repository root, `mise x -- cargo update -p notify --precise 8.2.2`. A planning-time `--dry-run` of that command printed exactly the changes listed in "Done when"; compare the real run against it with `git diff Cargo.lock`. Do not run a bare `cargo update` and do not touch `crates/app/Cargo.lock`.
    - On this host run `cargo`, `python` and `node` through `mise x --`. Never build the full root workspace locally (it has run this host out of memory): build and test only `-p handpic-engine`.
    - The stress run:
      - Build the test binary once (`mise x -- cargo test -p handpic-engine --no-run`), then loop the built binary (or `cargo test -p handpic-engine -- <filter>`) with the filters `treewatch::`, `watch::` and `rename::`. `watch::` as a substring also matches `treewatch::`; that only adds runs. Count failures by exit code and keep the output of any failing iteration.
      - The load must saturate every core for the whole loop, as the earlier measurements did (24 spinning threads on this host). A scratch load generator and the loop driver live in the scratchpad, not the repository; nothing scratch is committed.
      - Close other running Handpic / debug app instances first so they do not hold watches or skew the load.
      - For scale: without the barrier the rename failed about 5 in 300 test runs on 8.2.0 and 6 in 900 scratch iterations on 8.2.2, so 300 iterations per module is the minimum that would have shown the original defect.
      - If any iteration fails: do not add a retry or loosen a test. Record the failure in `learnings.md`, stop, and return the decision to the caller (see Trade-offs).
    - The docs:
      - `docs/agents/shared-crates.md`: the paragraph starting "Re-checked on notify 8.2.2" holds both stale statements (the parenthesis "the root lock stays at 8.2.0, which CI's `treewatch` tests run" and the closing sentence "Whether to align the two locks ... is still open"). Keep the measured 2026-11-23 facts; change only the lock statements and add the new run. Say `notify` is aligned, not its whole dependency subtree (`notify-types` is 2.1.0 in the root lock and 2.3.0 in the app's). The section's opening "Measured with `notify` 8.2" and the delete paragraph's "on notify 8.2.2 and again on 8.2.0" are records of past measurements and stay. Add this plan to the section's source line at its archived path, worded the same way the existing sources are written.
      - `docs/agents/app.md` (around line 1086): reword so it no longer claims the root lock is at 8.2.0; the point that the app workspace resolves its own lock stays.
      - `crates/app/src/datefolder.rs`, the doc comment of `mod watch_release` ("8.2.2 in `Cargo.lock`, 8.2.0 in the root lock"): change the comment only, to wording that names no root-lock version; the rest of the comment (the engine's tests cannot run from the app workspace) is still true.
    - `docs/plans/20261109-gpui-port/plan.md` also mentions the versions; it is another plan still in flight and is not edited here.
    - For the wrap-up phase: the `todo.md` item to delete is headed `### Engine: align the two \`notify\` locks so CI tests the version the app ships`. `todo.md` is edited concurrently by other sessions, so locate it by that exact line.

## Trade-offs and risks

- **The `datefolder.rs` comment.** Rewording it puts a `crates/app` path in the diff, so `mise run ci:local` selects the app's check and tests for a comment change. Taken: reword it, since leaving it leaves a false comment.
- **`docs/agents/app.md` was not in the requested file list.** Its rule names 8.2.0 as the root lock's version and becomes false, so the step fixes it.
- **The stress run is a local build and test loop.** The behavior under test is Windows-specific and load-dependent, and the acceptance criteria ask for it, so the step runs it locally, limited to `-p handpic-engine`. **Update:** the user dropped this run (it starved other sessions on the host); see `learnings.md` for the partial result. The PR's three-platform `test` job stands in for it.
- **If the stress run fails on 8.2.2.** A failure means the shipped app has the defect today; the options are then to pin `crates/app/Cargo.lock` to 8.2.0 instead, or to fix the release path. Neither is decided here: the step stops and reports.
- **`notify-types` and the rest of the subtree still differ.** `--precise` on `notify` alone leaves `notify-types` at 2.1.0 in the root lock (2.3.0 in the app's). Left, per "no unnecessary bumps".
- **The requirement in `crates/engine/Cargo.toml` stays `notify = "8.2.0"`.** Not raised.
- **Nothing keeps the two locks aligned afterwards.** A guard comparing the two locks' `notify` entries is out of scope and could be a new `todo.md` item.
- **macOS backend change.** 8.2.2 replaces `fsevent-sys` with `objc2-core-foundation` / `objc2-core-services`; the PR's macOS `test` job is the first run of the engine's tests on it.

## Progress

- (2026-11-27) Step 1 complete: root lock at notify 8.2.2 (Round 1 restored the seven `windows-sys` edges cargo had moved), guides and comment reworded; the local stress run was dropped, verification rests on the PR checks
