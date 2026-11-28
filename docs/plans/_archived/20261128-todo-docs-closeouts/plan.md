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

# todo.md docs closeouts

## Purpose

`todo.md` holds items that are stale (duplicates of other items, checks written
against a settings `Dialog`, a `textinput.rs` and status-row menus that no
longer exist, decisions already recorded in archived plans, a delegation whose
fix has merged) and items whose only work is a documentation correction. This
plan deletes the stale ones and makes the documentation corrections, so
`todo.md` lists only open work and the docs, the agent definitions and the doc
comments say what the code and `SKILL.md` do today. No code behavior changes:
the only `.rs` edits are doc comments.

## Steps

- [x] Step 1: Delete the stale `todo.md` items and fix the documentation-only items, removing each from `todo.md`
  - Done when:
    - Stale deletions in `todo.md`:
      - the whole section `### App: real-device checks of the settings as a full-window layer (native-settings-component Step 1)` is gone (its one open box, the drop with the settings open, stays as the fourth box of `### App: real-device check of the folder drop (gpui-folder-drop)`)
      - in `### App: real-device checks of the gpui-component adoption (...)`, the three boxes `(Step 1: init and theme)`, `(Step 2: `Input`)` and `(Step 2 catch-up: the tree's inline rename editor ...)` are gone with their sub-bullets, the Step 4, 5 (Windows, macOS, Linux), 6 and 7 boxes are unchanged, and the section's heading and intro paragraph no longer present `textinput.rs` / `Input` / the settings controls as something its boxes check
      - in `### Core: face/eye-aware focus check for culling`, the boxes "Faces rolled past 90 deg ..." and "Suggest the sharpest-eye frame within a burst group ..." are gone; the other six boxes, the mesh misfit label review among them, are unchanged
      - in `### App: real-device checks of Import… (gpui-folder-dialogs Step 5)`, no text says the maximum wait is delegated to the session `dlg-c4-import-watch`; the Background says the watcher now reports an ongoing burst every `MAX_WAIT` (#1358), and the open box still asks for the real-device run of the strip filling during an import
    - Doc fixes, each read back against its source of truth:
      - `docs/agents/app.md` and `CLAUDE.md` and the doc comments in `crates/app/src/settingsview.rs` say a refusal or failure shown in the settings (the refused capture key, the group threshold refusal, a failed switch, a folder dialog error) is shown by the settings view's own line (`SettingsView::status`, drawn in red under the settings body), not the window's status line (Decision 2)
      - `docs/humans/cameras.ja.md` matches `docs/humans/cameras.md` paragraph by paragraph and row by row: the first column of every table row is the same list in both files apart from the header cells
      - `.claude/agents/planner.md` § "Writing the acceptance criteria" asks a hands-on Done-when line to name its platform, input kind and material, in the terms of `.claude/agents/implementer.md` step 6
      - `CLAUDE.md`'s `src/eyes.rs` description and the doc comments of `EyesTicket` and `EyesCache::current` in `crates/app/src/eyes.rs` say that while a judgment runs only a `clear` supersedes it, and a newer request's id follows only a settle or a clear
      - `.claude/skills/develop/README.md` nowhere describes `NEEDS_APPROVAL` as a point where develop stops for the user; it describes the read-only check, the standing approval, the report, and the two stops that remain (the release PR, an `approval` line for a release-shaping path), the `/merge` flowchart's "needs approval" edge included (Decision 4)
      - the Git Bash `MSYS_NO_PATHCONV=1` rule is stated once, in `CLAUDE.md`'s Development section next to the `mise x --` rule (Decision 1), for every native-executable argument that starts with `/`, and `docs/agents/app.md`'s `ISCC.exe` entry and `.claude/skills/develop/SKILL.md` § Finishing the run point to it instead of restating the reason
    - The six `todo.md` sections of the doc fixes are gone: "the keymap capture refusal is drawn in the settings body ...", "cameras.ja.md lags behind cameras.md", "`planner.md` should have a plan name the input kind ...", "the eyes supersede rule is stale ...", "the develop README still describes `NEEDS_APPROVAL` ...", "state the Git Bash `MSYS_NO_PATHCONV=1` rule once ..."
    - `git diff` over `crates/**` shows comment lines only
    - `mise run fmt` leaves no diff, `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - General: every change is English except the body of `docs/humans/cameras.ja.md`. Edits are surgical: reword the sentences named below, leave the neighbours alone. A `todo.md` section is deleted whole (heading, Background, TODO) when its last box goes; a box is deleted with its sub-bullets. No remaining text refers to any deleted heading (checked by grep outside `_archived` and `review-history`).
    - Files: `todo.md`, `CLAUDE.md`, `docs/agents/app.md`, `docs/humans/cameras.ja.md`, `.claude/agents/planner.md`, `.claude/skills/develop/README.md`, `.claude/skills/develop/SKILL.md`, `crates/app/src/settingsview.rs` (doc comments), `crates/app/src/eyes.rs` (doc comments).
    - gpui-component adoption section (`todo.md`): reasons for the three deletions are that the settings became a full-window layer (`native-settings-component`), `crates/app/src/textinput.rs` no longer exists, and `Sort ▾` / `Filter ▾` moved to the strip header (dd365a02). The IME in a label name field stays covered by `### App: real-device check of the Japanese IME in the settings' label name fields`, and the rename editor's IME by `### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)`. Adjust the heading's parenthesis and the intro paragraph only as far as they announce the deleted boxes; keep the "Seen on Windows 11 on 2026-11-27" sentence, the plan path, the file list and the run instructions. Leave the Step 4 / Step 5 boxes' wording as it is (Decision 3).
    - face/eye section: the two decisions are in `docs/plans/_archived/20261010-mesh-roll/` (Decision) and `docs/plans/_archived/20261008-burst-keep-score/` (Decision 1); nothing needs copying.
    - Import section: the stale phrase is in two places, the Background's last sentence before "What is left open" and the open box's parenthesis. `crates/engine/src/watch.rs` now has `MAX_WAIT` (2 s), merged as d42e16fa (#1358). Keep the box: the strip filling during a run has still not been seen on a device.
    - Settings refusal line: in `crates/app/src/settingsview.rs`, `edit` stores the refusal in the view's `status` field (`Err(e) => self.status = Some(e.into())`), and `render` draws that field as the last child of the settings box, under the body, at 13 px in the theme's `danger` color. The window's status line is covered by the settings layer. Places: `docs/agents/app.md` about lines 2609, 2885, 2889 and 2994; `CLAUDE.md`'s Culling page description ("refused in the status line"); the doc comments on `commit_group` (about line 390), `edit` (about line 532) and `failed` (about line 634). `settingsmodal.rs:196` already says "The modal's status line" and needs no change.
    - `cameras.ja.md`: what differs from `cameras.md` today:
      - the second paragraph: the English has the "Handpic culls from the JPEG preview embedded in the RAW file ..." paragraph with the ten-year cutoff (released before 2016 as of 2026), the 1280 px floor, "not actively supported" and the yearly re-read; the Japanese still has the "2010 年より前" paragraph
      - 25 rows the English table no longer has: X-T1, X-T10, X-Pro1, X-E2, X-E1, X-A2, X-A1, X-M1, X100T, X100S, X30, X20, X10, XF1, XQ2, XQ1, X-S1 and the eight FinePix rows
      - the Fujifilm paragraph: it names X-A2 among the manual-focus samples, and ends "古い X シリーズと FinePix のカメラはどれも記録しません" where the English says none of the older X bodies at 1920x1280 records it
      - the RAF paragraph: "FinePix" and "1920x1280 から 2176x1448", where the English says the older X bodies' embedded JPEG is 1920x1280
      - the embedded JPEG size table: the `2176x1448` and `2048x1536` rows, and the `1920x1280` row's list, which the English has as X-H1, X-T2, X-T20, X-T200, X-T100, X-Pro2, X-E3, X-E2S, X-A7, X-A5, X-A3, X-A10, X100F, X70, XF10
      - Re-read the rest of both files side by side for anything else; the Japanese file keeps one paragraph per line. Verify the rows with a diff of the first table column of the two files.
    - `planner.md`: add to § "Writing the acceptance criteria" that a Done-when line needing a hands-on check names the platform, the input kind (what an agent can drive: MCP tools and screenshots, the log, a shell command, an installer; or what only the user can: a key press, a drag, the user's eye) and the material (a build, test photos, a camera body), the same terms as `implementer.md` step 6 and the wrap-up's "Not runnable here" reason in `.claude/skills/develop/SKILL.md` 3.3 / S.5. Do not restate the wrap-up's procedure.
    - Eyes supersede rule: `EyesCache::request` returns `None` while `in_flight` is set, so no ticket is issued while a judgment runs; `clear` stores `0` in `latest`. `docs/agents/app.md` § "An on-demand judgment is superseded through a shared atomic" already has the correct wording ("while one runs only a clear supersedes it; a newer request's id follows only a settle or a clear"); match it in `CLAUDE.md` (the `src/eyes.rs` description, about line 544) and in the two doc comments (about lines 16-18 and 60-62 of `crates/app/src/eyes.rs`).
    - develop README: the source of truth is `.claude/skills/develop/SKILL.md` § Merge and `.claude/skills/develop/references/pr-merge-lifecycle.md` § "The approval check reports; it does not gate": the main agent runs `check-merge-approval.sh` itself (read-only), starts `merger` with the approved flag whatever it printed, and reports every `approval` / `unclassified` line in the step's progress report; it stops only for the release PR (`release-please--branches--main`) and for an `approval` line on `.github/**`, `release-please-config.json` or `.release-please-manifest.json`; a `NEEDS_APPROVAL` that comes back anyway means the flag was not passed and is re-dispatched with it. Places in `README.md` that say otherwise:
      - the paragraph at about line 273-277 ("comes back to the main agent as `NEEDS_APPROVAL`, and that is a stop")
      - "Folding into one PR" risk 3 (about line 327) and the paragraph after the list that leans on it (about lines 332-335)
      - the step sequence diagram's `alt` block (about lines 496-503)
      - the `/merge` route comparison (about lines 874-875: "starts without the flag and stops at `NEEDS_APPROVAL`")
      - "Operational note: using it with `/goal`" (about lines 957-980), whose "thing to watch" and last mitigation rest on the stop
      - the "needs approval" edges back to the main agent in the two flowcharts (about lines 132-134 and 847); turn them into the release-shaping stop or drop them, and keep the Mermaid valid
    - `MSYS_NO_PATHCONV=1`: the rule is in two places today, the `ISCC.exe` entry of `docs/agents/app.md` (about line 860, which also names `MSYS2_ARG_CONV_EXCL='*'`) and the prose under the `/exit` command in `SKILL.md` § Finishing the run (about lines 1040-1042). Write the general rule once in `CLAUDE.md`'s Development section, in the Windows host paragraph next to the `mise x --` rule, keep the env prefix on the `herdr pane send-text` command line and in `tools/package/windows.sh` (not touched), and reduce the two existing sentences to the command-specific part plus a pointer.
    - This PR touches `.claude/**` and `CLAUDE.md`. `check-merge-approval.sh` flags `.claude/**`; under § Merge that is reported, not a stop.

## Decisions

Taken by the main agent without asking (develop § Deciding without asking):

1. The one `MSYS_NO_PATHCONV=1` rule lives in `CLAUDE.md`'s Development section, not a new `docs/agents/` guide: every session loads `CLAUDE.md`, and the `herdr` case was hit by a session that never reads `app.md`.
2. The "status line" correction covers every mention of the settings view's own line (`app.md` about 2609, 2885, 2889, 2994; `CLAUDE.md`'s Culling page; the `commit_group`, `edit` and `failed` doc comments), not only the keymap section: they all name the same `SettingsView::status`, and fixing one leaves the rest contradicting it.
3. The Step 4 / Step 5 boxes of the gpui-component adoption section keep their wording: the request deletes only Step 1, Step 2 and Step 2 catch-up, and rewording live checks is out of scope.
4. The `/merge` flowchart's "needs approval" edge in the develop README is fixed in the same pass: it is the same stale stop, and leaving it keeps the README contradicting itself.
5. The two comment-only `.rs` edits stay in this one PR, as the request asks; a flaky Windows app test is re-run if it fails.

Added later at the delegating session's request (the user's decisions):

6. Hand check 16 of `docs/plans/_archived/20261008-burst-keep-score/results.md` (`_DSC2445.ARW`, pitch -68 deg) is closed: the label stands and `crates/core/src/pose.rs` is not changed. If the face really looks up, -68 deg is right, and either way |-68| exceeds `GOOD_MAX_PITCH` (45 deg, `crates/engine/src/index.rs`), so the frame is out of the good-photo mark. Its box under `### Core: the review of the head-pose labels is still open` is deleted; the sheet review box stays.
7. `### Core: the time cost of pass 3's 0.6 floor is unmeasured on an idle host (candidates-floor)` is deleted whole: a todo that only waits for a future event (an idle host) is not kept.

## Trade-offs and risks

- **Comment-only `.rs` edits pull the app crate into the checks.** `mise run ci:local` and the PR run the `crates/app` clippy, tests and three-platform jobs, where the known flaky Windows tests can need a re-run.
- **`cameras.ja.md` is a translation.** The rows and numbers can be checked mechanically; the replaced policy paragraph is new Japanese prose and its wording is the user's to review in the PR.

## Progress

- (2026-11-28) Step 1 complete
