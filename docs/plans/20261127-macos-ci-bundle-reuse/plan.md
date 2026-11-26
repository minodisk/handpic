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

# Reuse the test build in the macOS app CI job's bundle step

## Purpose

The `app (macos-latest)` CI job has a `timeout-minutes: 30` limit and a
cold-cache run takes 24 to 29.5 minutes (28m33s on the rerun `todo.md`
records, 28m46s on 2026-11-26), so it is cancelled at the limit on some PRs.
About a third of a cold run is the "Bundle and check Handpic.app" step
(8 to 10 minutes): `mise run app:bundle:macos debug` runs a plain
`cargo build`, in which the dev-dependency `gpui-pre-platform` with
`test-support` is off, so the GPUI crates compile a second time in another
feature variant after `cargo test` built them with `test-support`.

This plan makes the bundle step build with the `screenshot` feature, which
turns on the same `gpui-pre-platform` `test-support` (and `image/png`) the
dev-dependencies turn on, so the step reuses the dependencies `cargo test`
built and compiles only the `handpic` binary. A cold run should then take
about 17 to 21 minutes, well inside the unchanged 30-minute limit. It closes
the `todo.md` item "Raise the job's `timeout-minutes` ... or avoid the bundle
step's second gpui build", leaving only the part later PRs can verify.

## Steps

- [x] Step 1: Bundle a `screenshot` feature build in the macOS app CI leg, and
      record the choice
  - Done when:
    - `mise.toml`'s `app:bundle:macos` can be given cargo features for its
      build, and without them builds exactly as today (`mise run
      app:bundle:macos` and `mise run app:package:macos` still make the
      release bundle with no feature; `.github/workflows/package.yml` is
      unchanged).
    - `.github/workflows/ci.yml`'s "Bundle and check Handpic.app" step bundles
      the debug build with the `screenshot` feature, with a comment saying
      why (the feature gives the build the dependency features `cargo test`
      already compiled). `timeout-minutes` stays 30. No other job or step
      changes.
    - `docs/agents/app.md` records the choice and the reason: the bundle
      step's second GPUI build was avoided rather than the timeout raised,
      with the measured step times (cold: "Lint and test" 15 to 19 min,
      "Bundle and check" 8 to 10 min, job 24 to 29.5 min; warm: job 2 to
      5 min) and a rule that the CI bundle build keeps the dependency
      feature set of `cargo test`. The sentences the change makes false are
      corrected: in "The `app` CI job runs on all three platforms", "the
      macOS leg already compiles a second variant of the app for its bundle
      step"; in "macOS gets `Handpic.app` from a mise task", the task's
      signature, "bundles the debug build" and the rule quoting `mise run
      app:bundle:macos debug`.
    - `todo.md`'s section "App: watch the macOS app CI job's cold-cache
      duration against the 30-minute timeout ..." is cut down to a partial
      close-out: its Background says what was done (the bundle step now
      reuses the test build, see `docs/agents/app.md`) and its only TODO box
      is the verification later PRs give: a cold-cache `app (macos-latest)`
      job finishes inside the 30-minute limit on three consecutive PRs (a
      cold run is one whose `actions/cache` step restores nothing), and if
      one does not, raise `timeout-minutes`. The two existing boxes are
      removed.
    - On the PR's `app (macos-latest)` job, the bundle step's log shows only
      `handpic-app` compiling (no `gpui-*` crate), and its three checks
      (`plutil -lint`, the executable, the `.icns`) pass.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Why the feature works: `crates/app/Cargo.toml`'s `screenshot` is
      `["dep:gpui-pre-platform", "image/png"]` with `gpui-pre-platform`'s
      `test-support`; the dev-dependencies are the same `gpui-pre-platform`
      with `test-support`, `image` with `png`, and `tempfile`. `cargo test`
      and `cargo build` share the dev profile's dependency artifacts in
      `crates/app/target/debug`. Do not change `Cargo.toml`.
    - The feature builds on macOS: the 2026-11-26 macOS device checks ran
      `mise run app:dev:screenshot` on Darwin arm64. CI has not built it on
      macOS before, so the PR's macOS job is the first check of that.
    - `mise.toml`: the task has `usage = 'arg "[profile]" default="release"'`
      and reads `usage_profile`. Add one optional input for the features
      (a usage flag such as `--features <features>`, or a second optional
      positional argument; decide by which mise 2026.9.0 hands the task
      cleanly, and check the variable name it sets) and pass it to both
      `cargo build` branches only when set. Keep the task the one place that
      lays out `Handpic.app` (the rule in `docs/agents/app.md`); do not
      build in `ci.yml` and copy by hand.
    - `ci.yml`: only the `mise run app:bundle:macos debug` line and the
      comment above the step change. The step's paths
      (`crates/app/target/debug/Handpic.app`) stay.
    - The PR's own run is probably warm (the restored cache holds both
      variants), so the step time will not show the saving. Read the
      `Compiling` lines of the bundle step instead. If a `gpui-*` crate
      compiles there, the feature sets differ: compare
      `cargo tree -e features --features screenshot` against the test
      build's before changing anything else, and fall back to raising
      `timeout-minutes` (see Trade-offs) if they cannot be made equal.
    - PR #1314 (`fix/ci-apt-mirror-timeout`) edits the same job's Linux apt
      step and `todo.md`; rebase onto it if it merges first.
    - `CLAUDE.md` mentions `app:bundle:macos` only by what it lays out; it
      needs a change only if the wording there becomes false.

## Trade-offs and risks

- **Avoid the second build (chosen) or raise `timeout-minutes`.** Raising the
  limit (to 45, say) is a one-line change with no risk, but keeps 8 to 10
  minutes of duplicate compilation on every cold run and the second variant
  in the macOS cache (2.4 GB now, in a repository whose caches overrun the
  10 GB limit and are evicted, which is why cold runs are frequent). Avoiding
  the build touches `mise.toml` and `ci.yml` and removes the cause. The user
  approved this choice on 2026-11-27.
- **CI's bundle holds a feature build, not the default one.** The step checks
  only the bundle's layout, which the feature does not change, and the
  release bundle comes from `package.yml`, untouched. But `ci.yml` then no
  longer compiles a default-feature, non-test build of the app on macOS (the
  Windows and Linux legs never did); `Package` still does. Clippy and the
  tests keep covering the default feature set's code.
- **Not both.** Raising the timeout as well would hide a regression of the
  saving. With about 9 minutes of expected margin the limit stays 30; the
  `todo.md` box says to raise it if a cold run still does not fit.
- **The saving is unproven until a cold run.** No local build is made and the
  PR's run is likely warm, so the three-consecutive-cold-runs check stays in
  `todo.md`.
- **Out of scope:** the cache eviction that makes cold runs frequent
  (`cargo-app-*` caches of 1.8 to 2.4 GB saved per PR ref), and the other
  jobs.

## Progress
