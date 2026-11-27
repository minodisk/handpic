<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record what you learn during implementation, and notable events, in `learnings.md` as they happen.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory; put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# File the three Windows UI tests that failed once on a docs-only PR (windows-flaky-ui-tests)

## Purpose

On PR #1336 (docs only), the first attempt of the `app (windows-latest)` job
of run 46020887142 failed on three tests of `crates/app`, and a re-run of the
failed job passed with no change:

- `folderops::tests::a_held_click_on_a_folder_a_split_removed_opens_the_first_date_folder`
- `folderops::tests::a_click_on_a_folder_under_a_rename_in_flight_is_refused`
- `judgment::tests::the_quit_drain_writes_a_key_inside_the_debounce`

The failing attempt's job log (job 140579952371) is no longer downloadable,
so the panic messages are not known; the test names come from the PR's
`pr-check-fixer` report. The user asked for a `todo.md` item so the flakiness
is investigated. Docs only: one step, one PR (single-PR mode).

## Steps

- [x] Step 1: Add a `todo.md` item for the three flaky Windows UI tests
  - Done when:
    - `todo.md` holds, under `## Cross-cutting / other`, a new item
      `### App: three Windows UI tests failed once and passed on a re-run (folderops, judgment)`
      with a `#### Background` (the three test names, the run and job ids,
      that the PR was docs only and the re-run passed, that the log expired so
      the panic is unknown, the files `crates/app/src/folderops.rs`,
      `crates/app/src/judgment.rs`, `crates/app/src/uitest.rs`) and a
      `#### TODO` with boxes: (1) when one of them fails again on CI, save the
      job log (`gh api repos/minodisk/handpic/actions/jobs/<id>/logs`) before
      it expires and record the panic and the phase `uitest.rs`'s watchdog
      reports; (2) read each test's waits (`wait_until` bounds, the judgment
      writer's debounce against the quit drain) for a timing assumption a
      loaded Windows runner can break, and fix it or widen the bound, without
      running the tests in a local loop (CLAUDE.md forbids load tests on the
      host).
    - The item notes that the first test is also the subject of the existing
      item `### App: confirm the split-deferred-open test fails without its fix`.
    - Only `todo.md` and this plan's files change; LF endings.
  - Implementation approach: edit `todo.md` by anchoring on `## Cross-cutting / other`; insert the item right after that heading line, keeping one blank line between items. Rebase on `origin/main` first; `todo.md` is edited by other sessions.

## Progress
