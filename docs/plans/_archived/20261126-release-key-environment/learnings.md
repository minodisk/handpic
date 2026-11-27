# Learnings

## Step 1

- `guard`'s `publish` output became constant once a non-`main` dispatch
  fails at `guard` (the environment rejects it; the explicit ref check
  only fires if the policy were loosened). It was removed with the
  `publish` job's use of it, which now runs whenever `build` succeeded.
- Verification (a), measured 2026-11-27 on the PR branch (run
  45846600132): `guard` failed with zero steps run, `build`, `publish` and
  `release-please` skipped, annotations "Branch "chore/release-key-environment"
  is not allowed to deploy to release due to environment protection rules."
  and "The deployment was rejected or didn't satisfy other protection
  rules." So the branch policy is enforced with `deployment: false`. The
  environment had no secret yet, so the rejection comes from the policy
  alone.
- Empty environment name, measured 2026-11-27 (run 45846638280, a plain
  `package.yml` dispatch on the PR branch): all four leg jobs (two macOS
  matrix legs, Windows, Linux) started with
  `${{ inputs.sign_production && 'release' || '' }}` resolving to `''`, so
  an empty name means "no environment". Cancelled once started.

## Deferred issues (todo candidates)

- Pending manual check (verification (b), the user's, after the merge and
  after deleting the repository-level `UPDATE_SIGNING_KEY` with `gh secret
  delete UPDATE_SIGNING_KEY --repo minodisk/handpic`): dispatch a release
  candidate from `main` (`gh workflow run release.yml --ref main -f
  version=<next unused x.y.z-rc.N>`) and expect `mise exec
  cargo:zipsign@0.2.2 -- bash .claude/skills/release/scripts/verify-release.sh
  v<version>` to report `STATUS=ok`, proving the legs sign with the
  `release` environment's secret. Basis: plan "Verification" (b); files
  `.github/workflows/release.yml`, `.github/workflows/package.yml`.
