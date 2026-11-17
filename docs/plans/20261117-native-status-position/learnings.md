# Learnings

## Step 1

- The helper moved to `pager::position(current, shown, listed) -> Option<String>`
  in `crates/native/src/pager.rs` (no `CLAUDE.md` change needed, `pager.rs`
  has no Layout entry). The meta pane's name wrapper `div` became the single
  name `div`, since it lost its only other child.
- The on-screen checks the plan asks for (position left of the circle and the
  buttons, not clipped with a long status text, popovers still anchored) were
  not done by the implementation agent: it cannot see a GUI window.
- clippy (`unnecessary_lazy_evaluations`) rejects `is_active().then(|| self.all.len())`;
  the call site uses `then_some`.
- This worktree had no `node_modules`, so `mise run fmt` failed on `vp`;
  `mise exec -- pnpm install --frozen-lockfile` fixed it (bare `pnpm` is not on
  the Git Bash PATH).

## Deferred issues (todo candidates)

- Pending manual check (native-status-position Step 1, `crates/native/src/main.rs`
  `status_row`): on Windows in a debug build (`mise run native:run` or
  equivalent), open a folder and verify that the status bar's right region
  shows `n / total` left of the progress circle (during a scan), `Sort ▾` and
  `Filter ▾`; with a filter on it reads `n / shown (listed)`; with a long
  status text (e.g. an error) the position is not clipped by the right
  region's `overflow_hidden`; the sort and filter popovers still anchor to
  their buttons; the meta pane shows the file name and no position. The
  step's checkbox was ticked on the automated criteria (unit tests, ci:local).
