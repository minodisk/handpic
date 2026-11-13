# Learnings

## Step 1

- `migrate::run` now delegates to `migrate_under(bases, identifier, rename,
  leftovers)`, the base-dir-independent seam. The old
  `the_current_identifier_is_not_a_migration` test called `run` on the real OS
  folders; with two legacy identifiers, `run(LEGACY_IDENTIFIERS[0])` would have
  migrated a developer's real `com.minodisk.riffle` folder into
  `com.minodisk.handpic`. Both that test and the new multi-legacy order test go
  through `migrate_under` on temp bases only. The macOS leftover removal moved
  behind a cfg'd `remove_leftovers` passed as the `leftovers` hook, under the
  same `all_moved` condition as before.
- `dev.minodisk.handpic` has the same length as `com.minodisk.handpic`, so the
  path tables in `docs/agents/app-log.md` kept their alignment.
- The native `logger.rs` derives its folder from `IDENTIFIER`; only its test
  literal named the old identifier.
- `mise run ci` runs only `lint` and `test`, which do not cover the separate
  `crates/native` workspace (the plan assumed they do). `mise run native:check`
  was run as well and passed (the `datadir`, `logger` and `settings` tests).
- The first `mise run ci` failed on clippy's `cloned_ref_to_slice_refs` for
  `&[root.clone()]` in the new tests; `std::slice::from_ref(&root)` fixed it.
- The worktree had no `node_modules`; `pnpm` is not on the Git Bash `PATH`, so
  `mise exec -- pnpm install --frozen-lockfile` was needed before `mise run fmt`.

## Deferred issues (todo candidates)

- **Pending manual check, Step 1, first launch on the development machine
  (Windows).** Not run by the agent: launching a build would migrate the real
  user data, and the GUI cannot be driven here. Steps: close any running
  Handpic, build and launch the app with the new identifier once. Expected:
  the data is under `%APPDATA%\dev.minodisk.handpic` and
  `%LOCALAPPDATA%\dev.minodisk.handpic`, no `com.minodisk.handpic` (or
  `com.minodisk.riffle`) folder remains there, no data-migration error dialog
  appeared, and the open folder, settings and ranking are intact. The step's
  checkbox was ticked on the automated criteria. Basis: the plan's
  "Verify during implementation if feasible"; `crates/app/src/migrate.rs`.
- **The in-flight `gpui-companions` plan still names
  `com.minodisk.handpic.native`** (`docs/plans/20261113-gpui-companions/plan.md`,
  two mentions of the native index folder). Left alone because it belongs to
  another session's in-flight plan; it now should read
  `dev.minodisk.handpic.native` (derived from `datadir::IDENTIFIER`, so no code
  of that plan is affected). Basis: the acceptance grep of this step.
