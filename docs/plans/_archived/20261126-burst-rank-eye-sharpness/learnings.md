# Learnings

## Step 1

- `burst::burst_ranks` takes `(full_eye_sharp, sharpness)` pairs in capture
  order and returns `Vec<Option<usize>>`; a stable `sort_by` on (tier, score
  descending) gives the capture-order tie rule for free.
- `burst_of` builds the pairs from `view.facts` in the position order it
  already sorts by, so the `eye_sharpness` and `sharpness` it reports are
  the very values ranked.
- Python `str.replace` edits through a quoted heredoc failed to match the
  `\`-continued lines of `crates/engine/src/mcp.rs`'s tool description;
  the Edit tool matched them directly.
