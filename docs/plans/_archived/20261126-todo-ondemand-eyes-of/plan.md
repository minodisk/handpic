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

# Fix CLAUDE.md's `src/ondemand.rs` callers (`eyes_of`)

## Purpose

Close the `todo.md` item "Docs: CLAUDE.md's `src/ondemand.rs` description
names an `eyes_of` that does not exist in the GPUI app".

`CLAUDE.md` says the engine's `src/ondemand.rs` sits "behind the app's
`faces_of` and `eyes_of`". Those were the removed Tauri app's commands. In the
GPUI app the on-demand reads go through `crates/app/src/overlay.rs`:

- `ondemand::read_faces` is called only by `overlay::detect`.
- `ondemand::read_eyes` is called only by `overlay::judge`.
- Both are reached from `analyze_current` in `crates/app/src/main.rs`, on what
  `Marks::wanted_faces` and `Marks::wanted_eyes` give.

An agent reading `CLAUDE.md` to find the callers greps for names that lead
nowhere useful. The only `faces_of` in `crates/app` is a test helper in
`src/faces.rs`, and the only `eyes_of` is an unrelated test helper in
`src/companion.rs`. The item was found during Step 4 of
[eyes-mark-tier-mcp](../20261123-eyes-mark-tier-mcp/plan.md).

## Steps

- [x] Step 1: Name the real callers in `CLAUDE.md` and delete the todo item
  - Done when:
    - `CLAUDE.md`'s `src/ondemand.rs` description no longer names `eyes_of`
      or `faces_of`; it names `overlay::detect` and `overlay::judge` as what
      the on-demand faces and eye judgment sit behind.
    - `grep -n "eyes_of" CLAUDE.md` finds nothing.
    - The `todo.md` section "### Docs: CLAUDE.md's `src/ondemand.rs`
      description names an `eyes_of` that does not exist in the GPUI app"
      (heading, Background, Files, TODO) is gone, and no other `todo.md` item
      changed.
    - The diff touches only `CLAUDE.md`, `todo.md` and this plan folder.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - `CLAUDE.md`: the one occurrence is in the `crates/engine` paragraph,
      the clause "behind the app's `faces_of` and `eyes_of`" just before
      "`src/trash.rs` the move behind…". Replace only that clause, with
      "behind the app's `overlay::detect` and `overlay::judge`". Do not
      re-wrap the surrounding lines or touch the rest of the sentence.
    - Leave the other parts of the same sentence alone (`read_preview`,
      "`get_view`'s fallback", "the `raw_only` guard"); they are not part of
      this item.
    - `todo.md`: the item is the last section of the file. Delete from its
      `###` heading to the end of the file, together with the blank line that
      separates it from the previous item, so the file ends with the previous
      item's last line and one trailing newline.
    - Do not change the remaining `faces_of` / `eyes_of` mentions outside
      `CLAUDE.md`: the doc comments in `crates/engine/src/ondemand.rs`,
      `crates/cli/src/main.rs`, `docs/agents/shared-crates.md`,
      `docs/agents/tract-onnx-inference.md` and the other `todo.md` items.
      They are code or history and outside a docs-only edit of this item.
    - `README.md` and `docs/humans/*.md` are not touched, so no Japanese
      translation needs syncing.

## Trade-offs and risks

- **Which name for the faces half.** The todo item said `faces_of` exists in
  `crates/app/src/faces.rs` / `companion.rs`. The code says otherwise: the
  only `faces_of` in `crates/app` is a test helper in `src/faces.rs`,
  `companion.rs` has none, and `ondemand::read_faces` is called only by
  `overlay::detect`. The user approved naming `overlay::detect` (2026-11-26).
- **The same stale names in code comments.** `crates/engine/src/ondemand.rs`
  and `crates/cli/src/main.rs` still say `faces_of` / `eyes_of` in doc
  comments. Left out to keep this docs-only.
- **`read_preview` in the same sentence.** `ondemand::read_preview` is not
  called from `crates/app`; its caller is the engine's `src/mcp.rs`. Left as
  is, being outside the item.

## Progress

- (2026-11-26) Step 1 complete
