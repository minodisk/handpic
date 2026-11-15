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

# Record that DescriptionList was declined for the native meta pane

## Purpose

On 2026-11-15, replacing the GPUI app's (`crates/native`) right-hand meta pane
with gpui-component 0.9.1's `DescriptionList` (optionally inside `GroupBox`)
was investigated and declined. Nothing in the repository says so, so the next
agent looking for gpui-component adoption candidates would repeat the
investigation. Once this is done,
[native-app.md](../../agents/native-app.md) states the decision and its reasons
next to the meta pane's section.

## Steps

- [x] Step 1: Add the declined-`DescriptionList` note to `docs/agents/native-app.md`
  - Done when:
    - `docs/agents/native-app.md` holds the note as a sibling section directly
      after "### The meta pane is `div` rows, and its cost lands in `present`
      (Measured)" (under "## Timings") and before "### A same-session A/B
      gates only work on the page path (Hit)".
    - The note states the decision (keep the `div` rows), the date
      (2026-11-15), the version (gpui-component 0.9.1), and every reason
      listed under the implementation approach below.
    - No file other than `docs/agents/native-app.md` and this plan folder
      changes; no code changes.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (the `lint` job's lychee run with `--include-fragments` covers the
      file's links).
  - Implementation approach:
    - Follow the file's conventions: a `###` heading that states the fact and
      ends in its tag, a short prose paragraph, then `- Rule:` bullets. The
      tags are defined at the top of the file (**Hit** broke something,
      **Measured** steered a design decision, **Inferred** comes from sources
      or docs). This judgment comes from reading the component's source, so
      it is (Inferred).
    - Heading: "### The meta pane does not use gpui-component's
      `DescriptionList` (Inferred)" (decided by the user: a sibling section,
      not a bullet in the (Measured) section).
    - An existing precedent for "this is not the gpui-component element" is
      the progress bar's paragraph ("It is not gpui-component's progress
      element: ...") in the same file; match that tone.
    - Reasons to record (confirmed against
      `gpui-component-0.9.1/src/description_list.rs`, `group_box.rs`,
      `theme/schema.rs`, `theme/theme_color.rs`, `crates/native/src/main.rs`'s
      `meta_pane` and `crates/native/src/theme.rs`):
      - `DescriptionList`'s defaults (bordered, 3 columns, a fixed 120 px
        label width, left-aligned values) would all need overriding.
        Reproducing today's look (the label at its content width, the value
        `min_w_0`, right-aligned and wrapping, an 11 px muted group heading
        with `gap_0p5`) means building each value as an `AnyElement`, which is
        no shorter than the hand-written `div` rows. In the 220 px pane a
        fixed label width also squeezes the value and can wrap long labels.
      - `DescriptionList` has no group heading; `GroupBox`'s title sits
        `gap_4` (16 px) above its content, much wider than today's gap.
      - The label colour `description_list_label_foreground` (and, when
        bordered, `tokens.description_list_label`) is derived from Default
        Dark at theme load; `Theme::update` does not re-derive it, so
        `theme.rs`'s `map` would have to set it explicitly to get the app's
        `MUTED_COLOR`.
      - The label is fixed at `text_sm()` while the pane is 13 px, so the
        label and value sizes may differ. This was not verified; say so in
        the note.
      - It is a rendering change, so the rule in "A same-session A/B gates
        only work on the page path (Hit)" requires the same-session A/B
        latency gate; the cost outweighs the gain.
    - Keep it short: a few lines to a short section. English only;
      `docs/agents` has no Japanese pair. No "revisit on a version bump"
      bullet (decided by the user: it reads as speculation).
    - `.md` files are not formatted by `vp` (`vite.config.ts` ignores them),
      so the only automated check on the text is lychee. Wrap lines near 80
      columns by hand, as the surrounding text does.

## Trade-offs and risks

- The `text_sm()` versus 13 px size mismatch is unverified. The note must mark
  it as not verified rather than state it as fact.
- The gate is `mise run ci:local` plus the PR's GitHub checks rather than the
  full `mise run ci`, because no agent runs the full `mise run ci` locally; the
  PR's `lint` job runs the same formatting and link checks.

## Progress
