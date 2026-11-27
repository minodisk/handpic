# Learnings: windows-device-checks-c7

## Step 1

- `origin/main` had not moved past `bb225319` when the step started, so the
  rebase was a no-op; `todo.md`'s sections were found by `###` heading.
- Of the three deleted sections only two ended in `---`; the
  `todo-three-decisions` one had none, and the `---` before it belongs to the
  Inno Setup uninstaller section, so it stays.
- The sections without a `#### Background` heading (the all-faces-check
  Steps 4 to 6) got their "Seen on" sentence after the paragraph's `Files:`
  list, the end of the opening paragraph as the plan asks.
- The results path is written in backticks in `docs/agents/` too, not as a
  link: the folder moves under `_archived/` only in the wrap-up, so a link
  would not resolve until then. The prefetch section's old backticked
  `../plans/20261123-todo-ten-closeouts/plan.md` (a folder since archived)
  was corrected to the `_archived/` path in the rewritten paragraph.
- Docs only: `mise run ci:local` and cargo builds were skipped on the
  caller's instruction; lychee ran on the changed Markdown.

## Run learnings (2026-11-27)

- The strip's height for a check was set by writing `stripHeight` into the
  test `--settings` store between launches, with no drag.
- `explorer.exe <file>.lnk` with arguments launched the release exe the way a
  double-click does, so the GUI subsystem could be checked with test stores.
- Hard links (`ln` in Git Bash on NTFS) of one RAW gave 300 RAW + `.xmp` for
  the Delete Sidecars timing and a 120-frame burst (one capture time) for the
  three-digit badges, without copying data.
- The MCP client had to print UTF-8: the Windows console's cp932 failed on a
  `–` in a reply. A Bash `python3 -` with no stdin hung the tool call.
