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

# Title bar usable while a confirmation dialog is open

## Purpose

While a confirmation dialog is open in the GPUI app (the strip's
`Move to Trash…` / `Delete Permanently…`, a folder operation's dialog such as
`Delete Sidecars…`, the Cache page's `Clear the index cache?`), the title
bar's minimize / maximize / close buttons and its window drag do nothing.
Measured on 2026-11-27 (Windows 11, debug build with the `screenshot`
feature, synthesized mouse input): without a dialog, minimize, maximize and
the drag work; with the trash dialog or the `Delete Sidecars…` dialog open,
none of the three does anything and the bar is dimmed; with the settings
open, all three work.

An ordinary desktop app lets the user move or minimize its window while a
modal dialog is open. `todo.md`'s section "App: real-device checks of the
title bar (native-title-bar)" announces this fix as a follow-up. Once done,
the title bar (window controls and drag) works under every dialog, as it
already does under the settings layer, while the dialog's keys and the
"a press on the backdrop does nothing" rule stay as they are.

Cause, read from the sources (gpui-component 0.9.1, gpui-base 0.9.1,
gpui-pre 0.3.12), not from our backdrop alone:

- `Dialog::render` (gpui-component `src/dialog/dialog.rs`) wraps every dialog
  in a viewport-sized `div().id("dialog")` that is `occlude()`d. It is not
  deferred; `Root`'s `WindowStateLayers` paints it after the app's content,
  so its `BlockMouse` hitbox lies above every hitbox of the app's normal
  paint pass, the title bar's included.
- On Windows, `WM_NCHITTEST` asks GPUI's `on_hit_test_window_control`
  callback, which answers only for a `WindowControlArea` hitbox that is in
  `window.mouse_hit_test.ids`. `Frame::hit_test` walks the hitboxes from the
  top and stops at the first `BlockMouse` one, so under an open dialog the
  bar's `Drag` / `Min` / `Max` / `Close` hitboxes are never reached and the
  press is a plain client press. On macOS / Linux the bar's own mouse-down
  (`start_window_move`) and the Linux controls' clicks are blocked the same
  way.
- `modal::backdrop` inserts no hitbox (a `div` with a background and a
  `canvas`); it only dims the bar and prevents the focus change. Shrinking
  the backdrop alone therefore does not bring the bar back.

Decisions taken (approved by the user on 2026-11-28): the bar is always
deferred, undimmed (the backdrop starts below it), the gear refuses to open
the settings under any dialog, and the UI test uses a `#[cfg(test)]` probe.

## Steps

- [x] Step 1: Draw the title bar above the dialog layer and keep the backdrop off it
  - Done when:
    - With a trash dialog, a folder operation's dialog or the cache
      confirmation open, the title bar's hitboxes are above the dialog's
      occluding layer: a press on the bar reaches the bar, and the bar is
      not dimmed.
    - A press on the backdrop below the bar, and a press on the bar, both
      leave the dialog open and the focus on the dialog's focused button;
      Tab / Shift+Tab, Escape and Enter in the dialog behave as before (the
      existing dialog tests still pass unchanged).
    - The gear in the bar opens nothing while any dialog is open (see the
      `open_settings` point below).
    - The bar keeps its place in the layout: `TITLE_BAR_HEIGHT` high, the
      main area starting right below it (the existing UI test
      `the_settings_layer_lies_under_the_title_bar_and_fills_the_rest`
      still passes).
    - A UI test in `crates/app/src/uitest.rs` pins the layering (see the
      approach), or `learnings.md` records why GPUI's test context could
      not express it.
    - `docs/agents/app.md`, `CLAUDE.md`, `todo.md` and `modal.rs`'s doc
      comments describe the new rule (see the approach).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Manual check (not a gate of the PR; the main agent runs it in the
    wrap-up phase on the Windows real device, following
    `docs/agents/app.md`'s "Driving the window in a real-device run on
    Windows"):
    - Build with `mise run app:dev:screenshot` (cargo with `-j 2`), run with
      its own `--index`, `--settings` and `--log` and `--mcp-port 47880`,
      on photos copied from `D:\Photos\tests\20261127-c5\step7\photos` into
      a folder of its own. Send synthesized input only while holding an
      exclusive lock on `D:\Photos\tests\.ui-lock`.
    - With the trash dialog open (strip right-click >
      `Move 3 Files to Trash…`) and again with the `Delete Sidecars…` dialog
      open: minimize gives `IsIconic`, maximize gives `IsZoomed`, a drag on
      the bar moves the window, and close closes the app (run close last,
      on a dialog that is asking, not running). A screenshot shows the bar
      undimmed and the rest of the window dimmed.
    - After each of the bar presses that keep the window, the dialog is
      still open and Enter / Escape still act on it (the focus did not
      leave the dialog's button). A press on the dimmed area below the bar
      still does nothing.
    - Record the result in `results.md` and `learnings.md` and update
      `todo.md` and the Measured / Inferred labels in `docs/agents/app.md`
      accordingly.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/main.rs` (`impl Render for Handpic`, the title
      bar's wrapper `div` and the `modal::backdrop(cx)` child;
      `open_settings`), `crates/app/src/modal.rs` (`backdrop` and the module
      and function doc comments), `crates/app/src/uitest.rs`,
      `docs/agents/app.md`, `CLAUDE.md`, `todo.md`.
    - The bar: wrap the whole title-bar wrapper (the
      `div().relative().flex_shrink_0()` holding `TitleBar::new()` and the
      folder-name overlay) in `gpui_kit::deferred(..)`. A deferred draw is
      prepainted after the whole normal pass, so its hitboxes come after the
      dialog's `occlude()`d `div` and the hit test reaches them first.
      `Deferred::request_layout` returns the child's layout, so the bar
      stays in flow. Wrap the wrapper as one unit: the folder-name overlay
      left outside would be painted under the bar's background.
    - Defer it always, not only while a dialog is open, so the element tree
      and `TitleBar`'s `window.use_state` do not change when a dialog opens.
    - Priority: keep the bar's deferred priority below 10. The dialog's host
      is `deferred(..).with_priority(10 + layer)` (gpui-base
      `src/dialog.rs`), the popups are at `POPUP_PRIORITY` 100 and the
      tooltips at 200; a dialog surface that reaches the bar in a short
      window must still paint over it. Deferred draws of one priority paint
      in push order, and the bar is the root's first child.
    - The backdrop: start it below the bar (`top(TITLE_BAR_HEIGHT)` with the
      other three edges at 0, as the settings layer's `absolute` `div`
      does), so the bar is not dimmed whatever its priority and whatever
      opacity a shipped theme gives `title_bar`. Keep its `canvas`'s
      capture-phase `prevent_default` listener as it is: `on_mouse_event`
      is window-wide, not bounded by the element, so it also keeps a press
      on the bar from moving the focus off the dialog's button (the
      dialog's host tracks its own focus handle and would take it). It does
      not stop propagation, so the Windows non-client press still starts
      the OS drag.
    - `open_settings`: the gear becomes pressable under a dialog. It refuses
      on `trash.busy()`, `folderops.dialog_open()` and `folderops.busy()`,
      but the backdrop is also drawn on `clear.asking()`, which is not among
      the refusals today. Check whether the cache confirmation can be open
      while the settings are closed (an MCP `clear_cache` without
      `mcpAllowDestructive` goes through `press_clear`); if so, add the
      refusal so the gear cannot open the settings under that dialog, and
      cover it in the test.
    - Verify during implementation: the bar's height and the main area's
      offset are unchanged; the `Sort ▾` / `Filter ▾` popovers and the
      right-click menu still stop under the bar; the gear's tooltip and
      click, and a click on the folder name falling through to the drag
      row, still work with no dialog open; the drop outline and a drop on
      the bar still reach the root `div`'s `on_drop`.
    - UI test: GPUI's test platform does not expose the window-control hit
      test (`TestWindow` stores `hit_test_window_control_callback` but
      nothing calls it), its `start_window_move`, `minimize` and `zoom` are
      `unimplemented!()` (so a simulated press-and-move on the bar would
      panic once the bar is reachable; use `simulate_click`, a press and
      release without a move, as the existing settings test does), and
      `Frame::hit_test` is crate-private. Pin the layering through a
      test-only probe: a left mouse-down listener on the bar's wrapper
      compiled under `#[cfg(test)]`, bumping a counter on the view. The
      test: open a folder, `confirm_trash` on one file, let the dialog open
      (`run_until_parked`), `simulate_click` on the bar: the press reached
      the bar, `trash.busy()` still holds and the run button still has the
      focus; `simulate_click` on a point below the bar outside the dialog's
      box: the bar saw nothing and the focus did not move. It must fail on
      the code before the change (the dialog's `occlude()` hides the bar).
    - `docs/agents/app.md`: in the title bar section, add the rule (the bar
      is a `deferred` draw under priority 10 so it lies above gpui-component's
      occluding `dialog` `div`, why shrinking the backdrop is not enough,
      and the `open_settings` consequence), and drop "the controls and drag
      while a dialog's backdrop covers the bar" from its Unmeasured bullet;
      in the trash dialog section's Shell bullet, correct "the component's
      full-window `dialog` `div` occludes the window, so nothing under it is
      pressed" to except the bar and say the backdrop starts below it; in
      the settings section's pointer rule, the dialogs' backdrop no longer
      covers the bar. Label what is read from source as Inferred until the
      wrap-up's real-device check.
    - `CLAUDE.md`: the `src/modal.rs` description says "`backdrop`, the
      dimmed window the view draws under it"; make it say the backdrop
      leaves the title bar out, and note in the title bar description that
      the bar is drawn above the dialog layer.
    - `todo.md`: in "App: real-device checks of the title bar
      (native-title-bar)", narrow the Windows item "while the trash dialog,
      a folder operation's dialog or the settings modal is open …" to what
      is still unchecked after this plan, and add an item to verify on macOS
      and on Linux that, with a dialog open, the window controls and the
      bar's drag work, the bar is not dimmed, and a press on the bar or the
      backdrop leaves the focus on the dialog's button. Touch no other
      `todo.md` item.

## Trade-offs and risks

- Backdrop-only fix (not taken): making `modal::backdrop` leave out the
  bar's height does not restore the controls, because the blocker is
  gpui-component's own `occlude()`d `dialog` `div`, which the app cannot
  resize (`.overlay(false)` removes only the component's backdrop child).
  The bar has to be drawn above that layer; the backdrop change is only
  what keeps the bar undimmed.
- Deferring the bar always versus only while a dialog is open: always is
  planned (stable element tree, one code path). The cost is that the bar is
  now painted after the whole normal pass in every frame, above anything of
  the main layout that might overlap it. Nothing is known to overlap it
  (the menus are capped under the bar, the settings layer starts below it),
  but this is read from the code, not seen running.
- The UI test's probe: a `#[cfg(test)]` mouse-down listener on the bar's
  wrapper puts test-only code in `render` and gives the wrapper a hitbox in
  test builds only.
- Close while a dialog is open: the close button becomes reachable while a
  trash run or a folder operation is running. Alt+F4 and the taskbar's
  close already reach the same close path, so no new path is expected
  (Inferred). The real-device check presses close only on an asking dialog.
- macOS and Linux are not verified by this plan beyond CI's build and
  tests; `todo.md` gets the item. On macOS the traffic lights are OS views
  and were likely never blocked; the drag was.

## Progress
