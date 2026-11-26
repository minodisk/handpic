# Learnings

## Step 1

- The planning snapshot's line numbers still held: the seven `- [x]` items
  were the blocks at lines 84-110, 120-131 and 150-182 of `todo.md`, deleted
  in one `sed '84,110d;120,131d;150,182d'` call after checking
  each block's last line and the next kept `- [ ]` line. `git diff --stat`
  shows 72 deletions, 0 insertions.
- No dangling reference was left: the remaining "de-rotate" mention belongs
  to the kept "Faces rolled past 90 deg" item itself, and "the items in the
  sections below" and "the per-frame good-photo mark above" both still point
  at surviving text.
