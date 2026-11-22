# Learnings: revert-page-present-now

## Step 1

- `git show 6809229f -- <clean paths> | git apply -R --index` reverted the
  code, `CLAUDE.md` and the performance docs exactly to `c57ec2ff`.
- `docs/agents/app.md` was touched after `c57ec2ff` only by #1219, #1220 and
  #1221, so `git checkout c57ec2ff -- docs/agents/app.md` gave the pre-#1219
  text exactly; the new Measured note ("A keyed page waits for the vsync tick
  on Windows") was then inserted where the removed section stood.
