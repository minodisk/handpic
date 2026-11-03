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

# Auto-advance after un-reject / un-pick

## Purpose

With `Auto-advance after a star, reject or pick` on, `1`-`5`, reject (`x`) and
pick (`p`) move to the next file; the `unflag` key (`u`, "Un-reject /
un-pick") does not, because `advance.ts` treats the clearing actions as
corrections that stay on the file. The user culls with un-reject / un-pick as
a third judgment in the same pass and wants it to move on like reject and
pick do. After this work, clearing the flag advances under exactly the gates
reject and pick advance under (one target, Auto-advance on, not comparing,
the file still current, not view-only), and a repeat press on a file with no
flag advances too, like a repeat reject or pick.

## Steps

- [x] Step 1: Make `unflag` advance like `reject` and `pick`
  - Done when:
    - With Auto-advance on, pressing `unflag` on a single current file moves
      to the next file, including when the file already has no flag (writes
      nothing, adds no undo entry, still advances), exactly as `pick` /
      `reject` do.
    - `unflag` stays on the file under every condition `pick` / `reject` stay:
      Auto-advance off, Compare mode, more than one selected target, no files
      or a view-only folder, and a file that dropped out of the filter (the
      `stillCurrent` guard).
    - `clear` (stars), `clearlabel`, `clearall` and the color labels still do
      not advance; `rate1`-`rate5`, `reject`, `pick` are unchanged.
    - `advance.test.ts` asserts `advancesAfter("unflag") === true` and
      `shouldAdvance` with `action: "unflag"` behaves like `pick`, and the
      "stays after corrections" list no longer includes `unflag`.
    - `docs/humans/usage.md` (the **Auto-advance** bullet, ~line 570) and
      `docs/humans/usage.ja.md` (~line 47, 自動送り) list un-reject / un-pick
      among the keys that move on, in the same PR.
    - `mise run ci` passes.
  - Implementation approach:
    - `crates/app/ui/src/advance.ts`: add `"unflag"` to the `ADVANCING` set
      and reword the comment above `advancesAfter` (it currently says the
      clearing actions stay on the file; now only the stars/label clears and
      the labels do).
    - `crates/app/ui/src/main.ts`, `runAction`, `case "unflag"`: change
      `judge(...)` to `judged = judge(...)` so the target count reaches
      `shouldAdvance`, matching the `pick` / `reject` cases. No other
      change: the post-switch `shouldAdvance(...) && move(1)` block and
      `judge` (returns 0 for view-only / no files, the target count otherwise)
      already encode every gate. Do not touch `clear`, `clearlabel`,
      `clearall`.
    - `crates/app/ui/src/advance.test.ts`: move `"unflag"` from the
      "stays" list to the "advances" list in the `advancesAfter` tests and
      add `"unflag"` to the `shouldAdvance` "advances when the single file
      already holds the value" loop.
    - Docs: keep the EN/JA pair in sync; `README.md` / `README.ja.md` do not
      describe the advancing keys, so they need no change. Leave the settings
      checkbox label (`crates/app/ui/index.html` ~line 689, "Auto-advance
      after a star, reject or pick") and its verbatim quotes unchanged.
    - Commit: `feat(app): advance after un-reject / un-pick when Auto-advance is on`.

## Decisions

- Scope: only `unflag` (clearing the flag) advances; `clear` (stars),
  `clearlabel` and `clearall` stay on the file.
- Repeat press: `u` on an already unflagged file advances, like a repeat `p`
  or `x`, because `judge` counts targets whether or not anything changed.
- Settings label: left as "Auto-advance after a star, reject or pick", so the
  quotes in `usage.md`, `usage.ja.md` and `todo.md` stay consistent; only the
  usage bullet's key list is amended.

## Risks

- No real-device check is possible from the implementation agent; the pure
  `shouldAdvance` tests and the one-line wiring change are the verification.

## Progress

- (none yet)
