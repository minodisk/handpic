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

# JPEG folder strip cells stay gray until pass 1 ends

## Purpose

Close the `todo.md` item "App: a JPEG folder's strip cells in view stay gray
until pass 1 ends". On 2026-11-23 (Windows 11, debug build, fresh index, 500
JPEGs, 1600 px window; finding D of
[results.md](../_archived/20261123-windows-device-checks-mcp-visual/results.md))
the cells at positions 3 to 9 stayed gray until pass 1 ended while positions
1, 2, 10 and 11 had thumbnails. Once this is done, the cells in view of a
freshly opened folder fill in as pass 1 commits their rows.

## What the code reading found

The root cause is not pinned. Nothing was built or run for this plan.

The stuck cells are exactly the first seven thumbnail requests:

- The cell pitch at the default strip height is 154 px (`Geometry::of`,
  `crates/app/src/strip.rs:33-45`); the strip's width is the window's content
  width (`crates/app/src/main.rs:990`, `7664-7667`).
- At 1600 px, cells 0..=10 are on screen and the viewport centre is at cell
  5.19. `Strip::next_to_request` asks nearest the centre first
  (`strip.rs:276-290`): 5, 6, 4, 7, 3, 8, 2, then 9, 1, 10, 0.
- The first seven are positions 3 to 9, the gray ones; the next four are
  positions 10, 2, 11 and 1, the ones with thumbnails.

Ruled out, the hot list missing the cells in view:

- It is built from `self.files` and `strip.visible_range`, the list and
  indices the cells draw from (`main.rs:4237-4248`), and set before the
  workers start (`main.rs:3509-3512`).
- Its path strings are the scan's (`crates/core/src/scan.rs:1001-1018`;
  `index::stat`, `crates/engine/src/index.rs:880-892`).
- A view-only folder on a fresh index has no capture times, so the order is
  the name order and nothing re-sorts during pass 1 (`read_entries` runs only
  on `Done`, `main.rs:3747-3752`, `3853-3881`).
- `todo` is in listing order, so files 2..8 are among the first taken even
  without a hot list.

Ruled out, rows unreadable until the pass ends:

- Pass 1 commits every `BATCH` = 10 files (`index.rs:213`, `1407-1468`),
  reports a path ready only after its commit (`index.rs:2400-2418`) and
  delivers every ready path (`index.rs:2446-2459`).
- The app applies every `Progress` (`main.rs:754-761`, `3738-3746`) and
  `mark_ready` clears `missing` (`strip.rs:341-348`).
- The reader is a separate read-only WAL connection (`index.rs:920-929`),
  shown to read during an open write transaction (`index.rs:6392-6425`).
- A cell read before its row exists is `missing`, not `failed`
  (`strip.rs:305-321`), and is asked again on `mark_ready` and `refresh`.

Suspects, none confirmed:

1. `thumbs::read` answers `NotScanned` for every error other than "no cached
   thumbnail", with no log (`crates/app/src/thumbs.rs:117-130`). A
   ready-reported cell gets one retry (`strip.rs:309-313`); a second such
   answer leaves it `missing` until `Done` (`main.rs:3750`). The reader's
   busy timeout is 300 ms (`index.rs:926`). No path that makes the reader
   fail was found.
2. The order in which a slow UI thread handles thumbnail answers and
   `Progress` events. The interleavings traced on paper all recover.
3. The image is in `Thumbs` but not painted (`main.rs:4575`, `7316-7331`).

## Steps

- [x] Step 1: Reproduce the gray cells in a test, fix the cause, and leave the real-device check in `todo.md`
  - Done when:
    - A UI test in `crates/app/src/uitest.rs` or a unit test fails on the
      code before the fix and passes after it, and its name or comment says
      which cause it demonstrates.
    - The fix changes only what that cause needs.
    - "What the code reading found" above is replaced by the confirmed cause
      with file and line references.
    - The `todo.md` item's TODO box is replaced by one real-device box:
      re-run the 500-JPEG fresh-index open on Windows and see the cells in
      view fill before pass 1 ends.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Fallback (decided by the user): if no test reproduces the fault, the
      step ships diagnostics instead of a fix: a log line for each non-row
      thumbnail read error in `thumbs::read` and for a `NotScanned` answer on
      a ready-reported cell in the strip, with tests for whatever pure logic
      they add. The `todo.md` item then keeps its TODO box, rewritten to say
      to re-run the 500-JPEG fresh-index open on the device and read those
      log lines in `Handpic.log` to find the cause. Record what was tried in
      `learnings.md`.
  - Implementation approach (as far as it is known; omit if unknown):
    - Reproduce first. The target state: a JPEG-only folder with more files
      than the strip's range, a fresh index, and in-range cells that hold no
      image after their paths arrived in a `Progress` and before `Done`.
    - `Ui::open_folder` writes DNGs and waits for idle (`uitest.rs:166-177`);
      the test needs JPEG files (the `jpeg` helper, `uitest.rs:241-251`) and
      a way to look at the strip before pass 1 ends. How to hold pass 1 open
      is not known; measure whether the view's `ReadSlots` pause is enough
      (a pause is honoured for at most 1 s per read,
      `crates/core/src/scan.rs:849`).
    - Check the three suspects in order against the reproduction. If the
      cause is in the strip's bookkeeping, demonstrate it in a `strip.rs`
      unit test next to `a_not_scanned_answer_after_ready_is_asked_again`.
    - Files expected to change: `crates/app/src/uitest.rs`, and whichever of
      `crates/app/src/strip.rs`, `crates/app/src/thumbs.rs`,
      `crates/app/src/main.rs`, `crates/engine/src/index.rs` holds the cause;
      `todo.md`.
    - Another session is editing `crates/app/src/main.rs` around the folder
      tree reveal and rename. Keep any change there inside `pump`,
      `thumbnail` and the `Event::Progress` / `Event::Done` arms of
      `scanned`.
    - If the fix changes when a cell is asked again, update the strip section
      of `docs/agents/app.md` and the `strip.rs` / `thumbs.rs` wording in
      `CLAUDE.md` in the same PR.

## Outcome of Step 1

No test reproduced the fault, so Step 1 shipped the fallback: a warn line
for a thumbnail read error other than no row (`crates/app/src/thumbs.rs`,
`read` and `no_row`) and one for a `NotScanned` answer on a cell the scan
reported committed (`warn` only when it was not already retried, `info` otherwise) (`crates/app/src/main.rs`, `thumbnail`, through
`Strip::after_ready` in `crates/app/src/strip.rs`). The causes above stay
unconfirmed; what was tried is in [learnings.md](./learnings.md).

## Trade-offs and risks

- The cause is unknown at planning time, so the test may not reproduce it:
  the fault was seen once, in a debug build, on 500 real JPEGs. The user chose
  the diagnostics fallback in Step 1's Done-when for that case, over stopping
  with no PR or hardening without a confirmed cause.
- Not in scope: the hot list is ordered nearest the current file
  (`crates/app/src/focus.rs:25-36`) while the strip asks nearest the
  viewport centre (`strip.rs:276-290`). It does not explain the gray cells.

## Progress

- (2026-11-27) Step 1 complete (diagnostics fallback; device re-run pending in todo.md)
