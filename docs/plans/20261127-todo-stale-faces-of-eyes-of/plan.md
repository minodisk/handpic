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

# Fix the stale `faces_of` / `eyes_of` caller names outside CLAUDE.md

## Purpose

Close the `todo.md` item "Docs: stale `faces_of` / `eyes_of` caller names
remain outside CLAUDE.md".

`faces_of` and `eyes_of` were commands of the removed Tauri app. In the GPUI
app the engine's on-demand reads are called from `crates/app/src/overlay.rs`:
`ondemand::read_faces` by `overlay::detect` and `ondemand::read_eyes` by
`overlay::judge`.
[The earlier plan](../_archived/20261126-todo-ondemand-eyes-of/plan.md) fixed
`CLAUDE.md` only. Doc comments and agent guides still name the two commands
as if they existed, so a reader greps for names that lead nowhere. Once this
is done, every remaining mention either names the real functions or says
outright that the name was a command of the removed Tauri app.

## Steps

- [x] Step 1: Replace or label the stale names in the four files and delete
      the todo item (single-PR mode)
  - Done when:
    - In `crates/engine/src/ondemand.rs`, `crates/cli/src/main.rs`,
      `docs/agents/shared-crates.md` and `docs/agents/tract-onnx-inference.md`,
      no text names `faces_of` / `eyes_of` as a present caller or function.
      Each remaining mention is in a historical account that says it was a
      command of the removed Tauri app. The test name
      `times_eyes_of_on_real_files` is kept (see "Decisions").
    - `git grep -n "faces_of\|eyes_of" -- crates/engine crates/cli docs/agents`
      shows only those labeled historical mentions and the test name.
    - `shared-crates.md`'s claim that `times_eyes_of_on_real_files` "left with
      the earlier app" is corrected: the test lives in
      `crates/engine/src/ondemand.rs`.
    - The `todo.md` section "### Docs: stale `faces_of` / `eyes_of` caller
      names remain outside CLAUDE.md" (heading, Background, TODO) is gone,
      and no other `todo.md` item changed.
    - The diff touches only those four files, `todo.md` and this plan folder;
      in the two `.rs` files only comments change. `CLAUDE.md` is untouched.
    - `mise run fmt` leaves no diff, `mise run lint` passes, `mise run
      ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Re-check the targets first with `git grep -n "faces_of\|eyes_of"`; the
      line numbers below are as of `739a17ee`.
    - `crates/engine/src/ondemand.rs` (present tense, replace):
      - line 8, the `FacesResponse` doc "The faces `faces_of` found": name
        `read_faces`.
      - line 102, the `EyesResponse` doc "The `eyes_of` answer": name
        `read_eyes`.
      - line 359, the ignored test's doc "The `eyes_of` breakdown on real
        files": name `read_eyes`.
      - These are the engine's own functions in the same file, so name them,
        not `overlay::detect` / `overlay::judge`: the engine does not depend
        on the app, and the struct is what the function returns.
    - `crates/cli/src/main.rs` line 520, "from the path `eyes_of` takes":
      the CLI's own `eyes_features` (line 773) already describes itself as
      "step for step as the app's `read_eyes`"; use the same wording
      (`ondemand::read_eyes`). Do not re-wrap the neighbouring lines.
    - `docs/agents/shared-crates.md`, the section "Scan, on-demand faces and
      eyes: timings taken through the earlier app" (lines 1038-1095). It is a
      historical account, so keep the names and label them:
      - lines 1041-1042, "(then the `faces_of` command)" / "(then
        `eyes_of`)": say they were the removed Tauri app's commands, and that
        the GPUI app calls the two reads from `overlay::detect` /
        `overlay::judge`.
      - the bullets at 1065 and 1078 ("**`faces_of`, now
        `ondemand::read_faces`**", "**`eyes_of`, now `ondemand::read_eyes`**"):
        state the Tauri label in each bullet.
      - line 1080: correct "left with the earlier app" to say the test is in
        `crates/engine/src/ondemand.rs`.
    - `docs/agents/tract-onnx-inference.md`:
      - line 140, "`faces_of` and the UI scale the boxes by it": present
        tense, replace. `ondemand::read_faces` hands the stored size on as
        `FacesResponse::width` / `height`; check in `crates/app/src/mark.rs`
        (`face_marks`) what scales the boxes and word it from the code.
      - line 217, "does not move the `eyes_of` time": present tense, replace
        with `ondemand::read_eyes`.
      - line 323, "Once `eyes_of` called it, that release app binary grew":
        history of the Tauri app's binary; label it as the removed Tauri
        app's `eyes_of` command.
      - line 255 names the test `times_eyes_of_on_real_files`; keep it.
    - `todo.md`: the item is the file's last section (lines 1578-1586).
      Delete the heading, Background and TODO, and leave the file ending in
      one newline after the previous item.
    - Do not re-wrap paragraphs beyond the lines whose words change.
    - Not in scope, leave as they are: `CHANGELOG.md`, `docs/plans/_archived/**`,
      `docs/plans/review-history/**`, the in-flight
      `docs/plans/20261109-gpui-port/plan.md` (lines 339-340, a plan's own
      record), the local test helpers `eyes_of` in
      `crates/app/src/companion.rs` and `faces_of` in `crates/app/src/faces.rs`
      (unrelated functions that exist), and
      `crates/core/src/allfaces.rs` (a substring match in a test name).

## Decisions

- **The test name `times_eyes_of_on_real_files` is kept.** Renaming it is a
  code identifier change, outside the comments-and-docs scope; the grep
  keeps its hits as a real identifier, not a caller reference.
- **`shared-crates.md:1080`'s "left with the earlier app" is corrected in
  this PR**, since the step already edits that line.
- **The in-flight `docs/plans/20261109-gpui-port/plan.md` is left alone**:
  it is that plan's own record, and the todo item names four files only.

## Progress
