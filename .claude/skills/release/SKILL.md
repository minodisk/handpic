---
name: release
description: Ships the release-please release PR — shows what it will release,
  merges it, waits for the release build, and verifies the published Release.
argument-hint: "[pr-number]"
allowed-tools: Agent, Bash, AskUserQuestion
disable-model-invocation: true
---

```
/release      → finds the open release PR, then merges and verifies it
/release 57   → the same, for PR 57
```

## The first native release (2.0.0)

2.0.0 is the first release of the native app (`crates/app`), and the first
release `release.yml` publishes from the release PR. Until it is published
and verified, a `/release` whose PR title reads `2.0.0` also runs this
checklist (step 1) and prepends the reinstall notice (step 2). The list is
the native-release plan's "What remains before PR #928 can be merged"
(`docs/plans/20261121-native-release/plan.md`, archived under
`docs/plans/_archived/` once that plan is done), as of 2026-11-22:

1. Done: the Windows `handpic.exe` is a GUI-subsystem binary (PR #1194).
2. Done: `release.yml` builds, signs with the production key and publishes;
   the `UPDATE_SIGNING_KEY` secret is registered (since 2026-11-26 in the
   `release` environment, which allows only `main`).
3. Done: the rc dry run on the real feed (`v2.0.0-rc.1` to `v2.0.0-rc.2`,
   both kept as prereleases). **Measured** on one Windows PC: the portable
   ZIP and the Inno Setup update, the feed and release-please leaving the
   release PR alone. The Inno setup is unsigned and SmartScreen prompts for
   it, as the install docs say.
4. **Unverified**, shipping with them is the user's decision:
   - the macOS `.app` (install, the swap in `/Applications`, the refusal
     from the DMG or Downloads, Gatekeeper and the folder-access grants);
   - the Linux `.tar.gz` with `install.sh` (install and update) and the
     `.deb` (install, the release-page notice, `Conflicts` / `Replaces:
     riffle`);
   - on Windows: a standard (non-administrator) account, the uninstall
     entries and shortcuts of an install beside Riffle's, and a real network
     cut during a download.

   No macOS or Linux machine was at hand; those rows rest on the engine's
   tests and CI. The record is
   `docs/plans/20261109-gpui-port/packaging-updater.md`, "The rc dry run on
   the real feed".
5. Done: the install docs (`README.md`, `docs/humans/usage.md` and their
   Japanese pairs), the reinstall notice
   ([`references/reinstall-notice.md`](./references/reinstall-notice.md))
   and this checklist in place of the suspension.
6. PR #928 green: step 1's check state.
7. Accepted risk: the release-please path of `release.yml` (upload to the
   draft, then publish) runs for the first time at this release. A failure
   leaves a draft that neither the updater nor `/releases/latest` sees.

After 2.0.0 is published and verified, the rc releases can be deleted
(`gh release delete v2.0.0-rc.N --cleanup-tag`), only when the user asks.

The release PR (branch `release-please--branches--main`, title
`chore(main): release X.Y.Z`) is the human gate described in
[`../develop/references/pr-merge-lifecycle.md`](../develop/references/pr-merge-lifecycle.md):
no agent merges it on its own. **`/release` is a command the user types, and
typing it is the approval to ship**, the same way typing `/merge` approves a
merge. `disable-model-invocation: true` upholds that premise; do not remove it.

What a merge sets off cannot be undone. On the push to `main`, the `Release`
workflow (`.github/workflows/release.yml`, see
[`../../../docs/agents/app.md`](../../../docs/agents/app.md#the-release-workflow-publishes-a-release-inferred)):

1. `release-please` tags `main` `v<version>` and creates a **draft** GitHub
   Release with the changelog as its notes.
2. `guard` checks that `crates/app/Cargo.toml` on the release commit reads
   the tag's version and that the `release` environment's
   `UPDATE_SIGNING_KEY` secret is a key matching
   `crates/app/packaging/update.pub`.
3. `build` (`package.yml`, about 75 minutes) builds every platform's
   packages and signs the updater's archives with the production key in
   each leg; `SHA256SUMS` is computed after signing.
4. `publish` checks the artifacts are exactly the eleven assets of
   `tools/package/assets.sh` and pass `sha256sum --check`, uploads them to
   the draft and only then publishes it as the latest release.

Every installed copy is offered the update on its next check. A revert on
`main` reaches none of that. A failed build leaves the draft, which neither
the updater nor `/releases/latest` sees.

## 0. Finding the release PR

A bare numeric token in `$ARGUMENTS` is the PR number. Otherwise look it up:

```bash
gh pr list --head release-please--branches--main --state open --json number,title,url
```

- None open: report that there is nothing to release. release-please only opens
  a release PR once a `feat:` or `fix:` commit has landed on `main` since the
  last release. Stop
- Given a number: check that its head branch is
  `release-please--branches--main`. If it is not, it is not a release PR; say so
  and stop (point at `/merge` for ordinary PRs)

## 1. Showing what will ship

```bash
gh pr view <number> --json title,headRefName,statusCheckRollup,mergeable,mergeStateStatus
gh pr diff <number> --color never
```

The diff is small: the version lines of the versioned files, the manifest, and
`CHANGELOG.md`. Report to the user, briefly: the version (from the title), the
CHANGELOG entries being added (the `+` lines under `CHANGELOG.md`, grouped as
release-please wrote them), and the check state.

**Stop without merging** when any check is not `SUCCESS` or the PR is not
mergeable: report the state and point at `/pr <number>`. Do not fix anything
here.

For 2.0.0, also report the checklist of
[the first native release](#the-first-native-release-200), naming every
**Unverified** row, and ask with `AskUserQuestion` whether to ship with them
unverified. Stop without merging unless the user says to ship.

Otherwise continue straight to step 2. The user already approved by typing
`/release`; do not ask again.

## 2. Merging

```
Agent(
  subagent_type: "merger",
  run_in_background: false,
  prompt: "PR number: <number>
Approved: yes (the user approved shipping this release by invoking /release; this is the release-please release PR, and the invocation is the human release gate)

Skip the approval judgment and go from the merge through post-merge waiting, main sync, and branch cleanup. Post-merge includes the Release workflow: release-please tags the release and makes a draft, then the guard, the four platform builds (signing the updater's archives) and the publish run, which takes about 75 minutes.")
```

For 2.0.0, add this to the prompt, so the reinstall notice is in the draft's
notes while the build runs, before the workflow publishes it:

```
Right after the merge, before waiting for the post-merge runs, run once in the foreground with timeout 600000:
bash .claude/skills/release/scripts/prepend-notice.sh v2.0.0
It waits for release-please's draft and prepends the reinstall notice to its notes. Report its STATUS line in your return.
```

`STATUS=ok` or `already` means the notice is in. For `published`, `missing`
or no STATUS line, tell the user that the notice is not in the notes and
that it can be prepended by hand with `gh release edit v2.0.0 --notes-file`
(the notes of a published release can still be edited).

Do not specify `model`. Branch on the return value as `/merge` does
([`../merge/SKILL.md`](../merge/SKILL.md) § 2). Only `MERGED` continues to
step 3; for `POST_MERGE_FAILED` / `POST_MERGE_TIMEOUT`, say that **the release
PR is merged and the tag may already exist**, and still run step 3 so the user
sees what did and did not get published.

## 3. Verifying the Release

The tag is `v` + the version from the PR title.

```bash
mise exec cargo:zipsign@0.2.2 -- bash .claude/skills/release/scripts/verify-release.sh v<version>
```

It checks that the release is published (not a draft), that its prerelease
flag agrees with the version, that exactly the eleven assets of
`tools/package/assets.sh` are attached (no `latest.json`), that they pass
`sha256sum --check` against its `SHA256SUMS`, and that the updater's archives
(the two macOS `.tar.gz`, the setup ZIP, the portable ZIP, the Linux `.tar.gz`)
pass `zipsign verify` with `crates/app/packaging/update.pub`. Optional second
and third arguments name another repository and verifying key (the test feed
and `crates/app/packaging/test/update.pub`).

- `STATUS=ok` (exit 0): report the Release URL
  (`https://github.com/minodisk/handpic/releases/tag/v<version>`) and finish
- `STATUS=incomplete` (exit 1): report the `missing` lines verbatim. A draft
  or a missing asset usually means a build or the publish job failed; a
  failed `sha256sum` or `zipsign` line means installed copies would refuse
  the update. Ask the user for direction — do not re-run workflows, edit the
  Release, or delete the tag on your own
- exit 2: the Release could not be read at all (the tag was not created, or
  `gh` failed), or zipsign 0.2.2 is not on `PATH`. Report it and ask for
  direction

## Cutting a release candidate

A release candidate is not cut through `/release` and needs no release PR.
It is a manual dispatch the user asks for, on `main`, with a version that has
a prerelease part:

```bash
gh workflow run release.yml --ref main -f version=2.0.0-rc.1
```

The workflow refuses a stable version and a version whose release or tag
exists. It builds and signs as above, then creates the tag `v<version>` at
the dispatched commit and a GitHub **prerelease** never marked latest, so
`/releases/latest` and stable installs are not affected; prerelease installs
are offered it. Nothing is committed and the release PR is untouched.
Verify it with the same script (`verify-release.sh v2.0.0-rc.1`). A dispatch
on another ref is rejected at `guard` by the `release` environment's
`main`-only branch policy and builds nothing; package a branch with the test
key instead (`gh workflow run package.yml --ref <branch> -f
update_test_repo=<owner/name>`). Delete a
candidate with `gh release delete v<version> --cleanup-tag` (only when the
user asks).
