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

# Native title bar: smaller text and the open folder's name

## Purpose

The native app's title bar draws "Handpic" at the window's inherited 16 px,
larger than any other text in the window (the panes, menus and status row use
11 to 14.4 px). The user asked for it to be a little smaller, and for the bar
to show the name of the open folder in its center, so the window says which
folder is being culled without looking at the tree.

## Decisions (agreed with the user)

- Size: `text_sm` (14 px) for "Handpic" and the folder name.
- Centering: across the whole window width (an overlay over `TitleBar`), not
  only within `TitleBar`'s inner row.
- A folder without a last component (a drive root such as `C:\`, or `/`):
  show the path itself, through `context::shown_path` (no verbatim prefix).

## Steps

- [x] Step 1: Draw the title text smaller and the open folder's name centered in the title bar
  - Done when:
    - "Handpic" next to the app icon renders at `text_sm` (14 px) instead of
      today's inherited 16 px.
    - While a folder is open, its name (the last component of its path, or
      the shown path for a root without one) is drawn centered across the
      whole title bar at the same size as "Handpic"; with no folder open,
      nothing is drawn there.
    - The name follows the open folder: another open, a folder rename and a
      `Rename by Capture Date…` that moves the open folder change it.
    - A long name is truncated with an ellipsis instead of running over the
      icon and "Handpic" or the gear.
    - The app icon (16 px), the gear button and the bar's height are
      unchanged, and a drag on the bar, the name included, still moves the
      window and the window controls still take their clicks.
    - `docs/agents/native-app.md`'s title bar section and the title bar
      sentence of `CLAUDE.md`'s Layout describe the text size and the folder
      name.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Pending manual check by the user (the only way to confirm the look):
      the size of "Handpic", the name's centering and truncation in a narrow
      window, and the bar's drag over the name, on Windows.
  - Implementation approach:
    - The bar is built in `crates/native/src/main.rs`, in
      `impl Render for Riffle`: `TitleBar::new()` with an
      `h_flex().gap_2().text_color(cx.theme().foreground)` of
      `img(icons::APP).size(px(16.))` and `"Handpic"`, then
      `self.settings_button(cx)`. The text sets no size and inherits the
      window rem, 16 px (`theme.rs`'s `FONT_SIZE`). Do not change
      `FONT_SIZE`: every rem-based size in the app hangs off it.
    - Size: put `text_sm` on the `h_flex` (or on the text alone); the icon
      keeps its explicit 16 px either way.
    - The folder is `Riffle::dir: Option<String>`, the canonical path, read
      in `render`, so no new state and no notify are needed. Take the last
      component with `Path::file_name`; when there is none, show the path
      through `context::shown_path`. On Windows the canonical path may carry
      the verbatim prefix; the last component is unaffected.
    - Centering: gpui-component 0.9.1's `TitleBar` puts its children in an
      inner `justify_between` row that starts after the left padding (12 px,
      80 px on macOS) and ends before the window controls on Windows /
      Linux, so a `TitleBar` child cannot be centered across the whole bar.
      Wrap the bar: a `div().relative().flex_shrink_0()` holding `TitleBar`
      and, after it, an
      `absolute().inset_0().flex().items_center().justify_center()` `div`
      with the name, in `cx.theme().foreground`. Keep that overlay free of
      listeners, ids, cursors and hover styles: `gpui-pre` 0.3.12's `div`
      inserts a hitbox only for those (`should_insert_hitbox`), which is
      what lets the press reach the bar's `WindowControlArea::Drag` row and
      the controls beneath. Verify during implementation that a drag
      starting on the name moves the window and that the controls and the
      gear still click; if it does not hold, fall back to a third
      `TitleBar` child and record it in `learnings.md`.
    - `TitleBar` already wraps itself in a `flex_shrink_0` `div`; check the
      wrapper does not change the bar's height (`TITLE_BAR_HEIGHT`, which
      the menus' height cap subtracts).
    - Truncation: give the name a maximum width and `truncate()` so it
      cannot reach the left group or the gear; the cap (a fraction of the
      bar, or the bar less a fixed margin a side) is decided while looking
      at a narrow window.
    - Keep the existing rules of the title bar section in
      `docs/agents/native-app.md`: no `track_focus` on the bar or its
      children, `title: Some("Handpic")` in the window options unchanged
      (the OS title, taskbar and Alt+Tab text stay "Handpic").
    - Not in scope: the gear, the OS window title, the Tauri app (frozen),
      the MCP `get_view` answer.

## Trade-offs and risks

- The overlay draws outside `TitleBar` and relies on a listener-free `div`
  having no hitbox (read from the `gpui-pre` source, not seen running); the
  fallback is a third `TitleBar` child, centered only between the left group
  and the gear.
- Narrow windows. A centered name overlaps the left group or the gear unless
  capped; with the cap, a very narrow window shows only a few characters and
  an ellipsis. The name is not shifted off-center to gain room.
- macOS and Linux are not checked by hand in this step: the overlay ignores
  the macOS 80 px traffic-light padding by design (it centers on the
  window), and under Linux client-side decorations `Root`'s window border
  pads the content, so the wrapper centers within the padded area.

## Progress
