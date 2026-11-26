# GitHub Actions workflows and `gh` secrets

Read this before touching a `.github/workflows/*.yml` that uses `gh`,
secrets, or a dedicated branch as a data store (like `stats`, written by
`.github/workflows/stats.yml`). It lists what writing and debugging such a
workflow took here, each with the reason.

The tags follow [`shared-crates.md`](./shared-crates.md): **Hit** broke something
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

### A dispatchable reusable workflow sees the repository secrets too (Inferred)

`package.yml` is both `workflow_dispatch` and `workflow_call`. On a direct
dispatch, `secrets.<NAME>` resolves to the repository secret of that name,
not to nothing, so a step gated on a repository secret being present would
use it on any dispatch. `UPDATE_SIGNING_KEY` moves to the `release`
environment (see the next section) and its repository copy is deleted once
that lands; until then a direct dispatch on any ref still resolves it to the
repository copy, and from then on only a dispatch on `main` whose leg
declares the environment would reach it.

- Rule: gate what a secret enables on an input declared only under
  `workflow_call` (`sign_production`), which a dispatch cannot pass
  (`gh workflow run` refuses an undeclared input). Hand the secret through
  `env` to the one step that needs it, never through `GITHUB_ENV`.

### An environment secret reaches a reusable workflow's job, not its caller (Inferred)

GitHub's "Reuse workflows" docs (read 2026-11-26): a job that calls a
reusable workflow (`uses:`) cannot declare `environment`. A job inside the
reusable workflow declares it and then reads the environment's secrets, but
the caller must still pass the secret by name (`secrets: NAME: ${{
secrets.NAME }}`, allowed "even if it only exists in the environment"), or
it resolves to an empty string. `release.yml`'s `build` passes
`UPDATE_SIGNING_KEY` and `package.yml`'s legs declare the `release`
environment under `sign_production` (`name: ${{ inputs.sign_production &&
'release' || '' }}`, `deployment: false`); `guard` declares it always.

- Rule: put a secret a branch must not read in an environment whose
  deployment branch policy allows only `main`, declare that environment on
  every job that reads it, and pass the secret by name from the caller of a
  reusable workflow. `deployment: false` reads the secrets without creating
  deployment records.

### Prove a signing secret before the long build (Inferred)

A release build takes about 75 minutes, so an empty or wrong
`UPDATE_SIGNING_KEY` found by the legs' signing steps would cost that much.
`release.yml`'s `guard` job signs a scratch archive with the same script the
legs use (`tools/package/sign-production.sh`), which fails, naming the
secret, on an empty value, a value that is not base64 or not 64 bytes, and a
key that does not match `update.pub`.

- Rule: a workflow that needs a secret's content to be right checks it in a
  first job the build `needs`, by doing what the build will do with it.

## A manual workflow

### A `workflow_dispatch` workflow is dispatchable only from the default branch (Inferred)

GitHub lists a workflow, and `gh workflow run` accepts it, only once its file
is on the default branch; after that, `gh workflow run <file> --ref <branch>`
runs the file as the branch has it. `.github/workflows/package.yml` (the
app's `Package` workflow, see
[`app.md`](./app.md#the-package-workflow-builds-the-distributables-inferred))
is `workflow_dispatch` only, so the PR that adds it cannot dispatch it.

- Rule: to run a new dispatch-only workflow on its own PR, push a temporary
  commit adding a `pull_request` trigger (or `push: branches: [<branch>]`
  when the PR does not exist yet, as `package.yml` did in run 43986612196),
  inspect that run and its
  artifacts, and remove the trigger before the merge. A later change to the
  workflow runs with `gh workflow run package.yml --ref <branch>`.

### A new dispatch input works on a branch once the workflow is dispatchable (Measured)

`release.yml` was already `workflow_dispatch` on `main` with no inputs. Its
branch version added a required `version` input, and `gh workflow run
release.yml --ref native-release-step-2 -f version=2.0.0-rc.0` was accepted
(run 44880572048): the inputs come from the file at the dispatched ref, not
from the default branch's.

- Rule: no temporary trigger is needed to dispatch a branch's change to a
  workflow that `main` already dispatches, even with new inputs.

## Job conditions

### A job after a skipped one needs a status function in its `if` (Inferred)

On a dispatch of `release.yml`, `release-please` (push only) is skipped.
A job whose `needs` include a skipped job, directly or through another job,
is skipped too under the implicit `success()`, even when its direct
dependency succeeded.

- Rule: give such a job `!cancelled() && needs.<job>.result == 'success'`
  (or the output it depends on) instead of a bare condition, as `guard`,
  `build` and `publish` do. In YAML, start the expression with `${{` or use
  a folded scalar (`>-`): a plain scalar starting with `!` is a tag.

### `cmd | grep -q` under `pipefail` can report "not found" (Inferred)

GitHub's `shell: bash` runs `bash -eo pipefail`. `grep -q` exits at the
first match, the writer can then die of `SIGPIPE`, and the pipeline fails,
so `if gh release list ... | grep -q v1` reads as "no such release" when
there is one.

- Rule: put the command's output in a variable first, then `grep -q` it
  (`grep -qxF "$name" <<<"$list"`).

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
