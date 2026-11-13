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

# Todo item: drop the `.native` suffix from the native app's index and log folders at the switch-over

## Purpose

The GPUI native app (`crates/native`) keeps its index at
`<cache dir>/dev.minodisk.handpic.native/index.sqlite` and its log under a
`dev.minodisk.handpic.native` folder too. The `.native` suffix exists only so
the native app and the Tauri app never share one SQLite file (or one log
folder) while both run side by side during the port. Once the native app
ships in place of the Tauri app, the suffix has no reason left and both
folders should be `dev.minodisk.handpic`.

Nothing records that future change today. This plan adds one item to
`todo.md` so the rename, and the decision about the Tauri app's existing
index that comes with it, are not forgotten at the switch-over. It changes no
code. The user chose to cover both the index folder and the log folder in one
item, and not to edit the port plan's Step 10.

## Steps

- [x] Step 1: Add the `Native:` item about the `.native` suffix of the index and log folders to `todo.md`
  - Done when:
    - `todo.md` has one new `### Native: ...` item appended after the last existing item (`### Native: the per-folder EXIF filter sections in the filter menu (closes port Step 7)`), written in English, with a `#### Background` and a `#### TODO` heading.
    - Its background says why the suffix exists (the two apps must not share one index while both run during the port) and names what sharing breaks today, as `docs/agents/native-app.md`'s section "The native app keeps its own index (Inferred)" lists it: two processes on one file (the Tauri launch's `VACUUM` against a native scan's writes), two `EXTRACTOR_VERSION`s re-extracting each other's rows, and a native run's `reconcile` dropping the user's rows. It also says the log folder carries the same suffix.
    - It states the condition: the item is acted on when the native app replaces the Tauri app as the shipped app (the port's Step 10, the switch-over), not before.
    - It states the decision to make at that point: take over the Tauri app's existing index (the same folder) or rebuild it, naming the inputs (the taken-over rows were written under the Tauri app's `EXTRACTOR_VERSION` and hold `ratings`, `views`, `folders.last_viewed` and `embeddings`; a rebuild costs a cold pass 1 per folder and whatever the sidecars do not hold), without saying which is better.
    - It lists the files to update at that time: `crates/native/src/datadir.rs` (`index_path` and its test `the_default_is_the_native_folder_under_the_cache_dir`), `crates/native/src/logger.rs` (the log folder and its test), `CLAUDE.md`, `docs/agents/native-app.md` ("The native app keeps its own index"), `docs/agents/app-log.md` (the native log paths), and a check that `CONTRIBUTING.md`'s "The index" paths still hold (it has no `.native` mention). It notes the developer-only `dev.minodisk.handpic.native` folders are left behind.
    - `Plan:` points at `docs/plans/20261109-gpui-port/plan.md` (Step 10), in backticks.
    - `mise run ci` and `mise run lint` pass.
  - Implementation approach:
    - Only `todo.md` changes (plus this plan's tick and Progress entry). No code, no other docs, no edit to the port plan.
    - Check the facts above against the tree before writing (line numbers may have moved).

## Trade-offs and risks

- The port plan's Step 10 already rewrites `CONTRIBUTING.md`, `CLAUDE.md` and `docs/agents/**` for the native app but says nothing about the folder suffix. The todo item is the record; Step 10 is left alone at the user's choice.
- The recent Native items put the background directly under the `###` title; this one uses `#### Background`, as most of the file does. Either passes CI.

## Progress
