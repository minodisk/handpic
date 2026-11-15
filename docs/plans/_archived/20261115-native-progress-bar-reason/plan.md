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

# State why the native progress bar is not gpui-component's Progress

## Purpose

`docs/agents/native-app.md`, section "One progress bar sits on the status
row's top edge", says the bar "is not gpui-component's progress element: only
the settings modal and the tree's inline rename draw from it". That is not a
reason, and it is no longer true: the dialogs, the tooltips and the
scrollbars are drawn from gpui-component too. Since the app now adopts
gpui-component widely, the next reader will ask why this one element is still
two plain `div`s. Once this is done the section states the actual reasons,
verified against gpui-component 0.9.1, so nobody swaps the bar for `Progress`
without knowing what changes.

## Steps

- [x] Step 1: Replace the non-reason sentence with the reasons the bar stays two plain `div`s
  - Done when:
    - The section "One progress bar sits on the status row's top edge" in
      `docs/agents/native-app.md` states why gpui-component 0.9.1's
      `Progress` is not used: the track color cannot be set apart from the
      indicator's, and the value always animates.
    - The clause "only the settings modal and the tree's inline rename draw
      from it" is gone from that sentence.
    - No other part of the section, no other section (the heading at line 48
      included) and no code is changed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Change only `docs/agents/native-app.md`, and only the sentence starting
      "It is not gpui-component's progress element" (lines 1845-1848 at
      planning time), plus one `- Rule:` bullet in the same section. The rest
      of the section was checked against `crates/native/src/main.rs`'s
      `progress_bar` and `progress.rs` and is still accurate; leave it.
    - Match the doc's style: plain statements in the paragraph, and a
      `- Rule:` bullet for the instruction ("do not replace the bar with
      `Progress` while ..."), placed with the section's other rules. Name
      the version ("as verified at 0.9.1") so a version bump re-reads it.
    - Facts verified at planning time in
      `gpui-component-0.9.1/src/progress/progress.rs` and
      `gpui-base-0.9.1/src/motion.rs`:
      - The track is drawn with `bg.opacity(0.2)`, `bg` being the
        `.color(..)` value or the theme's `progress_bar` (which falls back
        to `primary`). There is no separate setter for the track. So the
        track would be `PRIMARY_COLOR` (`#e5e5e5`) at 20 % rather than
        `MUTED_FILL` (`#262626`), the Tauri `#scan-progress` track.
      - The indicator's width always goes through `transition` with the
        theme's `motion.duration_normal` (180 ms by default). The only ways
        off are GPUI's reduce-motion or setting
        `theme.motion.duration_normal` to zero, which the dialogs
        (`dialog/dialog.rs`) and `grid_layout.rs` read too.
      - While the transition runs it calls `window.request_animation_frame()`,
        against the section's rule that the bar adds no `notify` or timer.
      - The transition is keyed by the element id, so a drop of the value
        (pass 1 to pass 2, a rescan) is drawn as the bar shrinking from full
        to 0.
      - What it would bring is unused today: the accessibility progress
        role with `accessibility_label`, and the indeterminate `loading`
        mode.
      - Optional note of what would reproduce the look:
        `.with_size(Size::Size(px(2.)))`, `.rounded_none()` (with the
        theme's 6 px radius the bar is otherwise a 1 px pill), and `Styled`
        positioning.
    - Verify during implementation: the blended track color to quote. `#e5e5e5`
      at 20 % comes out near `#404040` over `PANE_COLOR` (`#171717`) and near
      `#363636` over the `#0a0a0a` background. Read which fill the status
      row has in `main.rs` and quote that one, or write only "`PRIMARY_COLOR`
      at 20 %, lighter than `MUTED_FILL`" without a hex.
    - Once the stale clause goes, the pointer to the section "gpui-component
      is initialized; the settings modal and the tree's inline rename draw
      from it" no longer supports the sentence: drop it, or keep it as a
      pointer for the theme with the heading's exact name.
    - A docs-only change: `mise run ci:local` runs only what the changed
      paths call for (the lychee link check of `mise run lint` among them).
      The new text adds no link, only quoted section names.

## Trade-offs and risks

- **The heading the sentence points to is stale too.** The section at line 48
  is titled "gpui-component is initialized; the settings modal and the tree's
  inline rename draw from it". The user chose to leave it as is in this PR.
- **The reasons are tied to 0.9.1.** A later gpui-component may add a track
  color or a way to turn the transition off. Naming the version in the text
  is the mitigation.

## Progress

- (2026-11-15) Step 1 complete
