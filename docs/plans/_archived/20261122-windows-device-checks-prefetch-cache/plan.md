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

# Record the Windows real-device checks of the prefetch ring and of Clear Cache (2026-11-22)

## Purpose

A real-device run on Windows 11 on 2026-11-22 exercises the Windows boxes of
two `todo.md` items: `### App: real-device check of the prefetch ring on a page
turn, a folder switch and a filter change` (three boxes) and `### App:
real-device checks of the Cache page and Clear Cache (native-clear-cache)` (one
box). This plan runs them, removes the boxes the run settles, narrows the rest
to what stays open, and adds whatever the run finds as new items. Docs and todo
only, no code change: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-6/plan.md`.

The run's results go to `results.md` in this folder. They are the source of
truth for every edit below.

## Rule for sub-points the run did not exercise

A box is deleted only when everything it expects was exercised and passed. A
part not exercised stays open in a narrowed box with the reason. A failure
becomes a new item (or stays open).

## Steps

- [x] Step 1: Run the checks, write `results.md`, and apply the results to `todo.md`
  - Done when:
    - `results.md` in this folder holds the setup, the incidents and one section per box, each marked PASS, FAIL / FINDING, or not run with the reason.
    - Each of the four Windows boxes is deleted, narrowed, or kept with a reason, as the results give; an item left with no box is deleted whole.
    - `git diff origin/main --stat` shows only `todo.md` and this plan's files.
    - No doubled blank lines are left where a box or an item went.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **The run.**
      - Build: the release build of `crates/app` (`cargo build --release`, NASM on `PATH`), as the prefetch boxes ask.
      - MCP on port 41981 (`--mcp-port`), `{"mcpEnabled": true}` seeded through `--import-settings` into a fresh `--settings` store; `mcpAllowDestructive` stays off.
      - Per launch: fresh `--settings` and `--log`; a fresh `--index` except the Clear Cache check, which reuses one test index across two launches so the second launch has cached counts. The real index and store are never touched.
      - Synthetic keys (`keybd_event`) only after `GetForegroundWindow` is the test instance's window (by PID); window captures and burst captures; the `page` lines read from the redirected stdout. Foreground work is done under `D:\Photos\tests\.ui-lock`.
    - **Material.** Copies under `D:\Photos\tests\20261122-prefetch-cache\`: `ring-a\` (20 ARWs of `2026-06-05-focus-sample`), `ring-b\` (20 ARWs of `2026-09-19-focus-sample`), `cache\c1\`, `cache\c2\`, `cache\c3\` (3, 5 and 7 ARWs of `2026-08-22-focus-sample`).
    - **Prefetch box 1.** Open `ring-a\`, wait for idle, press `→`; burst-capture and read its `page` line.
    - **Prefetch box 2.** From a file of `ring-a\`, open `ring-b\` from the tree; burst-capture the preview through the switch and compare each frame with `ring-a\`'s images.
    - **Prefetch box 3.** Rate files so a filter hides the current file's next neighbour (after the ring holds it), set the filter, press `→`; burst-capture and check the hidden file's image never shows.
    - **Clear Cache box.** Launch 1 lists `cache\c1..c3`; launch 2 on the same index shows them unlisted with cached counts; `Clear Cache` (Cache page, confirm); expect the cached counts gone.
    - **Edits (from `results.md`).** Every box passed, so both items are deleted whole: `### App: real-device check of the prefetch ring on a page turn, a folder switch and a filter change` (its three boxes were all Windows) and `### App: real-device checks of the Cache page and Clear Cache (native-clear-cache)` (its one box was Windows). No new item: the run found no defect.
    - **Editing.** Locate items by their `###` heading. Every "seen" sentence says "on Windows 11 on 2026-11-22" and names `docs/plans/_archived/20261122-windows-device-checks-prefetch-cache/results.md` (in backticks).

## Trade-offs and risks

- Synthetic keys and captures stand in for a person's eye; a frame between two captures can be missed, so the switch checks use bursts and the `page` lines.

## Progress

- 2026-11-22: Step 1 done. The run is in `results.md`, the notes in `learnings.md`. All four Windows boxes passed, so both items are deleted whole from `todo.md`: "App: real-device check of the prefetch ring on a page turn, a folder switch and a filter change" and "App: real-device checks of the Cache page and Clear Cache (native-clear-cache)". No new item.
