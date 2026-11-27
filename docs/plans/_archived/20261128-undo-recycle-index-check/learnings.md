# Learnings

## Step 1

- Docs-only change: the `todo.md` item was removed whole (heading,
  `#### Background`, `#### TODO`) and the "Seen" entry added after the `$I`
  cleanup paragraph of `docs/agents/shared-crates.md`. No local build or
  `mise run ci:local` was run, per the user's standing rule; the PR's GitHub
  checks verify the links and the Markdown lint.
- The real SID of the Recycle Bin folder is kept out of the repository
  (`$Recycle.Bin\<SID>\`).
