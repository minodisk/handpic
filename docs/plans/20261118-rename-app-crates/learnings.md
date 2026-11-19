# Learnings

## Step 1: Remove the legacy `.native` index migration

- The Done-when leftover check
  `git grep -nIE 'move_native_index|IndexMove|index-aside|\.native\b'` still
  prints one live hit after this step: `.github/workflows/ci.yml`'s
  `needs.native.result` (the CI job named `native`). It is not about the index
  and Step 2 renames it (`needs.native` → `needs.app`), so it is left alone
  here; the check holds over the live files once Step 2 lands.
- `todo.md`'s removed item had only one `---` separator next to it (after it;
  the item before it had none), so that one was kept to separate the icon item
  from the next one rather than removed.
- `datadir.rs`'s test helpers `put` and `read` went with the move tests; the
  packaging tests below them never used them.
- `mise run ci:local` for this step took about five and a half hours on this
  Windows host (started 01:38, `test native` began at 07:05 and then
  finished in minutes). It was not hung: it was a cold build of the native
  crate's clippy and test targets, and from 07:05 a second `ci:local` from
  another session was building on the same host at the same time. The Bash
  tool moved the run to the background after 180 s even with the maximum
  timeout, so its exit code came in a task notification.

## Step 2: Rename `crates/native` to `crates/app`

- The mise tasks renamed, listed from `mise.toml` on 2026-11-19: `dev`,
  `dev:screenshot`, `watch`, `release`, `check`, `bundle:macos`,
  `package:{macos,windows,linux}`, `sign`, `keygen` and `install:linux`, the
  same set the plan listed. `crates/app/mise.toml` only pins the toolchain and
  needed no change.
- `native:run` was the one `native:[a-z]` hit that is not a mise task: three
  pending manual checks in `todo.md` (and archived learnings) tell the user to
  run `mise run native:run`, a task that does not exist (`native:dev` is the
  debug run). The live ones were renamed with the prefix to `app:run`, keeping
  the leftover check empty; they are still wrong and recorded below.
- `git grep -nI 'native-app\.md)' -- docs/plans/_archived` also matches plain
  prose, not just link targets: `20261116-native-component-controls/plan.md`
  mentions `native-app.md)` twice in running text. Those two were changed to
  `app.md)` as well so the Done-when grep prints nothing; the eight real link
  targets `../../../agents/native-app.md` became `../../../agents/app.md`.
- The token `(fmt|clippy|test) native` with `\b` did not catch
  `$'clippy native\nclippy native screenshot\n'` in `local.test.sh` (the `\n`
  glues the `n` to `clippy`, so there is no word boundary); it was fixed by a
  plain replace of `clippy native screenshot`.
- `git mv crates/native crates/app` moved the ignored `target/` along with the
  tracked files on Windows (it renames the directory), so no leftover
  `crates/native/target` was left in this worktree.
- `cargo metadata` without `--locked` rewrote `crates/app/Cargo.lock` with the
  `handpic-app` entry moved ahead of `handpic-backend`; no other line changed,
  and the `--locked` run then succeeded.
- The token table missed a relative path: `crates/backend/src/update.rs`
  (added by the updater work after the plan was written) reads the updater's
  verifying keys with `include_bytes!("../../native/packaging/...")`. The
  first `ci:local` failed on it (`clippy handpic-backend`); it now points at
  `../../app/packaging/`. Grep `(\.\./|\b)native/` too, not only
  `crates/native`.
- Every `` `native` `` that named the CI job (in `CONTRIBUTING.md`,
  `.claude/skills/develop/README.md`, `docs/agents/app.md`, `todo.md` and the
  two in-flight plans) became `` `app` ``, history included, so a reader finds
  the job by its current name. The prose "the native app" was left for Step 4.

## Deferred issues (todo candidates)

- `mise run app:run` (formerly `native:run`) names no mise task. Basis: Step 2's
  leftover check (`native:[a-z]`). The pending manual checks in `todo.md` that
  say "`mise run app:run` or the built binary" / "or a debug build" should say
  `mise run app:dev`. Files: `todo.md` (the GPUI grayscale and title-bar folder
  name checks).
