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

## Releases are suspended

Releases are suspended until the first native release's gate opens (Step 5
of the native-release plan, `docs/plans/20261121-native-release/plan.md`,
after the rc dry run on the real feed). `.github/workflows/release.yml`
builds, signs and publishes the app's release, but the install docs, the
reinstall notice and the dry run are not in place yet. **`/release` stops
here**: tell the user that releases are suspended until the first native
release's gate opens, and do not merge the release PR. The rest of this
skill describes the native release as it will run once the suspension is
lifted.

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
   the tag's version and that the `UPDATE_SIGNING_KEY` secret is a key
   matching `crates/app/packaging/update.pub`.
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
on another ref only builds and signs, uploading workflow artifacts. Delete a
candidate with `gh release delete v<version> --cleanup-tag` (only when the
user asks).
