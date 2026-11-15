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

# Native status row: notes only, no current-file info

## Purpose

The GPUI app (`crates/native`) writes
`{index+1}/{len}  {file name}  {w}x{h}  {orientation}  {shutter}  {aperture}  {iso}`
into its status row on every preview load (`status_line` in
`crates/native/src/main.rs`, assigned in `present`). The right-hand meta pane
already shows the position (`meta::position`), the file name, the shutter, the
aperture and the ISO; the preview pixel size and the raw orientation value are
debug leftovers. The Tauri app's status never names the current file: its
`note` is a transient slot that `present` clears with `setStatus()` on every
decoded preview (`crates/app/ui/src/main.ts`).

Once this is done the native status row shows only notes (errors, the drop
refusal, the empty-folder / filtered texts) and the suffixes `status()`
appends (`1:1`, `no index: …`, the scan's progress or `N failed`), as the
Tauri app does.

## Steps

- [x] Step 1: Stop writing the current file's info to the native status row
  - Done when:
    - After a preview load the status row shows no position, file name, pixel
      size, orientation, shutter, aperture or ISO.
    - A preview load clears the transient note (`self.status` becomes empty),
      so a message the old line used to overwrite (an `error: …` from `fail`,
      `DROP_TEXT`) does not stay on screen past the next presented preview.
    - `1:1`, `no index: …`, the scan suffix / `N failed`, `fail`'s
      `error: …`, `DROP_TEXT`, `no folder`, `no RAW files in …` and
      `FILTERED_TEXT` still appear as before.
    - `status_line`, `dash` and the `n/total` rewrite in `refilter` are gone,
      with nothing left unused (no new warnings under clippy).
    - `docs/agents/native-app.md` no longer describes a status line `n/total`
      or a status line set per file, and the obsolete `todo.md` item on the
      status line's `n/total` lagging a refilter is removed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - All code changes are in `crates/native/src/main.rs`:
      - `present` (about line 3605): replace
        `self.status = self.status_line(index, px, orientation, &meta);` with
        clearing `self.status` (an empty `SharedString`). This mirrors the
        Tauri `present`, which calls `setStatus()` with no argument. `present`
        is the one place both the current arm (`show_current`, also the
        prefetch ring hit) and the stale arm (`stale`) go through, so both
        keep clearing the note exactly where the old line overwrote it. `px`,
        `orientation` and `meta` stay destructured: `present` still uses them
        for `OnScreen`, `marks.show` and `Shown::Read`.
      - Delete `fn status_line` (about line 3649) and `fn dash` (about line
        5271); `dash` has no other caller. `Metadata` and `file_name` are
        still used elsewhere, so no import goes.
      - `refilter` (about lines 2417-2426): delete the block that rewrites the
        status line's `n/total` (the `split_once("  ")` parse). With no
        position in the status it would be dead code, and worse, it could
        rewrite a note that happens to start with `a/b`. Keep the
        `filter::shown_after` update of `self.shown` just above it.
      - `status()` (about line 3676): every suffix is pushed with two leading
        spaces, written for a non-empty base. With `self.status` now usually
        empty the row would start with two spaces (`  1:1`, `  scanning …`).
        Drop the leading separator when the base is empty (e.g. trim the
        start of the built string); keep the suffix texts themselves
        unchanged. Check `scan::Status::suffix` in `crates/native/src/scan.rs`
        for its own leading spaces and leave its tests as they are.
    - Leave alone: `fail` and its `println!("error …")`, the two `DROP_TEXT`
      assignments, the `no folder` initial value, the empty-listing
      assignments in the listing handler and in `refilter`, `step_note`,
      `notes`, and the meta pane.
    - Verify while implementing: the status row keeps its height with an
      empty text. `status_row` is a flex row with `items_center` whose
      `Sort ▾` / `Filter ▾` buttons should hold the height, but an empty text
      `div` contributes none, so run the app (or measure) and confirm the
      preview area above does not shift between an empty and a non-empty
      status; if it does, give the row or the text `div` the line's height.
    - Nothing in `--auto`, the bench lines or the unit tests reads the status
      text (checked: the `println!`s are `error …`, the `page` / zoom /
      overlay lines and the summary table), so no harness needs updating.
    - Doc updates in `docs/agents/native-app.md` (search `n/total` and
      `status line`):
      - about line 703: the re-listing "also refreshes the status line's
        `n/total`" clause no longer holds; remove it.
      - about line 714: the list of what indexes into `files` names "the
        status line"; remove that item.
      - about line 804: remove "The status line's `n/total` is left alone."
      - about lines 1292-1294: a stale frame now clears the status note
        rather than setting the status line, and `self.shown` is no longer
        needed for `refilter`'s `n/total` rewrite; keep the re-request and
        meta pane reasons.
      - about line 1352: in the `show_current` list, "status line" becomes the
        cleared note (or is dropped).
      - Add a short statement, where the status row is described (the section
        around line 862), that the status text is a transient note cleared on
        every presented preview, as the Tauri `setStatus()`, and that the
        current file's facts live in the meta pane only.
    - `todo.md`: remove the item `### Native: the status line's `n/total`
      still lags a refilter that hides the shown file` (about line 2650) with
      its TODO; the thing it tracks no longer exists.
    - `CLAUDE.md` needs no change: its mentions of the status line / row are
      about the scan `Status`, the menus, the progress bar and `DROP_TEXT`.
    - Commit as e.g. `fix(native): keep the current file's info out of the
      status row`.

## Trade-offs and risks

- Clearing on a stale frame too. `present` is shared by the current frame and
  a stale frame (another file drawn on the way to the current one), so
  clearing there wipes an `error: …` as soon as any frame is presented. This
  is exactly when the old line overwrote it, so behavior for notes is
  unchanged. The alternative, clearing only in `show_current`, would keep an
  error visible slightly longer during fast paging but is a behavior change
  not asked for.
- Empty-state texts are left as they are. `no folder`, `no RAW files in …`
  and `FILTERED_TEXT` still go to the status row although the preview area
  already centers the same messages (`empty.rs`). Removing them would also be
  duplication cleanup, but the request is only about the current file's info;
  it is left as separate work.
- Trash / undo results are not in the status row in the native app: they are
  `step_note`, drawn in the meta pane. This plan does not move them.
- Leading separator in `status()`. Trimming the leading two spaces is a small
  change; the suffix texts are untouched.
- Removing the `todo.md` item is the one edit outside `crates/native` and its
  agent doc. Leaving it would keep a TODO for a line that no longer exists.
- An error from a failed preview read now stands alone (`error: <name>: …`)
  with no later file line replacing it until the next preview is presented,
  as before; the preview area also shows `PREVIEW_FAILED_TEXT`.

## Progress
