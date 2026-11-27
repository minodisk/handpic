# Learnings: cue-labels

## Step 1: the sample run (2026-11-28)

Run from the scratchpad as `mise x -- python -I -B sample.py
D:\Photos\tests\2026-10-12-rank-app D:\Photos\tests\2026-10-17-bad-labels
D:\Photos\tests\2026-10-18-discard-labels
D:\Photos\tests\2026-10-20-model-search
D:\Photos\tests\2026-11-28-cue-labels`, then again with `--check`. Both
runs' full output is beside the data as `run.md` and `check.md`.

### The library had not changed since the plan's count

Every number of "The design" came out as planned: 19 intact folders of
17,812 frames, 8 culled folders (14,647 dump frames, 1,388 still in the
library), 2,400 labeled frames excluded (1,342 in the intact folders: 682
strict, 349 discard-labels, 311 model-search), a population of 16,470.

### Strata as drawn

| Stratum | Intact frames | Removed by the exclusion | Population | Allocated | Drawn | `π` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `pitch45` | 461 | 27 | 434 | 300 | 300 | 0.691244 |
| `closed` | 1,193 | 125 | 1,068 | 100 | 100 | 0.093633 |
| `rest` | 16,158 | 1,190 | 14,968 | 500 | 500 | 0.0334046 (overall) |
| all | 17,812 | 1,342 | 16,470 | 900 | 900 | 0.0546448 |

No difference from "The design". The sum of `1/π` over the sample is
16,470.000000, the population. The sampled `pitch45` frames split 101 up /
199 down (population 158 / 276).

Per folder, sampled count by stratum (`rest` drawn with a floor of 8, or
the whole cell: `2026-07-23` has 2):

| Folder | Population | `rest` population | Sampled | `pitch45` | `closed` | `rest` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `2026-06-05` | 1233 | 1164 | 51 | 10 | 5 | 36 |
| `2026-06-13` | 868 | 806 | 35 | 4 | 6 | 25 |
| `2026-06-14` | 1420 | 1305 | 73 | 24 | 8 | 41 |
| `2026-06-20` | 974 | 873 | 43 | 5 | 11 | 27 |
| `2026-07-02` | 90 | 72 | 8 | 0 | 0 | 8 |
| `2026-07-05` | 1321 | 1215 | 67 | 21 | 8 | 38 |
| `2026-07-11` | 431 | 348 | 28 | 11 | 6 | 11 |
| `2026-07-18` | 1441 | 1327 | 72 | 26 | 4 | 42 |
| `2026-07-21` | 16 | 15 | 8 | 0 | 0 | 8 |
| `2026-07-23` | 3 | 2 | 2 | 0 | 0 | 2 |
| `2026-07-24` | 192 | 162 | 18 | 8 | 2 | 8 |
| `2026-07-31` | 313 | 281 | 21 | 11 | 1 | 9 |
| `2026-08-02` | 125 | 103 | 18 | 8 | 2 | 8 |
| `2026-08-08` | 383 | 317 | 21 | 1 | 10 | 10 |
| `2026-08-15` | 297 | 274 | 12 | 2 | 1 | 9 |
| `2026-08-16` | 40 | 38 | 8 | 0 | 0 | 8 |
| `2026-09-27-a` | 4485 | 4172 | 217 | 64 | 22 | 131 |
| `2026-09-27-b` | 1158 | 1075 | 52 | 15 | 3 | 34 |
| `2026-10-03` | 1680 | 1419 | 146 | 90 | 11 | 45 |

### Checks

| Check | Expected | Result |
| --- | --- | --- |
| Sampled frames in any of the three manifests (asserted) | 0 | 0 |
| Copies whose size or SHA-256 differs from the source | 0 | 0 |
| `.xmp` / `.dop` files in `sample\` | 0 | 0 |
| Files in `sample\` / manifest rows | 900 / 900 | 900 / 900 |
| Files in `sample\` not in the manifest | 0 | 0 |
| Library files (27 folders, recursive) before / after | equal | 44,243 / 44,243 |
| Library paths changed / added / removed (`library.before.tsv` against `library.after.tsv`) | 0 / 0 / 0 | 0 / 0 / 0 |
| `--check` run: library paths changed since `library.after.tsv` | 0 | 0 |
| `--check` run: `manifest.tsv` / `strata.tsv` byte-identical | yes / yes | yes / yes |

- `manifest.tsv` SHA-256:
  `c7d05891e897ac77f0b1b57e73c0d990a07b4a0bdfd3c60c089f20b778617abe`
  (900 rows, both runs).
- `strata.tsv` SHA-256:
  `6d9f0d48639ea86c00ba7a5389d6083c22a793f2c2c1a12a2d34901591f61770`.
- Disk use of `sample\`: 39.7 GB; D: had 467.6 GB free before the copy.
- Wall time: copy 528 s (900 copied, 0 skipped), SHA-256 of the sources and
  the copies 303 s, 837 s in all; the `--check` run 1 s.

### Notes

- A preview with `--check` into a scratch folder (before the real run)
  printed every table and stopped at the missing `library.after.tsv`; it
  showed the numbers matched before any copy. The scratch output was
  deleted.
- The copy ran at about 75 MB/s with D: both source and destination, much
  faster than the 8-22 MB/s the import benchmark measured on D:; it was run
  in the background with its logs in the scratchpad, one copy at a time.
- The rating and paging keys have no strip or meta-pane gate
  (`crates/app/src/main.rs`, `key_down` and `judge`), so `labeling.md`
  hides both. The Sort and Filter menus are in the strip header, so the
  guide sets them before hiding the strip.
- A Python heredoc through `mise x -- python -` in Git Bash lost
  backslashes (`\\2026` arrived as `\202` and became a control byte in
  `todo.md`); a script file fixed it. Write Windows paths into files from a
  script file, not a heredoc.
- `cues.md` links the earlier plans as `../_archived/<folder>/plan.md`, as
  `plan.md` does; the archive move has to rewrite them like `plan.md`'s.
