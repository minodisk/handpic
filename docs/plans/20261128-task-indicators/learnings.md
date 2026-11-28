# Learnings: task indicators

## Step 1

- `ProgressCircle` at gpui-component 0.9.1 has an indeterminate state,
  `.loading(true)` (a 1 s repeating arc animation, the value ignored), and
  takes children (`ParentElement`, centered in the ring). The indeterminate
  task draws with it, the icon and ring in `muted_foreground` for the dim.
- Icon placement: no screenshot was taken in this step. Building the app
  locally is off on this host (the user's standing rule: let the PR's GitHub
  checks build; parallel sessions have run the machine out of memory), so the
  `screenshot` build could not run. By measure instead: `.small()` is 12 px
  with a stroke of `min(width * 0.15, 5)` = 1.8 px, an inner hole of about
  8.4 px, too small for a legible Lucide icon. The icon is placed beside the
  circle (a 28 px slot: 12 px icon, `gap_1`, 12 px circle).
- The catalog names verified at `gpui-kit-assets` 0.9.1: `Trash`
  (`icons/trash.svg`) and `Download` (`icons/download.svg`) are not in the
  default `Assets` (`default-icons.txt`) and were added to `icon_assets!`.
  For Step 2: `triangle-alert` is already in the default `Assets` (only a
  path constant is needed), and the close `x` is the catalog's `X`
  (`icons/x.svg`), not in the default set, so it goes into `icon_assets!`.
  Neither constant was added here, as an unused constant fails clippy's
  `dead_code` under `-D warnings`.
- The idle gate exposed only the held operation's label; `IdleGate::held`
  gives the operation too, so the view can tell a held trash run, undo or
  redo (trash slot) from a sidecar run or rename (folder slot).
- `scan::Status::running` lost its last caller (the old `progress_circle`), so it was removed and its test now checks `pass()` with the pass names; kept, it would fail clippy's `dead_code` under `-D warnings`.
- `Task.label` is not drawn in Step 1 (Step 2's popover names tasks by it);
  it is passed as the circle's accessibility label so it is read.
- Local checks: only `mise run fmt` ran. `mise run ci:local` (clippy and the
  tests of `crates/app`) was not run locally per the user's standing rule;
  the PR's GitHub checks are the verification for compile, clippy, the unit
  tests in `progress.rs` / `icons.rs` and the new UI test
  `the_task_list_holds_the_scan_beside_a_held_rename_and_empties_at_the_end`.
- The UI test observes the scan's task with a `ReadSlots` pause held (taken
  from the view's `slots` before the folder opens, dropped after the
  assertions), so the scan's reads wait and its task is there to be seen
  however fast the machine is, not by catching it mid-pass.
- A folder rename and a `Rename by Capture Date...` split have no counter:
  once they run, `Sources::folder_uncounted` (fed from `running_op` being
  `rename::LABEL` or `datefolder::LABEL`) gives the folder slot an
  `Amount::Unknown` task with the tip `"{label}..."`, so the indicator does
  not vanish between the gate's "waiting" and the operation's end.

## Deferred issues (todo candidates)

- Pending manual check (Step 1 of `docs/plans/20261128-task-indicators`,
  status row task indicators, `crates/app/src/main.rs` `task_indicators`):
  on Windows 11 (any desktop platform), open a folder of RAW files and
  watch the status row's right end during the scan, then hold a Move to
  Trash over a scan and run one. Expected: four fixed slots right of the
  strip position, the scan's `scan-face` icon beside a filling circle, the
  trash slot's `trash` icon dim with a loading arc while the run waits and a
  filling circle while it runs, empty slots keeping their width so the
  position does not move, and hovering a slot shows its one line (for
  example `scanning 3/10`, `Move to Trash: waiting for the scan to
  finish`). Also judge from a screenshot whether the icon would read inside
  the 12 px circle (it is drawn beside it). Needs a debug or release build
  of `crates/app`, a folder of a few hundred RAW files, the MCP
  `screenshot` tool or the user's eye, and a pointer hover. The step's
  checkbox was ticked on the automated criteria.
