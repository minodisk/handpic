# Learnings

## Step 1

- The fix sits in one place: `date_folder_ran` takes the deferred open out
  with `Option::take_if` when `split.removed` and the deferred path is `dir`
  or under it (`tree::relation`). A `gone` flag then skips the first
  `settle_folder_run` arm, so the flow reaches the existing first-date-folder
  arms. Nothing else changed.
- Both new UI tests (`a_held_click_on_a_folder_a_split_removed_opens_the_first_date_folder`
  and `a_held_click_on_another_folder_opens_it_after_a_split`) share a
  `split_inner_with_deferred` helper. It holds the gate, clicks, changes the
  disk and calls `date_folder_ran` directly.
- The local targeted `cargo test` of `crates/app` never ran to the end.
  Claude Code moved it to the background after 180 s, then killed it
  because the host was low on memory (another `crates/app` build was running
  in the same worktree). `cargo fmt --check` passed. The tests, clippy and
  the check that the first test fails without the fix are left to the PR's
  GitHub checks.

## Deferred issues (todo candidates)

- Unverified locally: the new test
  `a_held_click_on_a_folder_a_split_removed_opens_the_first_date_folder` in
  `crates/app/src/folderops.rs` is meant to fail without the fix: the open
  of the missing `inner` runs and `view.dir` never ends with `2026-11-27`, so
  the test times out in `wait_until`. That failing run was never done,
  because the local test build was killed for lack of memory. Basis: Step 1's
  approach ("Check that the test fails without the fix"). To check, revert
  the `gone` lines in `date_folder_ran` and run
  `cargo test --bin handpic folderops::tests::a_held_click_on_a_folder_a_split_removed`
  in `crates/app` on a host with memory to spare.
