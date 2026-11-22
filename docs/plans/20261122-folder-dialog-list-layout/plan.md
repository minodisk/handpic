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

# Folder dialogs: the list starts at its first row and shows its last row

## Purpose

The folder menu's confirmation dialogs (`Sequence JPEG Timestamps…`,
`Import…`, `Rename by Capture Date…`, and the two sidecar dialogs, which share
the same body) show a blank band between their lines and the first row of
their list, and clip the list's last row. Seen on Windows 11 on 2026-11-22
(`docs/plans/_archived/20261122-windows-device-checks-5/learnings.md`): about
one row of blank on 10 files, about 360 px on 300 files with the scrollbar
thumb mid-dialog while the first rows show, and in Import and Rename by
Capture Date a last row cut mid-line until the list is scrolled. `todo.md`
item: `### App: the folder dialogs' file list leaves a blank band above its
first row and clips its last row`.

Inferred cause (from reading the source, to be confirmed by the failing test
of Step 1): `folderops.rs`'s `folder_dialog` is the only dialog that fills
both of the component `Dialog`'s slots, the lines through `.content(..)` and
the rows / lists through `.child(..)`. In gpui-component 0.9.1
(`src/dialog/dialog.rs`, `src/dialog/content.rs`) `DialogContent` is
`v_flex().w_full().flex_1()` and the wrapper of the scrolled body is
`div().flex_1().overflow_hidden()`, siblings in one column. GPUI's `flex_1`
is grow 1 / shrink 1 / basis 0 %, so once that column has a definite height
the two share it equally: the lines block is stretched to half (the blank
band is its surplus under the lines) and the list gets only the other half
(it scrolls and clips its last row although the dialog had room; on a long
list the scrollbar's track covers only the lower half). Import's numbers fit:
lines about 51 px, list about 86 px, each given about 68 px, so about one row
of blank and about one row cut. On 300 files the dialog is capped at its
maximum height and each half is about 360 px.

Once this is done the list sits right under the lines, takes all the height
the dialog has left, and scrolls only when it does not fit.

## Steps

- [x] Step 1: Stop the lines block of `folder_dialog` from sharing the
      dialog's height with the list
  - Done when:
    - A UI test opens a folder dialog with lines and a short list (about 10
      rows, plus a headed list as Import has) and asserts that the first row
      starts right under the last line (no more than the dialog's own gap
      between them) and that the last row's bounds lie wholly inside the
      body's visible area without scrolling. The test fails before the fix
      and passes after it. If it does not fail before the fix, the inferred
      cause is wrong: stop, record it in `learnings.md`, and re-investigate
      before changing the layout.
    - The same test, or a second one, with several hundred rows asserts that
      the first row starts right under the last line (the scrolled area, and
      so its scrollbar track, starts there) and, if scrolling the list in the
      test is practical (a simulated scroll wheel event over the list), that
      after scrolling to the end the last row's bottom is not below the
      body's visible bottom.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Real-device check on Windows 11, per `docs/agents/app.md`
      ("`render_to_image` captures the window behind the `screenshot`
      feature": the `screenshot`-feature build, MCP enabled on a test port,
      its own `--index` / `--settings` / `--log`): with copies created under
      `D:\Photos\tests\` only (a new dated folder; never open any other
      `D:\Photos` folder), each of `Sequence JPEG Timestamps…`, `Import…`
      and `Rename by Capture Date…` shows its first row right under the
      lines with no blank band, the scrollbar thumb at the top of its track
      on a long list, and the last row whole (without scrolling when the
      dialog has room, after scrolling to the end otherwise), on about 10
      items and on several hundred. Record the results in this plan's
      `learnings.md`.
    - `todo.md`: when the real-device check passed, the item `### App: the
      folder dialogs' file list leaves a blank band above its first row and
      clips its last row` is deleted, and the two pointers to it in the
      Background of the Sequence and Import real-device items ("its own item
      below", "the item on the folder dialogs' file list below") are reworded
      so they do not dangle. When the check could not be run, the item stays,
      its Background says the fix landed with the test, and its box becomes
      the real-device check only.
    - `docs/agents/app.md`'s section "The folder operations share one dialog
      model and live in `folderops.rs`" says why the lines block does not
      flex (the component's content and body are both `flex_1` and would
      halve the dialog).
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/folderops.rs` (`folder_dialog`),
      `crates/app/src/uitest.rs` (or `folderops.rs`'s own test module using
      `uitest::Ui`), `docs/agents/app.md`, `todo.md`. `crates/app/src/modal.rs`
      is expected to need no change: the trash and Clear Cache dialogs use
      only `.content(..)` and are not affected.
    - The intended fix is one line in the `.content(..)` builder:
      `body.flex_none()` next to `gap_2()`. `DialogContent` applies the
      caller's style after its own `flex_1` (`refine_style`), and GPUI's
      `flex_none` sets grow 0, shrink 0 and basis auto, so the lines keep
      their own height and the body wrapper is the column's only flexible
      child. Do not change the vendored component and do not move the lines
      into the scrolled body (see Trade-offs).
    - Verify during implementation that a dialog with lines and no list
      (`lists_empty`) and one with a list and no lines still lay out as
      before.
    - For the test, the elements need names: add `debug_selector(..)` to the
      last line, the first and last rows (or the list's container) and
      whatever marks the body's visible bottom (the footer's top works).
      `debug_selector` is a no-op outside debug builds with GPUI's
      `test-support`, which the tests have through the `gpui-pre-platform`
      dev-dependency; read the bounds with `VisualTestContext::debug_bounds`
      (`Ui::cx`).
    - The test can drive the shared body with a synthetic `dialog::Dialog`
      (`dialog::Content` with lines, rows and a headed list) through
      `show_folder_dialog` / `follow_folder_dialog`, instead of running each
      flow against files: all folder dialogs render through `folder_dialog`.
      `show_folder_dialog` and `FolderOps::dialog` are private to
      `folderops.rs`, so either put the test in that file's test module or
      widen the visibility of the one entry point to `pub(crate)`; decide by
      which is the smaller change. The dialog opens through `cx.defer`, so
      let the test platform run (`run_until_parked`) before reading bounds,
      and close the dialog before the `Ui` drops.
    - The test window's size decides whether the short list fits; assert the
      precondition (the dialog's surface is below its maximum height) so the
      test does not pass or fail for the wrong reason.
    - Do not touch the other `todo.md` items (the pending rename's Access
      denied, the status line's and the `Source:` line's verbatim prefix, the
      trash Undo count, the Rewrite Sidecars wording) beyond the two pointer
      phrases named above.

## Trade-offs and risks

- Fix placement. Taken: `flex_none()` on the lines block, one line, the lines
  stay fixed above the scrolling list as designed. Not taken: moving the
  lines into the scrolled body, which needs no style override but lets the
  lines scroll away with a long list, a visible behavior change nobody asked
  for. Not taken: patching or forking gpui-component; the equal split is the
  component's behavior for any dialog that uses both slots, and reporting it
  upstream is outside this work.
- The cause is inferred from the source, not reproduced. The numbers of the
  Import case and of the 300-file case fit; the 10-file Sequence case fits
  only roughly (about 50 px estimated against "about one row" observed). The
  failing-first test is the check; if it does not fail before the fix, the
  step stops instead of shipping the one-liner.
- "Thumb mid-track" is read here as the scrollbar's track starting mid-dialog
  with the thumb at the track's top. If the real device still shows a thumb
  away from the top of its track after the fix, there is a second cause
  (a non-zero scroll offset) and it needs its own investigation.
- Deleting the `todo.md` item leaves two sentences in other items pointing at
  it. The plan rewords only those two phrases (approved by the user on
  2026-11-22).
- The real-device check needs the `screenshot`-feature build and MCP on a
  test port. If it cannot be run in the implementing session, the item is
  narrowed to that check, not deleted, so the fix is not claimed as seen.
- The test covers the shared `folder_dialog` body with synthetic content, not
  the three flows end to end; a flow-specific layout difference would be
  caught only by the real-device check.
- `debug_bounds` returns `None` in a build without `debug_assertions`, so the
  new test would fail under a release-profile test run. The existing tests
  run in the debug profile.

## Progress
