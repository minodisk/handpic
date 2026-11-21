# Learnings

## Step 1

- `screengap.py` re-run against the measurement worktree's `out\20261121`
  gave output identical to the scratchpad's `screengap.txt` (line endings
  aside), so the copies in the archived record's `raw/` are the scratchpad
  files byte for byte and still reproduce.
- Writing the edit script inline through a Python heredoc string tripped
  over Windows paths: `out\20261121` in a non-raw string is an octal escape.
  Use raw strings for text holding backslashes.
- The median-difference estimate and the per-press gap agree within about
  1 ms except RAF Tauri (11.75 against 8.74 ms), whose wide
  `keypressToPixels` spread makes a difference of medians a poor stand-in
  for the median of differences.
