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

# Wrap the preview's empty-state text within the preview

## Purpose

When the preview area is narrow, the message centered over it (`empty.rs`'s
`text`: the no-folder `open_hint`, `NO_FILES_TEXT`, `FILTERED_TEXT`,
`PREVIEW_FAILED_TEXT`, `ZOOM_FAILED_TEXT`) is drawn on one line. It spills
over the folder tree on the left and is hidden under the meta pane on the
right. The preview's minimum width is 240 px (`panewidths.rs`'s
`PREVIEW_MIN`), so this happens in normal use, and text drawn over a
neighbouring pane reads as that pane's content.

Evidence (2026-11-27, Windows 11, debug build with the `screenshot`
feature): `panelWidths` `{"left":480,"right":480}`, window width 1200, no
folder. The preview is 240 px (x 480 to 720). The text was one line starting
at x=360 over the left pane and hidden past x=720 under the meta pane. A
click at x=489 or x=711 (just inside the 5 px bands) opened the picker; one
at x=486 or x=716 (on a band) did not; one on the spilled text at x=420 did
not.

Once done, the whole message is readable at every preview width and never
overlaps an adjacent pane.

Decisions already made (record them and their reasons in the PR body):

- The text wraps within the preview canvas's width less a 16 px margin on
  each side and stays center-aligned. The priority is that the whole text
  is readable and never overlaps an adjacent pane.
- The no-folder message's click target does not overlap the 5 px drag bands
  of the pane borders (`pane_handle`).
- Reason: `PREVIEW_MIN` is 240 px, so widths where the text does not fit on
  one line happen in normal use, and text over a neighbouring pane is
  confused with that pane's content.
- The click target stays where it is today, the full-size container over the
  preview (option A below): the request allowed the target to stay as it is,
  and the bands already occlude it.

## Steps

- [x] Step 1: Wrap the empty-state text inside the preview, with a UI test
      and the rule in `docs/agents/app.md`
  - Done when:
    - With `panelWidths` `{"left":480,"right":480}`, a 1200 px wide window
      and no folder, a screenshot shows the no-folder text wrapped onto
      several centered lines, all inside the preview (x 480 to 720) with
      the margin, nothing drawn over the tree or under the meta pane. Take
      it with the MCP companion's `screenshot` from an app launched with
      `mise run app:dev:screenshot`, with its own `--index`, `--settings`
      and `--log`, `--mcp-port 47890`, and the settings (`mcpEnabled`,
      `panelWidths`, `windowBounds`) seeded through `--import-settings`.
    - In a wide window the same text is still one line, as today
      (screenshot).
    - A synthetic click on the wrapped text opens the folder picker, done
      while holding the exclusive lock `D:\Photos\tests\.ui-lock`, with the
      relative `mouse_event` moves before the press that
      `docs/agents/app.md`'s "Driving the window in a real-device run on
      Windows" calls for. A click on a 5 px band still opens nothing.
    - A UI test in `crates/app/src/uitest.rs` fails without the fix and
      passes with it (see the approach).
    - `docs/agents/app.md`'s "The preview area says why it shows no photo"
      section records the rule, the cause and what was seen, and the "Not
      yet seen: the bands against the no-folder message in a narrow window"
      entry in the pane-width section is updated to what this run saw.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The PR body records the decisions above with their reasons.
  - Implementation approach (as far as it is known; omit if unknown):
    - The change is in `empty_message` in `crates/app/src/main.rs`. Today
      the string is a direct child of the `flex` / `justify_center`
      container. GPUI's text element takes a wrap width only when its width
      is known or the available width is definite (`elements/text.rs` in
      gpui-pre 0.3.12), so as a flex item its minimum width is its whole
      single line and it never shrinks. `docs/agents/app.md`'s "A meta pane
      value that does not fit beside its label goes under it" records the
      same behaviour and its cure: a `min_w_0` text `div`.
    - Put the string in a child `div` that may shrink to the container's
      content box (`min_w_0`, or `max_w_full`; measure which one wraps in
      the running app and in the test). Keep the container's `p_4`: it is
      already the 16 px margin, and it is wider than the 5 px bands, so the
      wrapped text cannot reach them. Keep `text_center`, the 14.4 px size
      and `muted_foreground`. In a wide window the child's basis is its
      one-line width, so nothing changes there; confirm by screenshot.
    - Leave `id("empty")`, `cursor_pointer` and `on_click` where they are,
      on the full-size container.
    - `crates/app/src/empty.rs` should need no change: the texts and
      `state` are not involved. Do not touch `pane_handle` or
      `panewidths.rs`.
    - UI test, in the style of
      `the_settings_layer_lies_under_the_title_bar_and_fills_the_rest`:
      give the text's `div` a fixed `debug_selector` (`debug_bounds` takes
      a `&'static str`), open `Ui::open()` with no folder, set the view's
      `pane_widths` to 480 / 480 through `ui.update`, and narrow the window
      with `ui.cx.simulate_resize` so the preview is `PREVIEW_MIN` (1200 px
      wide: `PaneWidths::drawn` leaves 480 + 240 + 480). Assert the text's
      bounds lie inside the preview (left edge at or right of 480 + 16,
      right edge at or left of 720 - 16) and that it is taller than the
      same element's bounds in the default 1600 px test window. Then
      `simulate_click` at the bounds' center and assert
      `ui.cx.did_prompt_for_paths()`.
    - The test platform's text system is `NoopTextSystem`, so glyph widths
      are synthetic: assert relations (contained, taller than when wide),
      not pixel sizes or line counts. Check first that the no-folder text
      is wider than 208 px and narrower than the wide window's preview
      under that text system; if it is not, pick widths for the test that
      make it so and say why in a comment. Confirm the test fails on the
      code before the fix.
    - Check whether `simulate_resize` alone re-renders the view or a
      `run_until_parked` is needed after it.
    - Host memory is low: build with `-j 2`. Run only the app crate's new
      test locally if a local run is needed; leave the rest to the PR's
      checks.
    - Docs: add the rule to the empty-state section (the text is a
      shrinkable child of the centered container; why a bare string child
      does not wrap; the 16 px padding keeps it off the bands; where the
      click target is) and a dated `Seen` entry with the measurements.
      `CLAUDE.md`'s layout paragraph does not describe this level of detail
      and should not need a change.

## Trade-offs and risks

- Where the no-folder click target is. Today it is the whole preview area
  (the full-size container carries the `on_click`), not the text's box: the
  evidence's clicks at x=489 and x=711, far from where the text should be,
  opened the picker. Decided: A.
  - A (chosen): keep the listener on the full-size container. No behaviour
    change, the 2026-11-22 "Seen" entries stay true, and the pointer cursor
    keeps covering the whole preview. The bands already occlude it.
  - B (not chosen): move `id`, `cursor_pointer` and `on_click` to the
    wrapped text's box. A click on the empty surround would stop opening
    the picker, a user-visible change.
  - Under either, the target cannot overlap the 5 px bands: A is occluded
    by them, B sits inside the 16 px padding.
- A single unbreakable word wider than the 208 px content box (a very long
  key name in the `Or press …` clause) could still overflow. No shipped
  text or key name comes near it, so the plan does not handle it.
- The UI test runs on synthetic glyph widths, so it proves the layout
  shrinks and wraps, not how the real fonts break the lines. The
  screenshots cover Windows; macOS and Linux stay not yet seen.
- `D:\Photos\tests\.ui-lock` is not described in `docs/agents/app.md`. The
  plan follows it as given; whether to document the lock is outside this
  change.
- This is one step, under the usual three to seven: the change is one
  element's layout, its test and its doc.

## Progress

- (2026-11-28) Step 1 complete
