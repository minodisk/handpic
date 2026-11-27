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

# Record that the release key environment's verification (b) passed

## Purpose

The release-key-environment work
([plan](../_archived/20261126-release-key-environment/plan.md)) moved the
production updater key `UPDATE_SIGNING_KEY` into the `release` GitHub
environment and left one check open, verification (b): that a real release
from `main` signs with the environment's secret once the repository-level
secret is gone. That check has now passed:

- The repository-level `UPDATE_SIGNING_KEY` secret was deleted on 2026-11-27,
  before the release; only the `release` environment's copy exists.
- v2.2.0 was released from `main` on 2026-11-27 (release PR #1308, merge
  commit `f5cab4bf`, `Release` run 46003797551). Every job succeeded,
  including `guard`, which read `UPDATE_SIGNING_KEY` from the `release`
  environment.
- `mise exec cargo:zipsign@0.2.2 -- bash .claude/skills/release/scripts/verify-release.sh v2.2.0`
  reported `STATUS=ok`: all eleven assets present, `sha256sum --check`
  passing, and `zipsign verify` of the five updater archives passing against
  `crates/app/packaging/update.pub`.

The agent guides still describe this as pending and still carry the "until
the repository secret is deleted" caveats, and `todo.md` still lists the
manual check. This plan makes the docs state what is now true and closes the
todo item.

## Steps

- [x] Step 1: Record verification (b) as passed in the agent guides and delete its todo item
  - Done when:
    - `docs/agents/app.md`'s decision Rule (the one that reads "decided
      2026-11-26, plan `20261126-release-key-environment`") no longer has a
      "Not yet verified" sentence. In its place a "Verified" sentence says
      that the release of v2.2.0 from `main` (measured 2026-11-27, run
      46003797551) passed `guard` and that `verify-release.sh v2.2.0`
      reported `STATUS=ok`, and that, with the repository-level secret
      already deleted, this shows the legs sign with the `release`
      environment's secret.
    - The same Rule no longer makes the branch guarantee conditional: the
      clause "once the repository-level secret is deleted (a manual setting
      after this lands; until then a job without the `environment:` key
      still reads it)" is replaced by a statement that the repository-level
      secret is deleted (2026-11-27) and only the environment's copy exists.
    - `docs/agents/github-actions-workflows.md`'s section "A dispatchable
      reusable workflow sees the repository secrets too" no longer says the
      repository copy "is deleted once that lands; until then a direct
      dispatch on any ref still resolves it to the repository copy". It
      states the present state: `UPDATE_SIGNING_KEY` lives only in the
      `release` environment, its repository copy deleted on 2026-11-27, so
      only a dispatch on `main` whose leg declares the environment reaches
      it. The section's general lesson, its Rule and its heading stay.
    - The archived plan `docs/plans/_archived/20261126-release-key-environment/plan.md`
      gets one Progress line recording that verification (b) passed (v2.2.0,
      run 46003797551, `STATUS=ok`); nothing else in the archived folder
      changes.
    - `todo.md` no longer holds the item "### Release: real-device check
      that a release from `main` signs with the `release` environment's key
      (release-key-environment)", nor the `---` separator that followed it;
      its neighbours are unchanged and separated by one blank line.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Docs only. Keep the rest of the app.md Rule word for word (the
      `deployment: false` explanation, the measured runs 45846600132 and
      45846638280, the "Still not covered" sentence). Follow the Rule's
      phrasing: "Verified (measured 2026-11-27, run <id>): ...".
    - Say "release" (the final release v2.2.0 through the release-please
      path), not "release candidate"; both paths share `guard` and the
      `package.yml` legs under `sign_production`.
    - Already correct, leave untouched: `CLAUDE.md`,
      `.claude/skills/release/SKILL.md`, the workflow comments and
      `tools/package/sign-production.sh`.

## Progress

- (2026-11-27) Step 1 complete.
