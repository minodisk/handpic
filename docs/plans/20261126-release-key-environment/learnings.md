# Learnings

## Step 1

- `guard`'s `publish` output became constant once a non-`main` dispatch
  fails at `guard` (the environment rejects it; the explicit ref check
  only fires if the policy were loosened). It was removed with the
  `publish` job's use of it, which now runs whenever `build` succeeded.
- The `release` environment's rejection text in `docs/agents/app.md` is an
  expected wording ("Branch "<branch>" is not allowed to deploy to release
  due to environment protection rules."), to be replaced by the measured
  annotation after verification (a).

## Deferred issues (todo candidates)

- Pending manual check (verification (a) and the empty environment name
  check, the main agent's after the "Before the merge" repository settings):
  on GitHub Actions, dispatch `gh workflow run release.yml --ref <pr-branch>
  -f version=0.0.0-rc.0` and expect `guard` to fail before any step with the
  environment protection annotation (record its exact text in
  `docs/agents/app.md`'s decision Rule), `build` and `publish` not running;
  and dispatch `gh workflow run package.yml --ref <pr-branch>` and expect the
  three legs to start with no environment (cancel once started). Basis: plan
  Step 1's "Verify on the PR"; files `.github/workflows/release.yml`,
  `.github/workflows/package.yml`, `docs/agents/app.md`. Step 1's checkbox
  was ticked on the automated criteria.
