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

# Rank a burst's frames by the full-size eye sharpness

## Purpose

`todo.md`'s "Core: decide which score ranks frames within a burst, the
AF-frame window or the eye-midpoint window" is open. The check behind it
([burst-rank.md](../20261123-todo-four-closeouts/burst-rank.md))
found that the stored `sharpness` (the AF-frame window) and the eye-midpoint
window it replaced disagree on the order of a burst's frames: median
per-burst Spearman 0.500, the same top frame in 62% of bursts.

The user's decision: the score that ranks frames within a burst reads the
stored `full_eye_sharp` (the eye sharpness measured on the full JPEG). A
frame without that value falls back to `sharpness` as today. How `sharpness`
is measured does not change, so `FACES_VERSION` is not bumped.

Today nothing in the app ranks a burst. The only per-frame burst score is
`burst[].sharpness` of the MCP `get_view` (`companion.rs`'s `burst_of`),
listed in capture order, which a client ranks by itself. After this plan,
`get_view` reports each burst frame's full-size eye sharpness and its rank
by one stated rule, so every client ranks a burst the same way, and the
todo item is gone.

## Steps

- [x] Step 1: Rank the frames of `get_view`'s burst by `full_eye_sharp`, falling back to `sharpness`
  - Done when:
    - A pure rank function in `crates/app/src/burst.rs` gives each frame of
      a burst its 1-based rank by this rule, and its tests name each clause:
      1. a frame with a `full_eye_sharp` ranks by it, higher first;
      2. a frame without one ranks by `sharpness`, higher first, below
         every frame that has a `full_eye_sharp` (the two scores are on
         different scales and are never compared with each other);
      3. a frame with neither score has no rank;
      4. equal scores keep capture order.
    - The tests cover: every frame with `full_eye_sharp` (the order follows
      it even where `sharpness` orders the frames the other way); no frame
      with it (the order follows `sharpness`, as today); a mixed burst (a
      `sharpness`-only frame with a huge value still ranks below every
      `full_eye_sharp` frame); a frame with neither; a tie.
    - `get_view`'s `burst` frames carry `eye_sharpness` (the stored
      `full_eye_sharp`, else null) and `rank` (the rule's rank, else null)
      beside the unchanged `sharpness`, still listed in capture order;
      `companion.rs`'s burst tests assert both fields, including a mixed
      burst.
    - The `get_view` tool description and the server instructions in
      `crates/engine/src/mcp.rs`, `docs/agents/app.md` (the `burst`
      paragraph of the MCP section), `docs/humans/usage.md` and
      `docs/humans/usage.ja.md` (the `Sharpness score` bullet and the
      `get_view` row) and `CLAUDE.md` (the `burstOf` clause and
      `src/burst.rs`'s) state the two fields and the rule.
    - The `todo.md` item "Core: decide which score ranks frames within a
      burst, the AF-frame window or the eye-midpoint window" is deleted
      with one of the two `---` rules around it.
    - `FACES_VERSION` and `crates/core/src/sharpness.rs` are unchanged.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - No engine, index or core change: `Facts.focus.eyes.full_eye_sharp`
      and `Facts.sharpness` are already in the view's `Facts`
      (`crates/app/src/filter.rs`), both patched by pass 2 through
      `filter::apply_face_ready`.
    - Keep the rank function free of `Facts` and JSON (for example, each
      frame's `(full_eye_sharp, sharpness)` pair in capture order in, a
      `Vec<Option<usize>>` out) so `burst.rs` tests it without a view;
      `burst_of` in `crates/app/src/companion.rs` builds the input from
      `view.facts` in the position order it already sorts by.
    - Rank over every member of the burst, the frames the filter hides
      included, as `burst_of` lists them (`visible: false`).
    - Do not change the meaning of `sharpness` in `get_view` or
      `get_photo`; `get_photo` already reports `full_eye_sharp` under
      `focus.eyes`.
    - The coverage table (`Coverage` in `companion.rs`) needs no row: no
      tool, action or menu item is added.
    - Say in the guide and the tool description that the ranks move while
      the scan's second pass is still filling the scores, so a client that
      needs settled ranks calls `wait_for_idle` first, as it does for
      `photo_tier`.
    - Word the docs as "rank by eye sharpness", not as a recommendation of
      the frame to keep: `docs/agents/shared-crates.md` records that the
      full-size eye measure says "not a miss" and must not be described as
      a best-of-burst quality ranking, and `todo.md` keeps "Suggest the
      sharpest-eye frame within a burst group" as measured and not shipped.
    - No UI change: no strip mark, no meta pane row.

## Trade-offs and risks

Decided with the user at plan approval (2026-11-26): tiers for a mixed
burst, a computed `rank`, a null rank for a frame with neither score.

1. **Ordering rule for a mixed burst** (chosen: tiers, as in Step 1).
   `sharpness` is a Laplacian variance (values like 123.5) and
   `full_eye_sharp` a ratio around 0.1 to 0.3, so they cannot be compared.
   A frame lacks `full_eye_sharp` when the cue judged no face on it (or the
   file holds no JPEG larger than the preview), so in a burst of a person it
   is the frame where the face was lost; ranking it below the judged frames
   matches the decision to rank by the eyes. Cost: a sharp frame whose face
   the detector missed ranks below a soft judged frame. Rejected: all or
   nothing per burst (one missed face sends the whole burst back to
   `sharpness`), and ranking only the `full_eye_sharp` frames (drops the
   fallback).
2. **A computed `rank` field** so the rule lives in one tested place rather
   than in untested prose each client applies differently. The docs wording
   keeps it from reading as the best-of-burst suggestion that was measured
   and not shipped.
3. **`sharpness` keeps its meaning**: overloading it with `full_eye_sharp`
   would mix two scales under one name.
4. **A frame with neither score** has a null rank, so "pass 2 has not
   reached it" does not look like "worst frame".
5. **Ties keep capture order**, so ranks are a strict 1..n over the scored
   frames.
6. The output grows by two fields per burst frame; existing fields keep
   their names and meanings.

## Progress

- (2026-11-27) Step 1 complete
