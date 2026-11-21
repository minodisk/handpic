<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# The app's release workflow and the rc dry run

## Purpose

Sub-milestone 4 of Step 10 of [the gpui-port plan](../20261109-gpui-port/plan.md).
The app (`crates/app`) can update itself since
[the native-updater plan](../_archived/20261118-native-updater/plan.md), but
nothing publishes what it updates from: `.github/workflows/release.yml` keeps
only the `release-please` job, `package.yml` signs with the test key only, and
the `release` skill says releases are suspended.

Once this plan is done:

- `release.yml` builds every platform's packages for a given version, signs
  the four updater archives with the production key, writes `SHA256SUMS`
  after signing and publishes a GitHub release of `minodisk/handpic` with the
  exact asset names `crates/engine/src/update.rs` asks for. A version with a
  prerelease part is published as a GitHub prerelease.
- A release candidate (`2.0.0-rc.1`) can be cut by hand without merging the
  release PR #928, and the final release is cut by merging it (`/release`).
- An rc.1 to rc.2 update from the real repository with the production key has
  been run on the Windows PC at hand and recorded as **Measured** /
  **Unverified** in
  [`packaging-updater.md`](../20261109-gpui-port/packaging-updater.md).
- The docs say how to cut a release and an rc, and what is left before #928
  can be merged.

**This plan never merges PR #928.** Publishing the first real release is the
user's `/release` decision.

### The eleven assets of a release

The updater pins a release by its tag `v<version>` and matches assets by
exact name. A release carries these and no `latest.json`:

| Asset                                         | For     | Signed (zipsign) |
| --------------------------------------------- | ------- | ---------------- |
| `Handpic-<version>-macos-arm64.dmg`           | people  | no               |
| `Handpic-<version>-macos-x86_64.dmg`          | people  | no               |
| `Handpic-<version>-macos-arm64.tar.gz`        | updater | yes              |
| `Handpic-<version>-macos-x86_64.tar.gz`       | updater | yes              |
| `Handpic-<version>-windows-x64-setup.exe`     | people  | no               |
| `Handpic-<version>-windows-x64-setup.exe.zip` | updater | yes              |
| `Handpic-<version>-windows-x64.zip`           | both    | yes              |
| `Handpic-<version>-linux-x64.tar.gz`          | both    | yes              |
| `install.sh`                                  | people  | no               |
| `handpic_<version>_amd64.deb`                 | people  | no               |
| `SHA256SUMS`                                  | both    | no               |

### What remains before PR #928 can be merged

Kept current by Step 4 and Step 5; the list as known at planning time:

1. The Windows `handpic.exe` built as a GUI-subsystem binary. It is a separate
   PR outside this plan, not on `main` at planning time
   (`crates/app/src/main.rs` sets no `windows_subsystem`).
2. Steps 1 to 3 of this plan merged and the `UPDATE_SIGNING_KEY` secret
   registered by the user.
3. The rc dry run of Step 4, its Windows rows **Measured**.
4. The macOS `.app`, the Linux tarball and the deb rows. Only a Windows PC is
   at hand, so they stay **Unverified** unless a machine turns up. The
   gpui-port plan asks for a real-device install, update and uninstall per
   format before the first release, so shipping 2.0.0 with these rows
   unverified needs the user's explicit decision.
5. Step 5 merged: the install docs, the reinstall notice for the first
   release's notes, and the `/release` skill no longer suspended.
6. PR #928 itself green after release-please has added `crates/app`'s version
   files to it (Step 1).
7. Accepted risk: the release-please path of `release.yml` (upload to the
   draft, then publish) first runs at the real release. See "Trade-offs and
   risks".

Not required before #928: sub-milestone 5 of the gpui-port plan (merging
`crates/app` into the root workspace).

## Steps

- [x] Step 1: Let release-please own the app's version
  - Done when:
    - `release-please-config.json`'s `extra-files` also name
      `crates/app/Cargo.toml` (`$.package.version`) and
      `crates/app/Cargo.lock` (the entries without a `source`).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - After the merge, release-please has updated PR #928 so that its diff
      sets `crates/app/Cargo.toml` and the `handpic-app`, `handpic-core` and
      `handpic-engine` entries of `crates/app/Cargo.lock` to `2.0.0`, and
      #928's own checks (the `app` job builds with `--locked`) pass. Record
      the observation in `learnings.md`. It is checked at the start of
      Step 2, since it can only be seen after the merge.
  - Implementation approach (as far as it is known; omit if unknown):
    - `crates/app/Cargo.lock` has exactly three entries without a `source`:
      `handpic-app` (`0.0.0`), `handpic-core` and `handpic-engine`
      (`1.10.0`). The root lock's existing rule,
      `$.package[?(@.source === undefined)].version`, sets all three to the
      release version, which matches the bumps of `crates/core` and
      `crates/engine`.
    - `crates/app/Cargo.toml` stays `0.0.0` on `main` until #928 is merged;
      a source build keeps reporting `0.0.0` until then.
    - Do not merge, close or hand-edit PR #928. If release-please does not
      pick the new files up, or #928's `--locked` build fails, fix the config
      in a follow-up PR of this step.
    - Also rewrite the "release-please bumps neither …" **Unverified** line
      of `../20261109-gpui-port/packaging-updater.md`.

- [x] Step 2: `release.yml` builds, signs and publishes; `package.yml` becomes its reusable build
  - Done when:
    - `package.yml` can be called by another workflow (`workflow_call`) as
      well as dispatched. Called with the production signing asked for, each
      platform leg signs its updater archives with the production key
      through `tools/package/sign.sh` against
      `crates/app/packaging/update.pub`, before that leg's checks, and
      `SHA256SUMS` is computed after signing as today. A direct dispatch of
      `package.yml` cannot ask for production signing; its `version` and
      `update_test_repo` inputs behave as before.
    - `release.yml` has two ways in, sharing the build and the publish:
      - **A release candidate:** `gh workflow run release.yml --ref main -f
        version=2.0.0-rc.1`. The version must have a prerelease part (a
        stable version is refused), the release `v<version>` must not exist
        yet, and the run must be on `main` to publish. It creates the tag
        `v<version>` at the dispatched commit and a GitHub **prerelease**,
        never marked latest, with the eleven assets.
      - **The final release:** when the `release-please` job reports
        `release_created`, the tag's version is built and the assets are
        uploaded to the draft release release-please made. The draft is
        published (`--draft=false --latest`) only after every asset is
        attached. A failed build leaves a draft.
    - A first job fails before any build when `UPDATE_SIGNING_KEY` is empty,
      does not decode to a 64-byte key, or does not match `update.pub`, with
      a message that names the secret.
    - Before publishing, the publish job checks that the artifact set is
      exactly the eleven assets for that version and that `sha256sum
      --check` passes. No file is renamed after signing, and no
      `latest.json` is attached.
    - A dispatch on a ref other than `main` builds, signs and uploads
      workflow artifacts but creates no tag and no release.
    - The user has registered the `UPDATE_SIGNING_KEY` repository secret (a
      manual step, see below).
    - A run dispatched on this step's branch (`gh workflow run release.yml
      --ref <branch> -f version=2.0.0-rc.0`) is green: the guard passes, the
      four archives are signed with the production key, `SHA256SUMS` is
      written after signing, and nothing is published. A run with the secret
      empty or missing was seen to fail at the guard (do this one before the
      user registers the secret).
    - `docs/agents/app.md` describes the workflow (the two ways in, the
      guard, the secret's format, what a non-`main` dispatch does),
      `docs/agents/github-actions-workflows.md` gains what this step hit,
      and `CLAUDE.md`'s packaging paragraph no longer says `package.yml` is
      `workflow_dispatch` only or that `sign.sh` runs only with
      `update_test_repo`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (`actionlint` and `shellcheck` run in `mise run lint`).
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged. Start by recording Step 1's post-merge
      observation of PR #928.
    - **This PR changes `.github/**`, so its merge needs the user's
      approval.** Pushing it needs the `workflow` scope (`gh auth refresh -h
      github.com -s workflow`).
    - **Manual user step: the secret.** The private key is a 64-byte binary
      file only the user holds. No agent reads it, copies it or asks for its
      content. The user stores it base64-encoded as the repository secret
      `UPDATE_SIGNING_KEY`: paste the output of `base64 -w0 <private-key>`
      into the secret's page on github.com, or pipe it to `gh secret set
      UPDATE_SIGNING_KEY` after checking it is non-empty. `gh secret list`
      cannot tell an empty secret from a set one
      ([`github-actions-workflows.md`](../../agents/github-actions-workflows.md),
      "An empty secret looks set"), which is why the workflow's guard
      exists. Ask the user for this once the guard's failing run has been
      seen.
    - Reuse, do not duplicate: `release.yml` calls `package.yml` with `uses:
      ./.github/workflows/package.yml`, passing the version and the secret.
      `set-version.sh`, `version.sh` and `sign.sh` are used as they are.
    - `release.yml` already has a bare `workflow_dispatch` on `main`, so the
      branch's version of it can be dispatched with `--ref <branch>` and no
      temporary trigger is needed. Verify that the new `version` input is
      accepted on a branch dispatch. If it is not, use the temporary-trigger
      rule of `github-actions-workflows.md` and remove the trigger before
      the merge.
    - In a reusable workflow that is also dispatchable, `secrets.<NAME>`
      resolves to the repository secret on a direct dispatch too. Gate
      production signing on an input that exists only under `workflow_call`,
      not on the secret being present.
    - Handling the key in a job: pass the secret through `env` to the one
      step that needs it, decode it to a file under `$RUNNER_TEMP` with a
      restrictive umask, sign, and delete the file in the same step. Never
      echo it. `sign.sh`'s own `zipsign verify` against `update.pub` fails
      on a mismatched key.
    - The guard should prove the key matches `update.pub` before the builds
      of about 75 minutes, for example by signing a scratch ZIP with
      `sign.sh` on `ubuntu-latest`.
    - The final-release path passes the tag's version to the build. The
      binary's version must equal the tag's, because the updater pins
      `v<version>` and the asset names carry it. After Step 1 the checked-in
      `crates/app` version already equals it. Decide whether the workflow
      asserts that equality or only relies on `set-version.sh` being
      idempotent, and record which.
    - The rc path writes the version only in the runner's checkout
      (`set-version.sh`). Nothing is committed, `CHANGELOG.md` and the
      manifest are untouched, and release-please is not involved.
    - `gh release create` with files uploads to a draft and publishes at the
      end, so an rc is never visible half-attached. Pass `--prerelease` and
      `--latest=false` so `/releases/latest` stays `v1.10.0`.
    - Concurrency: today's group `release-${{ github.ref }}` would queue an
      rc run of about 75 minutes against every push to `main` and queue two
      rc runs behind each other. Give the dispatch its own group (for
      example by version) so rc.1 and rc.2 can build in parallel and
      release-please keeps running.
    - The publish logic overlaps `tools/package/test-release.sh` (check the
      files against `SHA256SUMS`, upload unrenamed, prerelease by version).
      Decide whether a shared script serves both the workflow and
      `test-release.sh`, which must keep refusing `minodisk/handpic` for
      test builds, or whether the workflow's few lines stay inline. Keep one
      list of the expected asset names that Step 3's verify script can
      share.
    - The final-release path cannot be exercised without merging #928.
      Review it against the pre-removal workflow (`git show
      80221f61^:.github/workflows/release.yml`, the `publish` job from
      #364), whose draft-then-publish shape it restores.
    - The stale `TAURI_SIGNING_PRIVATE_KEY` and
      `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets are the user's to delete.
      Mention them in the PR; do not touch them. (Deleted on 2026-11-21 at
      the user's request.)
    - Files: `.github/workflows/release.yml`,
      `.github/workflows/package.yml`, possibly one new script under
      `tools/package/`, `docs/agents/app.md`,
      `docs/agents/github-actions-workflows.md`, `CLAUDE.md`.

- [x] Step 3: Verify a published release, and describe how to cut one
  - Done when:
    - `.claude/skills/release/scripts/verify-release.sh <tag>` checks the
      native release:
      - the release exists and is not a draft;
      - its prerelease flag agrees with the version (a prerelease part means
        a prerelease);
      - exactly the eleven assets are attached and none is named
        `latest.json`;
      - the downloaded files pass `sha256sum --check` against the release's
        `SHA256SUMS`;
      - the four updater archives pass `zipsign verify` with the verifying
        key.

      It keeps the `ok` / `missing` lines, `STATUS=ok|incomplete` and the
      exit codes 0 / 1 / 2. It no longer looks for `.msi`, `.rpm`,
      `.AppImage` or `latest.json`.
    - It was run against the test feed (`v2.0.0-rc.2` of
      `minodisk/handpic-update-test` with the test verifying key) and
      reports `STATUS=ok`, and against `v1.10.0` of `minodisk/handpic` and
      reports `STATUS=incomplete`.
    - `.claude/skills/release/SKILL.md` describes the native flow: what a
      merge of the release PR sets off (tag, draft, the build, the signing,
      the publish), the new verify step, and a section on cutting a release
      candidate by dispatch. Its "Releases are suspended" section stays
      until Step 5.
    - `CONTRIBUTING.md`'s promotion-stats note names the native assets:
      which are downloaded by people, which only by the updater (the macOS
      `.tar.gz`, the `-setup.exe.zip`), and which by both (the portable ZIP,
      the Linux `.tar.gz`).
    - `docs/agents/app.md` has the release runbook: cutting an rc, cutting
      the final release, verifying one, and deleting an rc (`gh release
      delete v<version> --cleanup-tag`).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged, so the docs describe the workflow as merged.
    - Give the script optional arguments for the repository and the
      verifying key, defaulting to `minodisk/handpic` and
      `crates/app/packaging/update.pub`, so it can be exercised on the test
      feed before any real release exists.
    - zipsign 0.2.2 is not on `PATH` outside mise; it is the `cargo:zipsign`
      tool of the `app:sign` task. Run the verify through mise, for example
      `mise exec cargo:zipsign@0.2.2 -- zipsign verify …`, or add a small
      task. zipsign binds a signature to the file's name, so verify the
      files under their release names.
    - `shellcheck` already covers `.claude/skills/*/scripts/*.sh` in `mise
      run lint`.
    - The skill keeps `disable-model-invocation: true` and the rule that no
      agent merges the release PR on its own.

- [ ] Step 4: The rc.1 to rc.2 dry run on `minodisk/handpic` with the production key, recorded
  - Done when:
    - `v2.0.0-rc.1` and `v2.0.0-rc.2` were published as prereleases of
      `minodisk/handpic` by `release.yml` dispatched on `main`, and
      `verify-release.sh` reports `STATUS=ok` for both.
    - `packaging-updater.md` has a section "The rc dry run on the real feed"
      with the run ids, the machine, and each row below tagged **Measured**
      (with date and machine) or **Unverified** (with the reason):

      | Install or subject           | Check                                                                                                                                                                                                                         |
      | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
      | Feed                         | Before any rc build: a local stable build (`set-version.sh 1.99.0`, no `HANDPIC_UPDATE_TEST_REPO`) reads the real repository's release list, old Tauri releases included, and logs `offered None`                             |
      | Feed                         | After rc.1 is out: `/releases/latest` is still `v1.10.0`; the same stable build is still offered nothing; a local `2.0.0-rc.0` build is offered `2.0.0-rc.1`                                                                  |
      | release-please               | With the rc tags on `main`, the next push to `main` leaves PR #928's title (`release 2.0.0`) and changelog as they were                                                                                                       |
      | Portable ZIP                 | rc.1 downloaded from the release page updates to rc.2 while running; `Restart Now` relaunches rc.2 with no console window; a judgment made just before is in the sidecar                                                       |
      | Inno Setup                   | rc.1's setup downloaded with a browser and installed by hand (record the SmartScreen prompt); rc.2 is staged in the background; a plain quit installs it; `Restart Now` relaunches rc.2 with no console window; uninstall from Settings removes it |
      | Inno Setup, standard account | The same quit and `Restart Now` on a standard (non-administrator) Windows account: no UAC and no SmartScreen prompt                                                                                                            |
      | Windows, beside Riffle       | rc.1's setup on a machine with Riffle 1.10.0 installed: what the two uninstall entries and shortcuts look like, and that the settings are seeded from the earlier install                                                     |
      | Any                          | The network cut for real (adapter disabled) at launch and on `Check for Updates`: the app stays usable and the failure is retryable; `Check for Updates` is disabled while a check runs; `Open Release Page` opens the browser |
      | `.app`                       | **Unverified** unless a Mac turns up: the swap in `/Applications`, the refusal from the DMG or Downloads, Gatekeeper and the folder-access grants                                                                              |
      | Tarball                      | **Unverified** unless a Linux machine turns up                                                                                                                                                                                |
      | deb                          | **Unverified** unless a Linux machine turns up, the `Conflicts` / `Replaces: riffle` question with it                                                                                                                         |

    - The rows of the record's "Unverified" list and of the test-feed table
      that this run answered are rewritten, and the ones it did not answer
      say so explicitly.
    - The `todo.md` item "App: real-device checks of the update flow on
      macOS, Linux and a standard Windows account (native-updater)" has its
      measured boxes ticked and its text pointing at this run. The macOS and
      Linux boxes stay open.
    - The gpui-port plan's Step 10 marks sub-milestone 4's workflow and dry
      run done through this plan and carries the current "What remains
      before PR #928 can be merged" list.
    - The rc releases and tags are in the state the cleanup decision asks
      for (see "Trade-offs and risks"), and the record says which.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 2 and 3 are merged and the secret is registered. It
      should also wait for the Windows GUI-subsystem fix to be on `main`, so
      the "no console window" rows can be measured. If the user wants the
      dry run before that fix, run it and leave those rows open.
    - Do the two "Feed" rows with local builds first. They cost minutes and
      are the first time the updater reads the real repository. A local
      `target/` binary is install kind unknown, so it stops at `Available`,
      which is enough for these rows.
    - Dispatch rc.1 and rc.2 from the same `main` commit. Each run takes
      about 75 minutes cold.
    - Follow the test-feed rehearsal's method (the native-updater plan's
      Step 7 and its `learnings.md`): each copy with its own `--settings`,
      `--index`, `--log` and `--mcp-port`; state read from the log, the
      exe's `ProductVersion`, the `HKCU` uninstall entry and MCP
      `get_status`. `Restart Now`, the hand install, the standard account
      and the adapter switch are the user's hands; ask for them together. A
      relaunch drops the command-line arguments (measured), so a relaunched
      copy uses the default data folders.
    - To set up the Inno `Restart Now` row a second time, rc.1's setup
      installs over rc.2 as a downgrade (measured on the test feed).
    - Do not publish a tampered release on the real repository. That row is
      **Measured** on the test feed and covered by the engine's tests.
    - Mark nothing **Measured** from CI output or from a machine that was
      not used.
    - A small fix found by a row goes into this PR. A gap in the updater
      engine's behavior becomes its own new step in this plan, placed before
      this one is closed.
    - A change to `.github/**` in this PR, if a row forces one, needs the
      user's approval at merge.

- [ ] Step 5: Open the release gate: the install docs, the reinstall notice and the `/release` skill
  - Done when:
    - `README.md` / `README.ja.md` "Install" and `docs/humans/usage.md` /
      `usage.ja.md` "Installing" and "Updating" describe the packaged
      builds:
      - which asset to download per platform and how to install it
        (`install.sh` with the digest from `SHA256SUMS`, the deb, the setup,
        the portable ZIP, the DMG);
      - how to pass Gatekeeper and SmartScreen;
      - that an install of the earlier app (up to 1.10.0) does not update
        and is replaced by hand, with what a new install picks up;
      - that release candidates are prereleases a final install is never
        offered.

      Each pair is in sync.
    - The reinstall notice for the first native release's notes is checked
      in as a text the `release` skill names, and the skill says when to
      prepend it to the draft release's notes (while the build runs, before
      the workflow publishes).
    - `.claude/skills/release/SKILL.md` no longer says releases are
      suspended. In its place, a "first native release" checklist that
      `/release` reports to the user before merging: the list "What remains
      before PR #928 can be merged" with its current state, the
      **Unverified** macOS and Linux rows named.
    - The gpui-port plan's Step 10 says what is left of sub-milestone 4
      after this plan: only the user's `/release`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged. Its results decide what the docs can
      promise.
    - **This step does not merge PR #928.** Lifting the suspension only
      makes the user's `/release` possible.
    - Merge this PR close to the release. Between its merge and the
      publication of 2.0.0 the docs name downloads that do not exist yet
      (the latest release is still `v1.10.0`). Word the install text so it
      stays true in that window where that costs nothing ("2.0.0 or later").
    - The usage guide's "Updating" section already describes the flow; it
      only loses "none is published yet".
    - `disable-model-invocation: true` stays on the skill.

## Trade-offs and risks

The user approved this plan on 2026-11-21 with every contested point decided
as recommended (A).

- **How an rc is cut without merging #928.**
  - A (planned): a `version` input on `release.yml`'s `workflow_dispatch`,
    prerelease versions only, publishing only from `main`. The version lives
    in the runner's checkout; no commit, no `CHANGELOG.md` change, and
    release-please is not involved. The tag is created by `gh release
    create` at the dispatched commit.
  - B: pushing a tag `v2.0.0-rc.1` triggers the build. It needs a local tag
    push, allows a tag that disagrees with the built version, and a tag
    trigger also fires on the tags release-please creates
    (`force-tag-creation`), so the two paths would have to be told apart.
  - Not taken: release-please's prerelease versioning. It needs a merged
    release PR and rewrites #928, which the constraint forbids.
- **Whether the rc tags disturb release-please.** Inferred, not measured:
  in manifest mode release-please looks up the release of the manifest's
  version (`v1.10.0`), so tags `v2.0.0-rc.N` on `main` should not move #928.
  Step 4 measures it. If #928 changes, delete the rc releases and tags at
  once and let the next push rebuild it.
- **Where the production signing runs.**
  - A (planned): in each platform leg, where the test key signs today. The
    path the rehearsal exercised stays the release path, and the legs'
    checks (`Expand-Archive` on the signed ZIP, `install.sh` on the signed
    tarball, `codesign` on the bundle extracted from the signed tarball) see
    the bytes that ship. Cost: the secret reaches four runners, in jobs that
    also run third-party actions.
  - B: one signing job on `ubuntu-latest` over the downloaded artifacts,
    before `SHA256SUMS`. The secret is in one job, but the Windows and macOS
    checks ran on unsigned bytes, and the release path differs from the
    rehearsed one.
- **The secret's scope.**
  - A (planned, as the constraint asks): a repository secret,
    `UPDATE_SIGNING_KEY`. Any collaborator's workflow on any branch could
    read it; the repository has one maintainer.
  - B: an environment secret in an environment restricted to `main` with the
    user as required reviewer. Stronger, and it adds a human approval to
    every signing run, but more to set up by hand and a click per run.
- **Cleanup of the rc releases.**
  - A (planned): keep `v2.0.0-rc.1` and `v2.0.0-rc.2` as prereleases until
    the macOS and Linux rows are measured or 2.0.0 is published and
    verified, then delete them with `gh release delete --cleanup-tag`. They
    are the only way to run the **Unverified** rows later without two more
    builds, a final install is never offered them, and `/releases/latest`
    does not move. Anyone who installs one becomes a prerelease install that
    follows to 2.0.0, which is the intended policy. Their notes say they are
    release candidates for testing. This holds only if release-please
    ignores the tags (above).
  - B: delete both right after the Windows rows are recorded. The Releases
    page and the stats stay clean, but a later macOS or Linux check needs
    new rc builds.
- **The final-release path is first exercised at the real release.** The
  build, the signing, the checks and the asset list are shared with the rc
  path and proven by the dry run. What differs is the upload to
  release-please's draft and the publish, the shape #364 introduced and the
  Tauri releases used. A failure leaves a draft, which neither the updater
  nor `/releases/latest` sees, and the failed jobs can be re-run. The
  alternative, making the final path dispatchable too, would allow
  publishing a stable release without the release PR, which defeats the
  `/release` gate.
- **The `/release` suspension.**
  - A (planned): Step 5 lifts it and puts a first-release checklist in its
    place, so the user sees what is unverified when typing `/release`.
  - B: this plan leaves the skill suspended and the user lifts it by hand.
    Nothing can be shipped by accident, but the plan ends with the release
    still blocked by a text.
- **The install docs in this plan (Step 5).**
  - A (planned): the last step, merged just before `/release`, with the
    short window in which the docs are ahead of the release.
  - B: left out as a remaining item for the release itself. Then the first
    release can go out while the README says nothing is released.
- **macOS and Linux stay Unverified.** Only a Windows PC is at hand. The
  `.app` swap, the tarball update and the deb notice are covered by the
  engine's tests and CI, not by a real machine. The gpui-port plan's gate
  ("a real-device install, an update from N to N+1 and an uninstall per
  format") is therefore not met for those formats by this plan; whether
  2.0.0 ships with them unverified is the user's decision at `/release`.
- **Risks to watch.**
  - The updater has never read the real repository's release list, which
    holds the Tauri releases with other asset names (Step 4's first rows).
  - An empty or wrong secret must fail at the guard, before the builds, not
    after 75 minutes.
  - A signed file renamed anywhere between the leg and the release breaks
    its signature.
  - An rc published by mistake as a non-prerelease would become
    `/releases/latest` and break the Tauri installs' `latest.json` lookup
    early; the workflow derives the flag from the version and passes
    `--latest=false`.

## Progress

- (2026-11-21) Step 1 complete
- (2026-11-21) Step 2 complete
- (2026-11-21) Step 3 complete
