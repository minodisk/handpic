---
name: pr
description: Create a PR, clear GitHub Checks / conflicts / review feedback, pass the safety judgment, and carry it through merge and post-merge. Given a PR number as an argument, runs the same process against that existing PR.
argument-hint: "[pr-number]"
allowed-tools: Agent, AskUserQuestion, Bash, ListAgents, SendMessage, TaskStop
---

```
/pr           → branch prep → local review → pr-runner → merger
/pr 42        → pr-runner (clears the problems on an existing PR) → merger
```

The actual work belongs to the `pr-runner` agent and its children. This skill is
responsible only for **argument parsing, branch prep, dispatch, branching on
return values, and asking the user**. The procedures for fixing CI, resolving
conflicts, and addressing review feedback live under
`.claude/agents/pr-runner.md`. **Do not copy them back here.**

- **Write PR titles and bodies in English**
- **Minimize pushes.** Every push burns CI and automated-review runner time.
  Batching into a single push is `pr-runner`'s job, so this skill never pushes
  (it commits at the end of step 1)
- **Do not use `/pr` when you need to iterate to verify behavior.** Create the
  PR by hand with `gh pr create --draft` and iterate while it is a draft (draft
  PRs are exempt from automated review, which keeps the push cost down). Once it
  is ready, switch it to "Ready for review" and then call `/pr <number>`.
  `pr-runner` treats `ACTION=draft` as a terminal state it cannot advance on its
  own and returns `aborted`, so throwing a draft at `/pr` will not get you
  anywhere

## 0. Parsing arguments

Scan `$ARGUMENTS` line by line and pull out the following.

| Argument                         | Meaning                                       |
| -------------------------------- | --------------------------------------------- |
| A bare numeric token (e.g. `42`) | **Existing-PR mode**. Jump straight to step 3  |
| `--no-branch`                    | Skip step 1 (branch prep)                      |

When the arguments are empty, first check whether the current branch already has
an open PR:

```bash
gh pr view --json number,state -q 'select(.state == "OPEN") | .number'
```

`gh pr view` returns the most recent PR for the current branch, so it succeeds
even when that PR is already MERGED / CLOSED. Filter on `state` and treat it as
**existing-PR mode** — jumping straight to step 3 — only when **the output
contains a PR number** (this avoids moving the branch and relocating the work).
If the **output is empty** (no open PR) or **the command fails** (no PR on that
branch), this is **new-PR mode**: run from step 1 in order.

Any remaining text beyond the numeric token and `--no-branch` is not discarded —
pass it to `pr-runner` as raw material for the PR title and body. If there is no
such material, let `pr-runner` compose the title and body too (do not read
`git diff` yourself to compose them).

> `--no-branch` is for callers that create the branch and commit themselves
> before invoking `/pr`.

## 1. Branch prep

**Skip this when a PR number was passed, or when `--no-branch` was given.**

Move the current changes onto a branch cut from the latest main. Pick a
kebab-case branch name prefixed with `feature/` / `fix/` / `docs/` /
`refactor/` / `chore/` / `test/`.

Stash the uncommitted changes under a unique tag (the `STASHED=true|false` in
the output tells you whether anything was stashed):

```bash
bash .claude/skills/pr/scripts/stash-work.sh
```

Sync to the latest main. **Always pass `dangerouslyDisableSandbox: true` to the
Bash tool** (`git checkout` fails with `Operation not permitted` because it
cannot unlink things under `.claude/`):

```bash
mise run git:main
```

Create the new branch and pop only the stash recorded by this run's marker:

```bash
bash .claude/skills/pr/scripts/restore-stash.sh <branch-name>
```

Exit codes: **1 = stash apply failed** (both the stash and the marker survive;
report to the user and ask them to resolve it by hand), **3 = branch creation
failed** (the stash is untouched). Distinguish exit 3 cases by stderr: on
`unable to unlink old '.claude/…'` retry with `dangerouslyDisableSandbox: true`;
if a branch of that name already exists, call it again with a different name
(the stash is still there, so it will be restored).

Finally run `mise run fmt`, then stage only the target paths and commit.

```bash
mise run fmt
git status --porcelain
git add <target paths>
git commit -m "<commit message>"
```

**fmt comes before the commit** (leave it until after and the formatting diff
stays uncommitted, so only the `ci:fmt` check fails on GitHub). `git add .` /
`git add -A` are forbidden. **Do not push** (that is `pr-runner`'s job). **This
skill (main) decides the commit message** (Conventional Commits, in English).
The "do not read `git diff` yourself" rule in § 0 applies to the PR title and
body, not to commit messages.

The reason to commit here is that the local review in the next step takes its
diff from `git diff origin/main...HEAD`. Left uncommitted, a fresh branch has
HEAD == origin/main, the diff is empty, and the APPROVED is meaningless.

## 2. Local review

**Skip this when a PR number was passed** (an existing PR was reviewed when it
was created).

Determine whether the current HEAD has already been APPROVED by a local review.
It also counts as already APPROVED when everything after the approved commit is
confined to `docs/plans/**` (review history files, Progress lines added to
plan.md). Read the final `need_review=true|false` line; on `false`, go to
step 3:

```bash
bash .claude/skills/pr/scripts/check-local-review.sh
```

On `true`, start `local-review-runner`. **The caller owns the review output
path**, so first get the branch name (`git rev-parse --abbrev-ref HEAD`) and the
time (`date +%Y%m%d-%H%M`). Also generate a watchdog run-id once, with
`date +%s`. The state file is keyed on the run-id alone (see the header comment
in `watch-local-review.sh` for why a branch + review-file pair can collide with
another run started in the same minute, and why second precision is enough).
`Bash(date *)` is already on the `.claude/settings.json` allowlist, so
generating it this way needs no extra permission. Reuse this run-id until the
local review finishes:

```
Agent(
  subagent_type: "local-review-runner",
  run_in_background: false,
  prompt: "Review output file: docs/plans/review-history/<branch>/review-<YYYYMMDD-HHmm>.md
Branch name: <branch>
Task context: <the PR's purpose and main changes>")
```

In the **same turn**, start the watchdog with `run_in_background: true` (this is
what keeps main from waiting forever if a grandchild agent dies on the harness
side):

```bash
bash .claude/skills/develop/scripts/watch-local-review.sh --branch='<branch>' --review-file='docs/plans/review-history/<branch>/review-<YYYYMMDD-HHmm>.md' --run-id='<run-id>'
```

`<run-id>` is the value just generated; always pass the same one, re-arms
included. How to handle the runner's return and the watchdog's exit (`TaskStop`
for cleanup plus `--run-id='<run-id>' --cleanup` to delete the state file /
`result=completed` means immediately re-arm with the same run-id to keep a
bounded wake until the runner returns / `result=deadline` means check the runner
and its grandchildren with `ListAgents`, and if needed ask for a status report
via `SendMessage` before re-arming with the same run-id) is identical to
"The watchdog (preventing an unbounded wait)" in develop's SKILL.md 2.3.

On `APPROVED`, go to step 3. On `MAX_ROUNDS_REACHED` / `LOCAL_CHECK_FAILED`,
report the return value to the user verbatim and ask for direction (**do not
proceed to PR creation on your own**).

## 3. Starting `pr-runner`

Do not specify `model` (leave it to `pr-runner`'s frontmatter).

```
Agent(
  subagent_type: "pr-runner",
  run_in_background: false,
  prompt: "<the material below>")
```

What the prompt must contain:

- **Existing-PR mode**: the PR number
- **New-PR mode**: the PR title and body (pass through any title/body material
  you were given; if there is none, say "compose it from the changes" and hand
  over whatever material you have, such as the plan file path and step number)
- If uncommitted changes remain (e.g. when `--no-branch` skipped step 1): the
  target paths and **the commit message** (`pr-runner` returns `aborted` by
  contract when there is a diff but no message)

## 4. Branching on the return value

- `ready`: report the PR URL and final status, then go to step 6
- `merged`: go to step 6 to finish the post-merge work on the merged PR
- `closed`: report that it was closed unmerged, and stop
- `aborted`: pass along the reason and the child's report verbatim, and stop.
  **Do not go read the CI logs or the diff yourself**
- `needs_discussion`: go to step 5

## 5. Handling `needs_discussion`

`pr-runner` cannot ask the user questions, so when the review plan contains
`discuss` it comes back with the plan JSON attached. This is the one point of
dialogue.

1. Present each `category: "discuss"` entry (the feedback and its `plan`) to the
   user with `AskUserQuestion`
2. Reassign `category` to `fix` / `reject` based on the answer, and rewrite
   `plan` to match
3. If leaving items pending means no `fix` / `reject` entries remain at all, do
   not re-dispatch: report what is pending and stop
4. Otherwise re-dispatch `pr-runner` with the converted JSON attached

```
Agent(
  subagent_type: "pr-runner",
  run_in_background: false,
  prompt: "PR number: <number>
Converted plan JSON:
<JSON array>")
```

A re-dispatched `pr-runner` skips `pr-review-planner`, resumes from
`pr-review-addresser`, and returns to its monitoring loop. When it comes back,
follow the step 4 branching.

A `needs_discussion` whose attached plan JSON is `[]` is a deadlock: zero
unresolved threads but a CHANGES_REQUESTED still standing, which needs a human
to dismiss the review. Do not re-dispatch; report to the user and stop.

`Agent`'s `prompt` takes structured data as-is, so do not detour through a
general-purpose subagent.

## 6. Merge and post-merge

For both `ready` and `merged`, follow
[`../develop/references/pr-merge-lifecycle.md`](../develop/references/pr-merge-lifecycle.md),
which is shared with develop. Only `MERGED` (post-merge workflow, main sync, and
branch cleanup all done) counts as a normal finish.

Note the part of that contract that differs from what you might expect: the
approval check is **run and reported, not used as a gate**. Run it yourself
(read-only), start `merger` with the approved flag regardless, and tell the user
what it flagged — especially anything that lets third-party code run on a
machine, such as a new dependency in `Cargo.lock`. The
exceptions are in the reference: never merge the release PR
(`release-please--branches--main`; it ships through the user's `/release`), and ask the user before merging when the
check flags a release-shaping path (`.github/**`, `release-please-config.json`,
`.release-please-manifest.json`).
