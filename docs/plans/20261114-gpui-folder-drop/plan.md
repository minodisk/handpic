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

# GPUI folder drop

## Purpose

The Tauri app opens a folder dropped onto its window (or the folder of a
dropped file). The GPUI app (`crates/native`) has no such drop, so its port of
the empty state had to leave the "Drop a folder onto the window" clause out of
the no-folder text. This plan ports the drop itself, so the native app opens a
dropped folder the way a click in the tree does and its no-folder text can
match the Tauri `OPEN_TEXT` again.

## Steps

- [x] Step 1: Open a folder dropped onto the native window, as the Tauri app does
  - Done when:
    - Dropping one folder onto the native window opens it through the same
      `open(path, None, None, cx)` a click in the tree and `pick_folder` use
      (whole, without `--size` / `--max`, revealed in the tree).
    - Dropping one file opens its parent folder (any file, not only RAW, as
      the Tauri `dropped_dir` does; the file is not selected).
    - Dropping no path, more than one path, or a path that is neither a
      directory nor a file opens nothing and puts
      `Drop a single folder or RAW file.` on the status line (the Tauri text,
      verbatim).
    - While an OS file drag is over the window, the main area shows the Tauri
      drag outline (3px, the `--ring` color), drawn through
      `drag_over::<ExternalPaths>` and cleared when the drag leaves or drops.
    - The pure decision logic (which single path a drop names, and which
      folder that path resolves to) is in a module of its own with unit
      tests: one path / none / several, a directory taken as is, a file by
      its parent, a missing path refused (the cases of the Tauri tests
      `dropped_directory_is_taken_as_is_and_a_file_by_its_parent` and
      `a_files_parent_at_the_filesystem_root_is_the_root_not_none` in
      `crates/app/src/commands.rs`).
    - The no-folder text is settled against `origin/main` at implementation
      time: if the native empty-state port is merged (a native module holding
      the no-folder text, the port of `crates/app/ui/src/empty.ts`), its text
      is exactly the Tauri `OPEN_TEXT`,
      `Drop a folder onto the window, or click here to choose one.`, and its
      test is updated; if it is not merged, the text is not touched and the
      PR body says so, so that the empty-state PR (or a follow-up) restores
      the clause.
    - `docs/agents/native-app.md` describes the drop, and any "drag-and-drop
      not ported" record in it or in `../20261109-gpui-port/plan.md` reads as
      ported; where no such record exists yet, the port plan gets a Progress
      line for the drop.
    - `CLAUDE.md`'s `crates/native` layout names the new module in the
      style of its neighbours.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The drop is checked by hand in the running app on Windows (a folder, a
      file, two folders, a drop while a dialog is open, the outline), the
      results recorded in `learnings.md` and, as Measured / Not verified, in
      `docs/agents/native-app.md`.
  - Implementation approach (as far as it is known; omit if unknown):
    - Tauri reference: the `tauri://drag-drop` listener in
      `crates/app/ui/src/main.ts` (near `dropCounter`) and `dropped_dir` /
      `dropped_folder` in `crates/app/src/commands.rs`. Do not change
      `crates/app`; `crates/native` must not depend on it, so the small
      `dropped_dir` rule is ported into the native crate, not shared.
    - New pure module in `crates/native/src/` (for example `dropped.rs`;
      avoid the name `drop`), holding the status text constant, the "exactly
      one path" rule over `&[PathBuf]`, and the port of `dropped_dir`
      (`is_dir` gives the path, `is_file` gives its parent, else `None`).
      Add its `mod` line to `main.rs`.
    - GPUI API, from the pinned `gpui-pre` 0.3.12: `ExternalPaths` (a
      `SmallVec<[PathBuf; 2]>`, `.paths()`) is the drag value of an OS file
      drag; `.on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| ...))`
      on a `div` receives the drop. Put it on the root `div` of the window's
      `render` in `crates/native/src/main.rs`, so a drop anywhere in the
      window counts, as in Tauri.
    - The directory test is a `stat`, which Tauri runs off the UI thread
      (`spawn_blocking`). Do the same with `cx.background_executor()` inside
      a `cx.spawn`, then call `open` in `this.update`.
    - Ordering, as Tauri: drops are ordered among themselves by a counter in
      the view (the port of `dropCounter`); a drop whose resolve finishes
      after a later drop does nothing. Unlike `pick_folder`, a drop is not
      discarded because `opens` moved on meanwhile; Tauri mints the folder
      token only after the path has resolved.
    - `open` already defers under a running trash / delete
      (`gate.in_flight()`, `deferred_open`), so the drop needs no gate of its
      own.
    - GPUI fires the drop listener only when the element's hitbox is hovered,
      and the settings modal, the trash dialog and the folder dialogs are
      `occlude()`d layers. A drop over an open dialog is therefore expected
      not to reach the root `div` and is ignored (decided: consistent with a
      click in the tree being blocked by the same layers). Verify this in the
      running app and record it.
    - Drop feedback (decided: port it): Tauri outlines `#main` (3px,
      `--ring`) while a drag is over the window (`body.dragging #main` in
      `crates/app/ui/style.css`). Use
      `.drag_over::<ExternalPaths>(|style, _, _, _| ...)` on the element
      corresponding to `#main`, with the native theme's ring color; measure
      that it draws and clears on leave.
    - Platforms: only the Windows platform crate was read at planning time.
      macOS and Linux delivery of the file drop is left to the CI build and
      noted as not verified, as the neighbouring sections of
      `docs/agents/native-app.md` do.
    - Documentation: add a section next to
      "`open` shows the OS folder picker, a later open winning" in
      `docs/agents/native-app.md`, in that file's Rule / Not verified style,
      and adjust "A launch without a folder shows only the tree", which says
      a folder is opened from the tree or the picker. In
      `../20261109-gpui-port/plan.md`, at planning time no "not ported"
      record of the drop existed on main; re-check after rebasing, since the
      empty-state session adds one, and flip it if present, else add a
      Progress line.
    - `docs/humans/usage.md` already describes dropping a folder or a file;
      it documents the product, not the port, and needs no change.

## Trade-offs and risks

- **The empty-state port is a moving target.** At planning time
  (`origin/main` at `61b0ec3e`) it was not merged: no native empty-state
  module and no no-folder hint text in `crates/native`. If it merges before
  this PR, this PR restores the clause and flips the "not ported" records it
  added (expect a rebase conflict in `main.rs`, `native-app.md`, `CLAUDE.md`
  and the port plan). If this PR merges first, the other session should keep
  the Tauri text unchanged; the PR body must say so.
- **A drop while a dialog is open** is ignored (GPUI occlusion), where Tauri's
  drop is window-wide except under `formatGate`. Accepted, consistent with the
  tree being blocked by the same layers.
- **Where `dropped_dir` lives.** It is copied into `crates/native` rather
  than moved into `crates/backend` for both apps, because changing
  `crates/app` is out of scope. The cost is two copies of a nine-line rule.
- **The status text says "RAW file"** although any file's parent opens. It is
  kept verbatim for parity with Tauri.
- **A dropped file is not selected** after its folder opens; this matches
  Tauri, which passes only the folder to `openDirectory`.
- **macOS and Linux** drop delivery is not verified on this host.

## Progress
