# PR merge lifecycle

The same procedure applies to every PR in `develop` and to a standalone `/pr`.
When `pr-runner` returns `ready` or `merged`, start `merger` without skipping
the post-merge work, even when the PR is already merged. Do not specify `model`;
leave it to `merger`'s frontmatter.

## The approval check reports; it does not gate

**An ordinary merge is not held for user approval.** Merging an ordinary PR
only changes `main`; nothing reaches users until a release PR is merged, so a
bad merge costs a revert and nothing else. The user has granted a standing
approval for merges on that basis, with two exceptions:

- **The release PR is the human gate.** No agent merges it on its own. It is
  identified by its branch, `release-please--branches--main`; when `pr-runner`
  hands you that PR, stop and leave the merge to the user. It ships only when
  the user types `/release` ([`../../release/SKILL.md`](../../release/SKILL.md)),
  whose invocation is the approval.
- **Paths that change what a release is or how it is built are not covered.**
  `.github/**`, `release-please-config.json` and
  `.release-please-manifest.json`. When the check prints an `approval` line for one of these, stop and ask the user before
  starting `merger` with the approved flag.

Everything else keeps the report-only behavior below.

That does not mean the judgment is skipped. Run the check yourself first — it
is read-only — so you know what it would have flagged:

```bash
bash .claude/skills/merge/scripts/check-merge-approval.sh {number}
```

Then, outside the two exceptions above, start `merger` **with the approved flag**,
whatever the check returned:

```
Agent(
  subagent_type: "merger",
  run_in_background: false,
  prompt: "PR number: {number}
Approved: yes (this repository grants a standing approval for ordinary merges; they only change main, so a bad merge costs a revert)

Skip the approval judgment and go from the merge through post-merge waiting, main sync, and branch cleanup.")
```

**Do not set `run_in_background: true`.** Waiting for CI and waiting for
post-merge are carved into the foreground inside the agent.

**Report every `approval` and `unclassified` line the check printed**, in the
progress report for that step. Merging without asking is not the same as merging
without looking, and the user keeps visibility through that report.

Call out prominently — still without stopping — anything in the class that can
make **code we did not write execute on a developer machine or in CI**:
`Cargo.lock`, `.cargo/**`, `mise.toml`, `mise.lock`, and the toolchain files.
Those are the only findings whose risk does not wait for a release: a new
dependency's build script runs the moment someone builds. Name the new dependency
in the report when the check flags one.

`build.rs`, `.claude/**` and `tools/**` stay approval-required in the check
because they are worth seeing, but they are our own code; report them and move
on. (`.github/**` is not in this list: it shapes the release build, so it stops
per the exceptions above.)

## What `merger` does

`merger` owns the squash merge, waiting for the post-merge workflow, syncing
main, and cleaning up the branch. It carries an approval judgment of its own
(with `.claude/skills/merge/scripts/check-merge-approval.sh` as the source of
truth), but the approved flag above skips it, so the judgment that matters here
is the read-only one you ran yourself. It does not bypass protections with
`--admin`, and branch deletion is left to `merger`'s existing cleanup.

**Its branch cleanup deletes every local branch already merged into
`origin/main` that is not checked out in any worktree, and a branch you just
created with no commits on it yet counts as merged.** It leaves every worktree's
checkout alone, so a checked-out branch survives, but do not create the next
branch while a `merger` is still running anyway. Wait for `MERGED` first.

All that comes back is a count summary and the URLs of failed runs — no run tables, no wait logs. Main
does not take on root-cause investigation: do not go fetch details that were not
returned, via `gh run list` or otherwise, outside of "The minimum corroboration
for R6" below.

Branch on the terminal state of the return value:

| Terminal state                             | What to do |
| ------------------------------------------ | ---------- |
| `MERGED`                                   | Report the merge commit SHA, whether this was a fresh merge or a recovery of an already-merged PR, and the post-merge summary, in one line. `develop` moves on to the next Phase / Step; a standalone `/pr` finishes normally |
| `NEEDS_APPROVAL`                           | Should not happen here (the approved flag is always passed). Go to "Handling `NEEDS_APPROVAL`" below |
| `NOT_READY`                                | Go to "Handling `NOT_READY`" below |
| `POST_MERGE_FAILED` / `POST_MERGE_TIMEOUT` | **Do not proceed.** Do the minimum corroboration below, then report the failed run URL and how long it waited. Add that the PR is merged but main sync and branch cleanup have not run |
| `FAILED`                                   | **Do not proceed.** Do the minimum corroboration below, then report the reason, the step it stopped at, and whether it merged |

Ask the user for direction on `POST_MERGE_*` / `FAILED`. Only when post-merge
succeeded (or had no runs to wait on) and main sync and branch cleanup also
succeeded does it finish normally as `MERGED`.

## The minimum corroboration for R6

`merger` is contractually not an investigator, so before asserting a
`POST_MERGE_*` / `FAILED` reason to the user, main runs one or two read-only
commands to corroborate the summary. No root-cause investigation, no reruns, no
re-merging.

1. For every terminal state, check once whether the PR merged and what its
   checks look like now.

   ```bash
   gh pr view <PR number> --json state,mergeCommit,mergeStateStatus,statusCheckRollup
   ```

2. If `POST_MERGE_FAILED` came with a failed run URL, check that URL once. Only
   when `POST_MERGE_TIMEOUT` came with no URL, check the run status once by
   merge commit SHA. This check covers the post-merge state only; do not use it
   to infer whether a deploy fired.

   ```bash
   gh run view <failed run URL> --json status,conclusion,url
   gh run list --commit <merge commit SHA> --limit 20 --json status,conclusion,url
   ```

Only when `merger`'s summary and the corroboration agree do you report the
reason, the merge state, and the outstanding post-merge work as established
fact. When they disagree, or when the corroboration alone does not support the
reason, do not re-dispatch `merger`: report only the established facts and the
discrepancy with the summary, and stop. Do not assert a cause; ask the user for
direction.

## Handling `NEEDS_APPROVAL`

**You should not see this state**, because § The approval check reports; it does
not gate has you start `merger` with the approved flag every time. If it comes
back anyway, the flag was not passed: re-dispatch with it, exactly as that
section describes, rather than asking the user.

`merger` itself keeps its fail-closed contract — that is deliberate, so the
agent stays correct for a repository that has something to release. The standing
approval lives in the caller, not in the agent.

A malfunction in the approval judgment itself is still `FAILED`, and `FAILED`
still stops and asks.

## Handling `NOT_READY`

To absorb changes between `pr-runner` returning `ready` and `merger` looking at
the state, retry automatically exactly once.

1. Re-dispatch `pr-runner` in existing-PR mode
2. On `ready` / `merged`, start `merger` again. `closed` / `aborted` /
   `needs_discussion` follow the caller's usual branching
3. If the restarted `merger` returns `NOT_READY` again, report the observed
   state and stop

Do not try a third time.
