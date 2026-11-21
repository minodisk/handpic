# Learnings

## Step 1: Let release-please own the app's version

- `release-please-config.json` gained two `extra-files`:
  `crates/app/Cargo.toml` (`$.package.version`) and `crates/app/Cargo.lock`
  with the root lock's rule, `$.package[?(@.source === undefined)].version`.
  The app's lock has exactly three entries without a `source`
  (`handpic-app` 0.0.0, `handpic-core` and `handpic-engine` 1.10.0), so the
  rule sets all three to the release version, matching the bumps of
  `crates/core` and `crates/engine`.
- The step's checkbox is ticked on what this PR delivers (the config, the
  rewritten line of `../20261109-gpui-port/packaging-updater.md`, the local
  checks). Its post-merge Done-when, release-please updating PR #928, can
  only be seen after the merge; the plan has Step 2 record it first. It is
  listed below so it is not lost.

## Deferred issues (todo candidates)

- **Pending post-merge check (Step 1's last Done-when).** After this step's
  PR is merged, check that release-please has updated PR #928 (do not merge,
  close or hand-edit it) so its diff sets `crates/app/Cargo.toml` and the
  `handpic-app`, `handpic-core` and `handpic-engine` entries of
  `crates/app/Cargo.lock` to `2.0.0`, and that #928's own checks pass (the
  `app` job builds with `--locked`). Record the observation here at the
  start of Step 2. If release-please does not pick the files up, or the
  `--locked` build fails, fix `release-please-config.json` in a follow-up PR
  of Step 1. Basis: plan.md Step 1's Done-when. Files:
  `release-please-config.json`, `crates/app/Cargo.toml`,
  `crates/app/Cargo.lock`. Step 1's checkbox was ticked on the other,
  pre-merge criteria.
