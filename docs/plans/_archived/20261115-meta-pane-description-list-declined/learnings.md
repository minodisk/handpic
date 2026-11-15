# Learnings

## Step 1

- The note went in as a sibling `###` section after the meta pane's
  (Measured) section under "## Timings", tagged (Inferred) since it comes from
  reading gpui-component 0.9.1's source. The main reason is a `- Rule:`
  bullet; the supporting reasons are plain bullets, as other sections mix them.
- The `text_sm()` versus 13 px point is marked "This was not verified." in the
  note, per the plan's trade-off.
- `mise run fmt` failed first in this fresh worktree with `Cannot find module
  ...node_modules/vite-plus/bin/vp`: `node_modules` was not installed.
  `pnpm` is not on the Git Bash `PATH`; `mise exec -- pnpm install
  --frozen-lockfile` fixed it.
