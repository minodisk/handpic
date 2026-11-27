<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# A featureless `cargo check` of the app on the Linux CI leg

## Purpose

Since [macos-ci-bundle-reuse](../_archived/20261127-macos-ci-bundle-reuse/plan.md)
switched the macOS bundle step to `--features screenshot`, no CI job before
the `Package` workflow builds `crates/app` without GPUI's `test-support`:
`cargo clippy --all-targets` and `cargo test` see it through the
`gpui-pre-platform` dev-dependency, and the bundle step through the
`screenshot` feature. A test-only GPUI API used from non-test code therefore
first fails in `Package`, in the middle of a release.

This plan closes the `todo.md` item "App: no CI job builds the app without
`test-support` before `Package`" by adding one featureless
`cargo check --locked` of `crates/app` to the Linux leg of the `app` job, so
the mistake fails on the PR that makes it.

Rationale (goes into the PR body):

1. Today no build without `test-support` runs until `Package`, at release
   time, so a test-only GPUI API in non-test code first breaks in the middle
   of a release. Paying for a check on every PR is cheaper than stopping a
   release.
2. The Linux leg, because it is the fastest runner and is not under the
   30-minute timeout pressure of the macOS leg (which "The macOS bundle step
   reuses the test build" in [app.md](../../agents/app.md) just trimmed); and
   a check, because it is cheaper than a build.

## Steps

- [x] Step 1: Add the featureless check to the Linux leg, update the rule in `docs/agents/app.md`, and delete the `todo.md` section
  - Done when:
    - `.github/workflows/ci.yml`'s `app` job has one new step, guarded by
      `if: runner.os == 'Linux'`, that runs `cargo check --locked` for
      `crates/app` with no `--features`, no `--all-targets` and no `--tests`.
      The Windows and macOS legs run nothing new, and the job's and the macOS
      leg's `timeout-minutes` are unchanged.
    - The rule "non-test code must not use a test-only GPUI API" under "The
      main view opens on GPUI's test platform" in `docs/agents/app.md` names
      the job and step that now catches it (the `app (ubuntu-latest)` leg's
      new step), and what still does not (code under a Windows-only or
      macOS-only `cfg`, which only `Package` builds without `test-support`).
    - The sentences of `docs/agents/app.md` the change makes false are
      corrected: "So `ci.yml` no longer builds a default-feature, non-test
      app on any platform; `Package` still does." in "The macOS bundle step
      reuses the test build", and the description of what each leg runs in
      "The `app` CI job runs on all three platforms".
    - The `todo.md` section headed "App: no CI job builds the app without
      `test-support` before `Package`" is gone, with the `---` separator
      that precedes it; no other section of `todo.md` is touched.
    - The PR's GitHub checks pass, the new step among them on
      `app (ubuntu-latest)`. `mise run ci:local` passes for this diff if it
      is run (it should select only the workflow and Markdown lints; no
      local cargo build is needed for this change).
    - The PR body carries the two-point rationale from "Purpose" and the
      new step's duration read from the first CI run of the PR.
  - Implementation approach:
    - Match the existing step style of the `app` job: `shell: bash`,
      `working-directory: crates/app`, and
      `mise exec rust@1.98.1 -- cargo check --locked`. The job deliberately
      does not use `mise run` tasks for cargo (see the comment above "Lint
      and test the app"), so do not add a mise task for this.
    - Do not add the check to `tools/ci/local.sh` or to `mise run app:check`
      (see "Trade-offs and risks").
    - Position: put the step before "Lint and test the app" (after the cache
      step). The clippy and test commands run without `--locked` and would
      rewrite a stale `Cargo.lock` on the runner, after which `--locked`
      could no longer fail.
    - Give the step a comment in the style of the neighbouring ones saying
      why it exists: clippy, the tests and the macOS bundle step all build
      with `test-support`, and this is the one build before `Package`
      without it.
    - Why a plain `cargo check` has `test-support` off (confirmed from
      `crates/app/Cargo.toml`, not by a build): the package is edition 2024
      with an empty `[workspace]` and no `resolver` key, so resolver 3
      applies and dev-dependency features are not unified into a build with
      no test target; `gpui-pre-platform` is otherwise only an optional
      dependency behind `screenshot`. Verify it on the first CI run: the
      step's log should show `Checking gpui-*` lines (the variant without
      `test-support` being checked afresh). If it shows only
      `Checking handpic-app`, the check is sharing the `test-support`
      variant and does not do its job; stop and record it in `learnings.md`.
    - Reading the duration: take the step's start and end from the first
      run of `app (ubuntu-latest)` on the PR (for example
      `gh run view <run-id> --json jobs`, the step's `startedAt` /
      `completedAt`). Note in the PR body whether the cache was restored on
      that run. `actions/cache` saves only when the primary key
      (`cargo-app-Linux-<hash of crates/app/Cargo.lock>`) missed, so the
      new variant's artifacts are not cached until `Cargo.lock` next
      changes; the first run's duration is the per-run cost until then.
      Also note the Linux leg's total against its 30-minute limit.
    - `docs/agents/app.md` sections carry measurements (their headings are
      tagged `(Measured)`); add the measured step duration to the updated
      text in a follow-up commit on the same PR once the first run has
      finished.
    - `CLAUDE.md` does not describe the `app` job's steps and needs no
      change. No `docs/humans` page is affected.

## Trade-offs and risks

- **Step position and `--locked`.** Before "Lint and test the app",
  `--locked` is effective and a stale `Cargo.lock` fails the Linux leg on the
  PR, as it would fail `Package`'s `cargo build --release --locked` at
  release. The cost: when the check fails, the Linux leg's clippy and test
  output is not produced (the Windows and macOS legs still show it).
- **One platform's `cfg` only.** The check compiles the Linux configuration.
  A test-only GPUI API used in non-test code under `cfg(windows)` or
  `cfg(target_os = "macos")` is still first caught by `Package`. This follows
  from the decision to check on one leg; the updated rule says so.
- **No local mirror.** `mise run ci:local` and `mise run app:check` will not
  run the featureless check, so it is first seen on the PR's checks. Adding
  it locally would compile a second GPUI variant on a host shared with other
  sessions. The existing rule "keep the feature command string identical in
  `ci.yml`, `tools/ci/local.sh` and `app:check`" concerns the `screenshot`
  clippy command and is not affected.
- **A check is not the release build.** `cargo check` type-checks the dev
  profile and neither generates code nor links. A missing test-only API is
  a type-check error and is caught; a link failure or code under
  `cfg(not(debug_assertions))` is still first seen in `Package`.
- **Cache size.** At the next `Cargo.lock` change the Linux cache gains the
  second check variant (metadata only, no object code). The repository's
  caches already overrun the 10 GB limit; the added size is expected to be
  small but is not measured.
- **Per-run cost until the cache key changes.** As described in Step 1, the
  new variant is re-checked from scratch on every run until `Cargo.lock`
  changes. If the measured duration is large enough to threaten the Linux
  leg's 30-minute limit, record it in `learnings.md` and raise it with the
  user rather than changing the timeout in this PR.

## Progress

