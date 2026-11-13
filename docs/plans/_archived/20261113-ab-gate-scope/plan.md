<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Scope the GPUI port's same-session A/B latency gate

## Purpose

Every GPUI port plan so far ended with a same-session A/B latency gate,
which needs the machine left idle for 30 to 60 minutes and collides with
other sessions' builds. The user decided (2026-11-13) that the gate is
needed only for work that can move the pager's latency: rendering, AI,
the preview and the thumbnails. Record that rule where port plans are
written from, so a planner includes the A/B step only when it applies.

## Steps

- [x] Step 1: Add the gate's scope to `docs/agents/native-app.md`
  - Done when:
    - `docs/agents/native-app.md` has a rule (in the section that describes
      the per-plan same-session A/B, or a new short section next to it if
      none exists) saying a port plan includes the A/B gate only when it
      touches one of: rendering (per-frame drawing, overlays, new drawn
      parts such as the status row, strip marks, the pane, menus' buttons),
      AI (pass 3 embeddings, genres, the ranking, the face and eye models),
      the preview (read, decode, orient, prefetch, the 1:1 view), the
      thumbnails (their read, decode and the scan that makes them), or
      background work that runs while paging (scan passes, watchers,
      folder reads on the open). Dialogs, context menus, file operations,
      the settings UI, code moves into the backend and fixes off the page
      path need no A/B. When in doubt, the plan says which side it is on
      and why.
    - Any existing wording there that says every plan ends with an A/B is
      adjusted to match; nothing else changes.
    - `mise run ci` passes (lychee, formatting).
  - Implementation approach: docs only; English; match the file's existing
    "Rule:" bullet style.

## Progress

- (2026-11-13) Step 1 complete
