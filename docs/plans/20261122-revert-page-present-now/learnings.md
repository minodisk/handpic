# Learnings: revert-page-present-now

## Step 1

- `git show 6809229f -- <clean paths> | git apply -R --index` reverted the
  code, `CLAUDE.md` and the performance docs exactly to `c57ec2ff`.
- `docs/agents/app.md` was touched after `c57ec2ff` by #1219, #1220, #1221
  **and #1222** (`e433fdf5`, the Windows real-device checks). Checking out
  `c57ec2ff -- docs/agents/app.md` dropped #1222's hunks too (review Round 1,
  item 1), so they were re-applied with
  `git show e433fdf5 -- docs/agents/app.md | git apply`. List
  `git log <base>..origin/main -- <file>` before restoring a file from an old
  commit. The new Measured note ("A keyed page waits for the vsync tick on
  Windows") was inserted where the removed section stood.
