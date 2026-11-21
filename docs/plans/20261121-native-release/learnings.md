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

### After the merge (observed at the start of Step 2)

- PR #1195 merged as 9f650e9f. Its Release run, 44879202000, logged
  "PR https://github.com/minodisk/handpic/pull/928 remained the same", and
  #928's files still lack `crates/app/Cargo.toml` and
  `crates/app/Cargo.lock`. The config names them correctly. Inferred cause:
  release-please does not rewrite an open release PR whose title and body
  (the changelog) are unchanged, and a `chore` commit does not change the
  changelog. Expect the next `feat` / `fix` commit on `main` to rewrite #928
  with the app files; Step 2's PR (or whichever `feat` / `fix` lands first)
  is when to look again.
- The post-merge CI of #1195 failed once on Windows with the known uitest
  timeout (`uitest::a_filter_choice_narrows_the_files_and_lands_on_the_anchor`,
  "timed out waiting for the folder's listing, scan and preview", run
  44879202091) and passed on a rerun.

## Step 2: `release.yml` builds, signs and publishes

- Decisions:
  - **The final path asserts the version.** The `guard` job checks that
    `crates/app/Cargo.toml`'s version on the release commit (written by the
    release PR since Step 1) equals the tag's, and fails otherwise; the
    build then writes the same version again (`set-version.sh` is
    idempotent). Relying on `set-version.sh` alone would ship a binary
    whose version disagrees with the checked-in files without anyone
    noticing.
  - **One shared list, inline publish.** `tools/package/assets.sh <version>`
    prints the eleven asset names; the `publish` job diffs the artifact set
    against it, and Step 3's `verify-release.sh` can use it too. The publish
    lines stay inline in `release.yml` instead of a script shared with
    `test-release.sh`: they differ in what matters (the real repository
    versus a refusal of it, the draft upload of the final path), and the
    common part is three commands. `test-release.sh` is unchanged.
  - **One signing script for the guard and the legs.**
    `tools/package/sign-production.sh` reads the secret from the
    environment, decodes it into a `mktemp -d` folder under `umask 077`,
    checks it is 64 bytes and signs through `mise run app:sign` against
    `update.pub`, removing the folder on exit. The guard signs a scratch
    `.tar.gz` with it, so the check is the path every leg takes.
  - **`sign_production` is a `workflow_call`-only input** of `package.yml`,
    and a caller has no `update_test_repo`, so a call never makes a test
    build and a dispatch never signs with the production key.
  - Concurrency: a dispatch's group is `release-candidate-<version>`, the
    pushes keep `release-<ref>`.
  - The rc's release, tag and tag collision are checked in `guard` only on
    `main` (`gh release list`, which sees drafts, and `git ls-remote`).
- Checked locally: `sign-production.sh` refuses an empty value, a non-base64
  value and a 3-byte value with messages naming the secret, and with the
  checked-in test key (64 bytes, not the production pair) `zipsign verify`
  against `update.pub` fails with "no matching key/signature pair found" and
  the script's mismatch message. The Git Bash `mktemp -d` path reached
  zipsign through `mise run app:sign` as `C:/Users/.../Temp/...`, so the
  Windows leg's MSYS path conversion is fine.
- A Python edit script fed through a Bash heredoc lost its `\` + newline
  pairs (the line continuations of the workflow's shell lines), so the
  replacement strings did not match. Writing the script to a file with the
  Write tool avoided it.
- The branch dispatch `gh workflow run release.yml --ref
  native-release-step-2 -f version=2.0.0-rc.0` was accepted (the new
  `version` input works on a branch dispatch), so no temporary trigger was
  needed. Run 44880572048: the `guard` job failed at "Check the signing
  key" with "sign-production.sh: the UPDATE_SIGNING_KEY secret is empty or
  missing" (the step's environment printed an empty `UPDATE_SIGNING_KEY:`
  line, the secret not existing yet); "Resolve the version" printed
  "version 2.0.0-rc.0, publish false"; `build` and `publish` were skipped,
  and no tag or release `v2.0.0-rc.0` exists. This meets the Done-when "A
  run with the secret empty or missing was seen to fail at the guard".
- The stale `TAURI_SIGNING_PRIVATE_KEY` and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets are still listed by `gh
  secret list`; they are the user's to delete (mention in the PR).

## Deferred issues (todo candidates)

- **Pending post-merge check (Step 1's last Done-when), still open after
  Step 1's merge** (see "After the merge" above: #928 "remained the same"
  on the `chore` merge). After the next `feat` / `fix` commit on `main`
  (Step 2's PR or an earlier one), check that release-please has updated PR #928 (do not merge,
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
- **Pending (Step 2's Done-when): the `UPDATE_SIGNING_KEY` secret.** The
  user registers the repository secret: the 64-byte private key
  `mise run app:keygen` wrote, base64-encoded on one line (`base64 -w0
  <private-key>`), pasted on the secret's page on github.com or piped to
  `gh secret set UPDATE_SIGNING_KEY` after checking it is non-empty. No
  agent reads, copies or asks for the key. Basis: plan.md Step 2's
  Done-when and "Manual user step: the secret". Files:
  `.github/workflows/release.yml`, `tools/package/sign-production.sh`.
- **Pending (Step 2's Done-when): the green branch run.** After the secret
  is registered, dispatch `gh workflow run release.yml --ref
  native-release-step-2 -f version=2.0.0-rc.0` and check that it is green:
  the guard passes, each leg's "with the production key" signing step ran
  (the four archives signed and verified against `update.pub`), the
  `checksums` job wrote `SHA256SUMS` after them, the `publish` job is
  skipped, and no tag or release `v2.0.0-rc.0` exists on `minodisk/handpic`.
  Record the run id here. Step 2's checkbox stays unticked until then.
  Basis: plan.md Step 2's Done-when. Files: `.github/workflows/release.yml`,
  `.github/workflows/package.yml`.
