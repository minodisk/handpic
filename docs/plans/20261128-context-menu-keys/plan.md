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

# Right-click menus take the keyboard

## Purpose

In the GPUI app the right-click menus of the folder tree and the thumbnail
strip (gpui-component's `PopupMenu`, drawn by the view in a `deferred` /
`anchored` layer) do not take the keyboard. Seen on Windows 11 with a menu
open: Up / Down close the menu and reach the view (on the strip, Down ran
the burst move: `get_view`'s `current.position` went 7 to 8), and Enter right
after the right-click closes the menu and runs nothing. The other paths
behave as expected: `p` closes then picks, Right closes and pages the strip,
Escape closes, a press outside closes, another right-click replaces the menu,
and an item click runs and returns the keyboard to the view.

Decision (state it in the PR body): the behaviour follows
`docs/agents/app.md` (the key context table's `PopupMenu` row and "The
right-click menu is gpui-component's `PopupMenu` at the pointer") and
[the native-context-menu plan](../_archived/20261116-native-context-menu/plan.md).
Up / Down move the highlight, Enter runs the highlighted item, and the first
Down highlights the first item (gpui-component's default: `select_down` with
no selection picks the first clickable item). The paths that already work
keep working.

Once done, a menu opened by a right-click can be driven by the keyboard as
the docs already say, and a UI test pins it.

## Steps

- [x] Step 1: Keep the focus on the right-click menu after the press that opens it, with a regression test
  - Done when:
    - With a menu open on the strip and on the tree, `down` and `up` leave
      the menu open (`Handpic::context` still set, the menu's focus handle
      focused) and do not reach the view (the current file does not change
      on the strip), and `enter` after the Downs runs the highlighted item
      and closes the menu, with the focus back on the view's handle.
    - The paths that already worked still do, now through the menu's focus:
      `escape` closes only; a keymap key (`p`) closes and acts; `right`
      closes and pages the strip.
    - A UI test in `crates/app/src/uitest.rs` covers the above on both
      menus and fails without the fix (check this once by running it
      against the unfixed code; record the result in `learnings.md`).
    - `docs/agents/app.md` says what the code does (see the approach).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Suspected cause, read from source and not yet run: `open_context`
      (`crates/app/src/main.rs`) focuses the menu
      (`menu.focus_handle(cx).focus(window, cx)`) inside the right-press
      listener of the tree row / the strip's cells `div`. The view's root
      `div` has `.track_focus(&self.focus)`, and GPUI registers on every
      focus-tracking `div` a mouse-down listener (any button, bubble phase)
      that focuses its handle unless `window.default_prevented()`
      (`gpui-pre-0.3.12` `src/elements/div.rs`, "If this element can be
      focused, register a mouse down listener ..."). The root is an ancestor
      of both press targets, so its listener runs after theirs in the same
      dispatch and takes the focus back from the menu. With the focus on the
      view, the `PopupMenu` key context is not on the dispatch path, so
      every key reaches `Handpic::key_down`, whose "any key closes the
      right-click menu" arm closes it and acts. This also fits the untraced
      2026-11-22 note in `docs/agents/app.md` (opening the strip's menu did
      not end the `g` hold, and the key-up still reached the view).
    - Write the test first and confirm it fails; that is the check of the
      cause. If it passes without a fix, the cause is elsewhere (candidates:
      the focus set on a handle not drawn yet, another focusable ancestor
      such as `Root`, the test platform differing from Windows). Then stop,
      record it in `learnings.md` and update this plan before changing
      anything else.
    - Expected fix, one line: `window.prevent_default()` in `open_context`,
      next to the focus call, with a comment saying why. `open_context` is
      called only from the two right-press handlers. `main.rs` already uses
      `window.prevent_default()` in the menu buttons' handlers. Do not
      change `key_down`'s menu-closing arm: keys other than the menu's
      bindings still bubble from the menu to the root `div` and must close
      it and act.
    - After the fix, check that nothing else in the same press reads
      `default_prevented` and now misbehaves: the strip's selection and
      focus move on a right-click, the tree's keyboard state staying
      unchanged, and a second right-click replacing the menu (the old
      menu's `on_mouse_down_out` runs in the capture phase).
    - Test, in `uitest.rs`'s style (`Ui::open`, `open_folder`,
      `ui.cx.simulate_mouse_down(position, MouseButton::Right, ..)`,
      `simulate_keystrokes`, `ui.cx.read_from_clipboard()`):
      - Positions: add a `debug_selector` to the strip's cells `div` and to
        a tree row, as `settingsview.rs`'s `settings-box` and
        `folderops.rs` do, and read them with `ui.cx.debug_bounds(..)`; or
        compute the strip's position from its geometry. Decide during
        implementation; add no more selectors than the test uses.
      - The tree row: the tree may not hold the opened temp folder as a
        drawn row (an existing test pushes a `tree::Row` by hand for that).
        Measure whether a row is drawn in the test window before relying on
        it.
      - `PopupMenu::selected_index` is private, so assert the highlight
        through its effect: which item Enter runs. Use the copy items and
        the test clipboard. Tree order: the reveal item, `Copy Path`,
        `Copy Folder Name`, so `down down enter` copies the path and
        `down down down up enter` checks Up. Strip order: `Select All`, the
        reveal item, `Copy Path`, `Copy File Name`, so
        `down down down enter` copies the path. Do not let a test run the
        reveal item.
      - Also assert that after the right press the menu's focus handle is
        focused and the view's is not, and that `escape`, `p` and `right`
        behave as listed above.
    - Docs: in `docs/agents/app.md`'s "The right-click menu is
      gpui-component's `PopupMenu` at the pointer", add the rule that the
      open prevents the press's default so the root's focus-on-press does
      not take the focus back; replace its "Not verified headless" mention
      of the key routing with the UI test; and add the cause to the
      2026-11-22 `g` hold note only if the test confirms it. No other doc
      change unless something it says turns out wrong.
    - Files: `crates/app/src/main.rs`, `crates/app/src/uitest.rs`,
      `docs/agents/app.md`.

## Trade-offs and risks

- The cause is read from source, not measured. The test-first order in
  Step 1 is the guard. If the test platform does not reproduce it, the
  options are to ship the fix with the manual check as the only proof (no
  regression test), or to keep a test that passes either way, which pins
  nothing. Caller's decision if it comes to that.
- The manual check (the main agent, in the wrap-up, on this Windows 11
  host): launch with the screenshot feature, own `--index` / `--settings` /
  `--log` in a temp folder, `--mcp-port 47881`, `--import-settings` with
  `{"mcpEnabled": true}`, never the user's real index, settings or photo
  folders; check the window is foreground by PID before every synthetic
  input; follow `docs/agents/app.md`'s real-device run rules. On the tree:
  right-click, Down, Down, Enter runs `Copy Path` (the second item). On the
  strip the second item is the reveal item, which opens Explorer, so use
  Down, Down, Down, Enter for `Copy Path` (or Down, Enter for `Select All`).
  Check Up on both. Hovering an item also sets the highlight, so do not move
  the pointer after the right-click. Record the run in `results.md` in this
  folder and in the PR body.
- `todo.md`, heading "App: real-device checks of the right-click menus drawn
  as gpui-kit's PopupMenu (native-context-menu)": after the manual check,
  delete or rewrite only the last box (the line starting "On Windows, with a
  menu open") to reflect what was verified. Another session is rewriting the
  other boxes under that heading; leave them. Whether the line is deleted
  or narrowed to what the check did not cover (for example the press on the
  menu handing back the tree's keyboard) depends on the check's coverage.
- Once the menu really holds the focus, the view's `cx.on_blur` releases a
  held `g` (grayscale) when a menu opens, and key-up no longer reaches the
  view while it is open. `docs/agents/app.md` already states this as the
  rule; the 2026-11-22 observation was the bug's side effect. It is a small
  visible change.
- Fixing in `open_context` (one place) rather than in each press handler
  keeps the change to one line; a future caller of `open_context` outside a
  mouse dispatch would call `prevent_default` with no effect.

## Progress

- 2026-11-28: Step 1 done (open_context prevents the press's default, the two UI tests, docs/agents/app.md; manual check deferred, see learnings.md)
