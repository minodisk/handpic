# Learnings

## Step 1

- The table rows were inserted after the `Leica M11-P` row in both cameras
  files with one `sed` `r` command, so the two files hold byte-identical rows.
- The plan's text was applied as written; the Japanese paragraphs stay one
  line each, the English ones wrapped at the file's 78-column width.
- Local checks: only `mise run fmt` and the Markdown lint were run, per the
  caller's instruction to leave the rest to the PR's GitHub checks.
