# Move to Trash: `delete_all` in chunks against per-file `delete` on Windows

Measured 2026-11-07 on Windows 11 (10.0.26200), `trash` 5.2.9, D: (NTFS, 4 TB
SATA SSD, Crucial BX500).

## Decision

**Chunk the move, on Windows only, with `CHUNK = 100`.** A chunk of 100 moves a
file about 27-30x faster than per-file `delete`, far over the plan's 2x rule.

- Per-file costs about 200 ms a file, whatever the file's size. With sidecars
  for half the RAWs that is about 3.3 RAWs per second, in line with the 3-5 of
  the session that started this plan.
- 100 rather than 20: 20 leaves 12-13x on the table for no gain.
- 100 rather than larger: 200 reaches 35x and a whole 300-RAW set 48x, but the
  status line then moves every couple of seconds or not at all, and a failed
  chunk's retry covers more files. At 100 a real move is already short: about
  4000 files at 7.3 ms is about 30 s, against about 13 minutes per-file.
- Windows, not non-macOS: this is the only platform measured, and on Linux
  `::trash::delete_all` is a loop of single moves, so chunking gains nothing
  there. macOS keeps its per-file `trash_file`, which reports where each file
  went.

## Method

A throwaway program outside the repository, built against `trash` 5.2.9 with
its default features, as the app is. Riffle took no part in the timing. Timed
variants:

- `per-file`: `::trash::delete` on each RAW, then on its sidecar, as
  `trash_one` does.
- `chunk-N`: `::trash::delete_all` on the RAWs of up to N groups, then on their
  sidecars, so two calls per chunk, as the plan's chunking does.
- `whole`: the same with the entire set as one chunk.

The timer covers the moves only. After each move the program checked that the
folder was empty and the Recycle Bin had grown by exactly the set, then
restored the set with `restore_all`, checked that every file was back at its
size and that the bin was back at its baseline, and went on. It would have
stopped on any change to the count of unrelated bin items, or under 8 GB free
on D:; neither happened. The order of the variants rotated each repetition.

| Set | RAWs | Sidecars | Files | Bytes | RAW size | Repetitions |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| `set` | 100 | 50 | 150 | 4.36 GB | 26.6-61.8 MB, mean 43.6 MB | 8 |
| `big` | 300 | 150 | 450 | 13.08 GB | three copies of `set` | 6 |
| `tiny` | 100 | 50 | 150 | 10 MB | 100 KB | 4 |

The RAWs of `set` are copies of ARWs from the samples folder; `big` adds copies
of those; `tiny`'s are synthetic. The `.xmp` sidecars are synthetic, about
2.2 KB each, for half the RAWs.

## Numbers

Milliseconds per file, median (min-max), and the speedup of the medians over
`per-file`.

| Variant | `set` (150 files) | Speedup | `big` (450 files) | Speedup | `tiny` (150 files) | Speedup |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `per-file` | 200.6 (191.6-466.7) | 1x | 199.2 (128.1-211.9) | 1x | 201.5 (190.5-204.7) | 1x |
| `chunk-20` | 15.5 (13.7-16.8) | 12.9x | 16.2 (14.0-19.9) | 12.3x | 17.3 (15.1-17.8) | 11.7x |
| `chunk-50` | | | 9.3 (8.5-10.9) | 21.5x | | |
| `chunk-100` | 6.6 (5.9-7.1) | 30.6x | 7.3 (6.9-9.0) | 27.4x | 7.2 (6.7-9.0) | 27.9x |
| `chunk-200` | | | 5.6 (5.2-7.2) | 35.4x | | |
| `whole` | 6.5 (5.5-8.0) | 31.1x | 4.2 (3.9-5.3) | 47.8x | 7.3 (6.3-8.3) | 27.5x |

- In `set` and `tiny`, `chunk-100` and `whole` are the same operation (100
  RAWs); `big` exists to tell them apart.
- The 466.7 of `set` is the very first run of the measurement, on a cold
  cache, 70 s against about 30 s after it. The medians are not moved by it, and
  the decision does not depend on it.
- File size does not matter: `tiny` is within about 10% of `set` in every
  variant, with files over 400 times smaller. A same-volume recycle is
  per-operation work, not per-byte work.
- The data fit about 180-200 ms per shell operation plus about 3.5 ms per
  file. Hence chunking's gain, and the shrinking return past 100.
- One thing unexplained: a first smoke run of 9 tiny files ran per-file at
  13 ms a file. Something makes a short burst of calls cheap. It was not looked
  into, since a move worth speeding up is never that short, and every set of
  150 or more settled at 200 ms.

## How `delete_all` fails

Probed on tiny bench files, since the mover has to get these right.

- **A locked file among six** (held open with no sharing): `Err("Some
  operations were aborted")`, but the shell went on past it and moved the other
  five. So a chunk's `Err` does not mean nothing moved. What is gone afterwards
  must be recorded, or Undo cannot bring it back.
- **The same file alone through `delete`** gives the same `Err`, so the per-file
  retry reports it as today.
- **A missing path in the list**: `Err` from the path lookup before anything
  moves. So one file gone meanwhile would otherwise fail a whole chunk.
- **Undo's way of restoring**, one `restore_all([item])` per item as
  `restore_run` calls it, brought back all the items two `delete_all` calls had
  moved, listing unchanged.

## What this does not cover

- It ran outside Riffle, so without the folder watcher or the index. The plan's
  manual check of 500+ files compares a real move with these numbers.
- One disk, one Recycle Bin of about 2200 items. Other volumes were not
  measured.

## Safety record

The plan expected about 12 GB free on D:. It had 310 GB, and the Recycle Bin
quota was readable, so the real-size set of the plan was safe to use, and the
reduced sizes the safety limits allowed for were not needed. `big` went beyond
the plan's set, to tell `chunk-100` from `whole`.

| | Bin bytes | Bin items | D: free bytes |
| --- | ---: | ---: | ---: |
| Before | 96,341,975,995 | 2186 | 333,591,625,728 |
| After | 96,341,975,995 | 2186 | 333,572,358,144 |

- Bin quota for D: 192,819 MB (`MaxCapacity` under `BitBucket\Volume`). The bin
  peaked at about 104,400 MB (91,879 MB plus the 12,475 MB of `big`), far under
  it, so nothing can have been evicted; the 2186 items were there throughout.
  Lowest free space: 316 GB, with all three sets on disk.
- The bin's size and item count match exactly. **Free space does not: it ended
  19,267,584 bytes (about 18 MiB) lower.** The most likely reason is the NTFS
  master file table, which grew for the roughly 23,000 metadata files below and
  does not shrink when they go: the drop came in `big`, as that count rose, and
  deleting the files did not return it. This is not confirmed, as reading the
  table's size needs administrator rights. No bench file remains on D:.
- `restore_all` leaves the `$I` metadata file of each restored item behind in
  `$Recycle.Bin`. The shell does not list them, so the bin's size and count do
  not show them. The measurement left 23,450, found only by counting the folder.
  Each names its original path, so exactly those under the bench folder with no
  `$R` partner were deleted; 0 remain. 5327 older orphans naming other paths
  and the 2186 real items were left as they were.
- The bench folder is gone, with every copy in it. The sources were only read.
