# Learnings

## Step 1

- `pnpm` is not on `PATH` in the Bash tool's Git Bash; run it as
  `mise exec -- pnpm exec vp ...` from the repository root, since the
  `crates/app/ui` directory is not a pnpm workspace package.
