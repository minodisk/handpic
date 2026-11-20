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

# Rename the app crates (`crates/native` → `crates/app`, `crates/backend` → `crates/engine`)

## Purpose

The Tauri app is gone. `crates/native` (`handpic-native`) was named for "not
the Tauri web frontend", and `crates/backend` (`handpic-backend`) reads as "the
backend of a web frontend". Neither contrast exists any more, so the names
mislead a reader of the layout, the mise tasks, the CI jobs and the guides.
The last piece of code that still knows the old contrast, the migration of an
index out of the `<IDENTIFIER>.native` folder, goes with them.

### Decisions (made by the user, 2026-11-18)

| Old | New |
|---|---|
| `crates/native`, package `handpic-native` (bin `handpic`, unchanged) | `crates/app`, package `handpic-app` |
| `crates/backend`, package `handpic-backend`, `handpic_backend::` | `crates/engine`, package `handpic-engine`, `handpic_engine::` |
| mise tasks `native:*` (every one, including `native:keygen` and `native:sign`) | `app:*` |
| `docs/agents/native-app.md` | `docs/agents/app.md` |
| CI job `native`, cache keys `cargo-native-*` | job `app`, keys `cargo-app-*` |
| `ci:local` checks `fmt native`, `clippy native`, `clippy native screenshot`, `test native` | `fmt app`, `clippy app`, `clippy app screenshot`, `test app` |
| `not_in_native`, "{feature} is not implemented in Handpic's native app yet" | `not_in_app`, "{feature} is not implemented in Handpic yet" |
| `datadir::move_native_index` and the `<IDENTIFIER>.native` index migration | removed; an index left under `.native` is not picked up |
| Conventional Commit scopes `native`, `backend` | `app`, `engine` |

- `crates/app` was the Tauri app's path, and `handpic-app` its package, in the
  archived plans. That is accepted: the archives are dated and frozen.
- `engine` names the stateful machinery the app drives (the index, the scan
  passes, the sidecar writer, the watchers, the trash runs, the MCP server)
  against `handpic-core`'s stateless parsing and analysis. Renaming it changes
  the log targets written to `Handpic.log` from `handpic_backend::*` to
  `handpic_engine::*`; that is accepted.
- The index has lived at `<cache dir>/<IDENTIFIER>/index.sqlite` since #1156.
  The app never shipped with its index under `.native`, so only a developer's
  cache is affected by dropping the migration, and pass 1 rebuilds it.

### What does not change

`IDENTIFIER` (`dev.minodisk.handpic`), the Inno Setup `AppId`, the settings
keys, the bin name `handpic`, the package file names, and `LEGACY_IDENTIFIERS`.
`LEGACY_IDENTIFIERS` (`com.minodisk.handpic`, `com.minodisk.riffle`) has
nothing to do with the index: its one use is `settings.rs`'s `seed_sources`,
which seeds a new settings store once from a Tauri build's `settings.json`
under one of those folders. It stays.

### What is not rewritten

In any step: `docs/plans/_archived/**` (except the archived link targets Step 2
names), `docs/plans/review-history/**`, `CHANGELOG.md`, and the un-archived
plan folders `docs/plans/20261109-gpui-port/`,
`docs/plans/20261109-rename-to-handpic/` and
`docs/plans/20261010-burst-best-mark/`, which use `crates/app` for the Tauri
app and would contradict themselves after a replace.

The two in-flight plans that will still be executed,
`docs/plans/20261118-native-updater/` and `docs/plans/20261118-native-qa/`,
**are** rewritten, `plan.md` and `learnings.md` both, so their steps name
paths and tasks that exist and the wrap-up's learnings extraction does not
copy stale paths into `docs/agents/`. Their folder names stay (auto memory and
merged PRs cite them).

Below, "the live files" means `git ls-files` minus the folders and the file
named in the first paragraph of this section, minus this plan's own folder,
and minus binary files.

## Steps

- [x] Step 1: Remove the legacy `.native` index migration
  - Done when:
    - `crates/native/src/datadir.rs` no longer has `move_native_index`,
      `IndexMove`, `with_companions`, `rename_present`, or the tests of the
      move with the test helpers only they used. `index_path`,
      `settings_path`, `IDENTIFIER`, `LEGACY_IDENTIFIERS` and their tests are
      unchanged.
    - `open_index` in `crates/native/src/main.rs` goes straight to
      `datadir::index_path`; the log lines of the move
      (`index: moved in from the .native folder`, its aside variant,
      `failed to move the .native index`) are gone.
    - This prints nothing over the live files:
      `git grep -nIE 'move_native_index|IndexMove|index-aside|\.native\b'`.
    - The docs no longer describe the move: `CLAUDE.md`'s `datadir.rs`
      sentence, `docs/agents/native-app.md`'s "The native app's index" entry,
      `docs/agents/app-log.md`'s line about an earlier build's log under
      `dev.minodisk.handpic.native`, and the `todo.md` item
      "Native: real-device check of the index move out of the `.native`
      folder (remove-tauri Step 3)", removed whole.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - This step runs before the renames, so its paths are still
      `crates/native`. It goes first because it removes the tokens
      (`move_native_index`, `.native`, a `native` test variable) the later
      steps would otherwise have to steer around.
    - `datadir.rs`: `with_companions` and `rename_present` have no other
      caller, so they go too; in the test module check `put` and `read`
      (clippy's `-D warnings` on the test targets reports a helper left
      unused). Keep `INDEX_FILE` (used by `index_path`).
    - `main.rs`: remove the `if flag.is_none() && let Some(cache) = ...`
      block at the top of `open_index`, nothing else.
    - `CLAUDE.md`: the sentence becomes "places its index at
      `<cache dir>/<IDENTIFIER>/index.sqlite` (or `--index <path>`) and its
      settings store at ...", dropping the clause about `<IDENTIFIER>.native`
      and `move_native_index`.
    - `docs/agents/native-app.md`, "The native app's index": keep where the
      index lives, `--index`, the `IDENTIFIER` sentence, the "cannot be
      opened" sentence and the rule; drop the migration history, leaving at
      most one sentence that an index under an earlier folder is not picked
      up, worded so the leftover check prints nothing.
    - `todo.md` is edited directly here (the item is deleted, not curated):
      remove the heading, its paragraphs, the `Files:` line, the `#### TODO`
      list and one of the surrounding `---` separators.
    - `LEGACY_IDENTIFIERS` and `settings.rs` are not touched.
    - Commit and PR title:
      `refactor(native): remove the legacy .native index migration`.

- [x] Step 2: Rename `crates/native` to `crates/app` (package `handpic-app`, mise `app:*`, CI job `app`, guide `app.md`)
  - Done when:
    - `crates/app` holds the crate; `crates/native` no longer exists in the
      tree. The root `Cargo.toml` excludes `crates/app`; `.gitignore` ignores
      `crates/app/target`.
    - This prints nothing over the live files:
      `git grep -nIE 'crates/native|handpic[-_]native|native:[a-z]|native-app\.md|cargo-native-|needs\.native|(fmt|clippy|test) native'`
      and `git grep -nI 'native-app\.md)' -- docs/plans/_archived` prints
      nothing (no archived markdown link targets the old guide name). Any hit
      of `native:[a-z]` that is not a mise task name is checked and kept.
    - `IDENTIFIER`, `LEGACY_IDENTIFIERS` and
      `packaging/windows/handpic.iss`'s `AppId` are unchanged
      (`crates/app/src/datadir.rs` is a pure rename).
    - `mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path crates/app/Cargo.toml`
      succeeds, and the lock's diff is only the `handpic-native` entry
      leaving and the `handpic-app` entry appearing, no dependency version
      moved.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (`lint`, `test` on three platforms, the renamed `app` job on three
      platforms including the macOS `Handpic.app` bundle check, and `ci`).
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged.
    - Land this when no implementation PR that touches `crates/native` is
      open (the `native-qa` and `native-updater` plans are in flight); a
      branch cut before the rename must be rebased, and any file it adds under
      `crates/native/` must be moved by hand. The leftover check catches a
      recreated `crates/native/`.
    - `git mv crates/native crates/app`. On Windows this fails while a file
      under it is open (a running `handpic.exe`, rust-analyzer), so stop those
      first. If the ignored `crates/native/target` is left behind, move it to
      `crates/app/target` by hand or delete it.
      `git mv docs/agents/native-app.md docs/agents/app.md`.
    - Scripted replace over the live files. Replace these exact tokens only,
      never the bare word `native` (it also occurs in `alternative`,
      `ModalKey::Native`, `not_in_native`, `native_router`, "a native
      dialog", a `native` variable in `crates/core/src/decode.rs`, and
      archived folder names such as `20261114-native-app-icon` that live docs
      cite):
      - `crates/native` → `crates/app`
      - `handpic-native` → `handpic-app` (not in `Cargo.lock`; see below)
      - every mise task name `native:<name>` → `app:<name>` (list them from
        `mise.toml`'s `[tasks."native:..."]` at the time of the step; on
        2026-11-18 they include `dev`, `dev:screenshot`, `watch`, `release`,
        `check`, `keygen`, `sign`, `bundle:macos`, `package:{macos,windows,linux}`
        and `install:linux`)
      - `native-app\.md` → `app.md` (with the dot, so `native-app-icon` stays)
      - `cargo-native-` → `cargo-app-` (cache keys in
        `.github/workflows/ci.yml` and `package.yml`)
    - By hand, because they are bare words:
      - `.github/workflows/ci.yml`: the job key `native:` → `app:`,
        `needs: [lint, test, native]`, `needs.native.result`, the step name
        and comments. Branch protection requires only the `ci` job, so the job
        rename needs no settings change.
      - `.github/workflows/package.yml`: comments that name "ci.yml's
        `native` job".
      - `tools/ci/local.sh` and `tools/ci/local.test.sh`: the check names
        `fmt native` / `clippy native` / `clippy native screenshot` /
        `test native` → `fmt app` / `clippy app` / `clippy app screenshot` /
        `test app`, the variables `native_rust`, `fmt_native`, `native`,
        `native_clippy`, the `case` arms and the comments. Keep the `case`
        arm order in `run_check` and `main`: the app arms must stay ahead of
        the generic `"clippy "*` / `"test "*` arms.
      - `tools/package/set-version.sh`: `crate="handpic-native"` (covered by
        the token replace; check it).
      - `CONTRIBUTING.md`, `.claude/skills/develop/README.md` (the list of CI
        jobs), `.claude/agents/{implementer,planner,pr-check-fixer}.md`,
        `docs/agents/*.md`, `todo.md`, `CLAUDE.md`: mentions of the `native`
        job and of the `ci:local` check names.
      - The archived plans that link to the guide
        (`git grep -nI 'native-app\.md)' -- docs/plans/_archived`): change
        only the link target `../../../agents/native-app.md` to
        `../../../agents/app.md`, leaving the link text. lychee checks
        `docs/**/*.md` and would fail on them otherwise. The Tauri removal
        edited archived links the same minimal way (#1158).
      - `docs/agents/app.md`: the title follows from the token replace; a
        heading whose anchor another guide links to (e.g.
        `#the-package-workflow-builds-the-distributables-inferred` from
        `docs/agents/github-actions-workflows.md`) must not change in this
        step.
    - `crates/app/Cargo.toml`: `name = "handpic-app"`. Do not text-replace the
      name in `crates/app/Cargo.lock`: lock entries are sorted by name and
      `handpic-app` sorts before `handpic-backend`, so let cargo rewrite it
      (`mise exec rust@1.98.1 -- cargo metadata --format-version 1 --manifest-path crates/app/Cargo.toml`
      without `--locked`), then confirm with `--locked` and read the diff.
    - Left for Step 4: prose such as "the native app" and the identifiers
      `not_in_native` and `native_router`. Left for good: the job names in
      `.claude/skills/pr/scripts/wait-pr-actionable.test.sh` (fixture data,
      not a reference to the workflow). The Rust module paths do not change
      (the bin is `handpic`, so the app's log targets `handpic::*` and
      `env!("CARGO_CRATE_NAME")` in `src/logger.rs` stay as they are).
    - Because the PR changes `mise.toml` and `tools/ci/`, `ci:local` selects
      every check. If the host runs out of memory, record it in
      `learnings.md` and run the printed checks one at a time.
    - The first CI run is a cold build of the app on every platform (the
      cache path and keys change). Watch the `app` job's timeout and record
      the cold time in `learnings.md`.
    - Commit and PR title: `refactor(app): rename crates/native to crates/app`.

- [x] Step 3: Rename `crates/backend` to `crates/engine` (package `handpic-engine`, `handpic_engine::`)
  - Done when:
    - `crates/engine` holds the crate; the root `Cargo.toml` lists it in
      `members`; `crates/app/Cargo.toml` depends on
      `handpic-engine = { path = "../engine" }`.
    - This prints nothing over the live files:
      `git grep -nIE 'crates/backend|handpic[-_]backend|\.\./backend'`
    - `cargo metadata --locked --format-version 1` succeeds at the root, and
      `mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path crates/app/Cargo.toml`
      succeeds; both lock diffs are only the renamed entry and its mentions in
      dependents' lists.
    - `release-please-config.json`'s `extra-files` names
      `crates/engine/Cargo.toml`.
    - `docs/agents/app-log.md` names the targets `handpic_engine::*` and says
      in one sentence that a log written by a build before this rename
      carries `handpic_backend::*`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged (paths below are `crates/app`).
    - `git mv crates/backend crates/engine`, then the scripted replace over
      the live files with the tokens `crates/backend` → `crates/engine`,
      `handpic-backend` → `handpic-engine` (not in the two `Cargo.lock`s),
      `handpic_backend` → `handpic_engine`, and `../backend` → `../engine`
      (the path dependency, `app:watch`'s `--watch ../backend` in
      `mise.toml`, and its mention in `docs/agents/app.md`). Never the bare
      word `backend` (`wayland-backend`, "the Vulkan and GL backends", the
      archived folder `20261109-gpui-backend-crate` that live docs cite).
    - Regenerate both lock files with cargo, not by text replace
      (`handpic-engine` sorts after `handpic-core`), and read the diffs.
    - `tools/ci/local.sh` and `local.test.sh`: `root_packages`, the path
      `case` arms (`crates/backend/*`), the `handpic-backend` strings and the
      comments.
    - `crates/app/src/logger.rs`: its tests and doc comments use
      `handpic_backend::index` / `handpic_backend::photolab` as example
      targets; the replace keeps them true.
    - The in-flight updater plan calls its new module "the update engine";
      after the replace it would read "the update engine in
      `crates/engine`". Reword it there to "the updater" (or similar) so the
      two uses of "engine" do not collide.
    - As in Step 2, `ci:local` selects every check here.
    - Commit and PR title:
      `refactor(engine): rename crates/backend to crates/engine`.

- [x] Step 4: Sweep the prose and the two leftover identifiers ("the native app" → "the app", "the backend" → "the engine")
  - Done when:
    - In the live files, "the native app" and its variants ("the native
      crate", "the native tests", "the native lock", "Handpic's native app",
      ...) read "the app" / "the app crate" / ..., and "the backend" / "the
      backend's" naming the crate read "the engine" / "the engine's".
      `git grep -nIwE 'native|backend'` over the live files lists only uses
      that mean something else, each one checked: an OS-native thing ("a
      native dialog", "native resolution"), `ModalKey::Native`, the `native`
      variable of `crates/core/src/decode.rs`, a graphics or Wayland backend,
      archived plan folder names, and history told as history.
    - `not_in_native` is `not_in_app` and `native_router` is `app_router`,
      with their tests' names; the refusal text reads
      "{feature} is not implemented in Handpic yet", and the test that
      compares it passes.
    - `CLAUDE.md` says in one clause what separates `handpic-engine` from
      `handpic-core` (the stateful machinery the app drives, against the
      stateless parsing and analysis).
    - `README.md` / `README.ja.md` and each changed `docs/humans/<name>.md` /
      `<name>.ja.md` pair are changed together and say the same thing.
    - Fragment links into `app.md` still resolve (lychee's
      `--include-fragments`).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 to 3 are merged.
    - This step is read, not scripted: about 150 "the native app" and 90 "the
      backend('s)" mentions, most in `todo.md` and `docs/agents/app.md` /
      `shared-crates.md`, the rest in `CLAUDE.md`, `CONTRIBUTING.md`,
      `tools/ci/`, `tools/package/`, `.github/workflows/`, `mise.toml`
      descriptions, `.claude/`, `crates/app/packaging/` (`NOTICE-gpui`'s first
      line, `handpic.iss`'s comment), the two in-flight plans, and code
      comments in `crates/app/src` and `crates/engine/src`. A scripted first
      pass of the exact phrase "the native app" → "the app" is fine if every
      other hit is then read. `todo.md` headings prefixed "Native:" are part
      of the sweep.
    - The refusal text is what an MCP client reads from the tools
      `companion.rs` refuses; it is not persisted. `CLAUDE.md` names
      `not_in_native`; update it there.
    - Do not touch the wording of `.claude/skills/release/SKILL.md` and
      `.github/workflows/release.yml` beyond the phrase itself ("the native
      release flow" → "the app's release flow").
    - Commit and PR title:
      `docs: call the app and the engine by their new names` (the two
      identifier renames and the refusal text ride along as a separate
      `refactor(app)` commit inside the PR).

## Trade-offs and risks

- Steps 2 and 3 could be one PR. They are split so each diff is one token
  table and either can be reverted alone.
- The mise prefix could have been dropped instead of renamed (`dev`, `watch`,
  `release`). Not taken: `check` and `release` would read as whole-repo tasks
  next to `lint`, `test` and `ci`, and the app is still a separate workspace
  those do not cover.
- After Step 1, a developer machine that still holds an index under
  `<cache dir>/dev.minodisk.handpic.native/` keeps that folder untouched and
  unused; the app builds a fresh index in the plain folder.
- After Step 3, greps of `Handpic.log` for `handpic_backend::` find only lines
  written before the rename.
- The three stale un-archived plans (`20261109-gpui-port`,
  `20261109-rename-to-handpic`, `20261010-burst-best-mark`) keep
  `crates/native` and `crates/backend`; archiving them is outside this plan.
- Steps 2 and 3 change `mise.toml` and `tools/ci/`, so `ci:local` runs every
  check on this host, close to the full root build that has exhausted its
  memory before.
- A wrong `Cargo.lock` is not caught by the PR's checks (only the manual
  Package workflow builds with `--locked`), hence the `cargo metadata --locked`
  line in Steps 2 and 3.
- In-flight work on `crates/native` (the `native-updater` plan's remaining
  steps) conflicts with Step 2 if a PR is open across it; land Step 2 between
  their PRs.
- Outside the repository: the user's auto memory (`gpui-only.md` says
  "Handpic is crates/native only") goes stale; the wrap-up updates it.
- Local leftovers after the pull: an ignored `crates/native/target` stays on
  every checkout and worktree that had one; delete it or move it to
  `crates/app/target`.

## Progress

(2026-11-19) Step 1 complete
(2026-11-19) Step 2 complete
(2026-11-20) Step 3 complete
