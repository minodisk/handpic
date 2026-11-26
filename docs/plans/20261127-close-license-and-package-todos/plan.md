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

# close-license-and-package-todos

## Purpose

Two `todo.md` items are already met by evidence gathered on 2026-11-27, and `todo.md` lists only unfinished work (a completed item is deleted, not ticked; `.claude/agents/todo-curator.md`, "Write the deletion proposals"). This plan removes them. It changes `todo.md` only; no code changes.

Evidence:

- GitHub's license detection: `gh api repos/minodisk/handpic --jq .license` gives `{"key":"apache-2.0","spdx_id":"Apache-2.0",...}`, `gh api repos/minodisk/handpic/license` gives the path `LICENSE-APACHE`, and the public repository page's sidebar reads "Apache-2.0, MIT licenses found" with links to `LICENSE-APACHE` and `LICENSE-MIT`.
- The `Package` run: the v2.1.0 `Release` run 45712125456 (2026-11-25, after #1241's merge `3bafc9b`) ran `package.yml` through `workflow_call`. Its Windows job's log lists `Handpic.exe` in the installed folder and `Handpic\Handpic.exe` in the extracted ZIP. The published v2.1.0 assets (`SHA256SUMS` checked) hold `Handpic/Handpic.exe` in the portable ZIP and `Handpic.app/Contents/MacOS/Handpic` in both macOS tarballs.

## Steps

- [x] Step 1: Delete the two closed items from `todo.md`
  - Done when:
    - The whole section `### Repo: post-merge check that GitHub detects the MIT OR Apache-2.0 license (dual-license)` is gone, from its heading through its one TODO line, with exactly one blank line left between the sections around it.
    - In `### App: real-device checks of the capital executable name (`Handpic.exe`, `Contents/MacOS/Handpic`)`, the last TODO line ("Dispatch the `Package` workflow by hand on the branch; ...") is gone and the four other TODO lines are unchanged.
    - That section's Background no longer says nobody ran a `Package` run, and holds one sentence recording the v2.1.0 `Release` run evidence above.
    - `git diff --stat` shows `todo.md` and this plan folder only; no other `todo.md` section changed.
    - `mise run fmt` leaves no diff, `mise run ci:local` passes locally, and the PR's GitHub checks pass.
  - Implementation approach:
    - File: `todo.md` only. At planning time the capital-executable section starts at line 1420 and the license section at line 1438 (it ends before `### Core: check whether the PIL resampling port needs a Pillow notice`); find them by heading, not by line number.
    - Background edit in the capital-executable section: change "Nobody ran a real upgrade over a 2.0.0 install, a real portable update from a test feed, the macOS bundle or a `Package` run." so it no longer names the `Package` run (for example "... a real portable update from a test feed or the macOS bundle."), and add the evidence sentence in the style of the existing "Seen on macOS on 2026-11-26 ..." sentence. Touch no other sentence of that paragraph.
    - The license evidence has no place left in `todo.md` once its section is deleted; put it in the PR description (it is also in this plan's Purpose).
    - `mise run fmt` is `cargo fmt` only and does not format Markdown, so it cannot catch a stray blank line; check the spacing around the deleted section by eye in the diff.
    - This plan folder's Markdown is checked by lychee in `mise run ci:local`; it holds no relative links.

## Trade-offs and risks

- The capital-executable Background ends with a stale backticked plan path (the folder is now under `docs/plans/_archived/`). It is out of scope and left alone (user decision, 2026-11-27).
- The deleted TODO line asked for a hand dispatch "on the branch" with listings of the ZIP's contents, the installed folder and the bundle. The evidence is a `workflow_call` run from `main` after the merge, its Windows log listings, and the published assets' contents for macOS (not a log listing of the bundle). The Background sentence says what was actually seen, so the difference stays on record.
- The license section's note that `mise run ci:local` had been stopped locally for memory pressure is history of the dual-license PR, not an open task, and goes with the section.

## Progress
