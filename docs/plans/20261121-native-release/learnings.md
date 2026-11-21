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
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets were deleted on
  2026-11-21 by the main agent at the user's request; no workflow
  referenced them.
- The user registered the `UPDATE_SIGNING_KEY` repository secret on
  2026-11-21 (08:25:08Z) with `base64 -w0 <key> | gh secret set` after
  checking the value was non-empty. No agent read the key.
- The green branch run: run 44884797276 (`gh workflow run release.yml --ref
  native-release-step-2 -f version=2.0.0-rc.0`) concluded success. `guard`
  succeeded (the production key signed a scratch `.tar.gz` and matched
  `update.pub`), and `build / windows`, `build / linux`, `build / macos`
  (`macos-15-intel`, x86_64), `build / macos` (`macos-latest`, arm64) and
  `build / checksums` all succeeded; `release-please` and `publish` were
  skipped. Afterwards `gh release view v2.0.0-rc.0` reported "release not
  found" and `git ls-remote --tags origin v2.0.0-rc.0` was empty: nothing
  was published. With this the remaining Done-when items are met and Step
  2's checkbox is ticked.
- Accepted risks from the local review (not findings):
  - A branch dispatch signs unreviewed branch code with the production key.
    Its output is only workflow artifacts and nothing is published, but the
    signed archives exist.
  - "Never publishes from a non-`main` ref" holds only for the workflow code
    on `main`: a branch dispatch runs that branch's `release.yml`, so a
    branch could change the check. A protected environment holding
    `UPDATE_SIGNING_KEY`, or tag / release protection rules, would be the
    stronger guarantees.

## Step 3: Verify a published release, and describe how to cut one

### Step 1's post-merge check, after Step 2's merge

- PR #1202 (`feat(release): …`, 762d70f5) merged. Its Release run on
  `main`, 44895914551, updated the release-please branch (to e435bd62), and
  PR #928 now changes `crates/app/Cargo.toml` (`handpic-app` 0.0.0 → 2.0.0)
  and `crates/app/Cargo.lock` (`handpic-app` 0.0.0 → 2.0.0, `handpic-core`
  and `handpic-engine` 1.10.0 → 2.0.0). This confirms the inference from
  Step 1: release-please rewrites an open release PR only when its
  changelog changes, so the `chore` merge of #1195 left it alone and the
  `feat` merge of #1202 brought the app's files in. #928 was not touched.
- #928's CI run on that head, 44895983568, was still running when this was
  written; the main session adds its result.
- That run 44895914551 was the new `release.yml`'s first push on `main`:
  only `release-please` ran; `guard`, `build` and `publish` were skipped
  (no `release_created`), as intended.

### The verify script

- `verify-release.sh <tag> [<owner/name> [<verifying-key>]]` takes the
  asset list from `tools/package/assets.sh` (which also refuses a tag that
  is not `v<semver>`, exit 2), downloads the attached ones of the eleven
  into a `mktemp -d` folder under their release names, and needs zipsign
  0.2.2 on `PATH` (exit 2 otherwise), run as `mise exec
  cargo:zipsign@0.2.2 -- bash .claude/skills/release/scripts/verify-release.sh
  …`. No mise task was added. It reads with `gh --jq`, so it no longer
  needs `jq`.
- "The four updater archives" in the plan counts the four kinds (one per
  build leg: macOS, Windows setup, Windows portable, Linux); they are five
  files, since macOS has two architectures. The script verifies every
  `.zip` and `.tar.gz` of the list, which is exactly those five.
- Besides `sha256sum --check`, it also checks that `SHA256SUMS` names the
  ten other assets (the `publish` job's check), since `--check` alone
  passes a file the sums leave out.
- zipsign prints `OK` on stdout for each good file; the script sends
  zipsign's and `sha256sum`'s output to stderr so stdout is only the
  `ok` / `missing` lines and the status.
- Run on 2026-11-21 (read-only against GitHub):
  - `mise exec cargo:zipsign@0.2.2 -- bash
    .claude/skills/release/scripts/verify-release.sh v2.0.0-rc.2
    minodisk/handpic-update-test crates/app/packaging/test/update.pub`:
    every line `ok` (published, prerelease flag true for 2.0.0-rc.2, the
    eleven assets, SHA256SUMS names the ten others, `sha256sum --check`,
    `zipsign` on the five archives), `STATUS=ok`, exit 0.
  - The same release with the default (production) `update.pub`: the five
    `zipsign` lines `missing`, `STATUS=incomplete`, exit 1, so the key is
    really checked.
  - `verify-release.sh v1.10.0` (`minodisk/handpic`): `ok` published and
    `ok` prerelease flag false for 1.10.0; `missing` all eleven assets,
    `missing  no other asset (found …)` for `latest.json` and each of the
    16 Riffle files, `missing  sha256sum --check (no SHA256SUMS)` and the
    five `zipsign` lines; `STATUS=incomplete`, exit 1.
  - Without zipsign on `PATH`: "needs zipsign 0.2.2 on PATH", exit 2. A tag
    `vfoo`: exit 2.
- The skill's "Releases are suspended" section stays (until Step 5) but no
  longer calls the rest of the skill and the script stale; the rest now
  describes the native flow and has a "Cutting a release candidate"
  section. The runbook is `docs/agents/app.md`'s "The release runbook".

## Deferred issues (todo candidates)

- **Done for the files (2026-11-21, after #1202's merge; see Step 3
  above), #928's CI result added by the main session. Was pending post-merge
  check (Step 1's last Done-when), still open after Step 1's merge** (see "After the merge" above: #928 "remained the same"
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
- **Done (2026-11-21, registered 08:25:08Z by the user; see Step 2 above).
  Was pending (Step 2's Done-when): the `UPDATE_SIGNING_KEY` secret.** The
  user registers the repository secret: the 64-byte private key
  `mise run app:keygen` wrote, base64-encoded on one line (`base64 -w0
  <private-key>`), pasted on the secret's page on github.com or piped to
  `gh secret set UPDATE_SIGNING_KEY` after checking it is non-empty. No
  agent reads, copies or asks for the key. Basis: plan.md Step 2's
  Done-when and "Manual user step: the secret". Files:
  `.github/workflows/release.yml`, `tools/package/sign-production.sh`.
- **Done (2026-11-21, run 44884797276 green, nothing published; see Step 2
  above). Was pending (Step 2's Done-when): the green branch run.** After
  the secret is registered, dispatch `gh workflow run release.yml --ref
  native-release-step-2 -f version=2.0.0-rc.0` and check that it is green:
  the guard passes, each leg's "with the production key" signing step ran
  (the four archives signed and verified against `update.pub`), the
  `checksums` job wrote `SHA256SUMS` after them, the `publish` job is
  skipped, and no tag or release `v2.0.0-rc.0` exists on `minodisk/handpic`.
  Record the run id here. Step 2's checkbox stays unticked until then.
  Basis: plan.md Step 2's Done-when. Files: `.github/workflows/release.yml`,
  `.github/workflows/package.yml`.
