# Learnings

## Step 1

- The plan names `crates/native/...` paths; the crate had been renamed to
  `crates/app` by #1179, so the changes are in `crates/app/src/pager.rs` and
  `crates/app/src/main.rs`. An untracked `crates/native/target/` left over from
  before the rename sits in the worktree and was not committed.
- `docs/agents/app.md`'s status row paragraph describes `pager::position`'s
  format, so it got the selected count too (CLAUDE.md names neither and needed
  no change).
- The blank lens test lives in the backend (`a_blank_lens_is_no_lens`):
  `exif_key`, `groups_of` and `filter_offers` all read `Exif.lens`, so `None`
  there covers the filter menu and `get_view` alike.
- The `todo.md` entry for the DMG size was removed with the fix. If the
  warm-cache `Package` dispatch on the PR's branch is not green, restore it
  narrowed to "confirm on a warm-cache `Package` run" (plan, "Decisions").

## Deferred issues (todo candidates)

- Pending manual check (macOS, CI): dispatch `Package` on this PR's branch
  (`gh workflow run package.yml --ref <branch>`) with a warm cache and confirm
  both macOS legs (arm64, x86_64) produce their DMG with "Check the DMG"
  passing. Basis: Step 1's Done-when on `tools/package/macos.sh`'s new
  `-size`. The step's checkbox was ticked on the automated criteria. Files:
  `tools/package/macos.sh`, `todo.md`.

- (2026-11-21) The `Package` proof run on the PR branch (run 44757162994) was green on all legs, both macOS DMG legs included, so the DMG entry stays removed from `todo.md`.
