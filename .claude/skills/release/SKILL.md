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

The release PR (branch `release-please--branches--main`, title
`chore(main): release X.Y.Z`) is the human gate described in
[`../develop/references/pr-merge-lifecycle.md`](../develop/references/pr-merge-lifecycle.md):
no agent merges it on its own. **`/release` is a command the user types, and
typing it is the approval to ship**, the same way typing `/merge` approves a
merge. `disable-model-invocation: true` upholds that premise; do not remove it.

What a merge sets off cannot be undone: release-please tags `main` and creates
the GitHub Release, the `build` job in `.github/workflows/release.yml` uploads
installers and `latest.json`, and every installed copy is offered the update on
its next launch. A revert on `main` reaches none of that.

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

Skip the approval judgment and go from the merge through post-merge waiting, main sync, and branch cleanup. Post-merge includes the Release workflow: release-please tags the release, then four build jobs upload installers and latest.json, which takes a while.")
```

Do not specify `model`. Branch on the return value as `/merge` does
([`../merge/SKILL.md`](../merge/SKILL.md) § 2). Only `MERGED` continues to
step 3; for `POST_MERGE_FAILED` / `POST_MERGE_TIMEOUT`, say that **the release
PR is merged and the tag may already exist**, and still run step 3 so the user
sees what did and did not get published.

## 3. Verifying the Release

The tag is `v` + the version from the PR title.

```bash
bash .claude/skills/release/scripts/verify-release.sh v<version>
```

- `STATUS=ok` (exit 0): report the Release URL
  (`https://github.com/minodisk/handpic/releases/tag/v<version>`) and finish
- `STATUS=incomplete` (exit 1): report the `missing` lines verbatim. A missing
  installer usually means one build job failed; a missing or incomplete
  `latest.json` means installed copies will not be offered this update. Ask the
  user for direction — do not re-run workflows, edit the Release, or delete the
  tag on your own
- exit 2: the Release could not be read at all (the tag was not created, or
  `gh` failed). Report it and ask for direction
