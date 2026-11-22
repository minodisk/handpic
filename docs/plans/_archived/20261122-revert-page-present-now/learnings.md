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

## Deferred issues (todo candidates)

- Add a revert-procedure check to the `develop` skill: list intervening
  commits before restoring a file from an old commit. In Step 1,
  `git checkout c57ec2ff -- docs/agents/app.md` silently dropped #1222's
  unrelated hunks (`e433fdf5`); local review caught it. Change: when
  reverting, prefer `git show <commit> -- <paths> | git apply -R --index`; if
  a file must be restored from an old commit, run
  `git log <base>..origin/main -- <file>` first, re-apply unrelated later
  hunks and confirm with `git diff`. Done when the rule is in the `develop`
  skill (or a decided guide with a stated trigger), or the issue is closed as
  too rare to document.
