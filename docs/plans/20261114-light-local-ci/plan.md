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

# Light Local CI

## Purpose

The agents of the `develop` / `pr` workflows run `mise run ci` before every
commit or push. That is `mise run lint` plus `mise run test`, and `test` ends
in `cargo clippy --all-targets` and `cargo test` over the whole root
workspace, the Tauri crate included. In a fresh worktree (no `target/`, no
shared `CARGO_TARGET_DIR`) this is a cold build, it takes close to 10
minutes, and it has been killed twice by the host running out of memory,
after lint and the frontend tests had already passed.

GitHub Actions already runs all of it per PR, on free runners because the
repository is public: `lint` on Ubuntu, `mise run test` on Ubuntu / macOS /
Windows, and the `native` job (clippy and tests of `crates/native`; PR #1023
turns it into the same three-platform matrix, and this plan assumes that).

After this work the agents run one light task, `mise run ci:local`, which
selects every check from the branch's changed paths: a Markdown-only change
runs only the offline link check, a Rust change checks and tests only the
crates it touched, and the Tauri crate (`crates/app`, being retired in favour
of the GPUI app) and the frontend tests are never built or run locally. The
rest is left to GitHub Actions. `mise run ci` stays the full run.

## Steps

- [x] Step 1: Add `mise run ci:local`, selected from the changed paths, and
      make every agent, skill and script call it instead of `mise run ci`
  - Done when:
    - `mise run ci:local` exists and runs `tools/ci/local.sh`. `mise run ci`,
      `mise run test`, `mise run native:check` and `.github/workflows/` are
      unchanged. `mise run lint` is unchanged except that its shellcheck
      covers `tools/ci/*.sh` and it runs `tools/ci/local.test.sh` next to the
      three existing `*.test.sh`.
    - Changed paths: the working tree against
      `git merge-base HEAD origin/main` (committed and uncommitted) plus the
      untracked, unignored files, without fetching. A rename counts as both
      its old and its new path, and the script knows whether any path was
      deleted or renamed.
    - Each check runs only under its condition:
      - `cargo fmt --all -- --check` at the root: a `.rs` under `crates/app`,
        `crates/backend`, `crates/cli` or `crates/core` changed. The same
        with `--manifest-path crates/native/Cargo.toml`: a `.rs` under
        `crates/native` changed.
      - `cargo clippy -p <package> --all-targets -- -D warnings` at the root,
        for each of `riffle-core` (`crates/core`), `riffle-backend`
        (`crates/backend`) and `riffle-cli` (`crates/cli`) that has a changed
        path under its directory; the root `Cargo.toml` or `Cargo.lock`
        counts as a change to all three. Only the changed packages, not their
        dependents. `riffle-app` (`crates/app`) is never built.
      - `mise exec rust@1.98.1 -- cargo clippy --all-targets -- -D warnings`
        in `crates/native`: a path under `crates/native`, `crates/core` or
        `crates/backend` changed (or one of the conditions that count as a
        change to those crates).
      - `cargo test -p <package>` at the root for the same changed packages
        as clippy, after clippy, and not for their dependents.
      - `mise exec rust@1.98.1 -- cargo test` in `crates/native`: a path
        under `crates/native` changed, after the native clippy. This one is
        kept only if the measurement below allows it.
      - `pnpm install --frozen-lockfile` then `pnpm exec vp check`: a path
        under `crates/app/ui` changed, or `package.json`, `pnpm-lock.yaml`,
        `vite.config.ts` or `tsconfig.json` changed, or a changed file has an
        extension vp's formatter handles (the list taken from oxfmt's
        supported file types and written in the script) and is neither `.md`
        nor under `.claude/`.
      - shellcheck (`-x`) on the changed `.sh` files that fall under the
        globs `[tasks.lint]` passes it, `tools/ci/*.sh` included, and on no
        other file.
      - `actionlint`: a path under `.github/workflows` changed.
      - Each bash test only when its script or the test itself changed:
        `check-merge-approval.test.sh`, `wait-post-merge-runs.test.sh`,
        `merge.test.sh` (with `merge.js`), and `tools/ci/local.test.sh` when
        a path under `tools/ci` changed.
      - lychee over the whole repository, with the arguments `[tasks.lint]`
        uses: a `.md` changed, or any path was deleted or renamed.
      - The frontend tests (`vp test`) never run.
    - So a change of Markdown files only, none deleted or renamed, runs
      lychee and nothing else.
    - When the merge base cannot be found, or `mise.toml` or `.shellcheckrc`
      changed, every check above runs (all three root packages, the native
      clippy and test, `vp check`, shellcheck on every covered script,
      actionlint, the four bash tests, lychee); it fails toward checking.
      With no changed path at all it runs nothing and says so.
    - The script prints which checks it selected and which it skipped before
      running them, so a log shows what was left to GitHub Actions.
    - `tools/ci/local.test.sh` covers the selection as a pure function of the
      path list (with the deleted / renamed flag), without a git repository.
    - Measured on this Windows host and recorded in `learnings.md`: the wall
      time of `mise run ci:local` for a Markdown-only change, a
      frontend-only change, a change in each of `crates/core`,
      `crates/backend`, `crates/cli` and `crates/native`; for the Rust ones
      the wall time and peak memory of clippy and of the tests, from a cold
      `target/` and from a warm one. `learnings.md` also records whether the
      native test stayed in `ci:local`, the criterion that decided it and the
      numbers, and any `--jobs` cap with the measurement behind it.
    - No file under `.claude/` tells an agent to run `mise run ci` as its
      local check any more: `implementer.md`, `local-review-addresser.md`,
      `local-review-runner.md`, `pr-runner.md`, `pr-check-fixer.md`,
      `pr-conflict-resolver.md`, `pr-review-addresser.md`,
      `skills/develop/scripts/commit-push.sh`,
      `skills/develop/scripts/commit-review-history.sh` (its comment),
      `skills/develop/SKILL.md` and `skills/develop/README.md` name
      `mise run ci:local`. `grep -rn "mise run ci" .claude` leaves only
      `mise run ci:local`, the `*.test.sh` header comments ("or via
      `mise run ci`", still true through `lint`), the `Bash(mise run ci)`
      fixture string in `merge.test.sh`, and sentences that describe the full
      run on purpose.
    - `implementer.md` states that a plan's Done-when naming `mise run ci` or
      `mise run native:check` is met by `mise run ci:local` passing locally
      and the PR's GitHub checks passing, and that neither full task is run
      locally as a gate. `planner.md` tells the planner to write Done-when
      lines against `mise run ci:local` and the PR's checks.
    - `pr-check-fixer.md` says how to confirm a fix for a failed GitHub check
      that `ci:local` does not run (a dependent crate's test, the Tauri
      crate, a frontend test, a platform-only failure): re-run the narrowest
      failing command where the platform allows it (one test of one crate,
      one Vitest file), then `mise run ci:local`; it does not fall back to
      the full `mise run ci`.
    - `CONTRIBUTING.md`'s Checks section describes `ci:local`, how it selects
      and what it leaves to GitHub Actions. `docs/agents/native-app.md`'s
      rule "check a backend change with both `mise run ci` and
      `mise run native:check`" says what the agents run locally now
      (`ci:local`: the changed crates, and the native clippy for a core or
      backend change) and that the PR's `test` and `native` jobs cover the
      rest.
    - `learnings.md` holds, under `## Deferred issues (todo candidates)`, the
      full text of a `todo.md` item for the wrap-up of this same PR to add:
      when the Tauri app and `crates/app/ui` are removed, move the JSON /
      YAML formatting from vp (oxfmt through pnpm) to a standalone Rust
      formatter such as dprint installed through mise, so `lint` and
      `ci:local` no longer need node. It follows `todo.md`'s entry form (a
      `### <Area>: <title>` heading, `#### Background`, `#### TODO` with a
      checkbox that says when it is done).
    - `mise run ci:local` and `mise run lint` pass on the final tree.
  - Implementation approach (as far as it is known; omit if unknown):
    - `[tasks."ci:local"]` in `mise.toml` follows the existing tasks:
      `shell = "bash -c"`, `set -euo pipefail`, and it only calls
      `bash tools/ci/local.sh`. The logic lives in the script so shellcheck
      and a test cover it. `tools/ci/` is new; `tools/git/` and
      `tools/stats/` are its siblings and `tools/git/_lib.sh` shows the house
      style.
    - Keep the selection a pure function: path list and the deleted / renamed
      flag in, the list of checks out. `local.test.sh` feeds it lists, in the
      style of `.claude/skills/merge/scripts/check-merge-approval.test.sh`.
      Cases worth a test: Markdown only; Markdown with a deleted file; a
      renamed `.md`; a plan folder with a `.json` (lychee off, `vp check`
      on); a `.py` only (nothing); `crates/app/ui/src/x.ts` (`vp check`, no
      cargo); `crates/app/src/x.rs` (root fmt check only);
      `crates/core/src/x.rs` (root fmt, core clippy and test, native clippy,
      no backend or cli, no native test); `crates/backend/src/x.rs`;
      `crates/cli/src/x.rs` (no native); `crates/native/src/x.rs` (native
      fmt, clippy, test; nothing at the root); a non-`.rs` file under a crate
      (clippy and test, no fmt check); root `Cargo.lock`;
      `crates/native/Cargo.lock`; a `.sh` inside and outside the covered
      globs; a workflow file; each bash test's script; `mise.toml`; an empty
      list.
    - For the changed paths use a rename-aware diff
      (`git diff --name-status -M <merge-base>`): take both paths of an `R`
      line and note any `D` or `R`. Add
      `git ls-files --others --exclude-standard`. `commit-push.sh` runs the
      checks before its commit, so the uncommitted paths must count.
    - Do not `git fetch` in the script: it runs under the sandbox and
      offline. A stale `origin/main` only moves the merge base back, which
      adds paths and so can only add checks.
    - For the `vp check` trigger, over-trigger rather than mirror
      `vite.config.ts`'s whole `fmt.ignorePatterns`: exclude only `.md` and
      `.claude/**`. The other ignored paths (the release-please files,
      `crates/app/tauri.conf.json`, `crates/core/genres/**`, ...) then run a
      `vp check` that was not strictly needed, which is harmless, and there
      is no second copy of the list to keep in step. Take the extension list
      from the oxfmt version `vite-plus` 0.3.3 ships, and say in a comment
      where it came from.
    - `vp check` covers the frontend's format, lint and types, so a
      `crates/app/ui` change is still type-checked locally; only its tests
      are left to GitHub Actions.
    - Copy the lychee arguments from `[tasks.lint]` exactly, including
      `--exclude-path 'docs.plans.review-history'`. It took about 0.2 s over
      the whole repository here. If two copies are awkward, let
      `[tasks.lint]` and the script share one definition without changing
      what `mise run lint` runs.
    - The native commands use the form `.github/workflows/ci.yml`'s `native`
      job uses (`mise exec rust@1.98.1 -- cargo ...` from `crates/native`),
      because `mise run native:check`'s task shell did not find nasm. NASM
      must be on `PATH` (`crates/native/build.rs` refuses to build without
      it); on this host winget put it under `%LOCALAPPDATA%\bin\NASM`. Check
      what the script has to do so the build script finds it, and make a
      missing NASM fail with a message that names it.
    - Check that `cargo clippy -p` / `cargo test -p` on one root package does
      not compile `riffle-app` or its Tauri dependencies (the point of never
      building the Tauri crate); `cargo tree -p <package>` shows it.
    - Measure before settling the Rust part: wall time and peak memory of
      each clippy and each test run, cold and warm, on this host (24 logical
      cores, about 32 GB, other sessions building at the same time). Cap with
      `--jobs` if the numbers call for it. For the native test, a workable
      criterion is that the largest realistic set (a `crates/native` change,
      cold `target/`) finishes inside the Bash tool's 600 s limit the agents
      run `ci:local` under, without the host running out of memory; drop the
      native test from `ci:local` if it does not, and record the criterion
      used and the numbers either way.
    - In the agent files the change is mostly a substitution, but each
      "`mise run ci` can take close to 10 minutes" paragraph needs rewording:
      `ci:local` is under a second for Markdown and can still take minutes
      for a cold Rust change, so the `timeout: 600000` and the "not in the
      background" instructions stay. `pr-runner.md`'s "Claude execution
      contract" line for `commit-push` ("runs `mise run ci` inside") and
      `local-review-runner.md`'s two descriptions of
      `commit-review-history.sh` change with the script.
    - `develop/README.md` keeps design rationale: its "Concentrate commits in
      `pr-runner`" section describes the old and new flows with "full CI";
      keep the history as written and add what the gate runs now and why
      (the out-of-memory kills, the Tauri crate being retired, GitHub Actions
      covering the rest). Its "implementer's `mise run ci` runs once, at the
      end" sentence becomes `ci:local`.
    - `implementer.md` forbids editing `todo.md` directly, so the dprint item
      goes through `learnings.md`'s `## Deferred issues (todo candidates)`
      heading (that exact string) and the wrap-up adds it to `todo.md`; in
      single-PR mode that is this PR.
    - PR #1023 (the three-platform `native` job) also edits
      `docs/agents/native-app.md`. If it merges first, rebase onto it and
      word the rule against its new section.
    - Leave alone: `.github/workflows/**`, `[tasks.ci]`, `[tasks.test]`,
      `[tasks."native:check"]`, `CLAUDE.md` (its "formatted, linted,
      type-checked and tested by `mise run ci`" stays true), every other plan
      folder (in-flight plans keep their "`mise run ci` passes" lines; the
      `implementer.md` rule covers them), `archive-plan.sh` and
      `commit-settings.sh` (neither runs a check), and
      `.claude/settings.json` (`Bash(mise run *)` already allows the new
      task).

## Trade-offs and risks

- **What only GitHub Actions finds now.** A test of a crate that depends on
  the changed one (a `riffle-core` change breaking a `riffle-backend`,
  `riffle-cli` or `riffle-native` test), anything in the Tauri crate beyond
  formatting, every frontend test, and any platform-only failure. Each costs
  a fix push, which moves the head and restarts the automated review.
  `pr-check-fixer` then confirms with the narrowest failing command, not the
  full run.
- **A Rust change in `crates/app`** runs the root `cargo fmt --check` and
  nothing else. This reads "nothing for the Tauri crate" as "no build"; the
  format check compiles nothing and `mise run fmt` has normally just run.
  Skipping it too is the other reading.
- **Root `Cargo.toml` / `Cargo.lock`** count as a change to all three root
  packages, and so also run the native clippy. That is the heaviest ordinary
  case. The alternative is to leave a root manifest or lock change to GitHub
  Actions.
- **`mise.toml` and `.shellcheckrc`** run everything, since a tool version or
  a shellcheck setting can change any result. A `mise.toml` edit that only
  touches a task then pays for the Rust checks once.
- **No changed path** runs nothing. It only happens on a checkout equal to
  `origin/main`, where there is nothing to push.
- **The native test may be dropped** after the measurement. Then a
  `crates/native` change is linted locally and tested only by the `native`
  job.
- **Memory is still unmeasured.** Which of `cargo clippy` / `cargo test`
  killed the earlier runs is not known. Not building the Tauri crate removes
  the largest dependency tree and its test binaries, but a cold
  `cargo test -p riffle-core` and the native build are heavy in their own
  right; the measurement and the `--jobs` cap are the answer, not an
  assumption.
- **NASM** makes the native clippy and test able to fail for a reason of the
  machine rather than of the change.
- **The `vp check` trigger over-triggers** on files the formatter ignores,
  by design. It still needs node and pnpm; the todo item recorded in this
  step is about removing that once the Tauri app is gone.
- **Done-when lines of plans in flight** say "`mise run ci` and
  `mise run native:check` pass", and this plan does not edit them, since
  other sessions are working on them. With the new `implementer.md` rule a
  step's checkbox is ticked before the full tests have run anywhere; they run
  on the PR, and `pr-runner` already waits for those checks before a merge.
- **What this does not remove.** The implementer still compiles and runs the
  tests it writes while implementing, in a worktree with its own `target/`.
  A `CARGO_TARGET_DIR` shared by the worktrees would attack the cold builds
  themselves and is not part of this plan.
- **A new task, not a lighter `mise run ci`.** `mise run ci` keeps meaning
  "the same checks CI runs" for a human contributor; the price is the text
  changes across `.claude/`.

## Progress
