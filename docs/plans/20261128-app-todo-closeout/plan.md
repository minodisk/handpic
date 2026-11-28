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

# Close three `crates/app` items of todo.md (app-todo-closeout)

## Purpose

Three small, independent `crates/app` items sit in `todo.md` (headings as of
`origin/main` 97760424):

1. `### App: the strip's Copy Path copies the Windows verbatim prefix`: the
   strip menu's `Copy Path` puts `\\?\D:\...` on the clipboard while the
   tree's puts `D:\...`.
2. `### App: three Windows crates/app tests failed once and passed on a re-run (folderops, judgment)`,
   its second checkbox only: read the three tests' waits for a timing
   assumption a loaded Windows CI runner can break, and fix it or widen the
   bound. The first checkbox (save the job log on the next failure) stays.
3. `### App: real-device check of the meta pane's Analysis group and the AF rows in Maker note`:
   its one open check (after another folder is opened, the meta pane shows no
   index rows of the previous file until the new folder's first preview) was
   inconclusive by screenshot and needs a fast automated check.

Once done, the clipboard holds a path every Windows program accepts, the
three tests no longer depend on a wall-clock bound a busy runner can miss,
and the meta pane's behaviour across a folder open is pinned by a test.

Out of scope: every other `todo.md` item, including
`### App: confirm the split-deferred-open test fails without its fix`.

No load tests on the host: no test is run in a loop or under generated load
(CLAUDE.md). A targeted single run of one test is allowed; CI verifies the
rest.

## Steps

- [x] Step 1: The strip's `Copy Path` copies paths without the Windows verbatim prefix
  - Done when:
    - The strip menu's `Copy Path` writes each target path through
      `context::shown_path`, one per line, so a `\\?\D:\...` listed path
      reaches the clipboard as `D:\...` and a `\\?\UNC\server\share\...` one
      as `\\server\share\...`.
    - A test that does not depend on the OS covers the joined text for a
      verbatim drive path, a verbatim UNC path and an unprefixed path.
    - The existing UI test
      `the_strip_menu_takes_the_arrows_and_enter_and_hands_other_keys_on`
      expects the clipboard without the prefix (it asserts the raw listed
      path today, at two places, and would fail on Windows CI otherwise).
    - `docs/agents/app.md`'s rule on the copy items no longer says the
      strip's paths are written "prefix and all, as the Tauri app wrote
      them"; it states the new rule and why (the tree's spelling, the shell
      and other programs refusing the prefix).
    - The `todo.md` item
      `### App: the strip's Copy Path copies the Windows verbatim prefix` is
      removed whole.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - The handler is `Handpic::files_action` in `crates/app/src/main.rs`
      (`context::Action::CopyPath`, next to `RevealFile`, which already uses
      `context::shown_path`). `CopyFileName` is unaffected (`file_name`).
    - For the OS-independent test, a small pure function in
      `crates/app/src/context.rs` that maps the paths through `shown_path`
      and joins them with `\n`, called by the handler and unit-tested next to
      `a_shown_path_drops_the_verbatim_prefix`, is the least code. The UI
      test alone only exercises the prefix on Windows, where `scan::list`'s
      canonical paths carry it.
    - In the UI test, compare against `context::shown_path(&b)`.
    - `Copy Path` is `Coverage::Excluded(CLIPBOARD)` in `companion.rs`; no
      MCP tool changes.
    - `todo.md` is edited by other sessions: rebase on `origin/main` first
      and anchor on the item's heading, not on line numbers.

- [ ] Step 2: A UI test that the meta pane shows no index rows of the previous file across a folder open
  - Done when:
    - A test on the `crates/app/src/uitest.rs` harness opens two folders
      whose file names differ and checks that, from the moment the second
      folder's listing is applied until its first preview is presented, the
      meta pane's index lookup (the `Facts` behind the `Analysis` group and
      the `Maker note` AF rows) yields nothing, while the pane still names
      the first folder's file; and that after the first preview it yields
      the new file's own `Facts`.
    - The test is not vacuous: it fails if the test asserts on fewer than
      one observation of that in-between state, and in that state the new
      folder's file at the stale index does have `Facts` in the view (so a
      stale index would have shown another file's rows).
    - If the test shows a bug, it is fixed in the same PR and recorded in
      `learnings.md`.
    - The `todo.md` item
      `### App: real-device check of the meta pane's Analysis group and the AF rows in Maker note`
      is removed whole (its TODO holds only this check).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Reading the code found no bug: `Handpic::opened` (non-carry branch)
      sets `self.shown`'s index to `None`, `meta_pane` looks `facts` up only
      through that index, and a stale preview is dropped by the `order`
      check in `send`. The test pins this.
    - The row lookup is inline in `meta_pane`
      (`at.and_then(|i| self.files.get(i)).and_then(|p| self.facts.get(..))`).
      Moving just that lookup into a small `&self` method that `meta_pane`
      calls gives the test the pane's own rule to assert on, rather than a
      copy of it. No other change to `meta_pane`.
    - Both the listing and the preview load are `background_spawn` tasks on
      GPUI's test dispatcher, so one `run_until_parked` may run both and the
      in-between state cannot be seen from outside. Record it from inside,
      for example with an entity observer on the view that samples the open
      folder, `shown`'s name and the lookup's result on every notify.
      `opened` and `loaded` run in separate `this.update` calls. Verify that
      `opened` notifies before the load lands; the "at least one
      observation" assert catches it if not.
    - A never-scanned second folder has no `Facts` at its open (`set_entries`
      fills them from the index rows), which would make the check pass even
      with the bug. Open B and wait idle, open A and wait idle, then open B
      again and assert on that third open. Verify that the generated DNG's
      index entry gives a `Facts` entry after the scan.
    - `Ui::open_folder` writes a fixed `photos` folder; the second folder
      needs its own name (write it in the test with `dng`, or give the helper
      a name parameter). Different file names per folder, e.g. `a1.dng` /
      `a2.dng` and `b1.dng` / `b2.dng`.
    - Wait with `wait_until` / `wait_idle`, never a fixed sleep (app.md's UI
      test rules).

- [ ] Step 3: Remove the wall-clock assumptions the three flaky Windows tests rest on
  - Done when:
    - `judgment::tests::the_quit_drain_writes_a_key_inside_the_debounce` no
      longer fails when the flushed sidecar write takes longer than the
      engine's 2 s `sidecar::DRAIN_TIMEOUT` on a busy runner: its flush, and
      every other asserted `flush(DRAIN_TIMEOUT)` / `drain(DRAIN_TIMEOUT)`
      in `judgment.rs`'s tests, is bounded by a test-only bound far above a
      write's time, with a comment that it only bounds a loaded runner and is
      no timing claim (the wording
      `keys_inside_the_debounce_are_written_once_with_the_last_value` already
      uses for its 10 s deadline).
    - The `Ui` drop no longer asserts the quit's wall-clock time against
      `QUIT_MAX`: it asserts the quit handlers' own drain results instead (a
      handler whose stop or drain timed out fails the test), per the decision
      below. `WAIT` stays 10 s.
    - `a_click_on_a_folder_under_a_rename_in_flight_is_refused` reports why a
      rename failed when `renamed` is missing (its last assert cannot tell a
      failed rename from a timing failure today).
    - `docs/agents/app.md`'s UI test section matches the changed check.
    - In `todo.md`, the item
      `### App: three Windows crates/app tests failed once and passed on a re-run (folderops, judgment)`
      loses only its second checkbox. The first stays, and the Background
      gains one sentence on what was read and changed, so the next failure is
      read against the new bounds.
    - No test was run in a loop or under generated load.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - What the reading found (record in `learnings.md`):
      - Judgment test: `Writer::flush` sends `Message::Flush`, and the writer
        thread then writes everything pending at once and replies.
        `DEBOUNCE` (300 ms) plays no part, so there is no race between the
        two constants. The assumption is that the XMP write plus the index's
        `mark_written` (SQLite) complete within 2 s of wall clock.
        `crates/engine/src/sidecar.rs`'s own test comment records that the
        write alone can take over 300 ms on CI, and that test does not
        assert `flush`'s result.
      - The two `folderops` tests' explicit waits are `ui.wait_until`, 10 s
        (`WAIT` in `uitest.rs`): the roots, two tree listings, then the date
        folder's open and idle, or the rename's end.
      - Every `Ui` drop also asserts `quit < QUIT_MAX` (1 s wall clock) over
        the quit handlers (scan stop, judgment stores, sidecar writer and
        settings writes, each bounded at 2 s). app.md's UI test section and
        `../_archived/20261128-uitest-watchdog/learnings.md` record this
        assertion failing at 1.5 s to 4.0 s on a loaded host. It is the
        tightest timing assumption the two `folderops` tests share, and it
        fits three unrelated tests failing in one job. The failing log is
        gone, so this is a likely cause, not a confirmed one.
      - The rename's watcher-handle race on Windows is already closed
        (`watch::settle` after `unwatch`, held under the `TreeWatch` lock by
        `treewatch::with_released`).
    - Judgment tests: a test-only constant in `judgment.rs`'s `tests` module
      (for example 30 s) instead of `DRAIN_TIMEOUT`. The engine's
      `DRAIN_TIMEOUT` and the app's quit path are not changed. A long bound
      no longer shows that the flush beat the debounce; the engine's flush
      test covers that claim with a far deadline.
    - `Ui` drop: the quit handlers already compute `stopped`, `stored` and
      `drained`; make a timed-out stop or drain visible to the test (the
      smallest signal that fits, test-visible only) and assert on that in
      place of the `QUIT_MAX` wall-clock assertion.
    - Rename test: put the view's `step_note` (set by `rename_ran` on a
      failure) in the final assert's message.
    - Files: `crates/app/src/judgment.rs` (tests), `crates/app/src/uitest.rs`
      (`QUIT_MAX`, the drop), `crates/app/src/folderops.rs` (one test),
      `crates/app/src/main.rs` (the quit handlers' signal),
      `docs/agents/app.md`, `todo.md`.

## Decisions

Taken by the develop run without asking (each had a clear recommendation):

- `QUIT_MAX`: option A, replace the wall-clock assertion with the quit
  handlers' drain results. Options were A, B (raise it to just under 2 s) and
  C (leave it, improve the message). B only narrows a flake recorded at up to
  4.0 s; C fixes nothing.
- `WAIT` stays 10 s: two rules in `docs/agents/app.md` say a UI test timeout
  is diagnosed from its log lines, not by raising the wait, and nothing shows
  either test hit it.
- Flush bound scope: every asserted flush / drain in `judgment.rs`'s tests,
  since they share one assumption; `scan.rs` and `uitest.rs` siblings were
  not in the failing set and are left.
- Item 3's `todo.md` item is removed whole once the automated test lands, as
  the request asks.
- Three PRs (one per step): Step 3 changes documented test rules and can
  revert on its own.

## Trade-offs and risks

- Step 1 reverses a documented rule. `docs/agents/app.md` says the strip's
  copy items write the listed paths "prefix and all, as the Tauri app wrote
  them", so the prefix was a deliberate port. The risk is a user who pastes
  the path into something that needs the verbatim form (a path over
  `MAX_PATH`); none is known.
- Step 3 cannot be proven: the failing log expired and loops are forbidden,
  so the fix removes assumptions found by reading. The first checkbox stays
  for that reason.
- Step 3's option A adds a small test-visible signal to product code.
- Step 2 replaces a real-device check with a state-level test. It asserts the
  lookup the pane draws from, not pixels.
- All three touch `todo.md` and two touch `uitest.rs`, so later steps rebase.

## Progress
