# Learnings

## Step 1

- Re-running `endpoints.py` against the measurement worktree's
  `out\20261120` folder reproduced `endpoints.txt` exactly (`diff
  --strip-trailing-cr` empty), so the kept output matches the logs.
- `endpoints.txt` matched every median the plan expected, and its GPUI
  `nextFrame`, Tauri `painted`, `frame1` and `frame2` cells equal the
  existing tables' `keypressToPixels`, `frame1` and `frame2` cells per run and
  in the median row.
- In this Bash tool, backslashes inside a quoted heredoc reached the written
  file halved (`\` became `\`), which broke a Python string literal holding
  `out\20261120`. Writing the helper script with the Write tool avoided it.
