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

# Settings layer under the title bar

## Purpose

The settings (`ctrl+,`, `meta+,` on macOS, or the title bar's gear) are drawn
as an opaque, occluding layer over the whole window, gpui-component's
`TitleBar` included. On macOS the OS traffic lights, which sit at (9, 9) in
the client area, then overlap the layer's "Settings" heading, and on every
platform the title bar can no longer be pressed, so the window cannot be
dragged (nor, on Windows / Linux, its window controls reached) while the
settings are open.

Once done, the settings layer starts below the title bar on every platform:
the bar (app icon, "Handpic", the folder's name, the gear, the window drag,
the traffic lights / window controls) stays drawn and usable while the
settings are open, and the layer stays opaque and occluding over everything
below it.

## Steps

- [x] Step 1: Start the settings layer below the title bar
  - Done when:
    - With the settings open, the layer's box starts `TITLE_BAR_HEIGHT`
      below the top of the view's root and reaches its left, right and
      bottom edges; the title bar above it is drawn and takes presses (the
      window drag and the window controls), and the macOS traffic lights no
      longer overlap the "Settings" heading.
    - The layer is still opaque and still occludes the whole main layout
      below the title bar (the tree, the preview, the strip, the status
      row).
    - A press on the gear while the settings are open does nothing harmful
      (see the approach: it is already a no-op).
    - A UI test covers the layer's bounds.
    - `CLAUDE.md`, `docs/agents/app.md` and the code comments that call the
      layer full-window say it lies under the title bar.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - The change is in `crates/app/src/main.rs`, at the end of
      `impl Render for Handpic` (about lines 6524-6536): the wrapper around
      `self.settings_view.clone()` is
      `div().absolute().top_0().left_0().size_full()`. Give it
      `top(TITLE_BAR_HEIGHT)` with `left_0()`, `right_0()` and `bottom_0()`
      and drop `size_full()` (a full-height box moved down by the bar's
      height would overflow the window's bottom). `TITLE_BAR_HEIGHT` (34 px,
      gpui-component 0.9.1 `src/title_bar.rs`) is already imported in
      `main.rs`. Update the comment above it.
    - This is right on every platform because `TitleBar` is the first
      in-flow child of the root's `flex_col`, a `flex_shrink_0` row of
      `TITLE_BAR_HEIGHT` (see `docs/agents/app.md`, "The title bar is
      gpui-component's `TitleBar` ..."), and the outer `div().relative()`
      that holds both `main` and the layer has the same origin as `main`,
      inside `Root`'s `window_border()`, so the Linux client-side frame's
      padding is already outside it.
    - `settingsview.rs`'s `layer()` keeps `size_full()`, `occlude()` and
      `bg(cx.theme().popover)`: it fills the wrapper, so no change of logic
      is needed there, only its doc comment ("The box, filling the window
      ...") and the module header ("drawn by the main view over the whole
      window").
    - The gear while open: `settings_button`'s click calls
      `Handpic::open_settings`, whose first guard returns when
      `self.settings_view.read(cx).is_open()`. So the press is a no-op and
      no code change is needed. The gear's wrapper stops the press from
      reaching the bar's drag, as today. The settings box's
      `capture_any_mouse_down` (which cancels a shortcut capture) is on the
      box and so no longer sees a press on the title bar; check by hand
      whether a key capture under way surviving a title bar press / window
      drag reads sensibly, and leave it if so.
    - UI test, in `crates/app/src/uitest.rs` next to
      `the_settings_modal_drops_the_paging_keys_until_escape_closes_it`:
      give the box in `layer()` a
      `.debug_selector(|| "settings-box".to_owned())` (a no-op outside
      debug `test-support` builds; `folderops.rs` already uses it in
      non-test code with `folder-dialog-line` etc., and reads it back with
      `ui.cx.debug_bounds(selector)`), open the settings with the key, and
      assert that the bounds' top is `TITLE_BAR_HEIGHT` below the view's
      top and that its bottom / right reach the viewport's
      (`window.viewport_size()`, less `window_paddings` if the test
      platform reports any; verify during implementation that the test
      window has none). Per the rule in `docs/agents/app.md`, non-test code
      must not use a test-only GPUI API; `debug_selector` is not one.
    - Optional in the same test only if cheap: a `simulate_click` at a
      point inside the title bar with the settings open leaves them open
      and does not panic. Do not build new test infrastructure for the
      window drag itself; verify the drag, the traffic lights and the
      heading by hand on macOS (`mise run app:dev`), and say in the PR that
      Windows / Linux window controls were not checked on a device if they
      were not.
    - Docs to reword (all in the same PR):
      - `CLAUDE.md`, the `src/settingsview.rs` sentence: "as a
        full-window, opaque, occluding layer the window draws beside its
        own root `div` over the still-painted main layout" becomes a layer
        under the title bar covering the rest of the window.
      - `docs/agents/app.md`: "`Root` wraps the window's content" ("the
        settings are a full-window layer"); "The settings modal is its own
        entity, opened on `ctrl+,`" ("It is shown over the whole window",
        "in an `absolute` full-size `div`"; the closes rule's "A
        full-window view has no outside to press" and "`ctrl+,` while
        shown does nothing", to which the gear and the title bar being
        outside the layer should be added; the pointer rule's "blocks the
        hitboxes painted before it, the whole main layout", which now
        excludes the title bar). Record there what was verified and with
        which tag (Measured / Inferred), as that file does.
      - `docs/humans/usage.md` only says the settings open "as a window of
        pages"; it does not claim the whole window, so it and
        `usage.ja.md` need no change unless the wording is touched (then
        both, in sync).
    - Commit / PR title in Conventional Commits, e.g.
      `fix(app): keep the title bar above the settings layer`.

## Trade-offs and risks

- The gear while the settings are open. Option A (taken, minimal, approved by
  the user): leave it a no-op, as `open_settings` already is while open.
  Option B: make the gear toggle (close the settings when open), a behavior
  change beyond the request.
- A file drop on the title bar while the settings are open. The root
  `div`'s `on_drop::<ExternalPaths>` is hit-tested through the root's
  hitbox, which the layer's `occlude()` blocks today over the whole window.
  With the title bar exposed, a drop onto the 34 px bar probably reaches
  `drop_paths` and opens the folder under the open settings (not measured).
  The user chose not to guard it; check the behavior during implementation
  and record it in `docs/agents/app.md`.
- Tooltips and hover over the title bar now work while the settings are
  open (the gear's tooltip). Deferred layers the main view draws (a
  right-click menu, the modal backdrop) are unchanged.
- The UI test proves the layer's bounds, not the OS-level window drag or
  the traffic lights, which the test platform does not have; those are
  checked by hand on macOS only.

## Progress

(2026-11-25) Step 1 complete
