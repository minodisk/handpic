# Learnings: develop-decide-by-recommendation

## Step 1

- The asking points of `.claude/skills/develop/SKILL.md` were found by
  grepping for `ask`, `approv`, `dialogue` and `direction`; every one now
  either points to `§ Deciding without asking` or is one of its listed stops.
  No file links to the old `#02-user-approval` fragment, so the 0.2 rename
  broke no anchor.
- README's "Phase 1–3 / Phase S single point of dialogue" sentence about
  `NEEDS_APPROVAL` was reworded to "a stop" with a pointer to the shared rule,
  since it contradicted the new dialogue points; the rest of the stale
  `NEEDS_APPROVAL` text was left (below).

## Deferred issues (todo candidates)

- **Stale `NEEDS_APPROVAL` stopping point in the develop README.**
  `.claude/skills/develop/README.md` still describes `merger`'s
  `NEEDS_APPROVAL` as a point where develop stops for the user (the step
  sequence diagram, "Folding into one PR" risk 3, and the "Operational note:
  using it with `/goal`" section), while `.claude/skills/develop/SKILL.md`
  § Merge merges with a standing approval except the release PR and the
  release-shaping paths. Basis: the plan's "Stale README text left alone"
  trade-off; out of scope for this step.
- **Pending manual check: the session end through herdr `/exit`.** On a real
  `develop` run inside herdr (`HERDR_ENV=1`, Windows host), let the run reach
  the final `MERGED` with nothing else pending. Expected: the main agent writes
  the "Decisions taken without asking" list, then makes the background
  `sleep 5; herdr pane send-text "$HERDR_PANE_ID" "/exit"; herdr pane send-keys "$HERDR_PANE_ID" enter`
  call (unsandboxed) as its last tool call, `/exit` lands once the turn is idle
  and the Claude Code session closes, leaving the worktree for `wrangle`'s reap.
  Also watch whether the background command's completion re-invokes the agent
  before `/exit` is submitted. Related: `.claude/skills/develop/SKILL.md`
  § Finishing the run. Step 1's checkbox was ticked on the automated criteria;
  this check was not performed.
