---
name: develop
description: Drives everything from planning to PR merge and wrap-up through a chain of subagents. The main agent orchestrates plan → plan PR → per-step implementation, local review, and PR → wrap-up PR (with a single-PR mode that folds it all into one PR when there is only one step).
argument-hint: "[what you want to do]"
---

# Develop Skill

The main agent neither writes nor reads code. It starts subagents in order and
advances purely on their return values and the exit codes of scripts.

> **Always run the scripts by their relative paths.** Do not expand the
> `bash .claude/skills/...` invocations written in this document into absolute
> paths. Absolute paths do not match the relative-path allow rules in
> `.claude/settings.json`, so you get an unsandboxed-execution consent prompt
> every time. Run them with cwd at the repository root.

## Phase layout

```
Phase 0  0.1 plan → 0.2 user approval                                  ← no git operations
Phase 1  1.1 branch → 1.2 persist → 1.3 PR → 1.4 merge
Phase 2  2.1 branch → 2.2 implement → 2.3 review → 2.4 PR → 2.5 merge   ★repeat per step
Phase 3  3.1 branch → 3.2 learnings → 3.3 todo → 3.4 settings → 3.5 archive
         → 3.6 PR → 3.7 merge

Phase S  S.1 branch → S.2 persist → S.3 implement → S.4 review → S.5 wrap-up
         → S.6 PR → S.7 merge            ← replaces Phases 1–3 when there is one step
```

**When the plan has exactly one step, run Phase S (single-PR mode) instead of
Phases 1–3.** Nothing is dropped; only the PR boundaries fold into one
(§ Phase S). With two or more steps, run the usual Phase 1 → Phase 2 (per step)
→ Phase 3.

Phases 1–3 all have the same shape: **branch → work → PR → merge**. Only Phase 0
has no git operations, so interrupting it has no side effects. Branch creation
goes at the **head** of each phase and each loop (create it at the tail and one
phase ends up making the branch for the next one, which crosses responsibility
boundaries).

See the mapping table in [README.md](./README.md) for how this corresponds to
the old Phases 1–5.

## Shared procedures

### Creating a branch (shared by 1.1 / 2.1 / 3.1 / S.1)

```bash
bash .claude/skills/develop/scripts/ensure-new-branch.sh {branch-name} [{feature-name}]
```

- **Always call it with the Bash tool's `dangerouslyDisableSandbox: true`.** When
  the diff against `origin/main` includes `.claude/`, `git checkout` fails with
  `unable to unlink old '.claude/...': Operation not permitted` (measured; there
  is no way to tell in advance). Worse, `git checkout` itself returns exit 0, so
  the script has a safeguard that compares the state of `.claude/` before and
  after the checkout and exits with an error. By the time that safeguard fires,
  the ref has been created and the index updated, so manual recovery is needed
  (`git checkout -f {original branch}` + `git branch -D {branch}`, which also
  needs `dangerouslyDisableSandbox: true`)
- The script does `git fetch origin main` → checks for a name collision against
  both local refs and origin → creates the branch from `origin/main`.
  **Syncing to main is built into this script**, so there is no separate skill
  to call
- Passing a feature-name as the second argument validates that it is kebab-case
  and usable as a plan directory name (the same regex as `ensure-plan-dir.sh`)
- When there are uncommitted changes, the **main agent runs
  `git stash --include-untracked` before calling this script**, so unrelated
  changes are not carried onto the new branch (see the "Creating a branch" step
  in 1.1 for the exact command). **Do not `git stash pop`** — popping restores
  the stashed work onto the new branch as-is, and you proceed into implementer /
  review / PR creation carrying an uncommitted diff
- Exit 1 means a kebab-case violation, an existing-branch collision, or the
  `.claude/` diff detection above. The branch names in 2.1 and 3.1 are generated
  deterministically (`{feature-name}-step-{n}` / `chore/archive-{feature-name}`),
  so a leftover branch from a past failure will collide. In that case, ask the
  user to clean it up

### Starting `pr-runner` and its return values (shared by 1.3 / 2.4 / 3.6 / S.6)

```
Agent(
  subagent_type: "pr-runner",
  run_in_background: false,
  prompt: "{the material below}")
```

Do not specify `model` (leave it to `pr-runner`'s frontmatter). What the prompt
must contain:

- The PR title and body, or the material for the body (plan file path / step
  number)
- If there are uncommitted changes, **the paths to commit and the commit
  message**. By contract, `pr-runner` returns `aborted`
  (`missing_commit_message`) when there is a diff but no message

`pr-runner` takes care of `mise run fmt` → commit → `mise run ci:local` → push → PR
creation → watching checks → resolving conflicts, fixing CI failures, and
addressing review feedback. **The main agent never calls `commit-push.sh`**
(concentrating commits in `pr-runner` means the local check runs once, and CI
failures are handled in exactly one place). Neither CI logs nor diffs reach the
main agent.

Branch on the terminal state of the return value:

| Terminal state     | What to do                                                                                                      | Proceed? |
| ------------------ | --------------------------------------------------------------------------------------------------------------- | -------- |
| `ready`            | Mergeable. Report the PR URL and go to § Merge                                                                    | ✅       |
| `merged`           | Already merged (auto-merge, or the user merged it immediately). Go to § Merge anyway, for post-merge, main sync, and branch cleanup | ✅       |
| `closed`           | Abnormal termination: the PR was closed unmerged. **Do not proceed.** Report the situation and ask for direction | ❌       |
| `aborted`          | Gave up because it could not handle this automatically. Report with the reason (`draft` / `behind` / `blocked` / `wait_timeout` etc.) and ask for direction | ❌       |
| `needs_discussion` | The review plan contains `discuss`. See below                                                                     | —        |

`needs_discussion` comes back because `pr-runner` cannot ask the user questions,
and it is the one point of dialogue:

1. Present each `category: "discuss"` entry of the attached plan JSON (the
   feedback and its `plan`) to the user with `AskUserQuestion`
2. Reassign `category` to `fix` / `reject` based on the answer, and rewrite
   `plan` too
3. If no `fix` / `reject` entries remain at all, do not re-dispatch: report what
   is pending and ask for direction
4. Otherwise re-dispatch `pr-runner` with the converted JSON attached, and when
   it returns, go back to the branching table above

A `needs_discussion` whose plan JSON is `[]` is a deadlock: zero unresolved
threads but a CHANGES_REQUESTED still standing, which needs a human to dismiss
the review. Do not re-dispatch; report to the user.

`Agent`'s `prompt` takes structured data as-is, so do not detour through a
general-purpose subagent.

### Merge (shared by 1.4 / 2.5 / 3.7 / S.7)

Follow the shared contract in
[`references/pr-merge-lifecycle.md`](references/pr-merge-lifecycle.md). On
`MERGED`, move to the next Phase / Step without stopping to ask the user.

Two things from that contract you will use every time:

- **Ordinary merges are not held for approval here.** Run
  `bash .claude/skills/merge/scripts/check-merge-approval.sh {number}` yourself
  (read-only), then start `merger` with the approved flag whatever it returned —
  except that the release PR (`release-please--branches--main`) is never merged
  by an agent on its own (it ships through the user's `/release`), and an `approval` line for a release-shaping path (`.github/**`,
  `release-please-config.json`, `.release-please-manifest.json`,
  `crates/app/tauri.conf.json`) means asking the user first (see the reference).
  Put everything it flagged into the step's progress report — merging without
  asking is not merging without looking
- **Do not create the next branch until `merger` has returned `MERGED`.** Its
  branch cleanup deletes local branches already merged into `origin/main`, and a
  branch you just created with no commits yet counts as one. A branch checked
  out in any worktree is skipped, but one created and not yet checked out is
  still deleted

## Phase 0: Planning

### 0.1 Making the plan

```
Agent(
  subagent_type: "planner",
  run_in_background: false,
  prompt: "{purpose / constraints / acceptance criteria}")
```

Do not specify `model` (leave it to `planner`'s frontmatter).

> **Do not change the model setting in `planner`'s frontmatter on your own.**
> The planning model is pinned deliberately, so even when a change looks
> warranted, always check with the user first (it is set in
> `.claude/agents/planner.md`).

What the prompt should contain:

- The purpose (the user's requirements)
- Constraints (project conventions, target files, existing implementations)
- Acceptance criteria

Return value: **the full text of plan.md** (in a form you can write out
directly), the number of steps and the main files being changed, and the
trade-offs and open questions.

`planner` cannot ask the user questions, so anything genuinely contested comes
back under "trade-offs and risks" as two options. When the user needs to decide,
**the main agent asks with `AskUserQuestion`** (do not write the question in
prose).

### 0.2 User approval

Present the plan to the user and get approval.

- If they want changes, go back to 0.1
- Do not proceed to Phase 1 until there is explicit approval ("OK", "go ahead",
  etc.)

On approval, **the main agent decides the following itself** (do not ask the
user):

- **The feature name** (kebab-case, used as the plan directory name). Pick
  something concise from the plan
- **The branch name** (one of `feature/<topic>` / `fix/<topic>` /
  `docs/<topic>` / `refactor/<topic>` / `chore/<topic>` / `test/<topic>`)

**If `planner` returned exactly one step, say explicitly at approval time that
you will proceed in single-PR mode (Phase S).** The user can decline and ask for
the normal mode (Phases 1–3); if they do, follow that.

## Phase 1: Persisting the plan

### 1.1 Branch prep

#### Parallel-work check (run it first, before the stash)

`check-parallel-work.sh` is read-only and changes neither the current branch nor
the worktree. Run it at the **head** of 1.1 (before the stash and before
creating the branch) so that if it finds traces and you abandon the work, the
worktree is left completely untouched:

```bash
bash .claude/skills/develop/scripts/check-parallel-work.sh {feature-name}
```

It searches for the feature-name by **substring** across three sources: branches
checked out in other worktrees, branches on origin, and open PRs (the currently
checked-out branch is excluded from all three). You can pass the plan
directory's feature-name (in `YYYYMMDD-{feature-name}` form) directly: if it
starts with `YYYYMMDD-`, the script strips that off before comparing, so the
date part never causes a substring miss and a false "no traces".

Branch on the exit code:

- **exit 0**: No traces. Continue
- **exit 1**: Traces found. **Do not continue on your own.** Present the listed
  traces with `AskUserQuestion` and ask whether to continue or abort. The main
  agent cannot tell whether a trace is "something I made earlier" or "another
  session working right now", so the judgment belongs to the user
- **exit 2 / exit 3**: There are no traces to list; this is an error in the
  script or the environment. exit 2 is a bad feature-name argument (not
  kebab-case). exit 3 means the query to origin cannot be trusted, by one of two
  routes: (a) `git ls-remote` itself failed to reach origin, or (b)
  `git ls-remote` succeeded and produced output, yet not a single line matched
  the expected format (a 40-hex sha + tab + a `refs/heads/` prefix) and no
  branches could be read. (b) means a format change or broken line boundaries
  have defeated the filter; continuing would misjudge it as "no traces" and
  fail open, so it stops. Check the arguments, the network, and the raw output
  of `git ls-remote --heads origin`, fix it, and re-run. **Do not confuse this
  with a `gh` failure**: a failure of `gh pr list` itself (unauthenticated,
  network down, etc.) is only a warning and the git-based judgment continues
  (it does not produce exit 3)
- **Run this check before starting work even when resuming an existing plan**
  (i.e. skipping Phases 0–1 and entering at Phase 2; see the head of Phase 2)

#### Creating the branch

When there is an uncommitted diff, **the main agent runs this first**, so
unrelated changes are not carried onto the new branch (see § Creating a branch
for why there is no `git stash pop`):

```bash
git stash --include-untracked
```

Then follow § Creating a branch, passing the branch name and feature name
decided in 0.2:

```bash
bash .claude/skills/develop/scripts/ensure-new-branch.sh {branch-name} {feature-name}
```

**Persisting the plan (1.2) must come after creating the branch.** Create the
plan directory first and it sits uncommitted in the worktree, where the
`git stash --include-untracked` right after it sweeps the plan away too.

### 1.2 Persisting the plan

Create the plan folder. It prints `docs/plans/{YYYYMMDD}-{feature-name}` to
stdout:

```bash
bash .claude/skills/develop/scripts/ensure-plan-dir.sh {feature-name}
```

It exits 1 if a plan folder of that name already exists, either unarchived or
under `_archived/`. Change the feature name, or clean up the earlier plan
folder, and re-run.

Write `planner`'s output (the full plan.md) to `{plan-dir}/plan.md` **with the
`Write` tool**. `planner` returns it in plan.md form by contract, so no
reformatting is needed.

**Do not commit here.** The commit happens in 1.3, by `pr-runner`, after it runs
`mise run fmt`, and `mise run ci:local` runs afterwards.

Record the in-flight plan path in auto memory (`MEMORY.md`) — auto memory is a
user settings file outside the repository, so it is not under git:

```
- Plan in progress: docs/plans/YYYYMMDD-{feature-name}/plan.md
```

### 1.3 Creating and watching the plan PR

Start it per § Starting `pr-runner`. What to pass:

- **Title**: Conventional Commits, in English (e.g.
  `docs(plans): add the implementation plan for {feature-name}`)
- **Body**: an overview of the plan and `Plan: {plan file path}`
- **Paths to commit**: the plan directory you created
  (`docs/plans/YYYYMMDD-{feature-name}/`)
- **Commit message**: the same as the title

### 1.4 Merge

Start `merger` per § Merge. On `MERGED`, go to Phase 2.

A plan PR only touches `docs/plans/**`, so it normally merges automatically
without approval.

## Phase 2: The step loop

Run 2.1–2.5 for each step. **The main agent never reads the diff or the code
directly.** It all goes to subagents.

If you are resuming an existing plan and entering here with Phases 0–1 skipped,
run the 1.1 parallel-work check (`check-parallel-work.sh`) before starting the
first step.

### 2.1 Branch prep

Cut the step's branch per § Creating a branch:

```bash
bash .claude/skills/develop/scripts/ensure-new-branch.sh {feature-name}-step-{step_n}
```

### 2.2 Implementation

```
Agent(
  subagent_type: "implementer",
  run_in_background: false,
  prompt: "{plan file path / target step number and content / acceptance criteria}")
```

Return value: the terminal state, the commit hash, the list of changed files,
and the acceptance-criteria check results.

If the terminal state is `LOCAL_CHECK_FAILED` (it tried the local checks three
times, gave up, and stopped without committing), do not go to 2.3: report the
situation to the user and ask for direction.

### 2.3 Local review

**The caller owns the review output path** by contract, so first get the branch
name (`git rev-parse --abbrev-ref HEAD`) and the time (`date +%Y%m%d-%H%M`).
Also generate a watchdog run-id for this step's review once, with `date +%s`.
The state file is keyed on the run-id alone (see the header comment in
`watch-local-review.sh` for why a branch + review-file pair can collide with
another run started in the same minute, and why second precision is enough).
`Bash(date *)` is already on the `.claude/settings.json` allowlist, so
generating it this way lets you start the watchdog with no extra permission
setup. Reuse this run-id for the whole of 2.3, until this step's local review
finishes.

```
Agent(
  subagent_type: "local-review-runner",
  run_in_background: false,
  prompt: "Review output file: docs/plans/review-history/{branch-name}/review-{YYYYMMDD-HHmm}.md
Branch name: {branch-name}
Task context: Step {step_n} ({what the step covers})")
```

`local-review-runner` runs the reviewer ⇄ `local-review-addresser` loop for up
to 5 rounds and commits the review history. The main agent reads neither the
review content nor the `STATUS:` line.

The reviewer role starts `local-review-reviewer` (opus) via `Agent` every round.

#### The watchdog (preventing an unbounded wait)

If a grandchild agent (addresser / reviewer) dies on the harness side, the
failure notification is never delivered to the runner, and both the runner and
main wait forever (observed). To close this, start a watchdog with the `Bash`
tool's `run_in_background: true` **in the same turn** that you start the runner.
It exits either when the review history file has been pushed to that branch's
head on origin, or when it hits the deadline (1800 seconds by default), and that
exit is guaranteed to wake main up:

```bash
bash .claude/skills/develop/scripts/watch-local-review.sh --branch='{branch-name}' --review-file='docs/plans/review-history/{branch-name}/review-{YYYYMMDD-HHmm}.md' --run-id='{run-id}'
```

`{run-id}` is the value you just generated; always pass the same one, re-arms
within this step included (a `--branch` + `--review-file` pair alone can collide
with the state file of an independent run reviewing the same branch within the
same minute; see the header comment in `watch-local-review.sh`).

The runner's return and the watchdog's exit may arrive in either order. Branch
on whichever comes first:

- **The runner returns first**: stop the watchdog with `TaskStop` (ignore the
  error if it has already exited). On builds where `TaskStop` is unavailable,
  just ignore any watchdog notifications that arrive afterwards as "already
  handled". This matters most for `LOCAL_CHECK_FAILED`, where no push happens
  and the watchdog will not exit on its own, so this is the only cleanup. Also
  run
  `bash .claude/skills/develop/scripts/watch-local-review.sh --run-id='{run-id}' --cleanup`
  to delete this run-id's state file (leave it behind and the next run reviewing
  the same branch within the same minute may mistake the remnant for a re-arm)
- **The watchdog's `result=completed` (exit 0) comes first**: the push happens
  before the runner's Output, so this occurs routinely even on the happy path.
  Treat it as informational; it is not part of a recovery procedure. But if you
  stop waiting here, and the runner itself — or the runner → main notification —
  gets stuck after that final push, there is no path left to wake main again,
  and the unbounded wait this is meant to prevent comes back. **Re-arm the
  watchdog immediately with the same run-id** and keep a bounded wake until the
  runner returns (the state file is deleted on a `result=completed` exit, so the
  re-armed watchdog takes the current head as its new baseline; unless another
  push happens, it effectively becomes a timer that runs to the deadline).
  Branch on the next watchdog exit:
  - `result=completed` again: the push simply advanced further, so treat it as
    informational and keep re-arming
  - `result=deadline`: fall through into the "the watchdog's `result=deadline`"
    branch below (re-arms count against the same limit of 6)
- **The watchdog's `result=deadline` (exit 1)**: check the state of the runner
  **and its grandchildren (addresser / reviewer)** with `ListAgents`. In the
  incident this section closes (a grandchild's harness death not being delivered
  to the runner), the runner itself keeps showing as "running" in `ListAgents`,
  so the runner's state alone cannot distinguish it from a legitimately long
  review
  - A grandchild is completed / failed but the runner is still running: the
    grandchild's failure notification was never delivered to the runner and is
    stuck (exactly the incident this targets). Do not re-arm; go straight to "no
    response" below
  - Both the runner and the grandchildren are running, or `ListAgents` does not
    show grandchild state: a legitimately long review cannot be ruled out, so
    ask the runner for a status report on the current round and phase with
    `SendMessage`, **and in the same turn restart (re-arm) the watchdog with the
    same arguments**. The previous watchdog has already exited at this point, so
    without a re-arm there is no path to wake main again and nobody would notice
    if no response ever came. The re-armed watchdog's next exit
    (`result=completed` or `result=deadline`) is the only opportunity to check
    whether a response arrived
    - If a response arrived before that next watchdog exit, treat it as normal
      progress: on `result=completed` follow the `result=completed` handling
      above and keep re-arming; on `result=deadline` come back to this branch
      and check the state again (up to 6 re-arms, about 3 hours; past that,
      report the situation and ask the user for direction)
    - If no response arrived, go to "no response" below
  - No response (either of the above): treat it as a stuck state with the
    grandchild's failure notification undelivered. Check the debris in the
    working tree (uncommitted diff, the contents of the review history) and
    report to the user (stop here regardless of how many re-arms were used)

Branch on the terminal state of the return value:

- `APPROVED`: go to 2.4
- `MAX_ROUNDS_REACHED` / `LOCAL_CHECK_FAILED`: **do not proceed to PR
  creation**. Report the return value (the unresolved feedback, or the
  give-up report) to the user verbatim and ask for direction

### 2.4 Creating and watching the PR

Before appending anything, check the step's checkbox in the plan file
(`- [ ]`/`- [x]`). If it is not `[x]` (for instance, the addresser reverted it
to `[ ]` with a reason during review), do not append `Step N complete` and do
not proceed to PR creation. **An `APPROVED` from local review only guarantees
the quality of the diff, not that the step's acceptance criteria were met**, so
the two are tracked separately. In this case, report the reason and the
remaining work left in the plan file verbatim and ask for direction.

If it is `[x]`, **before** creating the PR, append
`(YYYY-MM-DD) Step N complete` to the Progress section of the plan file with
`Edit` (**do not include the PR number**). `implementer` is designed not to
touch Progress, so the main agent appends it here.

**Why before creating the PR (keeping automated review to one pass)**: push
after creating the PR and you move the head, which makes the review GitHub
auto-requested at PR creation stale against an older head; the approver then
judges the head "unreviewed" and cannot approve, and you are stuck. Getting the
Progress line in before PR creation means the head under review is the final
head from the start, and the review happens once. The link between the PR and
the plan is traceable through GitHub's commit links, so the Progress line does
not need a PR number.

Start it per § Starting `pr-runner`. What to pass:

- **Material for the body**: the plan file path and the step number.
  `pr-runner` reads the plan file and composes the title and body
- **Paths to commit**: the plan file
  (`docs/plans/YYYYMMDD-{feature-name}/plan.md`)
- **Commit message**: `docs(plans): record the completion of Step {step_n}`

### 2.5 Merge

Start `merger` per § Merge. On `MERGED`, go back to 2.1 if there is another
step, or to Phase 3 if not.

A step PR never stops for approval — § Merge has you pass the approved flag
every time — but it does get reported. Run the read-only check before starting
`merger` and put whatever it flagged into that step's progress report, calling
out any change that lets third-party code run on a machine (a new dependency in
`Cargo.lock` or `pnpm-lock.yaml`, and friends).

## Phase 3: Wrap-up (after every PR is merged)

Run this once, after every step's PR is merged. **The final step's implementer
does not move the plan to the archive** (2.4 refers to plan.md at its current
path, so moving it inside the final step breaks that reference). `planner`'s
step-splitting rules are likewise written on the assumption that the archive
move and the auto memory update are not part of any step.

### 3.1 Chore branch prep

Cut the chore branch per § Creating a branch:

```bash
bash .claude/skills/develop/scripts/ensure-new-branch.sh chore/archive-{feature-name}
```

### 3.2 Extracting learnings

```
Agent(
  subagent_type: "learnings-extractor",
  run_in_background: false,
  prompt: "Path to learnings.md: docs/plans/YYYYMMDD-{feature-name}/learnings.md")
```

Branch on the terminal state of the return value:

- `EXTRACTED`: the return value has two sections — **apply automatically**
  (consolidations and corrections into the existing guides under `docs/agents/`)
  and **record only** (new guides / candidates for promotion into `CLAUDE.md` /
  deferred judgments). Skip only the sections that are empty.
  1. **The apply-automatically part**: the main agent applies it to the existing
     guides without user consent. Creating or appending to `docs/learnings/` is
     forbidden.
  2. **The record-only part**: before 3.3, the main agent saves the concrete
     change, the rationale, and the completion criteria — without duplication —
     under `## Deferred issues (todo candidates)` in the feature's
     `learnings.md`. It then goes through `todo-curator`'s proposal and lands in
     `todo.md` in 3.3 (merging with an existing identical issue if there is
     one). Do not edit `CLAUDE.md` or new guides directly. List it in the PR
     body too, but that alone does not count as filing it.
  Do not commit at this point (3.6 bundles it; the feature's own records go into
  the 3.5 archive).
- `NOTHING_TO_EXTRACT` / `NO_LEARNINGS`: normal; skip.

Keep measurements, reproductions, and history in the feature's `learnings.md`.
3.5 moves the whole folder under `_archived/`, and links from the guides should
use that post-completion path.

### 3.3 Curating todo.md

```
Agent(
  subagent_type: "todo-curator",
  run_in_background: false,
  prompt: "Plan file path: docs/plans/YYYYMMDD-{feature-name}/plan.md
Path to learnings.md: docs/plans/YYYYMMDD-{feature-name}/learnings.md
Feature name: {feature-name}
Task headings being closed out: {the `### ...` headings passed in via the todo skill, or \"none\"}")
```

Branch on the terminal state of the return value:

- `PROPOSED`: the main agent applies the deletions (target heading / reason /
  confidence / for a partial close-out, an edit for what remains) and the
  additions (target section / pasteable Markdown) to `todo.md` **with `Edit`**,
  without user consent. **Apply every deletion regardless of confidence**
  (certain / needs checking) — confidence is information for the 3.6 PR body,
  not a filter. Do not apply deferred judgments; list them in the 3.6 PR body.
  **Do not commit** (the `pr-runner` in 3.6 commits it all together)
- `NOTHING_TO_DO` / `NO_SOURCES`: skip

The additions include one item per manual check the feature left pending (an
unmet "verified by hand on ..." Done-when line, or a check `learnings.md`
records as pending on the user). The main agent applies them like any other
addition; a check that only appears in a PR body or `learnings.md` is not
filed.

`archive-plan.sh` in 3.5 only commits what `git mv` staged, so the unstaged
`todo.md` you edited here cannot get swept into the archive commit.

### 3.4 Promoting settings

```
Agent(
  subagent_type: "settings-promoter",
  run_in_background: false,
  prompt: "Promote this worktree's settings.local.json into settings.json")
```

It folds the permissions granted in this worktree (`.claude/settings.local.json`)
into the git-tracked `.claude/settings.json` and commits (it does not push). If
you developed in a throwaway worktree, anything not promoted here disappears
with it. All the main agent gets back is a count; the merged JSON is never
returned.

Branch on the terminal state of the return value:

- `PROMOTED` / `NO_CHANGES`: go to 3.5
- `FAILED`: it stopped at JSON validation or similar. Report to the user with
  the state of the working tree and ask for direction

### 3.5 Archiving the plan + tidying auto memory

Move the plan folder under `_archived/` (this script commits and pushes):

```bash
bash .claude/skills/develop/scripts/archive-plan.sh docs/plans/YYYYMMDD-{feature-name}
```

**Always call it with the Bash tool's `dangerouslyDisableSandbox: true`** (same
as `ensure-new-branch.sh`). The script runs `git push -u` when no upstream is
set, then verifies immediately that the upstream was set. Setting an upstream
writes to the shared `.git/config`, which **always** fails under the sandbox
with `could not lock config file ...: Operation not permitted` (measured;
`allowWrite` does not lift it).

Remove the in-flight plan entry from auto memory (`MEMORY.md`). Auto memory is a
user settings file outside the repository and not under git, so no commit or
push is needed (trying to commit here gives you a pathspec error or an empty
commit). Updating auto memory is the main agent's job; subagents do not touch it.

### 3.6 Creating and watching the chore PR

Start it per § Starting `pr-runner`. What to pass:

- **Title**: `chore: wrap up {feature-name}`
- **Body**: the breakdown of learnings extraction / todo curation / settings
  promotion / archiving. **Always include the following** (this is the only path
  by which a human can check, after the fact, that the deletions and additions
  were right):
  - The todo headings (`### ...`) **deleted** in 3.3, with the reason and
    confidence
  - The todo headings **added** in 3.3
  - The todo headings added for **pending manual checks** in 3.3 (or the
    existing heading they were merged into), so a human can see after the fact
    that no hands-on check was lost
  - The todo headings **edited for a partial close-out** in 3.3 (existing items
    whose content was rewritten), with what changed
  - The paths of the existing guides **consolidated** in 3.2, and the gist
  - The **record-only** items (candidates for promotion into `CLAUDE.md` /
    proposed new files under `docs/agents/` / deferred judgments from learnings
    and todo). Distinguish "the target was not edited" from "it was reflected
    into the feature records and todo"
- **Paths to commit**: the existing files under `docs/agents/` consolidated in
  3.2, and the `todo.md` edited in 3.3. **Pass only what you actually wrote to**
  (if you wrote to neither, pass nothing; if `settings-promoter` and
  `archive-plan.sh` already committed and there is no diff, `pr-runner` skips
  the commit)
- **Commit message** (only when passing paths to commit; match it to the paths):
  - Both: `docs: apply the learnings from {feature-name} and curate todo`
  - Learnings only: `docs(agents): consolidate what we learned about {topic}`
  - `todo.md` only: `docs(todo): curate the items {feature-name} closed out`

**Do not let automated review be skipped.** 3.4 sometimes puts permissions into
settings.json, and dangerous entries such as an over-broad wildcard or a
host-dependent absolute path cannot be caught by mechanical exclusion patterns —
only by review. Do not change this even when there is no settings diff (an
archive-only PR passes review immediately, so it costs nothing).

### 3.7 Merge

Start `merger` per § Merge.

On runs where the 3.4 settings promotion put a diff into
`.claude/settings.json`, the chore PR falls under `.claude/**` and the read-only
check flags it. It still merges (§ Merge), but **say in the progress report
which permissions were promoted** — that is the only place a human sees what the
agents are now allowed to do without asking.

**The path by which a human checks that the deletions and additions were right
is the enumeration in the 3.6 PR body (after the fact)**, so do not let what was
removed and what was added fall out of that body.

## Phase S: Single-PR mode (one step)

Run this **instead of** Phases 1–3 for a one-step plan. **Nothing is dropped**
from the work (persisting, implementing, local review, extracting learnings,
curating todo, promoting settings, archiving) — only the PR and the merge fold
into one. See "### Folding into one PR with single-PR mode (Phase S)" in
[README.md](./README.md) for the design rationale and the risk it accepts.

The plan directory created in S.2 is included in the first commit by the S.3
`implementer`, together with the implementation files (the general contract in
step 8 of `.claude/agents/implementer.md`). That first commit and the review
history (S.4) are pushed to origin at S.4 time by `commit-review-history.sh`.
**What stays unpushed until the S.6 `pr-runner` runs is only the diff added in
S.5** (additional learnings extraction, `todo.md` curation, the Progress line in
plan.md, the archive rename) **and the settings promotion commit**
(`commit-settings.sh` is designed not to push). If you interrupt, none of that
can be resumed from another session or another machine.

### S.1 Branch prep

Same as 1.1. Run the parallel-work check (`check-parallel-work.sh`) → if there
is an uncommitted diff, `git stash --include-untracked` → § Creating a branch,
in that order. Use the normal branch name decided in 0.2 (`feature/<topic>`
etc.) as-is; do not use `{feature-name}-step-1` (implementation, plan, and
wrap-up all ride on this one branch).

### S.2 Persisting the plan

Same as 1.2. `ensure-plan-dir.sh` → `Write` plan.md → record the plan path in
auto memory. **Do not commit in this step itself** (the S.3 `implementer` makes
the first commit with the plan directory included alongside the implementation;
see the head of Phase S).

### S.3 Implementation

Same as 2.2. Start `implementer` on Step 1. On `LOCAL_CHECK_FAILED`, do not go
to S.4: report the situation to the user and ask for direction.

### S.4 Local review

Same as 2.3. Start `local-review-runner` (and the 2.3 watchdog in the same
turn). On `APPROVED`, go to S.5. `MAX_ROUNDS_REACHED` / `LOCAL_CHECK_FAILED` do
not proceed to PR creation; report instead.

### S.5 Wrap-up

Run 3.2–3.5 in the same order. **Do not cut a new branch** (stay on the S.1
branch).

**Before starting**, check Step 1's checkbox in the plan file (`- [ ]`/`- [x]`)
— same reason as 2.4. If it is not `[x]` (for instance, the addresser reverted
it to `[ ]` with a reason during review), do not proceed into the wrap-up
(learnings extraction onwards). **An `APPROVED` from local review only
guarantees the quality of the diff, not that the step's acceptance criteria were
met**, so the two are tracked separately. In this case, report the reason and
the remaining work left in the plan file verbatim and ask for direction.

If it is `[x]`, run the following in order:

1. `learnings-extractor` (same as 3.2; the main agent consolidates into the
   existing guides and saves the record-only part as the feature's todo
   candidates before moving on. Do not commit)
2. `todo-curator` (same as 3.3; the main agent applies the deletions and
   additions with `Edit` without consent, and deferred judgments go into the
   S.6 PR body. Pending manual checks become todo items here too: one per
   unmet "verified by hand on ..." Done-when line or check `learnings.md`
   records as pending on the user)
3. `settings-promoter` (same as 3.4; this agent commits)
4. Append `(YYYY-MM-DD) Step 1 complete` to the plan file's Progress with `Edit`
   (the reason to do it before creating the PR is the same as 2.4)
5. Move the plan folder under `_archived/`. **Do not call `archive-plan.sh`**;
   use a plain `mv`:

   ```bash
   mv docs/plans/YYYYMMDD-{feature-name} docs/plans/_archived/YYYYMMDD-{feature-name}
   ```

   The plan directory is already tracked from the S.3 `implementer`'s commit, so
   `archive-plan.sh`'s `git mv` would not itself fail. The reason to avoid it is
   different: that script also **commits and pushes** as it moves (which is
   correct for 3.5, an independent chore PR). Phase S is designed so the S.6
   `pr-runner` bundles it with learnings / todo / settings into one commit, and
   pushing separately here would split that up. Use a plain `mv` to move only,
   and leave the staging to S.6. A destination collision was already checked by
   `ensure-plan-dir.sh` in S.2, `_archived/` included
6. Remove the in-flight plan entry from auto memory (`MEMORY.md`) (same as 3.5;
   outside git, so no commit or push)

### S.6 Creating and watching the PR

Start it per § Starting `pr-runner`. What to pass:

- **Material for the body**: the plan file path (**the post-archive
  `docs/plans/_archived/YYYYMMDD-{feature-name}/plan.md`**), step number 1, and
  the wrap-up breakdown (learnings extraction / todo curation / settings
  promotion / archiving). As in 3.6, always include **the deleted todo headings
  with reason and confidence / the added todo headings / the todo headings
  edited for a partial close-out and what changed / the todo headings added
  or extended for pending manual checks / the paths of the
  consolidated existing guides and the gist / where the record-only items were
  recorded and how todo curation resolved them**
- **Paths to commit**: both the plan directory's **old path
  `docs/plans/YYYYMMDD-{feature-name}/`** (still tracked but gone from disk, so
  `git add` stages the deletion) and its **new path
  `docs/plans/_archived/YYYYMMDD-{feature-name}/`**, plus the existing guides
  you wrote to and the `todo.md` you edited in S.5. The `learnings.md` holding
  the record-only items is part of the new path. **Pass only what you actually
  wrote to** (settings promotion, review history, and the implementation are
  already committed; settings is not pushed yet, but the S.6 `pr-runner` pushes
  every unpushed commit on the branch together, so it does not need to be listed
  here)
- **Commit message**: one line (e.g.
  `docs: record the plan and wrap-up for {feature-name}`)

**Do not let automated review be skipped** (same reason as 3.6).

### S.7 Merge

Start `merger` per § Merge. `MERGED` completes develop.

This PR carries the implementation, plan.md, learnings, `todo.md`, and
`.claude/settings.json` together, so the read-only check flags `.claude/**` on
any run that promoted settings. As in 3.7 it merges anyway, with the promoted
permissions named in the progress report. The rightness of the `todo.md`
deletions and the learnings additions is checked through **the enumeration in
the S.6 PR body (after the fact)**.

### Recovering from an interruption or failure in S.6 / S.7

The archiving and auto memory tidying in S.5 happen **before** PR creation and
merge, so if S.6 ends in `aborted` / `closed`, or S.7 in `FAILED`, you stop in a
state where the plan has been moved to `_archived/`, the auto memory tracking
entry is gone, and nothing has been merged. To recover:

1. Move the plan folder back to its original path:
   `mv docs/plans/_archived/YYYYMMDD-{feature-name} docs/plans/YYYYMMDD-{feature-name}`
2. Write the in-flight plan entry back into auto memory (`MEMORY.md`)

**Do not delete the branch.** The promotion commit made by `settings-promoter`
in S.5 has not been pushed, and `settings-promoter` has already overwritten the
`.gitignore`d `settings.local.json` with an empty file, so the original is gone.
Delete the branch and that promotion commit goes with it, leaving the contents
of `settings.local.json` unrecoverable.

Then report the situation to the user and ask how to resume.

### When it turns out not to fit in one step (demotion)

If partway through S.3 / S.4, or from their return values, it becomes clear the
work does not fit in one step, **do not quietly keep cramming it into one PR**:

1. Stop, and report the situation and a re-split proposal to the user for
   approval (the equivalent of 0.2 again)
2. Rewrite plan.md into multiple steps (restart `planner`, or have the main
   agent update it with `Write`). Nothing has been merged yet, so you are free
   to rewrite it on the same branch and in the same PR
3. **Keep the current branch as Step 1's implementation branch** and let plan.md
   ride along in Step 1's PR. Run 2.3–2.5, and include the **unarchived** plan
   directory (`docs/plans/YYYYMMDD-{feature-name}/`) in `pr-runner`'s paths to
   commit. **Do not archive.** Run the normal Phase 2 for Step 2 onwards, and
   the normal Phase 3 wrap-up at the end
4. If the implementation is still small enough that throwing it away is cheaper,
   the user may tell you to discard it and start over from Phase 1 in normal
   mode

## Reporting

At the end of each phase, report briefly in this format:

```
---
## Develop progress

- Phase: {phase name}
- Step: {N}/{Total}
- Just did: {what you did}
- Next: {what happens next}
---
```

## Notes

- The main agent starts subagents and looks only at return values and exit
  codes. It does not read or edit code, diffs, CI logs, or conflicts itself
- Exceptions, all done by the main agent: branch operations / stash, writing
  plan.md and the 3.2 targets (the existing files under `docs/agents/` and the
  feature's `learnings.md`; `CLAUDE.md` is never written) and `todo.md`,
  appending Progress in 2.4 / S.5, the plain `mv` of the plan directory in S.5
  (a file move, not a git operation), updating auto memory, and calling scripts
- The main agent never runs `git add` / `git commit` / `git push` /
  `gh pr merge` (those belong to `pr-runner` / `merger` / `settings-promoter` /
  `archive-plan.sh` / `local-review-runner`)
- Apart from the 0.2 approval, there is no user confirmation between steps or
  during wrap-up (Phase 3 / S.5–S.7) — the plan was agreed in advance, and
  merges carry a standing approval (§ Merge). The only dialogue points left are
  the abnormal cases (`settings-promoter`'s `FAILED`, Step 1 not being `[x]`
  when S.5 starts, and the `aborted` / `FAILED` states below). Learnings
  additions and `todo.md` deletions and additions are applied without consent,
  and their content is checked after the fact through the enumeration in the PR
  body
- When a subagent returns `aborted` / `closed` / `needs_discussion` /
  `LOCAL_CHECK_FAILED` / `MAX_ROUNDS_REACHED` / `FAILED` / `POST_MERGE_FAILED` /
  `POST_MERGE_TIMEOUT`, do not move on by yourself: ask the user for direction.
  `NOT_READY` alone gets one automatic retry (§ Merge)
