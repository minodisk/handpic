# Learnings

## Step 1

- The stale code span sat at line 3347 of `docs/agents/app.md`, not 3199 as at
  planning time; the search found it, the line number did not.
- The Done-when search command also prints
  `docs/plans/_archived/20261122-tauri-gpui-latency-ab-rerun/results.md:9`,
  whose link `../20261121-fit-preview-downscale/results.md` is relative to
  `docs/plans/_archived/` and so already resolves to the archived folder. It is
  a false positive of the `grep -v "_archived/..."` filter, not a stale
  reference, and was left unchanged. No other tracked reference needed a fix.
