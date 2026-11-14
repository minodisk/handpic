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

# Progress bars in the GPUI app

## Purpose

The GPUI app (`crates/native`) reports the progress of its long operations
only as text: the scan's `scanning d/t` / `analyzing d/t` on the status line,
and the Move to Trash / Delete Permanently counts as a note in the right pane,
which is invisible while that pane is hidden. The Tauri app has a thin scan
progress bar above the strip bar (`crates/app/ui/src/progress.ts`,
`#scan-progress`); the native app has no equivalent and the GPUI port plan
(`../20261109-gpui-port/plan.md`) does not list one.

After this plan the scan and the trash operations show a determinate progress
bar, through one shared pure module and one bar element that the folder
operations (Sequence JPEG Timestamps, Import) can feed later. The existing
status texts stay as they are.

Inventory of the native operations (2026-11-14):

| Operation | Has `(done, total)` | Shown today | After this plan |
| --- | --- | --- | --- |
| Scan pass 1 (`Event::Progress`) | yes | status line `scanning d/t` | bar |
| Scan pass 2 (`Event::FacesProgress`) | yes, `(0, 0)` until counted | status line `analyzing d/t` | bar (same bar, a second fill) |
| Move to Trash / Delete Permanently / trash redo (`Event::TrashProgress`) | yes | pane note | bar |
| Sequence JPEG Timestamps, Import | yes | pane note | left to the folder-dialogs plan (`../20261113-gpui-folder-dialogs/plan.md`); filed in `todo.md` |
| Undo of a trash run, Rewrite Sidecars from Index / Delete Sidecars runs | no callback | text or nothing | left as text; filed in `todo.md` |
| Folder listing, sidecar reconcile, dialog previews, thumbnail loads, the gate's wait, quit drains | no total | text or nothing | left as text |

User decisions (2026-11-14): Sequence and Import are wired in by the
folder-dialogs plan, not here; pass 2 shares the one bar with pass 1; one
window-wide bar slot; operations without a total stay as text this time.

## Steps

- [x] Step 1: The shared progress bar, fed by the scan and by Move to Trash / Delete Permanently
  - Done when:
    - A new pure module `crates/native/src/progress.rs` holds the fraction
      of a `(done, total)` (a port of `progress.ts`'s `progressWidth`: 0
      when `total` is 0, never above 1) and the choice of which running
      operation's progress the bar shows when several run (the trash run
      first, then a folder operation's run, then the scan), both with unit
      tests (zero total, done above total, each operation alone, each
      overlap, nothing running).
    - While pass 1 or pass 2 of the scan runs, and while a Move to Trash, a
      Delete Permanently or a trash redo runs, the window shows a thin
      determinate bar; it disappears when the operation ends, is canceled or
      is stopped by a re-listing (wherever `scan_status.clear()` and
      `trashing = None` happen today).
    - The bar is visible with any combination of panes hidden.
    - Showing or hiding the bar does not change the preview canvas's size
      (the `viewport` cell), so the fit scale, a 1:1 crop and the peaking
      layer are not recomputed by it.
    - The status line's `scanning d/t` / `analyzing d/t` and the pane's
      trash note are unchanged.
    - `docs/agents/native-app.md` has a section on the bar (where it is
      drawn, the no-layout-change rule, the priority among operations, that
      it uses plain GPUI elements and not gpui-component, how another
      operation such as Sequence or Import feeds it), `CLAUDE.md`'s layout
      names `src/progress.rs`, and `../20261109-gpui-port/plan.md`'s
      Progress gains a line for this plan (a native addition the milestones
      did not list).
    - The pending hand check (open a cold folder of 500+ files: the bar
      appears, fills through pass 1, then pass 2, and disappears; a Move to
      Trash of many files shows it with the right pane hidden) is recorded
      in this plan's `learnings.md`, done or pending.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/native/src/progress.rs` (new), `crates/native/src/main.rs`
      (`mod progress`, the state for the trash counts, the draw),
      `crates/native/src/scan.rs` (an accessor on `Status` for the running
      pass's `(done, total)`, next to `suffix`, with a test),
      `docs/agents/native-app.md`, `CLAUDE.md`,
      `docs/plans/20261109-gpui-port/plan.md`. Do not touch `crates/app`,
      `crates/backend`, `folderops.rs`, `sequence.rs`, `import.rs`,
      `sidecars.rs`, `datefolder` code or anything burst-related.
    - Draw with plain GPUI `div`s: a track and an indicator whose width is
      `relative(fraction)`. Do not use gpui-component's progress element:
      nothing calls `gpui_component::init` yet, and `native-app.md` requires
      the first adopter to check its key bindings against the keymap.
    - Colors: the Tauri `.progress` uses `--muted` for the track and
      `--primary` for the indicator; `main.rs` already has those values as
      `MUTED_FILL` and `PRIMARY_COLOR`. Height 2 px as `#scan-progress`.
    - Placement: one bar slot spanning the window on the top edge of the
      status row, absolutely positioned inside it (the row made `relative`),
      so that the row's height and the canvas above it never change. The
      status row stays when the strip is hidden, so the bar does too. Do not
      add or remove a flex row: that would resize the canvas on every scan
      start and end.
    - The choice function takes each operation's `Option<(usize, usize)>`,
      the folder-operation input included now (passed `None` here), so the
      folder-dialogs plan only passes a value in.
    - `Riffle.trashing` is an `Option<String>` today (the note's text). Keep
      the counts next to it or hold them and format the note with
      `trash::progress_status` where it is drawn; the note's text must not
      change. The late `TrashProgress` after the run came back must stay
      ignored as today.
    - The scan already notifies on every `Progress` / `FacesProgress`
      message and the trash on every `TrashProgress`; the bar adds no notify
      or timer of its own.
    - Pass 2 starts at `(0, 0)` and pass 1 can start with a total of 0: the
      fraction is 0 there, so the track shows empty. Note in `learnings.md`
      whether a scan with nothing to do flashes an empty track.

## Progress
