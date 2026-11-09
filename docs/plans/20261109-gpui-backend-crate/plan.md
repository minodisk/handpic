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

# Extract the Tauri-free backend into a shared `crates/backend` library

## Purpose

This is milestone Step 4 of the GPUI port
(`../20261109-gpui-port/plan.md`). `riffle-app` (`crates/app`) is a bin-only
crate (`main.rs`, no `lib.rs`), so `crates/native` cannot reuse its SQLite
index, sidecar writer, trash runs, keymap, EXIF formatting or PhotoLab lookup;
the port's Steps 5-8 need all of them. This plan moves the modules of
`crates/app/src` that do not depend on `tauri` into a new library crate,
`riffle-backend` (`crates/backend`), that both apps depend on, then `watch`
behind a callback seam, with no behavior change to the shipped Tauri app. From
then on a judgment-logic change lands once, in the library, for both apps.

**`perf.rs` and `ai.rs` are out of scope, by the user's decision.** The native
app needs them only in the port's Step 9. `ai.rs`'s real Tauri coupling is eight
touches, and moving it would bring `reqwest`, `rustls` and `ring` (about 60
packages and a C/asm build on every cold native CI run) into the native lock
with no consumer. Their seams are designed in Step 9's plan, when a consumer
exists.

### What the investigation found (2026-11-09, `origin/main` at `af45035d`)

The parent plan's counts were textual and predate the prefetch and migration
work. The real couplings today, from `grep` of `tauri` and `crate::` in each
module plus a read of the touches:

| Module | Lines | `#[test]` | `tauri` | Depends on (`crate::`) | Notes |
|---|---|---|---|---|---|
| `exif.rs` | 705 | 22 | 0 | — | `riffle_core::arw`, serde |
| `shortcuts.rs` | 1251 | 50 | 0 | — | serde, serde_json, log |
| `photolab.rs` | 710 | 20 | 0 | — | rusqlite, log |
| `index.rs` | 6550 | 89 | 0 | `exif` | rusqlite, serde, serde_json, log; 5 `pub(crate)` items |
| `sidecar.rs` | 2321 | 31 | 0 | `index::{lock, Index}`, `photolab` | 2 `pub(crate)` |
| `trash.rs` | 1975 | 42 | 0 | `index::{SidecarStat, lock}`, `sidecar::SidecarFormat`; **tests** use `commands::{read_listing, stat_sidecars}` | `trash` crate; macOS `objc2-foundation`, `percent-encoding` |
| `output.rs` | 353 | 22 | 2 | `commands::canonicalize` (a 3-line fn) | the `outputs_of` command wrapper is the only Tauri code |
| `diagnostics.rs` | 119 | 4 | 1 | — | `log_frontend` command; the `invariant!` macro uses `$crate::diagnostics::invariant_line` |
| `watch.rs` | 330 | 6 | 4 | `index::lock`, `sidecar::SidecarFormat`, `treewatch::settle` (a one-liner; `treewatch` depends back on `watch`) | `Watch::spawn(AppHandle)` emits `folder-changed`; `set` / `release_under` take `&AppHandle` and read `app.state::<Watch>()` |
| `perf.rs` | 461 | 10 | 3 | — | stays until the port's Step 9 |
| `ai.rs` | 454 | 6 | 1 (`use`) | `index::{lock, with_suffix}` | in fact eight touches; stays until the port's Step 9 |
| `migrate.rs` | 454 | 13 | 1 (comment) | — | computes Tauri's identifier-based data dirs; app-specific, **stays** |

`commands::read_listing` / `stat_sidecars` / `Listing` and `folders::Media` /
`is_file` are Tauri-free file-system code that `trash.rs`'s tests use (and
`commands.rs` uses for `scan_folder`), so moving `trash.rs` means moving them
too (Step 3). `canonicalize` is the same case for `output.rs`.

Total `#[test]` in `crates/app/src`: **498**. The moved modules carry 92
(Step 1), 120 (Step 2), 68 (Step 3) and 6 (Step 4).

Lock facts: the root `Cargo.lock` (648 packages) already holds every
dependency the backend needs at the versions the app uses (`rusqlite 0.40.2`,
`libsqlite3-sys 0.38.2`, `notify 8.2.0`, `trash 5.2.9`). The native lock (642
packages) already has `serde 1.0.229`, `serde_json 1.0.154`, `log 0.4.35`,
`chrono 0.4.45`, `windows 0.61.3` / `0.62.2`, `percent-encoding`, and
`dirs`; it lacks `rusqlite`, `libsqlite3-sys`, `notify` and `trash`. No version
conflict is expected: nothing GPUI pins shares a major with what the backend
brings except `windows 0.62`, which already agrees.

Open work next to this plan: the rename-to-handpic plan
(`../20261109-rename-to-handpic/plan.md`) renames the crates to `handpic-*`
in its Step 3 and edits comments in `exif.rs`, `sidecar.rs`, `perf.rs`,
`diagnostics.rs` and `watch.rs` in its Step 2; the prefetch plan is
archived; no non-release PR is open (`#928` is release-please's).

### Rules every step follows

- **A move is a move.** Each file goes over with `git mv`; the only content
  edits are `crate::x` → `crate::x` (unchanged when `x` moves too) or
  `riffle_backend::x`, `pub(crate)` → `pub` where the app still uses the
  item, and `use tauri::…` removals the seam needs. No reformatting, no
  comment edits, no renames of items, so git's rename detection stays above
  its threshold and a concurrent PR's hunk on the old path rebases onto the
  new one.
- **The app keeps its paths.** `crates/app/src/main.rs` gains
  `use riffle_backend::{exif, index, …};` next to its `mod` lines, so every
  `crate::index::…` path in `commands.rs`, `rank.rs`, `mcp.rs` and the rest
  resolves unchanged (a `use` at the crate root is a name in the root module).
  Call sites change only where a signature changes (Step 4).
- **The test count is compared, not assumed.** Before the move, on `main`:
  `cargo test -p riffle-app -- --list 2>/dev/null | grep -c ': test$'`
  (and the `Doc-tests` line of `cargo test -p riffle-app`). After:
  the same for `-p riffle-app` plus `-p riffle-backend`; the two sums must
  be equal and are written in `learnings.md` under the step. `mise run ci`
  (`cargo test` at the root) runs the backend's tests because it is a
  workspace member.
- **Both toolchains build it.** The root workspace is on Rust 1.97
  (`mise.toml`); `crates/native` on 1.98.1. The backend is `edition = "2021"`
  like `crates/app` / `crates/core`, so the moved code compiles and formats
  exactly as before (edition 2024 would force `unsafe extern` and rustfmt
  changes into a move PR). Each step runs `mise run ci` and `mise run
  native:check` (with `C:\Users\daisu\AppData\Local\bin\NASM` on `PATH`).
  Clippy lints only workspace members, so `native:check` compiles but does not
  lint the backend; the root `cargo clippy --all-targets -- -D warnings` does.
- **Locks.** The root `Cargo.lock` diff must be the new `riffle-backend`
  package entry plus dependency-list moves between `riffle-app` and
  `riffle-backend`; no version changes (check `git diff Cargo.lock` in the
  PR; never run `cargo update`). `crates/native/Cargo.lock` gains the
  backend's new dependencies through `cargo check`, and the PR body lists
  the packages that were added and any that became duplicates.
- **Concurrent `crates/app` changes.** Before branching a step, check
  `gh pr list` and `git worktree list` for work on the files it moves. Keep a
  move PR open for hours, not days: open it, get it reviewed, merge the same
  day. If another PR touching a moved file lands first, rebase; git carries
  the hunk to the new path when rename similarity holds (the "a move is a
  move" rule keeps it high), and when it does not, re-apply the hunk by hand
  at the new path and say so in the PR. If this plan's PR lands first, the
  other PR rebases the same way (the `pr-conflict-resolver` agent handles
  it). The handpic rename's Step 2 comment edits are the likely case.
- **Manual smoke check of the Tauri app** per step (`mise run tauri:dev`,
  a folder under `D:\Photos\tests\`), named in each step, because no test
  covers the Tauri wiring of moved state. The result goes in `learnings.md`.
- **Docs move with the code.** `CLAUDE.md`'s layout paragraph gets the
  moved module's sentence moved into a `crates/backend` sentence, and the
  `docs/agents/*.md` path mentions listed per step are updated. No
  `.claude/agents/*.md` names these paths (checked); the merge skill's test
  names `crates/app/src/main.rs`, which does not move.

## Steps

- [x] Step 1: Scaffold `crates/backend` with `exif`, `shortcuts` and `photolab`, make both apps depend on it, and prove the native link
  - Done when:
    - `crates/backend/Cargo.toml`: package `riffle-backend`, `version =
      "1.10.0"` (the workspace's), `edition = "2021"`, a library (`src/lib.rs`
      with `pub mod exif; pub mod photolab; pub mod shortcuts;`); dependencies
      `riffle-core = { path = "../core" }`, `serde` (derive), `serde_json`,
      `log`, `rusqlite = { version = "0.40", features = ["bundled"] }` — the
      same spec lines as `crates/app/Cargo.toml`. The root `Cargo.toml`
      `members` gains `"crates/backend"`. `release-please-config.json`
      `extra-files` gains the backend's `Cargo.toml` `$.package.version` entry
      (the `Cargo.lock` entry covers it already through the source-less
      filter).
    - `crates/app/src/exif.rs`, `shortcuts.rs`, `photolab.rs` are moved with
      their tests; `main.rs` replaces the three `mod` lines with
      `use riffle_backend::{exif, photolab, shortcuts};`; `crates/app` keeps
      building with no other diff than visibility (`pub(crate)` → `pub`) where
      needed (none expected: all three modules have 0 `pub(crate)` items).
    - `crates/native/Cargo.toml` gains `riffle-backend = { path = "../backend" }`;
      `crates/native/Cargo.lock` is updated by `cargo check` only. The link
      proof: the native status line (`index/total  file name  WxH
      orientation`) appends the shutter, aperture and ISO labels of
      `riffle_backend::exif::exif(&arw.shot)` (the `Arw` that
      `reader::read_preview` already returns carries `shot`), each `-` when
      `None`. That is the smallest visible use that exercises the library at
      run time and compiles its serde derives; it adds no UI beyond the pager's
      one text line.
    - `mise run ci` passes; `mise run native:check` passes; the test count
      rule holds (expected: 498 total, 92 of them now in `riffle-backend`).
      The root `Cargo.lock` diff is the new package entry and the moved
      dependency lines only.
    - Tauri smoke check: open a folder, the meta pane shows the EXIF rows;
      open the settings' Shortcuts tab and rebind one key, it takes effect;
      write one `.dop` for a file in a PhotoLab-registered folder and confirm
      PhotoLab opens it without a virtual copy (skip the PhotoLab part and say
      so when no registered folder is at hand; `photolab.rs`'s 20 tests cover
      the lookup).
    - Docs: `CLAUDE.md` layout paragraph gains a `crates/backend` sentence
      (the Tauri-free backend both apps depend on; a root-workspace member,
      used by `crates/native` by path and so compiled twice like
      `riffle-core`) holding the three modules' descriptions moved out of the
      `crates/app` sentence; `docs/agents/tauri-app.md` lines 162 and 193
      (`shortcuts.rs`) and `docs/agents/photolab.md` lines 4 and 231
      (`photolab.rs`) point at `crates/backend/src/…`; `CONTRIBUTING.md`'s
      native subsection says the native crate depends on `crates/backend`
      and that a backend change is checked by both `mise run ci` and
      `mise run native:check`.
  - Implementation approach:
    - Dependency order: these three have no `crate::` dependency, so they go
      first and the crate's shape (lib, edition, version, member) is settled
      on the smallest diff.
    - Check that `cargo fmt --all` at the root already formats the backend
      (a member) and that `lint`'s native fmt line needs no change.
    - Record in `learnings.md`: how long the native CI job took with the
      backend (rusqlite's bundled SQLite compiles once per lock change),
      the list of packages the native lock gained, and whether clippy under
      1.98.1 said anything about the backend (it should not, as a non-member).
    - Crate name: `riffle-backend` matches today's `riffle-core` /
      `riffle-app`; the handpic rename's Step 3 renames it with the others
      (that plan's Step 3 bullet should be told to include it — note it in
      this PR's description).
- [x] Step 2: Move `index` and `sidecar`
  - Done when:
    - `crates/app/src/index.rs` and `sidecar.rs` are moved with their tests
      (89 + 31); `lib.rs` gains `pub mod index; pub mod sidecar;`; `index.rs`'s
      `use crate::exif::…` stays as is (both in the backend now);
      `sidecar.rs`'s `use crate::index…` / `use crate::photolab` likewise.
      The 5 + 2 `pub(crate)` items become `pub` where `commands.rs`, `ai.rs`,
      `rank.rs`, `mcp.rs`, `trash.rs`, `watch.rs`, `datefolder.rs`,
      `rename.rs`, `foldersidecars.rs`, `import.rs`, `sequence.rs`,
      `treewatch.rs`, `update.rs` or `main.rs` use them; `main.rs` gains
      `use riffle_backend::{index, sidecar};`.
    - The backend's `Cargo.toml` adds nothing (rusqlite, serde, serde_json
      and log are there); `crates/app/Cargo.toml` keeps `rusqlite` only if
      something left in the app still uses it directly (check with `grep
      rusqlite crates/app/src`; `photolab.rs` and `index.rs` were its users).
    - `mise run ci` and `mise run native:check` pass; the test count rule
      holds (212 in the backend); the root lock diff is dependency-line moves
      only.
    - Tauri smoke check: open a folder with an existing index (the scan is
      skipped for unchanged rows), rate a file and flag one, confirm the XMP
      and `.dop` sidecars are written (format `Both`), quit and reopen at the
      same file (`last_viewed`), and `Clear Cache` from the settings works.
    - Docs: `CLAUDE.md` moves the `src/index.rs` and `src/sidecar.rs`
      sentences into the `crates/backend` sentence; `docs/agents/tauri-app.md`
      lines 1058 and 1320 (`index.rs`), `docs/agents/photolab.md` line 5
      (`sidecar.rs`) and `docs/agents/raw-metadata-parsing.md` line 107
      (`index.rs`) point at the new paths.
  - Implementation approach:
    - Assumes Step 1 merged. `index.rs` is 6550 lines; the diff must be a
      rename with visibility hunks only, so the review is the hunks.
    - `index.rs` has no Tauri data-dir coupling: `Index::open(&path)` takes
      the path `main.rs` computes from `app.path()`, which stays in the app.
    - `sidecar::Writer::spawn(index, log_fn)` is already callback-shaped;
      nothing to abstract.
- [x] Step 3: Move `trash` with the listing helpers, `output` with `canonicalize`, and `diagnostics` without its command
  - Done when:
    - New `crates/backend/src/listing.rs` holds, moved from `commands.rs`
      and `folders.rs`: `Listing`, `read_listing`, `stat_sidecars`, `Media`
      and `is_file` (all `pub`), plus `canonicalize`; the Tauri commands in
      `folders.rs` and `commands.rs` call them through
      `riffle_backend::listing::…` (or `use` lines at the top of those
      files). `commands.rs`'s `#[cfg(test)]` `list_arw_in` / `list_folder_in`
      helpers stay in `commands.rs` and call the moved functions.
    - `trash.rs` is moved with its 42 tests, its test `use` pointing at
      `crate::listing::{read_listing, stat_sidecars}`; the `trash` crate
      dependency and the macOS `objc2-foundation` / `percent-encoding`
      target dependencies move to the backend's `Cargo.toml` with the same
      spec lines (the app keeps them only if another app module still uses
      them; check with grep).
    - `output.rs` is moved minus the `outputs_of` command, which stays in
      `crates/app/src/output.rs` as a thin file (the command and its
      `spawn_blocking` call into `riffle_backend::output::outputs_in`). Its 22
      tests go with the logic.
    - `diagnostics.rs` is moved minus `log_frontend`, which stays in the
      app's `diagnostics.rs` next to the `pub use` lines the app needs. The
      `invariant!` macro becomes `#[macro_export]` in the backend with a
      `pub use crate::invariant;` inside `riffle_backend::diagnostics` so the
      app's `crate::diagnostics::invariant!(…)` call sites compile unchanged;
      `invariant_line` becomes `pub`. `install_panic_hook` moves.
    - `mise run ci` and `mise run native:check` pass; the test count rule
      holds (280 in the backend).
    - Tauri smoke check: Move to Trash… two files with sidecars, undo, redo;
      Delete Permanently… one file; the strip's `Output` filter on a folder
      with an `Output` subfolder; `Riffle.log` shows a `panic:` line when a
      corrupt `.ARW` is opened (the core catches it; the hook still logs).
    - Docs: `CLAUDE.md` moves the `trash.rs`, `output.rs` and
      `diagnostics.rs` sentences (the app's sentence keeps `trash_files`,
      `outputs_of` and `log_frontend` as the commands that stay);
      `docs/agents/app-log.md` line 9 and `docs/agents/tauri-app.md` line 1311
      (`diagnostics.rs`) point at the backend path while still naming
      `log_frontend` in the app.
  - Implementation approach:
    - Assumes Step 2 merged. `read_listing` is the one piece whose move is
      more than a file rename (it is cut out of a 5564-line `commands.rs`);
      keep its body byte-identical and move its doc comment with it.
    - `trash.rs` uses `crate::index::lock` and `SidecarStat`, both in the
      backend after Step 2, so no path edits there.
- [ ] Step 4: Put `watch` behind a callback seam and move its logic
  - Done when:
    - `crates/backend/src/watch.rs` holds `Watch`, `State`, `set`,
      `release_under`, `release`, `triggers`, `run` and `settle` (moved from
      `treewatch.rs`), with `Watch::spawn(on_changed: impl Fn(String) + Send
      + 'static)` instead of `AppHandle` and `set(watch: &Watch, dir, owner)`
      / `release_under(watch: &Watch, dir)` instead of `&AppHandle`. The app's
      `crates/app/src/watch.rs` becomes the Tauri glue: `Changed`, a `spawn`
      that passes a closure doing `app.emit("folder-changed", Changed{dir})`,
      and `set(app, …)` / `release_under(app, …)` wrappers that read
      `app.state::<Watch>()`, so `commands.rs`, `rename.rs`,
      `datefolder.rs` and `treewatch.rs` call sites do not change;
      `treewatch.rs` imports `settle` from the backend. The 6 tests move
      (they test `run` / `release` / `triggers`, not the emit). The `notify`
      dependency line moves to the backend if the app no longer uses it
      directly (`treewatch.rs` may keep it in the app too; check with grep).
    - The event name and payload (`folder-changed` `{dir}`) and their timing
      are unchanged; the frontend has no diff.
    - `mise run ci` and `mise run native:check` pass; the test count rule
      holds (286 in the backend, 212 left in the app). The native lock's
      additions (`notify` and its dependencies) are listed in the PR.
    - Tauri smoke check: copy a RAW into the open folder from Explorer, the
      strip gains it within about a second; rename the open folder from the
      tree (the watcher is released and restored).
    - Docs: `CLAUDE.md` moves the `watch.rs` sentence, keeping the event name
      on the app side; `docs/agents/tauri-app.md` line 2085 (`watch.rs`) is
      updated; `docs/agents/native-app.md` gains a "Backend" section stating
      the seam rule (the backend never names an event or an `AppHandle`; the
      app's thin module owns the event name and payload) and the compile-twice
      fact; `docs/agents/tauri-app.md` gains the same rule from the Tauri side.
  - Implementation approach:
    - Assumes Step 3 merged. This is the only step that changes signatures,
      hence the thin app-side module: the diff outside `watch.rs` and
      `treewatch.rs` should be `Cargo.toml` and `main.rs` only.

## Trade-offs and risks

- **`perf` and `ai` stay in the app for now (the user's decision).** Moving
  them would have settled their seams now and closed the parent milestone as
  first written, at the cost of a seam with no consumer until the port's Step 9
  and `reqwest` + `rustls` + `ring` in the native lock. The parent plan's
  Step 4 text is amended to say so, and `crates/app/src/ai.rs` / `perf.rs` stay
  Tauri-bound until Step 9.
- **Crate name `riffle-backend` now vs `handpic-backend` from the start.**
  Premises: the rename-to-handpic plan renames `riffle-core` / `riffle-app` /
  `riffle-cli` to `handpic-*` in its Step 3 and has not started it; a crate
  named `handpic-backend` next to `riffle-core` would be the one odd name
  until then. `riffle-backend` is consistent today and adds one line to the
  rename's Step 3 (this plan's Step 1 PR notes it).
- **Thin app-side modules vs changing call sites.** Keeping
  `crates/app/src/watch.rs`, `output.rs`, `diagnostics.rs` as Tauri glue files
  leaves three small files in the app whose names shadow the backend's modules
  (`crate::watch` is the glue, `riffle_backend::watch` the logic). The
  alternative, rewriting every call site to pass `app.state::<Watch>()` and
  closures, touches `commands.rs` (5564 lines, the file most likely to see
  concurrent edits) in dozens of places. The glue files keep the move PRs small
  and conflict-free; the port's Step 10 removes them with the app.
- **Step 2 is one 8.9k-line move.** `index.rs` (6550) and `sidecar.rs`
  (2321) could be two PRs, but `sidecar` depends on `index` and `photolab`
  and nothing in between is reviewable on its own; the review unit is the
  visibility hunks, not the moved text. If the reviewer wants two PRs, split
  at `index` first.
- **Edition 2021 for the backend.** The native crate is edition 2024; the
  backend stays 2021 so the moved code's text and rustfmt output do not
  change in a move PR. A later chore may bump it with the rest of the
  workspace.
- **Double compile.** `riffle-backend`, like `riffle-core`, is compiled by
  both workspaces; the bundled SQLite adds a C build to `crates/native`'s
  cold builds and CI runs. The parent plan accepted this for the separate
  workspace and defers the merge to its Step 10.
- **Rename detection.** A concurrent PR's hunk on `crates/app/src/index.rs`
  follows the rename only while git's similarity stays high; the "a move is
  a move" rule exists for that. A PR that reformats or edits a moved file
  heavily (the handpic rename's Step 2 comment edits are small) is the case
  where a hunk has to be re-applied by hand, and the PR that lands second
  says so.
- **The native link proof is tiny on purpose.** The EXIF labels on the
  status line use the backend at run time without adding a pane or a store.
  The alternative, a compile-only `use riffle_backend as _;`, proves the
  link but not that the code runs under 1.98.1 / edition-2024 callers.

## Progress

- (2026-11-09) Step 1 complete
- (2026-11-09) Step 2 complete
