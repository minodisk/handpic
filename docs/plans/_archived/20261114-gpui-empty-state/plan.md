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

# Port the empty state to the GPUI app

## Purpose

The Tauri app says in the preview area why it shows no photo: no folder is
open (with a clickable hint that opens the folder picker and names the `open`
key), the folder has no RAW or JPEG files, the filter hides every file, or the
current file's preview could not be shown (`crates/app/ui/src/empty.ts`,
drawn by `renderEmpty` in `main.ts`). The GPUI app (`crates/native`) shows a
black preview area in all four cases and says why only on the status line
(`no folder`, `no RAW files in …`, `FILTERED_TEXT`, `error: …`).

Once this is done the native window shows the same four messages in its
preview area, and the no-folder message opens the folder picker on a click.
The native app has the `open` action and the picker but no folder
drag-and-drop, so its no-folder text says only what works: it drops the Tauri
text's "Drop a folder onto the window" clause. Folder drag-and-drop is being
ported in a separate change, which restores the clause once it lands; this
plan records it in the port plan as not ported yet.

## Steps

- [x] Step 1: The empty state's pure module and its message over the preview canvas
  - Done when:
    - `crates/native/src/empty.rs` exists as a pure module (no GPUI types)
      with:
      - the state (`none`, `no-folder`, `no-files`, `filtered`,
        `preview-failed`) and its judgment in `empty.ts`'s order: no folder,
        then no listed file, then no shown file, then a failed preview, else
        none;
      - `NO_FILES_TEXT` and `PREVIEW_FAILED_TEXT` with `empty.ts`'s wording,
        and the filtered state's text being the existing `FILTERED_TEXT`;
      - the open hint: `Click here to choose a folder.` (or wording to that
        effect, without any drop clause), followed by ` Or press <keys>.`
        with the `open` action's keys joined by ` or ` and shown the way the
        keymap shows a key (`space` as `Space`), the key clause left out when
        `open` has no key or is absent from the bindings.
    - Its unit tests port every case of `empty.test.ts`: the five
      `emptyState` cases and "the other states win over a failed preview",
      and the five `openHint` cases (one key, several keys, a rebound key
      shown as `Space`, no key bound, no `open` binding at all), with the
      expected strings in the native wording.
    - The native window shows the state's message centered in the preview
      area, in the muted color, for each of the four states, and nothing for
      `none`.
    - A left click on the no-folder message opens the OS folder picker
      through `pick_folder`; a click in any other state does nothing.
    - On a failed preview of the current file the previous file's image is
      not on screen, the message is shown, and the area stays without an
      image until a later preview succeeds; the message goes away as soon as
      another file becomes current, and is not shown while the 1:1 view is on.
    - The existing uses of `filter::FILTERED_TEXT` (the status line in the
      open path and in `refilter`, and the meta pane) behave as before.
    - `docs/agents/native-app.md` has a section on the empty state, and the
      "A launch without a folder shows only the tree" section no longer says
      the preview is empty without the hint.
    - `docs/plans/20261109-gpui-port/plan.md` records the port (a note under
      the step it fits and a Progress entry) and notes that folder
      drag-and-drop onto the window is not ported yet (a separate change is
      porting it) and that the no-folder text therefore omits the drop clause
      until then.
    - `CLAUDE.md`'s `crates/native` description lists `src/empty.rs` in the
      style of the other modules (what it holds, which `.ts` it ports).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Tauri reference: `crates/app/ui/src/empty.ts`, `empty.test.ts`,
      `renderEmpty` and the `emptyEl` click listener in
      `crates/app/ui/src/main.ts`, and `#empty` / `.empty` in
      `crates/app/ui/style.css` (absolute over the viewer, centered, muted,
      0.9rem, 1rem padding; pointer cursor only for no-folder; no pointer
      events for preview-failed). `crates/app` is not changed.
    - Files: new `crates/native/src/empty.rs`; `crates/native/src/main.rs`
      (`mod empty;` and the preview canvas in `render`); `CLAUDE.md`;
      `docs/agents/native-app.md`; `docs/plans/20261109-gpui-port/plan.md`.
    - The clear on failure already exists and is to be confirmed, not
      rewritten: `loaded`'s `Err` arm for the current file calls
      `clear_image()` and sets `self.shown` to `Shown::Failed` with the
      file's index; `pager::Finished::Stale` never draws a failure; a failed
      crop releases an earlier page's held crop in `zoom::arrived`. Read
      those paths and the prefetch ring's hit path (`show_held`) to confirm
      nothing puts an older image back after a failure; fix only what the
      reading finds.
    - The judgment's inputs from the view: whether `self.dir` is set,
      `self.all.len()`, `self.files.len()`, and "the current file's preview
      failed", which is `self.shown` being `Shown::Failed` at
      `Some(self.pager.current())` and `!self.zoom.is_on()` (Tauri's
      `previewFailedSeq === seq && !comparing && !zoomed`; the native app has
      no compare view). Because `shown`'s index stops matching once the pager
      moves, the message disappears on a page turn without extra state, as
      Tauri's `seq` comparison does. Check that `refilter`'s
      `filter::shown_after` keeps this true across a refilter or re-listing.
    - `self.dir` is set when a listing arrives, the same moment Tauri sets
      `openDir`, so the hint stays up while the first open's listing runs,
      as in Tauri. If that is visibly wrong on a launch that restores
      `lastFolder`, record it in `learnings.md` and report it rather than
      adding an input `emptyState` does not have.
    - The hint's keys come from `self.settings.keymap.bindings()`
      (`riffle_backend::shortcuts::Binding { action, keys }`), which is
      replaced live when the Keyboard Shortcuts tab edits the keymap, so the
      hint follows a rebinding without more wiring. Show a key through the
      existing `settingsmodal::display_key`; do not add a third copy of that
      rule. The tests pass bindings in, so they do not depend on the
      platform's default (`ctrl+o` / `meta+o`).
    - `FILTERED_TEXT` stays in `filter.rs`; `empty.rs` uses it for the
      filtered state. No existing use changes.
    - Drawing: a `canvas` takes no input (`native-app.md`, "A `canvas` gets
      no input; its wrapping `div` does"), so wrap the preview canvas in a
      `relative` `div` that takes the canvas's `flex_1().h_full()` (plus
      `min_w_0` if the layout needs it), make the canvas `size_full()`, and
      add the message as an `absolute`, full-size, centered child with
      padding, `MUTED_COLOR` and a text size near the pane's. Only the
      no-folder message gets an `id`, `cursor_pointer()` and the left-click
      listener calling `pick_folder`; the other states register no listener.
      The canvas's `bounds` feed `self.viewport`, the fit, the 1:1 crop and
      the perf band, so confirm the wrapper leaves the canvas's bounds
      unchanged (the preview still fits and 1:1 still centers).
    - The settings modal, the trash dialog and the folder dialogs are
      deferred, occluding layers; confirm a click on them does not reach the
      no-folder message underneath. The context popover and the sort /
      filter menus are deferred too.
    - Leave the status line's `no folder` and `no RAW files in <dir>` texts
      and the meta pane's `FILTERED_TEXT` line as they are.
    - The message layer and the click cannot be unit-tested (the crate does
      not enable GPUI's `test-support`). If the session can run the native
      app, check by hand: a launch without a folder (hint, click opens the
      picker, the key clause follows a rebinding and disappears when `open`
      is unbound), a folder without RAW or JPEG files, a filter hiding every
      file, and a file whose preview fails
      (`D:\Photos\samples\NEF\NIKON_D70_Nikon.nef` after a normal NEF, paging
      on and back, and `z` on it). Otherwise write what was not verified in
      the running app in the `native-app.md` section and in `learnings.md`,
      as the `open` section does.
    - `todo.md`'s "real-device check of the preview-failed state" item is
      the Tauri app's check and is not touched or closed by this step.
    - Commit as Conventional Commits, e.g.
      `feat(native): show the empty state in the preview area`.

## Trade-offs and risks

- `FILTERED_TEXT` stays in `filter.rs` and `empty.rs` reads it, so the three
  existing uses do not change. The alternative, moving it into `empty.rs`
  next to the other two texts as in `empty.ts` and re-exporting it from
  `filter.rs`, mirrors Tauri more closely at the cost of touching
  `filter.rs`. Either satisfies the criteria.
- The no-folder hint shows while the first open's listing is under way
  (`dir` is set when the listing arrives), as in Tauri. Hiding it then would
  need an "open under way" input the Tauri judgment does not have; not done.
- While the 1:1 view is on, the preview-failed message is hidden, as in
  Tauri. If the preview and the crop both fail, the canvas is blank and only
  the status line and the meta pane carry the error.
- Folder drag-and-drop is ported by a separate change; until it lands the
  no-folder text omits the drop clause, and that change restores it.
- The hand check of the message layer may not be possible in the
  implementing session; then it is left recorded as not verified.
- The status line keeps `no RAW files in <dir>` for a folder with no listed
  file, next to the preview area's "This folder has no RAW or JPEG files.".
  Aligning the two is outside this plan.

## Progress

- (2026-11-14) Step 1 complete
