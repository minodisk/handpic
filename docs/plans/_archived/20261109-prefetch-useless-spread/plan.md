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

# Remove the `unicorn/no-useless-spread` warnings in prefetch.ts

## Purpose

PR #943 (the preview prefetch ring) left three `unicorn/no-useless-spread`
warnings in `crates/app/ui/src/prefetch.ts`, which `mise run ci` reports on
every run. Each is a `for (const path of [...set])` loop whose body deletes
the current element from that same `Set`. Removing them keeps the lint output
clean so a new warning is noticed, without changing what the ring does.

Investigation result (planning time): at all three sites the loop body is a
single `if (pred(path)) set.delete(path)`, where `pred` closes over a local
`Set` (`wanted` in `plan`, `kept` in `retain`) and calls nothing else. The
only mutation of the iterated collection is deleting the element currently
being visited, which ECMAScript defines for `Set` iteration (deleted entries
are skipped, the remaining ones are still visited; confirmed with Node). The
same file's `drop` already iterates `this.held` directly while deleting from
it. So the spreads are not defensive copies that the behavior relies on; they
can be dropped outright. oxlint does not flag `Array.from(set)` in a `for…of`
either (verified), but that form would keep a copy nothing needs.

## Steps

- [x] Step 1: Iterate `crowded` and `refused` directly in `plan` and `retain`
  - Done when: `mise run ci` reports no warning for
    `crates/app/ui/src/prefetch.ts`; `crates/app/ui/src/prefetch.test.ts`
    passes unchanged; no other file changes except this plan's progress entry
  - Implementation approach:
    - In `crates/app/ui/src/prefetch.ts`, replace `[...this.crowded]` (in
      `plan`, ~line 59, and `retain`, ~line 131) and `[...this.refused]` (in
      `retain`, ~line 128) with `this.crowded` / `this.refused` as the
      `for…of` source. No other edits; the loop bodies stay as they are.
    - Do not edit `prefetch.test.ts`. Do not touch warnings in other files.
    - `docs/agents/tauri-app.md`'s prefetch section does not describe the
      copy, so no documentation change.
    - Verify with `pnpm exec vp lint crates/app/ui/src/prefetch.ts` (expect
      no output for the file) and `pnpm exec vp test src/prefetch.test.ts`
      (16 tests), then `mise run ci`.
    - Commit as `fix(app): iterate the prefetch ring's sets without copying`
      (or similar `fix(app):` / `refactor(app):` Conventional Commit).

## Trade-offs and risks

- Direct iteration vs `Array.from(...)`: both are lint-clean. Direct iteration
  is chosen because the copy serves no purpose (only the current element is
  deleted) and `drop` in the same file already iterates while deleting, so it
  is the consistent form. `Array.from` would signal to a reader that a copy is
  needed when it is not.
- Risk: if a future change makes the loop body delete an element other than
  the current one, or add elements, direct iteration would change the visited
  set. That is not the case today, and the test suite covers `plan` / `retain`
  behavior.

## Progress

- (2026-11-09) Step 1 complete
