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

# Record the decision not to adopt gpui-component's `StatusBar`

## Purpose

The GPUI app (`crates/native`) draws its status row as a plain `div`
(`main.rs`'s `status_row`). gpui-component 0.9.1 ships a `StatusBar`, and
after the gpui-component adoption work a later port could be tempted to
"finish" the adoption by swapping the row for it. The swap was examined from
the sources and the user decided against it. This plan records that decision,
its reasons and the condition for revisiting it in
[`native-app.md`](../../../agents/native-app.md), so the examination is not
repeated and the row is not replaced by mistake.

## Steps

- [x] Step 1: Add a section to `docs/agents/native-app.md` recording that the
      status row stays a plain `div` and is not gpui-component's `StatusBar`
  - Done when:
    - `docs/agents/native-app.md` has one new `###` section that states the
      decision (do not replace `status_row` with `StatusBar`; the user's
      decision), the reasons listed below, and the revisit condition (the row
      getting a border and a background of its own).
    - The section covers all five findings:
      1. What would be unaffected. `StatusBar` (gpui-component `=0.9.1`,
         `src/status_bar.rs`) is a `RenderOnce` `h_flex` with left / center /
         right regions and no id, focus, key context or listener, so the
         keymap and the presses (the popover's `on_mouse_down_out`, the
         buttons' `on_mouse_down` toggle) would not change. Each region is
         `overflow_hidden`, but `Deferred` in `gpui-pre` 0.3.12 calls
         `window.defer_draw` with no content mask
         (`elements/deferred.rs`), so the Sort / Filter popovers would not be
         clipped and `anchored` would still open from the button's top-right.
      2. The default style that would have to be undone through
         `refine_style`. `py_1` and `border_t_1` make the row about 9 px
         taller, which changes the preview canvas recorded as 1380 x 852.
         `bg` reads the theme's `status_bar` token, which `theme.rs` does not
         map, so it falls back to `Default Dark`'s `#171717` (today the row is
         transparent over the root's black). `text_xs` (12 px) and
         `muted_foreground` would shrink and dim the status text (today the
         root's default size, white).
      3. The regions' `gap_2` cannot be changed from the root's style. Keeping
         the 4 px (`gap_1`) between Sort and Filter needs both wrapped in an
         `h_flex().gap_1()` of their own, passed as one `.right()`.
      4. Every child lands inside a region, never as a direct child of the
         root. Taffy positions an `absolute` element against its direct
         parent, so the progress bar could no longer sit on the row's top
         edge from inside it: the `StatusBar` would need a `relative` wrapper
         with the bar as its sibling, changing the rule in "One progress bar
         sits on the status row's top edge".
      5. What would be gained: only the component's name and the
         `status_bar` / `status_bar_border` theme tokens.
    - No other file changes, apart from this plan's own files: not
      `crates/native/**`, not `CLAUDE.md`, not `docs/humans/**`, not anything
      under `docs/plans/_archived/`.
    - `mise run ci:local` passes locally (for a Markdown-only diff it runs
      lychee) and the PR's GitHub checks pass.
  - Implementation approach:
    - Placement: directly after the section "One progress bar sits on the
      status row's top edge (Inferred)" and before "Text is typed through
      gpui-component's `Input` (Inferred)". That section already states what
      is not drawn from gpui-component, and finding 4 is about its rule.
    - Heading in the file's style, a statement with a provenance tag, e.g.
      "The status row stays a plain `div`, not gpui-component's `StatusBar`
      (Inferred)". The tag is `Inferred`: every finding comes from reading the
      sources (gpui-component 0.9.1, gpui-pre 0.3.12, `main.rs`, `theme.rs`);
      nothing was built or measured. Word the consequences as "would", and do
      not present the 9 px or the changed canvas size as measured.
    - Follow the neighbouring sections' shape: a short opening paragraph (what
      `StatusBar` is, what the row is today), then bullets. Use `- Rule:` for
      the decision itself ("do not replace `status_row` with `StatusBar`;
      revisit when the row gets a border and a background of its own; the
      user's decision"), as "The sort and filter menus are deferred popovers
      over the window" does for its "a later port must not fix them back"
      rule. The findings can be plain bullets.
    - Refer to other sections by their quoted title, as the file does
      elsewhere; do not add `#fragment` links, which lychee checks with
      `--include-fragments`.
    - Refer to code by name (`status_row`, `progress_bar`, `menu_button`,
      `theme.rs`), not by line number; `main.rs` line numbers move.
    - Markdown is not formatted by `vp fmt` (`vite.config.ts` ignores `.md`);
      wrap lines near 80 columns by hand like the rest of the file.
    - Do not edit the existing sections. In particular the progress bar
      section's sentence "only the settings modal and the tree's inline rename
      draw from it" is out of date since the adoption work, but correcting it
      is not part of this plan (the user's decision).

## Trade-offs and risks

- Single step: one section in one file has no meaningful split, so single-PR
  mode (wrap-up in the same PR) fits.
- Stale neighbouring text is left untouched (the user's decision): the
  progress bar section and the heading "gpui-component is initialized; the
  settings modal and the tree's inline rename draw from it" still say only
  those two draw from gpui-component, although the dialogs, tooltips and
  scrollbars now do too.
- The findings are source-read only. If gpui-component is bumped past 0.9.1,
  `StatusBar`'s defaults may change; the section pins the version it describes
  so a later reader can tell.

## Progress

- 2026-11-15: Step 1 done. Added the section recording why the status row is
  not gpui-component's `StatusBar` (pinned to 0.9.1) to
  `docs/agents/native-app.md`, between the progress bar section and the `Input`
  section, leaving the neighbouring sections untouched. Verified with
  `mise run ci:local` (including lychee).
