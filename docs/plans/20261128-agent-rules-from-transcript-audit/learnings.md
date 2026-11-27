# Learnings

- `todo.md` was rewritten between the `### Agents: confirm ... hand-back`
  heading and the next `### Core:` heading by heading text, not line numbers,
  so the edit stays correct if other sessions moved the sections.
- The `cd` rule in `pr-runner.md` §4 reuses `merger.md` step 6's phrase "no
  `cd` prefix" and names the allow-rule reason (a line starting with `cd` is
  another shape than the allowed `bash .claude/skills/...` lines).
- The merger re-run applies to any failure of either step 6 command, without
  classifying the error text, as the plan decided.
