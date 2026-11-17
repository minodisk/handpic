# GitHub Actions workflows and `gh` secrets

Read this before touching a `.github/workflows/*.yml` that uses `gh`,
secrets, or a dedicated branch as a data store (like `stats`, written by
`.github/workflows/stats.yml`). It lists what writing and debugging such a
workflow took here, each with the reason.

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something
here, **Measured** steered a design decision, **Inferred** comes from sources
or docs only.

Source: the
[promotion-stats learnings](../plans/_archived/20260928-promotion-stats/learnings.md),
"Step 1" and "After the Step 2 merge".

## Secrets

### An empty secret looks set in `gh secret list` (Hit)

The first `workflow_dispatch` of the Stats workflow failed with
`gh: To use GitHub CLI in a GitHub Actions workflow, set the GH_TOKEN
environment variable`, although `gh secret list` listed `STATS_TOKEN`. The
secret had been registered with an empty value: the Git Bash recipe
`read -rs T && printf '%s' "$T" | gh secret set ...` stores whatever was
read, and nothing had been pasted before Enter. `gh secret list` shows only
names and dates, so it cannot tell an empty secret from a set one. The tell
is the run log: the step's environment prints an empty `GH_TOKEN:` line.

- Rule: set a secret's value on its page on github.com, or check the value
  is non-empty before piping it to `gh secret set`. When a `gh` step asks for
  `GH_TOKEN`, look for an empty `GH_TOKEN:` in the run log before suspecting
  the workflow.

### Pushing a workflow change needs the `workflow` scope (Hit)

Pushing a branch that adds or changes `.github/workflows/*` is refused with
"refusing to allow an OAuth App to create or update workflow" when the gh
token lacks the `workflow` scope.

- Rule: run `gh auth refresh -h github.com -s workflow` once before pushing
  a workflow change.

## A manual workflow

### A `workflow_dispatch` workflow is dispatchable only from the default branch (Inferred)

GitHub lists a workflow, and `gh workflow run` accepts it, only once its file
is on the default branch; after that, `gh workflow run <file> --ref <branch>`
runs the file as the branch has it. `.github/workflows/package.yml` (the
native app's `Package` workflow, see
[`native-app.md`](./native-app.md#the-package-workflow-builds-the-distributables-inferred))
is `workflow_dispatch` only, so the PR that adds it cannot dispatch it.

- Rule: to run a new dispatch-only workflow on its own PR, push a temporary
  commit adding a `pull_request` trigger (or `push: branches: [<branch>]`
  when the PR does not exist yet, as `package.yml` did in run 43986612196),
  inspect that run and its
  artifacts, and remove the trigger before the merge. A later change to the
  workflow runs with `gh workflow run package.yml --ref <branch>`.

## Linting

### `shellcheck` and `actionlint` are only on the `PATH` inside mise (Hit)

Both are pinned in `mise.toml` and run by the `lint` task (so by
`mise run ci`), but a plain Git Bash shell does not find them.

- Rule: run them ad hoc through mise, e.g.
  `mise exec -- shellcheck -x tools/stats/*.sh` or `mise exec -- actionlint`.

## A dedicated branch as a data store

### Start the branch as an orphan worktree, commit only when changed (Measured)

`.github/workflows/stats.yml` keeps its snapshots on the orphan `stats`
branch, so main's history, CI and release-please stay free of daily
commits. The first run finds no branch and starts it with
`git worktree add --orphan -b <branch> <dir>` (git 2.42+; `ubuntu-latest`
ships newer). Later runs fetch it into the shallow checkout with
`git fetch --depth=1 origin <branch>:<branch>` and then
`git worktree add <dir> <branch>`; the push reuses the credentials
`actions/checkout` persisted in the shared `.git/config`. After
`git add -A`, `git diff --cached --quiet` is the commit-only-when-changed
guard: checked locally that it also works on the unborn branch (0 before
`git add`, 1 after), so the guard survives the first run.

- Rule: follow `stats.yml` for a new data-store branch, and keep the
  `git diff --cached --quiet` guard so a same-day re-run with no change does
  not commit.
