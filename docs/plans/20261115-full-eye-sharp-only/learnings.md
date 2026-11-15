# Learnings

## Step 1

- `frames.tsv` was built from the scratch dump joined with the sidecars'
  `xmp:Rating` (by file stem, `D:\Photos\tests\<folder>`, read only), keeping
  the five rated folders and rating 1-3. 345 rows: 281 held-out, 64 fresh, of
  which 6 fresh frames have no judged face (empty value columns). The script
  divides by the frames with a judged face, which is what reproduces the
  planning numbers (fresh 27 / 18 / 13).
- The 2% rule alone does not pin the value: any threshold in
  (0.1034, 0.1171] puts 3/154 held-out 3-star frames under it. The script
  picks on a 0.01 grid, giving 0.110; 0.115 would put 2/27 fresh 3-star frames
  under it. Recorded in `low-threshold.md`.
- The fresh-folder AUCs were rechecked by rerunning `check0605.py` (same
  output as the plan's figures).
- Writing a Windows path with backslashes into a Python docstring through a
  shell heredoc and `sed` turned `\t` into a tab twice; the docstring is now
  a raw string and the path was fixed with the Edit tool.
