# Learnings

## Step 1

- `self.shown`'s index was not remapped on a fresh open (only a re-listing's
  `refilter` runs `filter::shown_after`): the previous folder's file stays on
  screen until the new folder's first preview arrives, while `files` is
  already the new folder's, so `files[at]` named another file. Before this
  step only `pager.finish` read that index. The open path now sets it to
  `None`, so the pane shows the old name with no index rows in that window,
  and `pager.finish` treats the old image as not the current file's (which is
  what it is). The re-listing path keeps the remapped index.
- `toFixed(3)` and `{:.3}` diverge on an exact decimal tie: `0.0625` is
  `0.063` in JavaScript and `0.062` in Rust (round half to even). Kept the
  divergence: only exact dyadic ties differ, which a measured score
  practically never is. `an_exact_tie_rounds_to_even_unlike_to_fixed` pins it.
- `Math.round` is `(x + 0.5).floor()`: `-5.5` gives `-5°` and `-0.5` gives
  `0°`, both tested.
- Writing a file from Python on Windows with `open(p, 'w')` turns LF into
  CRLF; normalize back (`sed -i 's/\r$//'`) or pass `newline='\n'`.
