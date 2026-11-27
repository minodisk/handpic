# Learnings

## Step 1

- Observation of the first real run of the develop session end (PR #1343's
  own run, 2026-11-28, Windows host, Git Bash, inside herdr):
  - The text argument `/exit` of `herdr pane send-text` was converted by MSYS
    path conversion to `C:/Program Files/Git/exit`, so the session did not
    end. The fix prefixes that one command with `MSYS_NO_PATHCONV=1`, the
    same rule `docs/agents/app.md` states for `ISCC.exe`'s `/`-options.
  - The key argument `enter` of `herdr pane send-keys` was delivered fine; it
    has no leading slash, so the prefix stays on `send-text` only.
  - The background command's completion re-invoked the main agent, and
    nothing visibly broke. A note only, no rule change.
- The README's only mention of the session end is prose with no copy of the
  command (a grep for `send-text` and `/exit` over `.claude/`), so it was left
  untouched.

## Deferred issues (todo candidates)

- Pending manual check (already tracked; do not file a second item): that
  `/exit`, sent with the `MSYS_NO_PATHCONV=1` fix, lands once the turn is idle
  and closes the session. Platform: Windows, Git Bash, inside herdr. Steps:
  let one real develop run reach its final `MERGED` with nothing else pending
  and watch the pane. Expected: `/exit` is submitted (not a path) and the
  Claude Code session closes, the worktree left for `wrangle`'s reap. Needs:
  a real develop run in herdr and the user's eye on the pane. This is the
  existing `todo.md` item "Agents: confirm the develop session end through
  herdr `/exit` on a real run (develop-decide-by-recommendation)", updated in
  this step and left open. Step 1's checkbox was ticked on the automated
  criteria. Files: `.claude/skills/develop/SKILL.md`, `todo.md`.

- Generalize the Git Bash `MSYS_NO_PATHCONV=1` rule beyond `ISCC.exe`. The rule lives only in the Inno Setup section of `docs/agents/app.md`, yet it has now bitten an unrelated command (`herdr pane send-text /exit` became `C:/Program Files/Git/exit`); Git Bash rewrites any native-exe argument starting with `/`. Sessions that shell out load `CLAUDE.md`, not `app.md`. Change: either (a) one line in `CLAUDE.md`'s Windows host paragraph next to the `mise x --` rule, or (b) a short Windows-shell guide under `docs/agents/` linked from `app.md` and the develop skill. Done when one option is chosen, the general rule is written once, and the `app.md` `ISCC.exe` entry points to it.
