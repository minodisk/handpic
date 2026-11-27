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

# ci-apt-mirror-timeout

## Purpose

`todo.md` holds the item "Agents: `test (ubuntu-latest)` hangs on docs-only commits". It asks for the cause of three Ubuntu CI jobs that ran into the 30-minute job timeout during the `ef-gate-fill` feature, and for either a fix or a recorded rerun-only rule.

The cause is now identified from the attempt-1 job logs: the runner's apt mirror was throttled, so the apt install step took 25 to 29 minutes and the job timeout landed on whatever step came next. It was not a flaky test, not a cache stall, and not related to the commits being docs-only.

| Run (attempt 1) | Job | apt step | apt's own summary |
| --- | --- | --- | --- |
| main, run 42128694204 | 127599427099 | 23:19:12 to 23:45:06 on 2026-11-05 | `Fetched 56.6 MB in 25min 31s (37.0 kB/s)` from `azure.archive.ubuntu.com` |
| PR 904, run 42123498474 | 127581623540 | 26m56s | `Fetched 56.5 MB in 26min 36s (35.4 kB/s)` |
| PR 901, run 42086725922 | 127453726517 | 29m21s | `Fetched 33.9 MB in 28min 59s (19.5 kB/s)` |

In the main run, "Build and test" started at 23:46 and was cut at 23:49; the `Terminate orphan process ... riffle_core-...` line only names what was running at the cutoff. In PR 901 the "stall in `actions/cache`" was only the step the timeout landed on. All reruns passed. A scan of every rerun or failed CI run since 2026-11-06 (1000 runs) found no recurrence of a long apt step.

Today the `test` job has no apt step (the Tauri dependencies are gone). The only apt step left in `ci.yml` is the `app` job's "Install the Linux GPUI dependencies", which normally takes about 9 seconds.

This plan closes the item by making a throttled mirror fail fast under the apt step's own name, and by recording how to read a timed-out job so the next one is diagnosed in minutes rather than misattributed.

The same scan found one unrelated timeout, a hung test: run 45091844682 attempt 1, `app (ubuntu-latest)` (job 137424185183, 2026-11-23, cold cache), logged `test uitest::opens_with_no_folder has been running for over 60 seconds` at 01:17:55 and was cancelled at the job limit at 01:32:39; the rerun passed. The user chose to record it as a new `todo.md` item in this PR, following the new guide rule.

## Steps

- [x] Step 1: Bound the apt step, record how to read a job that hit its timeout, replace the todo item
  - Done when:
    - `.github/workflows/ci.yml`: the `app` job's "Install the Linux GPUI dependencies" step has `timeout-minutes: 5`, with a short comment citing the 2026-11-05 mirror throttling. The job-level `timeout-minutes` values (`lint` 15, `test` 30, `app` 30) are unchanged.
    - `docs/agents/github-actions-workflows.md` has a new section `## A job that hits its timeout` holding a **Measured** entry that ends in a `- Rule:` line and cites the three runs above.
    - `todo.md` no longer has the item "### Agents: `test (ubuntu-latest)` hangs on docs-only commits" (heading, Background and TODO all removed).
    - `todo.md` has a new item for the hung `uitest::opens_with_no_folder` (run 45091844682, job 137424185183), in the file's item format (`### App: ...` heading, `#### Background` with the evidence above and `Files: crates/app/src/uitest.rs`, `#### TODO` with one checkbox whose "Done when" is: the hang is reproduced or its cause found from the test's code, and fixed, or a per-test bound added so it fails fast). No other todo item is touched.
    - `mise exec -- actionlint` reports nothing.
    - The PR's GitHub checks pass, including `app (ubuntu-latest)` with the apt step finishing well inside its 5 minutes (local `ci:local` is left to GitHub Actions per the session's rule).
  - Implementation approach:
    - `ci.yml`: the step is at the top of the `app` job's `steps`, under a two-line comment about what GPUI links against and gated by `if: runner.os == 'Linux'`. Add `timeout-minutes: 5` to that step and extend the comment in the file's style: the step normally takes about 9 s; in November 2026 a throttled runner mirror stretched the apt step to 25-29 minutes in three runs and the job's 30-minute limit fell on a later step, so the limit here makes a slow mirror fail under this step's name.
    - Do not add retries, a different mirror, or a cache for the apt packages; the change is the step timeout only.
    - Guide entry: follow the file's conventions, a `###` heading ending in the tag in parentheses (e.g. `### Read the timed-out attempt's step durations before blaming a step (Measured)`), prose, then a `- Rule:` bullet. Place the new `##` section before `## A dedicated branch as a data store`. Content:
      - A rerun overwrites what `gh run view` shows (latest attempt only). Read the timed-out attempt through `gh api repos/{owner}/{repo}/actions/runs/<id>/attempts/<n>/jobs` (step `started_at` / `completed_at`) and the job's log through `gh api --allow-escape-sequences repos/{owner}/{repo}/actions/jobs/<job id>/logs` (without the flag `gh` refuses a log holding ANSI colors).
      - The step the job timeout lands on, and the `Terminate orphan process ...` lines, name only what was running at the cutoff, not what used the time. Find the step with the long duration.
      - An apt step whose log ends in a low `Fetched ... (NN kB/s)` is the runner's mirror: rerun, nothing to fix in the repository. Cite the three runs and their figures; say the 2026-11-05 step was the since-removed "Install the Linux WebView dependencies" (and the binary the pre-rename `riffle_core`), and that `ci.yml`'s remaining apt step now carries its own `timeout-minutes: 5`.
      - A test step logging `has been running for over 60 seconds` names a hung test: rerun, and record that test in `todo.md` (run 45091844682 as the example).
    - The guide already has "Pushing a workflow change needs the `workflow` scope (Hit)"; do not duplicate it.
    - Everything in English. No `README.md` or `docs/humans/**` change, so no Japanese counterpart. `CLAUDE.md` needs no update.
    - No local cargo builds; nothing Rust changes.

## Trade-offs and risks

- **The 5-minute value.** Normal is about 9 s and the throttled runs fetched at 19 to 37 kB/s, so 5 minutes separates the cases widely while tolerating an ordinary slow day.
- **Fail fast, not self-heal.** A throttled mirror still fails the job and needs a rerun; it only fails in 5 minutes under the right step name. Retries or mirror switching were not taken: the throttling has not recurred in 1000 scanned runs.
- **No job-timeout change and no per-test timeout.** Neither addresses the identified cause.
- **The `app (macos-latest)` 30-minute limit** has its own todo item and is out of scope.

## Progress

- (2026-11-27) Step 1 complete
