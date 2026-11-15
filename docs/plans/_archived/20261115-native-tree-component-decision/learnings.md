# Learnings

## Step 1

- Re-read `gpui-base` 0.9.1's `src/tree.rs` before writing: `is_folder`,
  `set_items` clearing both indexes, `on_entry_click`, the wrapping
  `on_action_up` / `on_action_down`, `on_action_left` acting only on an
  expanded folder, `on_action_confirm` toggling, and `init` binding only the
  four arrows all match the plan's reasons.
- The pointer sentence in "gpui-component is initialized; ..." names the
  new section by a shortened quoted title ("The folder tree is not
  gpui-component's `Tree`"), as other cross-references in the file do.
- `mise run fmt` failed at first in this fresh worktree: `node_modules` was
  missing (`Cannot find module ...\node_modules\vite-plus\bin\vp`), and
  `pnpm` is only on `PATH` through mise. `mise exec -- pnpm install
  --frozen-lockfile` fixed it.
