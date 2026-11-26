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

# Uninstall removes the update staging folder

## Purpose

An Inno Setup install stages its update setup in
`<cache dir>\dev.minodisk.handpic\update`, and only the next launch of the
app clears that folder. After an uninstall there is no next launch, so the
staged setup and its ZIP stay on disk (observed in the native-release rc dry
run, 2026-11-21, rc.2; plan:
`../_archived/20261121-native-release/plan.md`). The staging folder is not
user data. This plan has the uninstaller delete it, while the index, the
settings and the logs stay as they do today, and closes the implementation
part of the `todo.md` section "App: the Inno Setup uninstaller leaves the
update staging folder (native-release)", leaving only the real-machine check.

## Steps

- [x] Step 1: Delete the update staging folder on uninstall and update the
      rule and the todo item
  - Done when:
    - `crates/app/packaging/windows/handpic.iss` has an `[UninstallDelete]`
      section whose one entry removes `{localappdata}\dev.minodisk.handpic\update`
      and everything in it, and names nothing else (not the
      `dev.minodisk.handpic` folder itself, which holds `index.sqlite` and
      `logs\Handpic.log`, and nothing under the config dir, which holds
      `settings.sqlite`).
    - A test in `crates/app/src/datadir.rs` reads `windows/handpic.iss`
      and asserts it names `{localappdata}\<IDENTIFIER>\update`, so the
      script and `update_staging` cannot drift apart.
    - The Inno Setup rule in `docs/agents/app.md` ("Rule: the Inno Setup
      install's setup") no longer says `handpic.iss` has no
      `[UninstallDelete]`. It says what the uninstaller deletes (the staging
      folder, through `[UninstallDelete]`) and what it keeps (the index, the
      settings and the logs), that the launch still clears the folder too,
      and that the uninstall behaviour is not yet checked on a real machine
      (**Unverified**, pointing at the `todo.md` item). The 2026-11-21
      observation stays as the dated reason for the entry.
    - In `todo.md`, the section "App: the Inno Setup uninstaller leaves the
      update staging folder (native-release)" keeps a short background and
      one pending manual-check item only: on a real Windows machine, with an
      update staged by an Inno Setup install, uninstall from Settings → Apps
      and confirm `%LOCALAPPDATA%\dev.minodisk.handpic\update` is gone while
      `index.sqlite`, `settings.sqlite` and the logs remain; once checked,
      record the result in the rc dry run section of
      `docs/plans/20261109-gpui-port/packaging-updater.md` and replace the
      **Unverified** note in the `docs/agents/app.md` rule. The
      implementation instruction is gone from the item. No other `todo.md`
      section changes.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - The path. `datadir::update_staging` is
      `cache.join(IDENTIFIER).join("update")` with `IDENTIFIER`
      `"dev.minodisk.handpic"`, and `main.rs` passes it `dirs::cache_dir()`,
      which on Windows is `FOLDERID_LocalAppData`, Inno Setup's
      `{localappdata}`. The rc dry run measured the same folder
      (`%LOCALAPPDATA%\dev.minodisk.handpic\update`). The install is per
      user (`PrivilegesRequired=lowest`), so `{localappdata}` is the
      uninstalling user's.
    - The entry: `Type: filesandordirs` with
      `Name: "{localappdata}\dev.minodisk.handpic\update"`. Write the
      identifier literally, as `AppId` does in the same file; add a comment
      in the file's existing style saying what the folder is
      (`datadir::update_staging`, cleared at launch, which an uninstall never
      reaches) and that the index, settings and logs beside it are kept on
      purpose.
    - The test. Use the `packaging(file)` helper already in
      `datadir.rs`'s test module (the macOS plist and Linux desktop tests
      use it) and build the expected string from `IDENTIFIER`. The engine
      has a test of the same shape
      (`crates/engine/src/update.rs`, `include_str!` of `handpic.iss`).
    - `docs/plans/20261109-gpui-port/packaging-updater.md` is not changed
      in this PR: its rc dry run row "Uninstall from Settings removes it"
      is a dated measurement of rc.2 and stays true as history. The new
      result is added there when the manual check is done.
    - `[UninstallDelete]` entries are recorded in the uninstall log when a
      setup installs, so an install made by an older setup gains the cleanup
      only once a setup built from this change has upgraded it. Say so in
      the `todo.md` item so the manual check uses such a setup.
    - Follow the existing pending-manual-check items in `todo.md` for the
      item's wording (for example "Pending manual check, on Windows: ...").
    - No PR check runs the uninstaller: `Package`
      (`.github/workflows/package.yml`) runs on dispatch or from
      `release.yml` only. The PR's automated evidence is the `datadir.rs`
      test; the behaviour itself is what the `todo.md` manual check covers.
    - Commit as Conventional Commits, for example
      `fix(app): delete the update staging folder on uninstall`.

## Trade-offs and risks

- The `datadir.rs` test goes beyond the stated acceptance criteria. It is
  in the step because it is the only automated check a PR runs on this
  change, and it is a few lines over an existing helper. The user approved
  keeping it.
- Extending the `Package` workflow's Windows leg was not taken. That leg
  already installs and uninstalls the setup silently; it could create
  `%LOCALAPPDATA%\dev.minodisk.handpic\update\<file>` after the relaunched
  app is stopped (an Inno install's launch clears the folder), run the
  uninstall, and assert the folder is gone while `logs\Handpic.log`
  remains. That would be an end-to-end check, but the workflow does not run
  on a PR, and it is outside the files named for this work.
- An install made by a setup older than this change keeps the old uninstall
  log until it is upgraded, so uninstalling such an install directly still
  leaves the staging folder. Every update path runs a new setup over the
  install, so this closes itself on the first update; nothing is done for
  it.
- The identifier is written literally in the `.iss` rather than through an
  ISPP `#define` shared with `AppId`. A define would be a second change to
  a line whose comment says never to change it; the test covers the drift
  instead.
- The uninstaller deletes the folder for the user who runs it. The install
  is per user, so that is the user whose launch staged it.

## Progress

- (2026-11-26) Step 1 complete (the uninstall on a real Windows machine is pending as the todo.md manual check)
