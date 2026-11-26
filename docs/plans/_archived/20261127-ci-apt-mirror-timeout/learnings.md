# Learnings

## Step 1

- The change is a step-level `timeout-minutes: 5` on `ci.yml`'s only apt
  step, the guide entry and the todo swap; nothing Rust changed, so no local
  cargo build or `mise run ci:local` was run (GitHub Actions verifies).
- The new todo item replaced the closed one in place, so no other item moved.

## Deferred issues (todo candidates)

- Pending check (not manual, CI): the PR's `app (ubuntu-latest)` job should
  show "Install the Linux GPUI dependencies" finishing well inside its
  5 minutes (normally about 9 s). The step's checkbox was ticked on the
  automated criteria (`actionlint` clean). Basis: Step 1's Done-when; file
  `.github/workflows/ci.yml`.
