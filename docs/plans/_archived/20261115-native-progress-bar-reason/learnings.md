# Learnings

## Step 1

- The status row (`main.rs`'s `status_row`) sets no background of its own, so
  there is no single fill to blend `PRIMARY_COLOR` at 20 % over; the doc
  states the track as "`PRIMARY_COLOR` at 20 %, lighter than `MUTED_FILL`"
  without a hex, as the plan allowed.
- The pointer to the section "gpui-component is initialized; the settings
  modal and the tree's inline rename draw from it" was dropped with the stale
  clause rather than kept, since it no longer supports the sentence.
- `mise run fmt` first failed with `Cannot find module ...node_modules/vite-plus/bin/vp`:
  this worktree had no `node_modules`. `pnpm` is not on the Git Bash `PATH`;
  `mise exec -- pnpm install --frozen-lockfile` fixed it.

## Deferred issues (todo candidates)

- The heading "gpui-component is initialized; the settings modal and the
  tree's inline rename draw from it" in `docs/agents/native-app.md` (line 48)
  is stale: dialogs, tooltips and scrollbars draw from gpui-component too.
  The user chose to leave it out of this PR (plan's Trade-offs and risks).
