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

## Step 3: Rename `crates/backend` to `crates/engine`

- The Done-when asks `docs/agents/app-log.md` to mention the old
  `handpic_backend::*` targets while the same Done-when's leftover grep
  (`handpic[-_]backend`) must print nothing over the live files. The sentence
  was worded around the token: a log from a build before the rename "carries
  the same targets with `backend` in place of `engine`".
- No relative `backend/` path outside the plan's tokens this time: the
  `include_bytes!` paths in `crates/engine/src` point at `../../core/` and
  `../../app/`, which the directory move does not affect. `../backend` had
  three hits (the app's path dependency, `app:watch`'s `--watch` and
  `docs/agents/app.md`), all covered by the token.
- The root `Cargo.toml`'s `members` was kept in alphabetical order
  (`crates/cli`, `crates/core`, `crates/engine`), as it was before the replace.
- Both lock files were rewritten by `cargo metadata` without `--locked`: in
  each, the entry moved after `handpic-core`, and the app lock's dependency
  list of `handpic-app` reordered; no version moved. Both `--locked` runs then
  succeeded.
- The `native-qa` plan had been archived by the time of this step, so only
  `20261118-native-updater` got the "update engine" → "updater" rewording
  (its plan Step 3 title and approach, its learnings' Step 3 heading and
  three mentions).

## Step 4: Sweep the prose and the two leftover identifiers

- A scripted first pass over the live files took the exact phrases ("the
  native app", "Handpic's native app", "the native crate / tests / lock /
  clippy / checks / workspace / deb", "the backend('s)" not followed by `s` or
  `-`, "a backend change", and the `todo.md` heading prefixes `Native:`,
  `Native app:`, `Native app docs:` and `Backend:`, which became `App:`,
  `App docs:` and `Engine:`). `\s+` in the pattern crosses a plain line break
  but not a comment prefix (`//!`, `///`, `#`) or a Rust string continuation
  (`\` + newline), so those splits were left by the pass and fixed by hand
  from the `git grep -nIwiE 'native|backend'` list.
- The Step 2 leftover tokens had escaped in three ways the earlier greps could
  not see: a check name broken across a line (`` `clippy `` / `` native` `` in
  `docs/agents/app.md`), Windows paths with backslashes
  (`crates\native\target\debug\handpic.exe` in five `todo.md` checks), and
  text merged after Step 2 (`todo.md`'s `crates/native/src/main.rs` from the
  latency A/B, and `mise run native:run` back in two pending checks). All
  fixed; the `native:run` ones now say `mise run app:dev`.
- Two `todo.md` items cite another item by its heading
  (`### Native: real-device checks of the folder tree, ...` and "Native:
  real-device checks of the f focus mark, ..."); they were updated with the
  heading prefix. No fragment link anywhere targets a heading that holds
  `native` or `backend`, so retitling `app.md`'s "Backend" headings broke no
  link.
- Left on purpose: plan folder names (`native-*`, `gpui-backend-crate`), the
  `riffle-native` executable told as history, `self_update`'s github backend,
  graphics / Wayland / freedesktop backends, OS-native menus and dialogs,
  `ModalKey::Native`, `decode.rs`'s `native` variable, "Native resolution" in
  `faces.rs`, the job fixtures of `wait-pr-actionable.test.sh`, "Backend
  measurement" of the Tauri-era jpeg-view-only learnings, and the
  `NATIVE_INSTRUCTIONS` constant in `crates/engine/src/mcp.rs` (not one of the
  two identifiers the plan names, and not a `-w` hit).
- The MCP tool descriptions in `crates/engine/src/mcp.rs` that said "the native
  app does not have ..." now say "the app" / "Handpic"; they are sent to the
  client, not persisted.

## Deferred issues (todo candidates)

- None from Step 4.
