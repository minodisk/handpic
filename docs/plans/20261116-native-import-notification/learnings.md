# Learnings: Native Import Notification

## Step 1

- `import_ran` runs from a `cx.spawn` task with only a `Context<Riffle>`, so
  the view now keeps the main window's `AnyWindowHandle` (`Riffle::window`)
  and the post runs in a `cx.defer` + `handle.update`, the pattern
  `modal.rs` already uses. The active check and the post share that one
  window update.
- `NotificationList::push` sends a `.system()` notification straight to
  `push_system` and returns before any in-app bookkeeping, so a system-only
  post leaves nothing on screen inside the window.
- User decision: a canceled run notifies too (title `Import canceled`)
  while the window is inactive.
- The fresh worktree had no `node_modules`, so `mise run fmt` failed at its
  `vp fmt` line after both `cargo fmt` runs; `mise exec -- pnpm install
  --frozen-lockfile` fixed it (plain `pnpm` / `node` are not on the Bash
  tool's PATH, only through `mise exec`).

## Deferred issues (todo candidates)

- **Pending manual check (Windows, on the user)**: the toast's delivery was
  not observed here. The step's checkbox was ticked on the automated
  criteria (unit tests of `import::done_notification`, `mise run ci:local`).
  Steps: build with `mise run native:dev` (or `native:release`), start an
  `Import…` from a card or folder, switch to another application, and wait
  for the run to end. Expected: one toast titled `Import finished` (or
  `Import canceled` after a cancel) whose body is the status line's summary;
  clicking it brings the Handpic window forward; an import that ends with
  the window active shows no toast; the taskbar icon and grouping of the
  running app look unchanged; `Handpic.log` has no
  `failed to register AppUserModelID` or
  `cannot show a system notification` warning. Basis: plan Step 1's
  Done-when; files `crates/native/src/folderops.rs`,
  `crates/native/src/main.rs`, `crates/native/src/import.rs`.
