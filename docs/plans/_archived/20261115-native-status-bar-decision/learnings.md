# Learnings

## Step 1

- The new section sits between "One progress bar sits on the status row's top
  edge" and "Text is typed through gpui-component's `Input`". The 1380 x 852
  canvas size it cites is the one measured in "The sort and filter menus are
  deferred popovers over the window"; the 9 px and the would-be canvas change
  are source-read, so the section is tagged `Inferred` and words them as
  "would".
- The neighbouring progress bar section still says only the settings modal and
  the tree's inline rename draw from gpui-component; left as is per the plan.
- `mise run fmt` failed at first with `Cannot find module ...node_modules\vite-plus\bin\vp`:
  the fresh worktree had no `node_modules`, and `pnpm` is not on the Git Bash
  `PATH`. `mise exec -- pnpm install --frozen-lockfile` fixed it.
