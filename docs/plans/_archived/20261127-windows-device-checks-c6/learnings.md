# Learnings

## Step 1

- `docs/agents/shared-crates.md` had a second stale line besides the one the
  plan named: the watcher paragraph before the rename / delete entry said
  deleting "was not measured (an open item in `todo.md`)", although the
  2026-11-23 table in the next entry measured it. It now points at that
  entry, since the `todo.md` item it named is gone.
- Boxes that pointed "above" at boxes this step deleted (the native-themes
  Linux box, the native-strip-menu-order macOS / Linux box and the
  always-visible scrollbars' macOS / Linux box) were rewritten to name their
  checks themselves. The strip menu box now spells the shortcuts as the menu
  draws them since `native-component-controls` (`P`, `U`, `X`).
- A bare `python3` in Git Bash hung for the full tool timeout, as
  `CLAUDE.md` warns; the edits were done with the Edit tool and `sed`.

## Deferred issues (todo candidates)

None from this step. The two failures the sweep found (the switch's off
state, the right-click menu's Up / Down / Enter) are owned by other sessions
(`dlg-c6-switch`, `dlg-c6-menukeys`) and their `todo.md` text was left as is.
