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

# Run the pending real-device checks in the develop wrap-up

## Purpose

`todo.md` holds 235 items, 198 of them under the App section, largely because
a session that finishes a feature files every hands-on check as a manual item
("verify on a Windows real device") instead of running it. The development
host can run most of them: the app starts with the `screenshot` feature
(`mise run app:dev:screenshot`) and is driven through the MCP companion
(`get_view`, `get_status`, `set_overlays`, `screenshot` and the rest), its log
can be read, and an installer or setup can be run. PR #1307 and PR #1262 are
precedents of real-device checks run that way and recorded.

Today the develop flow says the opposite: `todo-curator` turns every pending
manual check into a `todo.md` addition because "the check cannot be run by an
agent". This plan changes the wrap-up (3.3 and S.5) so that each manual-check
candidate is reviewed one at a time, the ones this host can run are run by the
agent and recorded, and only the ones this host truly cannot run reach
`todo.md`, each with a one-line reason. Working through the existing `todo.md`
items is out of scope.

## Steps

- [x] Step 1: Make the wrap-up run the manual checks this host can run, and file only the rest
  - Done when:
    - `.claude/skills/develop/SKILL.md` 3.3 describes, after `todo-curator` returns, a per-candidate procedure: every manual-check candidate is taken one at a time, classified as runnable on this host or not, the runnable ones are run by the agent, their results are recorded, and they are not added to `todo.md`. The paragraph after the `PROPOSED` branching (today "The additions include one item per manual check the feature left pending…") is replaced by it, and S.5 item 2 says the same by reference to 3.3.
    - The rule names what counts as runnable here (launch the app on this host and check through MCP and screenshots, read the log, run a shell command, run the installer or setup) and gives a closed list of reasons a check may stay in `todo.md`: it needs another OS than this host's (macOS, Linux, Windows), hardware this host lacks (a camera body and the like), or the user's own photos or by-eye judgment. Every check left in `todo.md` carries a one-line "not runnable here: …" reason in its item.
    - The outcome of each candidate is defined and none can be dropped: passed whole (no `todo.md` item), passed in part (an item holding only the sub-points not run, each with its reason), failed (a bug item, see "Trade-offs and risks"), not runnable here (an item with the reason).
    - The place of the results follows the existing convention: `results.md` in the feature's plan folder (a `## Setup` section, then one section per check with `PASS` / `PARTIAL` / `FAIL`, what was seen and a "Not done:" line, as in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`), lessons in the feature's `learnings.md`, a guide sentence under `docs/agents/` that names the check as pending or unseen updated, and the PR body listing what was seen per check.
    - 3.6 and S.6 ask for the PR body to list the checks run with their outcome and the path of `results.md`, next to the todo headings added for the checks left, each with its reason; 3.6's "paths to commit" names the archived folder's `results.md` and `learnings.md` when they were written.
    - `SKILL.md`'s Notes list running these checks among the main agent's exceptions.
    - `.claude/agents/todo-curator.md` no longer says an agent cannot run a check. It returns the manual-check candidates as a list of their own, separate from the addition proposals, each with the check's steps and expected result, its platform, its source, a host classification with a one-line reason, the pasteable `todo.md` Markdown to use if the check is left, and the existing `todo.md` item covering it if any. It still writes nothing and still returns every candidate.
    - `.claude/agents/implementer.md` step 6 asks for a pending manual check to be recorded with what it needs (platform, input kind, material) so the wrap-up can classify and run it.
    - `.claude/skills/develop/README.md` matches: the 3.3 and S.5 table rows and the paragraph on how `todo.md` is written.
    - `git diff origin/main --stat` shows only those four files, `docs/agents/app.md` if the pointer bullet was added, and this plan's files; `todo.md` is unchanged by the step itself.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Docs only, everything in English. The edits to `SKILL.md` are in 3.3 (lines 564–594), the 3.6 body and paths lists (653–678), S.5 item 2 (762–766), the S.6 body list (793–801) and the Notes (885–900).
    - Keep the invariant the current text was written for: `strip-keep-scroll-on-rescan` (#528) and `view-menu` (#545) lost their checks because a free-form bullet was read as a one-off task. Every candidate must end as either a recorded result or a `todo.md` item; say so in both `SKILL.md` and `todo-curator.md`.
    - Write "this host", not "Windows". PR #1307 was run on a Mac, and the `screenshot` tool works on Windows and macOS but returns "not implemented" on Linux (`docs/agents/app.md`, "`render_to_image` captures the window behind the `screenshot` feature"). Have the main agent pass the host OS to `todo-curator` in its prompt so the classification does not assume one.
    - A `todo.md` item has one `- [ ]` per platform. A check with a box for this host and boxes for other platforms is run for this host's box only; the other boxes stay, and the Background says what was seen here, on which date, and names `results.md` by its post-archive path as a backticked path.
    - Do not copy the launch recipe into `SKILL.md`; point to it. It is in `docs/agents/app.md`: "Driving the window in a real-device run on Windows" (the MCP plus `screenshot` rule and what it cannot close, the Git Bash path rule, the pass 2 batch rule) and the rules after "The MCP companion answers from the view, one channel hop away" (a `--mcp-port` other than 41917, fresh `--index`, `--settings` and `--log`, `{"mcpEnabled": true}` through `--import-settings`, the previous PID gone before the next launch). Add to `app.md` at most one bullet in the first section saying the develop wrap-up runs these checks, if a pointer back is wanted.
    - State in the rule that a timed check needs a release build: the `screenshot` feature build is a debug build and `app.md` says to take no timing from it.
    - State that a wrap-up with no manual-check candidate ("0 manual checks" from `todo-curator`) starts no build and no app.
    - The run happens before the archive move (3.5 / S.5 item 5) so `results.md` moves with the folder. In Phase 3 `archive-plan.sh` commits only what `git mv` staged, so a new `results.md` lands under `_archived/` untracked; that is why 3.6's paths must name it. Check this reading against the script while editing. S.6 already passes the whole new folder.
    - `learnings-extractor` (3.2) runs before the checks, so a lesson from the run is not consolidated by it. The step decides between two wordings: the main agent appends the lesson to the feature's `learnings.md` only, or it also updates a guide sentence that names the check as pending or unseen (what #1307 did by hand). The second is the recorded precedent; do not re-run `learnings-extractor`.
    - How the main agent reaches the app's MCP endpoint is not settled here: the earlier runs used a script client against their own `--mcp-port`, and a session may also have a `handpic` MCP server configured at a fixed port. Write the rule so it does not depend on one of them, and record in `learnings.md` which one the text assumes.
    - `todo-curator.md`: change the frontmatter description, the opening paragraph, §1's list, §4 "Pending manual checks" and the Output section. Keep its format rules for the item (platform-neutral heading, Background with the archived plan path, `Files:`, one box per platform, merging into an existing item), now as the Markdown used only for what is left.
    - `README.md`: rows 199 and 209 and lines 575–590. The mermaid diagrams need a change only if the executor becomes a new agent (see "Trade-offs and risks").
    - The defaults written into this step for the open points below are: the main agent runs the checks; the build is started only when a candidate is runnable; synthetic mouse and keyboard input is not used; a failed check is filed and not fixed; a blocked check is filed with what blocked it. The user approved these defaults on 2026-11-27.

## Trade-offs and risks

- **Who runs the checks.** Default: the main agent, as the requirement says and with no new agent. It puts a build, an app launch and screenshots into the orchestrator's context, against the skill's "the main agent neither writes nor reads code", and needs an entry in the Notes' exceptions. Alternative: a new `device-checker` agent that takes the candidates, writes `results.md` and returns each outcome. It keeps the orchestrator thin, and costs a new agent file, a model choice, README's "eight agents" and diagrams, and probably new permissions in `.claude/settings.json`.
- **A local build against the standing rule to leave builds to CI.** Launching the app needs `cargo run --features screenshot` in `crates/app` on a host where parallel sessions have been killed for low memory. Default: build only when at least one candidate is runnable, one build at a time. Alternative: ask the user before the build, which adds a dialogue point to a wrap-up that has none today.
- **Synthetic input.** Key presses, drags and right-clicks were closed on Windows before with `mouse_event` and `SetCursorPos`, under a lock file, and it takes over the desktop the user and other sessions share. Default: not used, so a check that needs it stays in `todo.md` with that as its reason, which stretches "hardware or the user" to "input an agent must not synthesize". Alternative: allow it under the rules of `docs/agents/app.md`; more checks close, at the cost of the user's foreground window.
- **A check that fails.** Default: a bug item in `todo.md` with the reproduction from `results.md`, named in the PR body, not fixed in the wrap-up. In Phase 3 the steps are merged, so this is the only route. In Phase S the implementation is still unmerged, so the alternative is to stop before S.6 and ask the user, at the cost of another abnormal-case dialogue point.
- **A runnable check that is blocked** (the build fails, no test material is at hand, the port is held). Default: it stays in `todo.md` with what blocked it and is listed separately in the PR body. This is a fourth reason outside the three the acceptance criteria allow. Alternative: stop and ask the user.
- **Not taken: running the checks in the implementation step.** Running them in 2.2 / S.3 would catch a failure before the merge, but the request scopes the rule to the wrap-up, and `implementer` commits before local review. Only its record of the check changes here.
- **`planner.md` is not changed.** Plans may still write "Manually verified on Windows" as a Done-when line; the wrap-up now resolves it. Rewording how plans state such criteria is a possible follow-up.
- **Risk: a wrong PASS removes a check for good.** The check is no longer in `todo.md`, and the only review is after the fact through `results.md` and the PR body. That is why the body must list every check run with its outcome.

## Progress
