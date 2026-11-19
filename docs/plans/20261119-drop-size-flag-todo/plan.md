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

# Drop the todo item based on a misread `--size` launch

## Purpose

#1175 added the todo item "Native: the first launch on a fresh index shows
the startup folder empty". Its premise is wrong. The launch it records also
carried `--size 1400x900`, which the item left out. `--size WxH` is not a
window size: it is the measurement flag that narrows the shown files to those
whose embedded preview JPEG is exactly WxH (`shown` in
`crates/native/src/main.rs`). No ARW preview is 1400x900, so `all` was empty,
while the index, the tree's count and the EXIF groups read from the index
still had every file, and `opened` wrote `no RAW files in <dir>`. Opening the
folder from the tree passes no size, and the relaunch had no `--size`, which
is why both listed the files.

Reproduced on 2026-11-19 with the same debug build on fresh `--index` /
`--settings` files: without `--size` a startup folder lists 5 of 5 and 12 of
12; with `--size 1400x900` it gives `count: 0` and `no RAW files in <dir>`.

There is no bug to fix, so the item goes. The user chose removal only (no
change to the empty-state or status wording).

## Steps

- [x] Step 1: Remove the item from `todo.md`
  - Done when:
    - The `### Native: the first launch on a fresh index shows the startup folder empty` item (heading, Background, Files, TODO) is gone from `todo.md`, with no doubled blank line left.
    - No other part of `todo.md` changes, and the PR changes no file other than `todo.md` and this plan's files.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The PR description states the real cause (`--size 1400x900`, a preview-size filter, not a window size).

## Trade-offs and risks

- A clearer message when `--size` / `--max` hide every listed file, and a UI
  test of a startup folder, were considered and not taken (the user chose
  removal only).

## Progress
