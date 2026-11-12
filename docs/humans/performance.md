<p align="center">English | <a href="./performance.ja.md">日本語</a></p>

# Performance

## Sony α7 V ARW (Apple Silicon Mac, n=20)

| Step | Target | Measured (median) |
|------|--------|-------------------|
| 1616x1080 preview extraction | 10ms | 4.4ms |
| JpgFromRaw full decode | 300ms | 84ms |
| 512px partial decode at the focus point | 50ms | 17.5ms |

## Leica M11-P DNG (Apple Silicon Mac, n=32)

Measured with release builds over 32 distinct real M11-P DNGs
(60.8MB to 78.0MB, 72.0MB mean), with the page cache warm (the files had been
read just before). Each DNG embeds a 2112x1408 preview and a 9504x6320 1:1
JPEG. The M11-P writes no `FocusLocation`, so the 512px crop is taken at the
image center (row ~3160), the fallback the app uses.

`riffle-cli bench` (two runs, medians):

| Step | Target | Measured (median) |
|------|--------|-------------------|
| 2112x1408 preview decode | 10ms | 7.5-8.0ms (α7 V 1616x1080: 4.4ms) |
| 9504x6320 1:1 JPEG full decode | 300ms | 105ms (p95 125-130ms) |
| 512px partial decode at the center | 50ms | 16.4-16.5ms (p95 ~20ms, max 22.7ms) |

The center crop sits well inside the 50ms budget on its own; this is the CLI
decode only, not keypress to pixels.

`riffle-cli scan` over the folder:

| Threads | Total | Throughput | Per file on a worker (mean / p95) |
|---------|-------|------------|-----------------------------------|
| 1 | 0.39s | 83 files/s | 12.1ms / 14.1ms |
| 12 | 0.05s | 596 files/s | 17.3ms / 24.2ms |

Thumbnails (528x352): 30,231 bytes per file on average (967,419 bytes over
32 files), against ~19KB on the α7 V.

## Nikon NEF, Canon CR3 and OM System / Olympus ORF (raw.pixls.us samples, Windows)

Measured with `riffle-cli bench` / `scan` on one raw.pixls.us sample per body
(12 NEF bodies, 13 CR3 bodies, 12 ORF bodies, the ones listed in
[What the camera records](./cameras.md)).

| | NEF | CR3 | ORF |
|---|---|---|---|
| Preview | 1620x1080 on every body | 1620x1080 on every body | 3200x2400 on every body |
| Preview decode | 14-26ms | 20-30ms | mean 42ms, max 49ms |
| Full-size JPEG decode | 27-260ms (Z 9 8256x5504: 260ms) | 178-607ms (EOS R5 8192x5464: 607ms) | none: the 1:1 view decodes the preview |
| Metadata within the 1MiB prefix | every body (ends by ~300KB) | every body (`moov` ends by ~90KB) | every body (the MakerNote's IFDs sit at its front) |
| Preview within the 1MiB prefix | all but the Z 6 and Z 50 samples | every body (`PRVW` ends by ~880KB) | the OM-1, OM-1 Mark II, OM-3, E-M1X and E-P7 samples (the rest end at 1.05-1.30MB) |

A preview past the prefix costs one ranged read, not a whole-file read. The
full-size JPEG is always a ranged read; on CR3 it sits at the start of `mdat`.
The NEF and CR3 thumbnails come out ~405x270 as on ARW. The ORF thumbnails
come out 800x600, since the 3200x2400 preview is scaled at the fixed 2/8.

## Fujifilm RAF (raw.pixls.us samples, Windows)

Measured with `riffle-cli bench` / `scan` on 46 raw.pixls.us samples of the
22 Fujifilm bodies listed in [What the camera records](./cameras.md) (a
compressed and an uncompressed sample per body, plus two crop-mode samples).

| | RAF |
|---|---|
| Preview (= full-size JPEG) | 4416x2944 on the X bodies, 4000x3000 on the GFX bodies; 1.27-5.51MiB |
| Preview decode | 48-147ms (mean 90.6ms, p95 120.7ms) |
| Metadata within the 1MiB prefix | every body (the JPEG's Exif ends by ~66KB) |
| Preview within the 1MiB prefix | none: every preview or 1:1 open is one ranged read (1-4ms warm) |
| `scan`, 1 thread | 221.8ms per file mean, 290.2ms p95 (5 files/s) |
| `scan`, 24 threads | 36 files/s |

A RAF carries one JPEG, so the preview and the 1:1 view decode the same
image. Since it is far larger than the ~1620x1080 previews of the other
formats, the RAF thumbnail is decoded with `thumbnail_jpeg_near` towards
`JPEG_THUMBNAIL_LONG_EDGE` as for plain JPEG files, instead of the fixed 2/8
scale: the thumbnails come out 404 px on the long edge (19.4KB mean, against
1104x736 and ~111KB at 2/8), and the thumbnail encode drops from 37-107ms to
16-60ms per file. The per-file scan cost stays high because the sharpness
score decodes the whole JPEG and, with no AF point read, faces are searched on
the whole image. Those two have since moved to the scan's second pass (see
"Which pass carries which cost" below), so the `scan` rows above no longer
match what `riffle-cli scan` measures today.

## Per-page preview read

The per-page cost on the Rust side only, on an Apple Silicon Mac.
**It excludes the IPC hop and `createImageBitmap`**; the end-to-end number is
in "End to end on Windows, keypress to pixels" below. Reading the whole 48MB ARW (`std::fs::read` plus `arw::parse`) is
compared with reading a bounded 1MiB prefix (`reader::read_preview`), which is
where the metadata and the embedded preview live:

| Folder | n | whole file | bounded prefix (Riffle) |
|--------|---|---------------------|------------------------|
| 5000 symlinks to one ARW, warm page cache | 300 | mean 6.6ms / p95 7.7ms | mean 0.06ms / p95 0.13ms |
| 20 distinct 48MB copies, first read | 20 | mean 16.0ms / p95 30.0ms | mean 5.1ms / p95 7.8ms |

Both columns were measured together, so they compare like with like. The symlink folder is 5000 symlinks to the same file and
the copy folder is 20 `cp` copies, read in file-name
order by a fresh process. The "first read" column cannot be guaranteed cold.

The whole-file read dominates, which is what the bounded read removes.

### End to end on Windows, keypress to pixels

From the key to the canvas draw returning, in ms as p50 / p95, measured on
2026-11-08 with `Timing logs` on and the overlay on: Windows 11, a
`mise run tauri:release:devtools` build, a SATA SSD, a folder already scanned
(warm, not otherwise established) and no scan pass running. At n below 20 the
p95 is the largest sample. WebView compositing and input latency are not
included.

| Case | n | keypressToPixels | wait | read | ipc | post | worker | decode | draw |
|---|---|---|---|---|---|---|---|---|---|
| 1616x1080 preview | 20 | 19.4 / 22.6 | 0.9 / 1.5 | 6.0 / 9.0 | 4.0 / 5.8 | 0.1 / 0.2 | 0.3 / 0.8 | 6.0 / 6.7 | 1.0 / 2.0 |
| RAF, 4416x2944 | 7 | 97.8 / 113.0 | 1.9 / 3.0 | 3.0 / 4.0 | 28.0 / 32.7 | 0.7 / 0.8 | 0.7 / 1.9 | 55.2 / 58.2 | 6.0 / 15.3 |
| RAF, 1920x1280 | 4 | 23.9 / 26.6 | 1.9 / 2.2 | 1.1 / 1.5 | 8.6 / 10.2 | 0.1 / 0.3 | 0.3 / 1.0 | 11.1 / 12.3 | 0.4 / 0.4 |

While the first pass ran only 3 presses landed, so this is a reference value and
not a p95: p50 138.2 ms, largest 151.5 ms, with `read` up to 139.5 ms, on a
folder never opened in the app whose files may still have been in the OS cache.

### Native preview (GPUI)

The experimental `crates/native` pager, from the key to the frame presented
(including the vsync wait and `Present`, which the Tauri numbers above do not).
In ms as the median of three runs' p50 / p95, each run n=20, timer-driven
presses, measured on 2026-11-09: Windows 11, release build with SIMD mozjpeg,
i7-13700, RTX 4060 Ti, 5120x1440 at 59 Hz, SATA SSD, files warmed by a throwaway
run, load up to 15 % during the timed presses but with a Tauri app open and other work
on the machine. The p95 is the largest of 20 samples.

| Case | keypressToPixels | read | decode | frameWait | paint |
|---|---|---|---|---|---|
| ARW, 1616x1080 | 20.9 / 30.0 | 1.6 | 7.8 | 8.1 | 0.7 |
| RAF, 4416x2944 | 64.8 / 75.2 | 3.6 | 47.6 | 8.5 | 3.5 |

These missed the thresholds first set for the port (17.6 ms and 62.9 ms). Those
were the throwaway spike's one n=20 run (15.56 / 57.89 ms) plus the margins, and
the same binary measured 19.86 / 64.59 ms in the same-session re-measurement
below, so it does not meet them either. Per-run numbers, the conditions and which stage grew are in
[`results.md`](../plans/20261109-gpui-port/results.md).

A re-measurement the same day, alternating the pager with the spike's own binary
with the Tauri app closed (three runs each, 60 presses over the 20 files), had
the pager at 20.2 ms on ARW and 65.9 ms on RAF and the spike's binary at 19.9 ms
and 64.6 ms (median of the three runs' p50 `keypressToPixels`). The pager's page
work matches the spike's; on RAF the roughly 1 ms left over is consistent with
its `orient` stage on portrait pages, which the spike does not rotate, but was
not isolated.

With the thumbnail strip, the index and the folder scan's first pass added
(2026-11-10, alternating with the pager from before them in the same session,
three runs each, 60 presses over the 20 files, the index already filled,
the median of the three runs' p50 / p95 `keypressToPixels`):

| Case | With the strip | Before the strip |
|---|---|---|
| ARW, 1616x1080 | 17.4 / 25.4 | 16.6 / 24.4 |
| RAF, 4416x2944 | 60.3 / 71.8 | 59.8 / 72.3 |

The strip adds 0.8 ms on ARW and 0.5 ms on RAF at p50, and nothing to `paint`.
While the first pass runs, the preview read stays near 1 ms, since the scan
reads the files around the current one first. Per-run numbers and the
conditions are in
[`results.md`](../plans/_archived/20261110-gpui-strip-index/results.md).

The absolute numbers here are lower than the 2026-11-09 session's on the same
files and machine (not explained); only the same-session difference compares.
The p95 here is sample 57 of 60, not the largest of 20.

While the folder scan's second pass (faces, focus, eyes) runs, paging costs
nothing measurable. That holds once the pass runs on an eighth of the cores
and pauses its work while a preview is read, decoded and painted (2026-11-13,
alternating with paging on a finished index in the same session, three runs
each, 60 presses over the 20 files, the median of the three runs' p50 / p95
`keypressToPixels`):

| Case | While the second pass runs | Second pass finished |
|---|---|---|
| ARW, 1616x1080 | 18.4 / 25.0 | 20.5 / 29.4 |
| RAF, 4416x2944 | 63.6 / 74.9 | 63.9 / 76.5 |

On all the cores, pausing only its reads, the second pass had added 9.4 ms on
ARW and 89 ms on RAF. The pass now takes about 5 minutes on 18,620 ARW and
4 minutes on 5,420 RAF files on 24 logical cores. Per-run numbers and the
conditions are in [`results.md`](../plans/20261110-gpui-focus-marks/results.md).

## Folder scan throughput

`riffle-cli scan` runs the folder extraction (bounded read, metadata parse,
404x270 thumbnail) over a folder on a dedicated rayon pool, with no database.
On a 12-core Apple Silicon Mac:

| Folder | threads | total | files/s |
|--------|---------|-------|---------|
| 5000 symlinks to one ARW, warm page cache | 1 | 34.9s | 143 |
| 5000 symlinks to one ARW, warm page cache | 8 | 5.35s | 935 |
| 5000 symlinks to one ARW, warm page cache | 12 | 4.46s | 1121 |
| 100 distinct 48MB copies, freshly written, page cache not guaranteed cold | 4 | 0.44s | 230 |
| 100 distinct 48MB copies, freshly written, page cache not guaranteed cold | 12 | 0.19s | 529 |

The thumbnails come to 19232 bytes each, i.e. ~96MB for 5000 files.
Extrapolating the freshly-written-copies column to 5000 files gives 9.5s at 12
threads and 21.7s at 4, but whether that holds on a real, cold-read folder is
not something these numbers establish. A single thread would not
make it (34.9s). More threads than cores does not help: 16 threads was flat
against 12 and doubled the per-file p95. On real cold reads this
extrapolation does not hold; see "Real folders on Windows" below.

What this cannot measure: a real 5000-distinct-file folder. Each
freshly-written-copies folder here is 100 `cp` copies scanned once, nothing is guaranteed cold, and the copies were
still likely warm in the page cache right after being written; the folders
were also scanned in the order they were written, which flatters the higher
thread counts. On a card reader or slow external disk the scan is disk-bound
regardless.

### Real folders on Windows

Measured on 2026-09-21 with the release app on Windows 11 Home, reading
real folders of Sony ARWs from a DRAM-less QLC SATA SSD (Crucial BX500 4TB)
with 22 extraction threads.
The numbers come from the `scan prepare` and `scan extract` lines of
`Riffle.log` (see "Measuring on your own folder" below). Each run used a
different folder:

| Run | files | prepare | extract | total | per file | files/s |
|-----|-------|---------|---------|-------|----------|---------|
| Cold first scan, Defender real-time protection on | 2677 | 1427ms | 42509ms | ~43.9s | ~16ms | ~61 |
| Cold first scan, folder excluded from Defender | 3401 | 639ms | 65605ms | ~66.2s | ~19ms | ~51 |
| First scan, warm page cache (index cleared, files read moments before) | 2134 | 47ms | 2084ms | ~2.1s | ~1ms | ~1000 |

Both cold runs finished with 0 errors. Excluding the folder from Defender did
not make the scan faster, so Defender is not the cause; the warm run shows
the CPU side costs ~1ms per file, so cold IO dominates. Extrapolated to 5000
files, a cold first scan takes ~82-97s. At a 1MiB bounded read per file and
~16ms per file, the effective throughput works out to ~63MB/s (derived, not
measured).

A thread-count sweep pins the cause on the drive. Measured on 2026-09-22 on
the same Windows 11 machine (i7-13700) with a release `riffle-cli scan`
cross-built for `x86_64-pc-windows-gnu`; each run used a different real
folder of Sony ARWs never read since boot, on the same drive, and all runs
finished with 0 errors:

| threads | files | total | files/s | per file on a worker (mean / p95) |
|---------|-------|-------|---------|-----------------------------------|
| 1 | 1337 | 41.08s | 33 | 30.7ms / 58.2ms |
| 4 | 1415 | 27.34s | 52 | 77.0ms / 103.5ms |
| 8 | 1520 | 29.47s | 52 | 154.9ms / 196.8ms |
| 22 | 1545 | 22.90s | 67 | 324.2ms / 369.5ms |

Throughput plateaus at ~50-67 files/s from 4 threads on, while the per-file
time on a worker roughly doubles with each doubling of threads: the workers
queue on one shared resource, the drive. A DRAM-less QLC SATA SSD is slow at
cold random reads: with no DRAM for the mapping table it needs extra NAND
reads, QLC read latency is high (worse for data written long ago), and SATA
has a single command queue. On one thread a file costs ~30ms, of which an
estimated ~10ms is CPU (from the warm run) and ~20ms is waiting on the drive.
The cause is the drive, not Riffle's code, and more threads help little.

The boot NVMe drive in the same machine (Crucial P5, TLC with DRAM) was not
measured, so there is no NVMe-vs-SATA comparison. The expected workaround,
not a measurement, is to keep the folders being culled on an NVMe drive,
internal or in an external USB NVMe enclosure with UASP; even over 5Gbps USB
it should be several times faster.

After a cache clear, the log once showed a `scan_id` superseded by the next
one with no `scan extract` line for the first (`todo.md`: "App: a scan can be
started twice after a cache clear / focus rescan").

### Sharpness scoring cost

Scoring sharpness adds a grayscale decode of the embedded preview to each
file's extraction. `riffle-cli scan`, release build, over 1000 symlinks to one
α7 V ARW (1616x1080 preview), warm page cache, on a 12-core Apple Silicon Mac,
runs alternated before and after:

| Threads | Runs | Per file on a worker before (mean / p95) | After (mean / p95) |
|---------|------|------------------------------------------|--------------------|
| 1 | 2 | 7.2-7.5ms / 7.6-7.9ms | 10.4-10.7ms / 10.9-11.2ms |
| 8 | 3 | 8.6-9.1ms / 12.1-13.3ms | 12.3-17.1ms / 15.7-22.0ms |

About +3.2ms per file on one thread (~+45%). The symlinks repeat one file, so
this is CPU cost with no IO variety. The score has since moved out of the first
pass into the second (see "Which pass carries which cost" below), so
`riffle-cli scan` is back to the "before" column and the ~3ms lands on
`riffle-cli candidates` and the app's `scan faces` instead.

### Face detection cost

The scan runs YuNet (2023mar, via `tract-onnx`) on each embedded preview
before scoring sharpness on the eyes: on a 480x480 crop around a trusted AF
point shrunk to a 320x320 input, or, without one, on the whole upright
preview at a 640x448 input (see "Whole-image recall (Windows 11)" below).
That detection now runs in the second pass, not the first (see "Which pass
carries which cost" below). The Linux WSL2 measurements up to
"Whole-image recall (Windows 11)" predate the move and the larger input (most
time a 320x320 detection inside `riffle-cli scan`). "Whole-image recall
(Windows 11)" and "DCT-scaled decode for the whole-image search (Windows 11)"
time the second pass (`riffle-cli detect` and `riffle-cli candidates`). The first measurement ran
on a Linux WSL2 machine (24 threads), release build, with synthetic input:
the OpenCV sample images `lena.jpg` and `messi5.jpg` upscaled to a 1616 px
long edge, single thread, 30 runs after one warm-up (the time includes the
downscale to the model input):

| Input | Mean | Median | p95 |
|-------|------|--------|-----|
| 1616x1616 | 18.2ms | 17.8ms | 21.5ms |
| 1615x1008 | 17.1ms | 16.9ms | 18.7ms |

The first call, which builds the model plan, took 46ms. Unstripped release
binaries on the same machine:

| Binary | Before | After |
|--------|--------|-------|
| `riffle-cli` | 1.7 MB | 31.3 MB |
| `riffle-app` | 26.8 MB | 47.8 MB |

The `riffle-app` growth came in through `riffle-core` (tract and the model).

A cold `cargo build --release -p riffle-cli` went from 10.8s to 140s.

#### Real files (Linux WSL2)

Measured on 2026-09-22 on the same Linux WSL2 machine (24 hardware threads),
release `riffle-cli`, files read from a Windows NTFS drive mounted in WSL,
warm page cache (each folder scanned once before timing).

Detection latency on real previews: `riffle-cli bench <file>` (row
`4. face detection`, one thread) on about 40 files sampled evenly from each
folder:

| Body / folder | Files | Preview | Mean | Median | p95 |
|---------------|-------|---------|------|--------|-----|
| α7 V ARW | 39 | 1080x1616 | 15.8ms | 15.4ms | 17.1ms |
| M11-P DNG | 36 | 2112x1408 | 16.1ms | 16.0ms | 17.1ms |

The first call (model build) took ~42-44ms.

`riffle-cli scan <dir> <threads>` before and after adding detection (before:
the scan passed no faces), runs alternated, two runs each:

| Folder | Files | Threads | Per file mean before -> after | p95 before -> after | Wall before -> after |
|--------|-------|---------|-------------------------------|---------------------|----------------------|
| α7 V ARW | 468 | 1 | 17.8 -> 43.3-47.2ms | 21.9 -> 50.7-54.4ms | |
| α7 V ARW | 468 | 12 | 29.3 -> 67-68ms | 35.9 -> 81-83ms | 1.13s (414 files/s) -> 2.63s (178 files/s) |
| M11-P DNG | 146 | 1 | 27.8 -> 57.9ms | 39.2 -> 73.9ms | |
| M11-P DNG | 146 | 12 | 45 -> 91-92ms | 63 -> 135-139ms | |

Sony bodies with face tracking engaged write the AF frame (`AFTracking`,
`FocusFrameSize`, `FocusLocation`), and the scan now scores the window on that
frame and skips detection for those files. Before and after the skip on the
same 468-ARW folder, runs alternated, two runs each:

| Threads | Per file mean before -> after | p95 before -> after | Files/s before -> after |
|---------|-------------------------------|---------------------|-------------------------|
| 12 | 74.6 / 80.8 -> 33.9 / 33.9ms | 90.3 / 97.5 -> 63.3 / 59.7ms | 160 / 147 -> 345 / 350 |
| 1 | 43.0 / 43.1 -> 19.6 / 19.8ms | 50.4 / 49.9 -> 40.6 / 39.3ms | 23 / 23 -> 51 / 51 |

441 of the 468 files took the AF-frame path; the other 27 (25 without
tracking, 2 with the focus point at the exact sensor center) still run
detection, which keeps the p95 above the pre-detection baseline.

Recall: on the same samples the detector found no face in 29 of the 36 DNGs
(9 of the 39 ARWs). Visual checks show boxes on the faces, but a group of 7
people on a swing (`L1005161.DNG`) yielded 2, and a basketball player in
three-quarter profile was missed. A hand count later showed that almost all
of those frames do contain faces, mostly 55-120 preview px, too small at the
320 px input; the whole-image search now uses a larger input and finds a face
in 23 of the 34 DNGs that contain one and all 7 people of `L1005161.DNG` (see
"Whole-image recall (Windows 11)" below).

Files with a trusted AF point but no camera face tracking now search a
480x480 crop of the upright preview around the AF point instead of the whole
preview; the scan still runs YuNet at most once per file. Measured on
2026-09-24 on the same Linux WSL2 machine (24 hardware threads), release
`riffle-cli scan` on the 2134-ARW α7 V folder (428 files take the crop path)
read from the Windows NTFS drive, warm page cache (one scan before timing),
before (the whole-preview detection) and after, runs alternated, four each:

| Threads | Per file mean before | Per file mean after | p95 before -> after |
|---------|----------------------|---------------------|---------------------|
| 1 | 22.0 / 22.5 / 22.9 / 22.5ms | 29.5 / 21.8 / 22.1 / 21.9ms | 43-46 -> 42.5-54.5ms |
| 12 | 36.8 / 38.2 / 36.0 / 35.5ms | 38.3 / 41.8 / 35.1 / 38.2ms | 66.5-71.4 -> 64.8-79.1ms |

On one thread the mean did not rise (the first after run, 29.5ms, was an
outlier the next three did not repeat); on 12 threads the runs overlap and
the spread is the drive's, not the detector's.

The faces the `f` focus mark draws are detected on demand by the `faces_of`
command (read the preview + `detect_around`), not at scan time. Measured once
on 2026-09-25 on the same Linux WSL2 machine, release build, calling the
command's body directly (no IPC) on two α7 V ARWs read from the Windows NTFS
drive, warm page cache, 20 calls each after a first one:

| File | First call | Mean | Median | Max |
|------|--------------------------|------|--------|-----|
| `_DSC3113.ARW` (1616x1080 preview, 3 faces) | 70.5ms (with the model build) | 29.9ms | 29.9ms | 34.1ms |
| `_DSC1942.ARW` (1616x1080 preview, 3 faces) | 34.0ms | 30.6ms | 29.7ms | 37.4ms |

The first call of a session also builds the model. A cold read
from a card or a slow disk adds its own latency on top; that, and the IPC and
redraw in the app, are not measured here. The timing ran from a temporary
`#[ignore]`d test removed before committing.

#### Whole-image recall (Windows 11)

Files without a trusted AF point (every Leica M file, Sony manual focus) are
searched on the whole upright preview. That search now feeds YuNet a 640x448
input (448x640 for a portrait frame, `faces::WHOLE_INPUT`) instead of the
320x320 square, so a 2112 px preview shrinks 3.3x instead of 6.6x and the
3:2 frame fills the input without a padded third. The crop around a trusted
AF point keeps the 320 px input, so the focus candidate cue does not change:
`riffle-cli candidates` printed the same per-file lines, AUC, precision and
coverage on the nine labeled folders and on the 2134-ARW folder before and
after.

Measured on 2026-10-06 on Windows 11 (Intel Core i7-13700, 24 hardware
threads, 32 GB), release `riffle-cli`, files read from the local NTFS drive,
warm page cache, before and after alternated. Recall is against a hand count
of 36 M11-P DNGs sampled evenly from a 146-DNG folder plus the swing group
`L1005161.DNG`: 34 of the 36 contain a face, 71 faces in all (faces below
about 40 px, cut by the frame or too occluded are left out). The faces found
were checked by eye on the `riffle-cli faces` images: every box sits on a
face, and the two files with no countable face get none.

| Model input | Files with a face found (of 34) | Faces found (of 71) | `L1005161` (of 7) | Files / faces at score >= 0.8 |
|-------------|--------------------------------|---------------------|-------------------|-------------------------------|
| 320x320 (before) | 8 | 10 | 2 | 4 / 6 |
| 320x320, score threshold 0.3 | 16 | 25 | 5 | 4 / 6 |
| 480x480 | 18 | 26 | 6 | 13 / 16 |
| 576x384 | 24 | 32 | 7 | 18 / 23 |
| 640x640 | 23 | 35 | 6 | 19 / 25 |
| 640x448, full-size decode | 23 | 36 | 6 | 19 / 25 |
| 640x448, 3/8 decode (now) | 23 | 37 | 7 | 19 / 24 |
| 704x480 | 26 | 44 | 7 | 22 / 30 |

The sharpness score only uses a face scoring at least 0.8
(`sharpness::FACE_CONFIDENCE`), so lowering the score threshold only adds
low-scoring boxes to the `f` mark; a larger input is what moves faces above
0.8. The missed faces left are mostly under goggles or neck warmers, turned
away, backlit or blurred; in `L1005161.DNG` the one missed is the woman
half hidden behind the toddler.

Cost per file on the 37 DNGs, `riffle-cli detect` (one thread; "detection"
includes the upright copy and the downscale into the input), three runs each:

| Model input | Decode mean | Detection mean / median / p95 | Inference alone |
|-------------|-------------|-------------------------------|-----------------|
| 320x320 (before) | 14.0-14.2ms | 24.2-25.0 / 24.1-24.3 / 25.5-30.4ms | 20.1ms |
| 640x448, full-size decode | 14.0-14.1ms | 63.9-64.0 / 63.1-63.8 / 67.7-69.9ms | 59.6ms |

The inference alone at the other inputs: 480x480 45ms, 576x384 46ms,
640x640 85-87ms, 704x480 69ms. With the decode at full size, a whole-image
file costs about 78ms instead of 38.5ms; the DCT-scaled decode below brings
it back to about 65ms.

`riffle-cli candidates <dir>` (pass 2) on the 146-DNG folder, three runs
each:

| Threads | Before | After |
|---------|--------|-------|
| 24 | 1.21 / 1.18 / 1.03s | 2.14 / 2.23 / 2.24s |
| 1 | 7.40 / 7.60 / 7.54s (51.6ms/file) | 13.76 / 14.19 / 15.43s (98.7ms/file) |

`riffle-cli scan` (pass 1) does not detect and did not change (0.17-0.18s on
24 threads, 1.92-2.01s on one). On the 2134-ARW folder, where every file has
a trusted AF point, `candidates` on 24 threads took 15.24 / 13.54 / 13.82s
before and 13.68 / 13.90 / 11.81s after, and its peak working set stayed at
393-399 MB; on the 146-DNG folder the peak rose from 509-514 MB to 646 MB
(the 640x448 plans and their buffers on 24 workers).

#### DCT-scaled decode for the whole-image search (Windows 11)

The whole-image search no longer decodes the preview at full size: it
decodes it at the smallest `n/8` scale whose long edge is not below the
640 px of the model input (`faces::decode_whole`; 3/8 for a 2112 px DNG
preview, 792x528, 4/8 for a 1616 px ARW one), rotates that upright, and maps
the faces back to the stored preview's pixels. The model input is still
box-averaged down from at least its own size, but from different pixels, so
the scores move by a few hundredths: on the 37 DNGs one more face is found
(the seventh person of `L1005161.DNG`, the woman half behind the toddler, at
0.64), files with a face found stay at 23 of 34, and one face dropped from
0.80 to 0.79 (19 / 24 at >= 0.8 instead of 19 / 25). The new boxes were
checked by eye: all on faces, none on the two empty files.

Same machine and setup as above, the build of the previous section against
this one, alternated, six runs each. The machine ran slower that day than
for the previous section (the same full-size build took 86.8ms per file
instead of 78ms), so compare within the table:

| Decode | Decode mean | Detection mean / median / p95 | Decode + detection |
|--------|-------------|-------------------------------|--------------------|
| Full size (before) | 14.4-16.5ms | 68.0-72.9 / 67.3-72.3 / 75.4-83.8ms | 86.8ms |
| 3/8 (now) | 5.0-5.5ms | 65.1-70.5 / 64.5-67.9 / 70.0-98.5ms | 72.4ms |

The decode drops by about 10.5ms and the upright copy and the downscale into
the input by about 4ms, 17% of the file. Scaled to the previous section's
day, that is about 65ms per file, 1.7x the 38.5ms of the 320x320 search
before the larger input.

`riffle-cli candidates` (pass 2), three runs each: on the 146-DNG folder
2.35 / 2.69 / 2.43s -> 2.01 / 2.14 / 2.18s on 24 threads and 17.31 / 16.04 /
16.23s -> 14.98 / 12.14 / 13.03s on one (113 -> 92ms per file); on the
2134-ARW folder, which takes the crop path only and printed the same
per-file lines, 15.32 / 14.94 / 14.91s -> 12.54 / 13.38 / 13.75s (no code on
that path changed, so this is the machine's spread). `riffle-cli scan` does
not detect: 0.22-0.26s -> 0.20-0.23s on the DNGs, 2.07-2.32s -> 2.00-2.15s on
the ARWs. On the nine labeled folders `candidates` printed exactly the lines,
AUC, precision and coverage of before.

The crop path keeps its single full-size RGB decode. Decoding the preview
twice instead, a full-size grayscale for the eye window's luma and a 6/8 RGB
for the crop (which then is 360 px of the scaled image instead of 480 of the
full one), was 11.6ms against 13.6ms per file (decode, luma and upright
copy, one thread, the 2134 ARWs, two runs). It does not keep the judgment:
mozjpeg's grayscale is the JPEG's Y channel rather than the BT.601 luma
of the decoded RGB, and the crop's model input comes from other pixels. On
the training folders 16 files changed state (faced frames 406 -> 404,
candidates 333 -> 329, precision 93.6%, coverage 91.4%, AUC 0.819 / 0.856),
on the held-out ones 6 (400 -> 399, 366 -> 363, 89.3%, 95.0%, AUC 0.646 /
0.769), so it was not adopted.

#### Focus candidate pass

The focus candidate cue (the AF eye in-focus probability) runs in a second pass after the scan,
so pass 1 no longer searches a crop around a trusted AF point: it detects
faces only without one (for the sharpness score), and pass 2 runs YuNet on
every trusted-AF file, eye-AF frames included. Measured on 2026-09-25 on a
Linux WSL2 machine (Intel Core i7-13700, 24 hardware threads, 16 GB),
release `riffle-cli`, the 2134-ARW α7 V folder `2026-09-19` read from the
Windows NTFS drive, warm page cache (one scan before timing, which took
19.0s cold), 24 threads.

Pass 1, `riffle-cli scan <dir> 24`, before (`962a5de`, crop detection in the
scan) and after (face-catch removed), runs alternated, four each:

| Run | Wall before -> after | Per file mean before -> after | p95 before -> after |
|-----|----------------------|-------------------------------|---------------------|
| 1 | 5.32 -> 4.58s | 59.3 -> 51.4ms | 95.0 -> 56.6ms |
| 2 | 5.81 -> 5.07s | 65.1 -> 56.8ms | 105.7 -> 63.2ms |
| 3 | 6.06 -> 5.09s | 68.0 -> 57.1ms | 111.0 -> 64.0ms |
| 4 | 6.01 -> 5.03s | 67.2 -> 56.4ms | 109.0 -> 62.3ms |

Pass 1 got ~0.8s faster and its p95 dropped by ~45ms: the 428 files that
used to run the crop detection no longer do.

Pass 2, `riffle-cli candidates <dir> 24` (read the preview, decode it once,
crop detection, eye window), three runs right after: 9.01 / 9.41 / 9.20s for
all 2134 files (1650 candidates, 302 not, 182 unknown). The command also runs
on the files without a trusted AF point, which return at once with no
decode; in the app `faces_todo` skips those before the pass starts. Thumbnails
therefore appear at pass 1's speed, and the marks fill in over the next ~9s.

The cue now also takes the mean edge width of the eye window, on the luma it
already decoded, so pass 2 did not get noticeably slower. Measured on
2026-09-26 on the same CPU under Windows 11, the same folder read from the
local NTFS drive, warm page cache, 24 threads, `riffle-cli candidates <dir>
24` before (`7dfe474`, Laplacian variance only) and after (`90729bc`),
alternated, four runs each: 9.79 / 10.33 / 10.23 / 10.31s before, 10.21 /
10.28 / 10.12 / 10.15s after.

The cue now runs the face mesh (MediaPipe Face Landmarker v2) on the face
nearest the AF point and measures the eyes over each eye's eyelid region,
falling back to the eye window when neither eye's region counts (under the 24 px floor or without a clear edge) (see
`docs/plans/_archived/20261008-mesh-eye-focus/`), and that costs pass 2 about two
thirds more. Measured on 2026-10-08 on the same CPU under Windows 11, the same
folder from the local NTFS drive, warm page cache (one run of each before
timing), 24 threads, `riffle-cli candidates <dir> 24` before (`4b5e0a39`,
the eye window) and after (`a7d2fbc4`), alternated, four runs each, reading
the `... total` line, which times only the scan's `extract_analysis` pass:
12.65 / 13.95 / 13.07 / 12.10s before, 22.20 / 21.89 / 21.09 / 20.64s after
(mean 12.9 -> 21.5s, +8.5s, +66%). The report's own second decode and mesh
run, which both builds do after that pass, are not in those times. The mesh
ran on the 1952 files with a face; it took 98.6ms mean, 97.5ms median and
114.7ms p95 per face with 24 workers sharing the cores (28.8ms mean on one
thread, on 78 faces of another folder), so the +8.5s is about 105ms of core
time per faced file. 1180 of those faces fell back to the window, 450 scored
the one eye that counted and 322 the sharper of two; 21 files changed state
(1700 -> 1697 candidates, 252 -> 255 not, 182 unknown). The peak working set
of the whole command, polled from PowerShell, stayed at 405-411MB, but the
report pass of both builds already ran the mesh, so that is an upper bound
on the scan pass's peak rather than a before / after comparison. The app was
not run for this measurement either.

The `scan extract` / `scan faces` log lines of an app open of this folder are
not recorded here yet: that needs the GUI, which was not run for this
measurement.

The scan now runs the face mesh only on a face whose box long side is at
least 60 px (`eyes::EYES_MIN_FACE`); a smaller face goes straight to the eye
window (see `docs/plans/_archived/20261008-mesh-face-gate/`). On this folder
that skips 261 of the 1952 faced files (13.4%), and the saving is within the
run-to-run spread. Measured on 2026-10-08 on the same CPU under Windows 11, the
same folder from the local NTFS drive, warm page cache (one run of each
before timing), 24 threads, `riffle-cli candidates <dir> 24` before
(`176a96b6`) and after (the face-size gate on top of it), alternated, four
runs each, reading the `... total` line: 20.28 / 20.94 / 20.68 / 20.51s
before, 19.98 / 21.13 / 19.84 / 19.88s after (mean 20.6 -> 20.2s, -0.4s,
-2%). No file changed state (1697 candidates, 255 not, 182 unknown in both);
one file's probability moved (a 58 px face from its one eye region to the
window, 0.209 -> 0.328, still not a candidate). On the labeled frames the
gate skips 104 of the 406 training faces and 102 of the 400 held-out ones,
and the AUC, precision and coverage stay at 0.882 / 93.9% / 91.4% and 0.800
/ 88.6% / 95.9%.

Measuring each eye inside its eyelid contour polygon instead of the
rectangle was tried and did not beat it on the held-out set (best cell AUC
0.797, precision 88.6%, coverage 95.0%), so the cue still uses the rectangle
(see `docs/plans/_archived/20261008-mesh-eye-mask/`).

#### Eye-state model survey (Windows 11)

YuNet's five landmarks carry no eyelid points, so telling closed eyes apart
needs a second model on the face nearest the AF point. Four candidates were
measured on 2026-10-07 on Windows 11 (Intel Core i7-13700, 24 hardware
threads, 32 GB), release build, one thread, against 504 hand-labeled faces:
432 from the 2134-ARW α7 V folder (240 drawn at random, 192 drawn toward
closed eyes) and 72 from the 146-DNG M11-P folder. The per-face latency
includes the crop from the full-size upright preview and the resize into the
model input (two runs per face for the per-eye models); the AUC is that of
the face's more closed eye against a closed / open hand label. All four load
in `tract-onnx` with `default-features = false`; the MediaPipe ones are ONNX
conversions of Google's TFLite files (tf2onnx, tflite2onnx).

| Model | License | ONNX size | Per face mean / median / p95 | Face AUC (eye AUC) | Precision / recall of closed |
|-------|---------|-----------|------------------------------|--------------------|------------------------------|
| Open Model Zoo `open-closed-eye-0001` (eye-crop classifier, 32x32) | Apache-2.0 | 46 KB | 0.52 / 0.48 / 0.79ms | 0.716 (0.718) | 0.35 / 0.70 at 0.5 |
| MediaPipe face mesh v1 (EAR, 192x192) | Apache-2.0 | 2.4 MB | 15.4 / 15.4 / 21.5ms | 0.886 (0.817) | 0.62 / 0.79 |
| MediaPipe Face Landmarker v2 (EAR, 256x256) | Apache-2.0 | 4.9 MB | 50.9 / 49.1 / 68.6ms | 0.974 (0.940) | 0.94 / 0.86 |
| MediaPipe Iris (eyelid contour, 64x64 per eye) | Apache-2.0 | 2.6 MB | 18.6 / 18.2 / 26.4ms | 0.863 (0.860) | 0.59 / 0.89 |

The precision and recall of the landmark models are at the EAR threshold
with the best F1. Labeled: 463 eyes and 253 faces (109 / 57 closed); the
rest could not be called. Of the 240 random ARW faces, 40% could not be
labeled: 25% had no eye to judge (turned away, occluded, false detections)
and 15% only eyes too small, blurred or dark to call. By face side in
preview pixels, 0 of 5 faces below 40 px were labelable, 6 of 31 at 40-59 px
and about two in three from 60 px up.

For scale, `riffle-cli detect` on 40 of those ARWs in the same session
took 13.4-13.6ms to decode and 31.8-32.7ms to detect per file, so the
classifier would add about 1% to the cue path's per-file cost, face mesh v1
33-35%, the iris model 40% and Face Landmarker v2 105-109%. None fits the
second scan pass: the one inside a +25% budget cannot tell the eye state
apart, and the one accurate enough for a filter costs about four times that.
Face Landmarker v2 was adopted on demand instead: it runs once for the file
being shown, outside both scan passes, so the pass-2 budget does not apply.
On the 3/8 decode
the whole-image search uses, the classifier's eye AUC fell from 0.78 to
0.59 on the same 20 DNG faces. The details are in
`docs/plans/_archived/20261007-closed-eyes-detection/model-survey.md`.

#### Closed-eyes judgment on demand (Windows 11)

The app judges closed eyes with Face Landmarker v2 for the shown file only:
the `eyes_of` command runs when a file is shown, outside both scan passes,
and the meta pane's Analysis group shows the `Eyes` row, open or closed with
the openness from 0 to 100 (`Eyes open: NN%`, the probability that the eyes
are open, when this was measured). It judges one face, the face nearest a trusted AF point, else the
largest face at or above 0.8, and none below a 60 px face side. For the face
nearest a trusted AF point, the second pass now stores the eye aspect ratio
and the head pose from the mesh it already runs for the focus candidate cue
(no extra model run), and the meta pane and the filter menu's `Eyes` section
read those; `eyes_of` still runs on every shown file, for the face parts
overlay and for a file with no stored value (no AF point, a face under 60 px).
Measured on
2026-10-07 on the same Windows 11 machine, release builds, one thread, on
`main` at `392f9c25` (the sizes before from the same day's builds before the
app called the model):

| Binary | Before | After |
|--------|--------|-------|
| `riffle-cli` | 24.6 MB | 29.6 MB |
| `riffle-app` | 47.5 MB | 52.5 MB |

Both grew by about the 4.9 MB ONNX file, which is embedded with
`include_bytes!` (`riffle-app` +4,994,560 B, +10.5%).

The model itself (crop, resize, run, eye aspect ratio), `riffle-cli eyes` on
the 504 labeled faces: mean 34.6ms, median 34.1ms, p95 41.2ms. The first
call, which builds the plan and runs once, took 182ms. That is faster than
the survey's 50.9ms, which ran three other models between its calls.

The whole `eyes_of` call (the `#[ignore]`d `times_eyes_of_on_real_files` test
in `crates/app/src/commands.rs`, no app running), the first file of each set
left out because it pays the plan builds (256ms on the ARW, 315ms on the
DNG):

| Files | Read | Decode | Detect | Model | Total |
|-------|------|--------|--------|-------|-------|
| 17 α7 V ARWs with an AF point and a face judged (of `_DSC1881`-`_DSC1899` in `2026-09-19`) | 0.8-6.1ms | 10.6-11.7ms | 27.4-32.7ms | 35.8-46.8ms | 80.2-91.9ms |
| 11 M11-P DNGs without one (`L1005148`-`L1005191` in `2026-02-01`, a face judged in each) | 7.0-9.7ms | 19.5-27.1ms | 73.8-80.0ms | 36.7-39.9ms | 139.9-156.1ms |

The two other ARWs had no face to judge and returned after the detection in
43.9 / 45.5ms. On the AF path one full-size decode serves both the detection
and the crop; without an AF point the decode is the 3/8 one of the
whole-image search plus a full-size one for the crop, and the whole-image
detection, not the second decode, is most of the cost. Every file of the two
sets labeled closed came back closed. The time from the preview to the row
in the running app (the `eyes` timing line with `Timing logs` on, IPC
included) has not been measured yet: it needs the GUI.

Accuracy, from `riffle-core` itself (`riffle-cli eyes`, which gives the
survey's eye aspect ratios exactly): face AUC 0.974 over the 253 labeled
faces (57 closed); at the threshold (closed iff the ratio is at most 0.137)
accuracy 0.957, precision 0.94, recall 0.86. Over the 246 faces at or above
the 60 px floor the app judges: AUC 0.974, accuracy 0.955, the same precision
and recall. The random ARW sample alone scores 0.918 (precision 0.85, recall
0.73), the DNG faces 0.986. A downcast eye reads as closed.

The head pose of the judged face (the meta pane's `Yaw`, `Pitch` and `Roll` rows) is a
port of MediaPipe's face geometry pipeline over the points of the same model
run (`crates/core/src/pose.rs`), so it adds no model run and no decode. The
solve takes 8.2µs per face at the median (p95 18µs, max 26µs; the 72 judged
faces of the M11-P folder, 200 repetitions each, one thread, release build).
The whole `eyes_of` call with the same test on the same files as the table
above, measured on 2026-10-07 on `main` at `9048e952` against `bde23660`, the
commit before the pose, built and run back to back on the same machine, the
first file of each set left out:

| Files | Total before | Total after |
|-------|--------------|-------------|
| 16 ARWs with a face judged | 60.7-73.1ms (median 65.3) | 61.4-68.2ms (median 65.9) |
| 2 ARWs without one | 32.8 / 33.5ms | 33.3 / 33.8ms |
| 10 DNGs | 109.2-119.1ms (median 116.0) | 110.8-120.7ms (median 117.1) |

The difference is run-to-run noise: the solve is three orders of magnitude
smaller, and the detection, which the pose does not touch, moved as much.
Both runs were faster than the table above; a first run of the after build
straight after compiling it read 81-92ms and 144-160ms. The binaries grew by
the code alone: `riffle-app` from 52,503,040 B to 52,520,448 B (+17,408 B,
+0.03%), `riffle-cli` from 29,556,224 B to 29,577,728 B (+21,504 B).

Accuracy, against 159 readable faces of 190 drawn from the same two folders
(with more turned and leaning faces than the folders have), labeled by eye with coarse classes and directions (there is no ground-truth
angle, and the labels are the agent's, not yet reviewed): the sign is right
on 76 of 81 turned faces (yaw, 94%), 73 of 77 raised or lowered ones (pitch,
95%) and 15 of 18 leaning ones (roll). The classes (yaw frontal under 15
deg, oblique 15-50, profile over 50; pitch and roll level within 15 deg)
agree on 68% (yaw), 79% (pitch) and 89% (roll). The yaw gap is almost all at
the frontal / oblique boundary: 28 faces labeled frontal read 15-42 deg, a
turn that still looks frontal, and no frontal face reads as a profile. Far
profiles past about 70 deg and sports sunglasses are rough, and a far
profile's yaw can read past 90 deg. 15 of the 2024 judged faces of the two
folders fitted upside down (a roll past 90 deg: backs of heads, ears,
blurs) and get no pose. The lens's own field of view in place of
MediaPipe's 63 deg camera changes none of the sign agreements (it flips
only angles within about 8 deg of zero) and moves an angle by a median of
about 1 deg.

### Which pass carries which cost

Since the sharpness score moved to the second pass, the first pass
(`riffle_core::scan::extract`, `riffle-cli scan`, the app's `scan extract`
line) reads the preview and computes the thumbnail and the metadata only. The
second pass (`riffle_core::scan::extract_analysis`, `riffle-cli candidates`,
the app's `scan faces` line) reads the preview once more and computes both the
focus candidate cue and the sharpness score:

| Cost | Pass before | Pass now |
|------|-------------|----------|
| Bounded read, metadata parse, thumbnail | first | first |
| Sharpness score (grayscale decode + window, ~3ms) | first | second |
| Whole-preview face search without a trusted AF point (~17ms at 320x320; ~64ms at 640x448 on Windows, after a 3/8 decode of ~5ms instead of the full-size ~14ms) | first | second |
| Focus candidate cue (crop detection + face mesh eye regions, eye window fallback) | second | second |
| HDR PQ CR3 HEVC decode (65-125ms) | up to three times per file | twice per file (thumbnail, analysis) |
| Closed-eyes judgment (Face Landmarker v2, ~35-45ms per face; 80-160ms per call) | - | the mesh and the pose solve also run in the second pass for the cue (~29ms per face on one thread), which stores the AF face's eye aspect ratio and head pose for the meta pane and the `Eyes` filter; `eyes_of` stays on demand for the shown file, for the overlay and the files with no stored value |

The first pass writes the rows in small batches (10) as their thumbnails finish, so the
thumbnails appear at the speed of the read and the thumbnail encode, and the
focus marks fill in during the second pass. The
numbers in "Sharpness scoring cost", "Face detection cost" and "Focus
candidate pass" above were measured before the move, except the two Windows
11 subsections of "Face detection cost" and the face mesh paragraphs of "Focus
candidate pass", which time the second pass. The `riffle-cli scan` "after"
figures in "Face detection cost" include costs that pass no longer carries.

In the app both passes run below normal OS priority (on Windows
`THREAD_PRIORITY_BELOW_NORMAL` for the first, `THREAD_PRIORITY_LOWEST` for
the second; on macOS the `UTILITY` and `BACKGROUND` QoS classes; on Linux a
nice of at least 5 and 10), and they take the shown file and the strip's visible cells before
the rest. `riffle-cli scan` and `candidates` run at normal priority and in
file-name order, so their numbers stay comparable with the tables above.

Not measured yet: the page latency (the `page` timing lines) while the passes
run, against the previous release, and the `scan extract` / `scan faces` wall
time with the lowered priority and the on-screen-first queue. Those hand
measurements are pending on a real folder in the app.

## Opening an indexed folder again

The second open of a fully indexed folder does no extraction: it stats every
file, reconciles the rows and queries them. On the 5000-file folder:

| Step | Target | Measured |
|------|--------|----------|
| First open, full scan (5000 files, 10 threads, 0 errors) | - | 5.55s |
| Second open (stat + reconcile + query, fully indexed) [^1] | 3s | 34.4ms |

Both rows were measured on **5000 symlinks pointing at one real ARW, with a
warm page cache**, so they carry the same caveat as the tables above. The
second open is a stat-and-query number, which the symlinks flatter less than
they flatter a read benchmark, but 5000 lookups of one cached inode's metadata
is still cheaper than 5000 distinct 48MB files' metadata on a card. The first
scan is the same folder and procedure as the scan throughput table above, at
10 threads, a thread count that table does not have a row for; for a real
folder of distinct files, see the Windows numbers below and "Real folders on
Windows" above; on a card reader or a slow external disk the first scan is
disk-bound regardless.

[^1]: Measured with a temporary `#[ignore]`d test that was removed before
committing, so this number is not reproducible from the committed tree.

On a real folder of 2677 Sony ARWs on the same SATA SSD (Windows 11 Home,
release app, 2026-09-21, from `Riffle.log`):

| Step | Target | Measured |
|------|--------|----------|
| Second open after a relaunch (`open list` 34ms + `open entries` 56ms + `scan prepare` 73ms) | 3s | ~163ms |
| Focus rescan of the unchanged folder (`scan prepare`) | - | ~65ms |

The focus rescan is not noticeable, so the watcher's debounce does not need
to grow for it. Two anomalies showed up in the log. Each open called
`open entries` twice (56ms and 76ms), and the second call is not counted in
the ~163ms above: it was the `scan-done` re-read of the rows, which every scan
did before 2026-09-26 and which is now skipped when the scan wrote nothing. A
rescan asked for during a scan (a focus of the main window, a watcher event,
`File > Reload Folder`) still runs once the scan ends, and reads the rows
only when it changed something; with `Timing logs` on, its `rescan
deferred: trigger=…` and `rescan: trigger=… deferred=true` lines name what
asked for it. And two focus rescans once fired at the same instant
(`todo.md`: "App: a scan can be started twice after a cache clear / focus
rescan").

### Measuring on your own folder

Riffle logs its own scan timings, so these numbers can be reproduced on any
folder. Clear the cache in `Riffle > Settings...` (the `Cache` tab) so the next
open counts as a first scan, and reopen the folder. Then quit and launch again
to get the second open, and use `Help > Open Log Folder` to find `Riffle.log`.

Every open writes `open list`, `open entries`, `scan list` (reading the
folder), `scan reconcile` (stat-ing the files against the index),
`scan sidecars`, `scan prepare` (the sum of those three) and `scan extract`,
which carries the whole extraction pass with its `files`, `done`, `errors`
and `threads` counts. The first scan's `scan extract` has `files` > 0; `scan
prepare` plus that `scan extract` is the first-scan total. A further
`scan_id` right after the cold scan's end with `todo=0` is a deferred rescan
(see the `rescan:` line before it), not part of the first-scan total.

The second open writes the same lines but with `todo=0`, and its
`scan extract` reads `files=0 done=0` with a near-zero duration — that
`files=0` line, not its absence, is what marks a cached open. It spans three
separate calls, so there is no single number for it: add the `open list`,
`open entries` and `scan prepare` lines of that open. Every line ends in
`in <n>ms` and names its directory, and the timestamps tell the two runs
apart.

Per-page latency is logged the same way. Turn on `Timing logs` in the settings
window (the item only shows in a `mise run tauri:release:devtools` or debug
build), page through a folder with the next/previous keys, and read the
`page wait=… read=… ipc=… post=… worker=… decode=… draw=… total=…
keypressToPixels=… jpegKB=… px=WxH scan=… source=…` lines in `Riffle.log`. `wait` is the
time a key spent behind an in-flight request, `read` the Rust read (the pause
of the scan's reads included), `ipc` the rest of the `preview` call's round
trip, `post` the main thread's work before handing the JPEG to the worker,
`worker` the two hops to the worker and back, `decode` its `createImageBitmap` (every stage but `wait` and `draw` is 0 for `source=prefetch`, a preview read ahead)
and `draw` the rest of the main thread's work to the drawn canvas (the meta pane
rebuild and the canvas draw). `total` runs from the request to the drawn canvas
and `keypressToPixels` from the keypress to the drawn canvas; `jpegKB` is the
payload size, `px` the decoded size and `scan` the scan pass running at the
request (`idle`, `1`, `2` or `3`). `wait` and `keypressToPixels` are left out
for a page not asked for by a key (a strip click or a folder open). A page turn
overtaken by the next one logs nothing.

## The 1:1 focus check path

The Rust side of one zoom keypress, on one α7 V ARW (Orientation 8,
`JpgFromRaw` 7008x4672 baseline 4:2:2, 5.76MB) on an Apple Silicon Mac. **One real file, warm
page cache, in-process, release build, n=20, medians.** Measured against the
functions the CLI and the app both call.
**The IPC hop and `createImageBitmap` are excluded** — they could not be
measured headlessly; the end-to-end numbers the user measured by hand are in
the next table.

| Step | Median |
|------|--------|
| Ranged read of the 5.76MB `JpgFromRaw` (`reader::read_full`) | 0.7ms |
| Crop at the focus point, 512 / 1024 / 2048 per axis (`partial::decode_focus_crop`, RGBA) | 18.4 / 21.5 / 29.8ms |
| 1024 crop at row 300 / 2336 / 4400 of the 4672-row JPEG (`partial::decode_crop`, RGB) | 10.8 / 27.3 / 44.1ms |

Crop size barely matters; the crop's **row** dominates. `jpeg_skip_scanlines`
on a baseline JPEG still entropy-decodes every skipped row, so a focus point
low in the frame costs four times one at the top, and 44ms leaves nothing of
the 50ms budget for the IPC hop and the bitmap. The payload is raw RGBA (4.2MB
at the 1024 cap) rather than a re-encoded JPEG because re-encoding that crop
with `mozjpeg::Compress`'s defaults measured 49ms at q85 — more
than the decode it follows.

### End to end, keypress to pixels

Measured by hand in the running app. **Conditions**:
optimized build (`mise run tauri:release:devtools`), `Timing logs` on in the settings window, **DevTools
open** (a webview can be slower with the inspector attached, so these may be
upper bounds), warm page cache (the folder had been opened before), one real
folder of Sony ARW files. `read` and `decode` come
from the `focus_crop` payload header (`Instant` inside `spawn_blocking`), the
rest from `performance.now()` on the frontend; **`ipc` is derived** as the
invoke elapsed minus `read` minus `decode`, so it is everything else on the
Rust side plus transport, not pure transport.

n=8 crops across 3 `z` presses:

| Measurement | n | Measured |
|-------------|---|----------|
| Keypress → pixels, for the crop the keypress itself asked for | 3 | 39 / 45 / 40ms |
| `read` | 8 | 1.8-2.7ms |
| `decode` | 8 | 21.9-38.7ms |
| `ipc` (derived) | 8 | 2.3-3.7ms |
| `bitmap` | 8 | 0-1ms |
| Total per crop | 8 | 26-45ms |

`decode` dominates. `read`, `ipc` and `bitmap` are noise beside it, so the
raw-RGBA-over-IPC decision (a 4MB payload at the 1024 cap) costs a few
milliseconds: **IPC is not the bottleneck**.

## What the sidecar pass adds to a folder open

Opening a folder also reconciles the XMP sidecars: one listing of the
directory, a `stat` per sidecar found, and a parse of only those whose
`(size, mtime)` changed since the app last saw them.

| Second open of a 5000-file folder | Measured (median) |
|-----------------------------------|-------------------|
| No sidecars in the folder | 31.3ms |
| 5000 sidecars, all with an unchanged stat | 67.4ms |

**These two numbers are not comparable to the 34.4ms above and do not replace
it.** They were measured on a different folder: 5000 one-KB regular files
named `*.ARW`, not symlinks and not real ARWs, on a local APFS disk with a
warm page cache, on an Apple Silicon Mac. The work timed is the same
sequence a folder open does on the Rust side — list, `stat` every file,
`reconcile`, reconcile the sidecars, `entries` — in a temporary `#[ignore]`d
test (release profile, median of 7 runs after 3 warm-up runs) that was
removed before committing, so neither number is reproducible from the
committed tree. What they do establish is the shape of the cost: on this
machine the sidecar pass roughly doubles a second open, adding about 36ms
for 5000 sidecars, and that cost is a listing plus a `stat` each, not a
parse, because an unchanged stat parses nothing. A first open of a folder
full of foreign sidecars pays the parse as well and was not measured.

## Copying from a memory card

`Import…` copies through `riffle_core::copy`: one thread reads the card in
order in 8 MiB chunks and feeds up to 4 buffers to one thread writing the
destination, which flushes each file before renaming it into place. No speedup
over Explorer is claimed. The one measurement so far found the destination
drive to be the limit, so it cannot tell the copy methods apart, and Explorer
itself was not timed.

Sony ARWs of about 44 MB, from a USB card reader (exFAT) to a Crucial BX500
4 TB SATA SSD (NTFS, 96 % full) on Windows 11. Each run used 50 files (~2.2 GB)
that had not been read before, so the card was read cold, with 60 s idle between
runs. In run order:

| Run | Method | MB/s |
|-----|--------|------|
| A | pipeline, 8 MiB × 4 | 21.6 |
| B | `robocopy /J` | 14.4 |
| C | `robocopy` (buffered) | 12.1 |
| D | pipeline, 32 MiB × 2, `FILE_FLAG_SEQUENTIAL_SCAN` | 12.9 |
| E | pipeline, 8 MiB × 4, no per-file flush | 11.1 |
| F | pipeline, 8 MiB × 4, `FILE_FLAG_SEQUENTIAL_SCAN` | 9.6 |
| – | read only, `dd bs=8M` to `/dev/null` | 134.7 |
| G | pipeline, 8 MiB × 4 (A repeated) | 8.1 |

The throughput fell from run to run whatever the method (A and G are the same
settings), while the card alone read at 135 MB/s. The BX500, DRAM-less and
nearly full, loses its write cache under sustained writes: an earlier 4.5 GB
run reached 45 MB/s cold and only 9 MB/s when repeated right after.
`FILE_FLAG_SEQUENTIAL_SCAN` and dropping the per-file flush showed no gain, so
neither was kept.

### Comparing with Explorer on your own card

`riffle-cli copybench <src-dir> <dst-dir> [chunk-mib] [buffers]` copies every
RAW and JPEG under `src-dir`, flat, into `dst-dir` through the pipeline the app
uses and prints the files, bytes, seconds and MB/s. A real import writes the same
figures to `Riffle.log`, as the `import:` line (`mb_per_s=`; see
[Using Riffle](./usage.md#features)). Time an Explorer copy of the same number of
files against either one, and for a fair comparison:

- Use a destination faster than the card, such as an NVMe drive or an emptier
  SSD; a slow one measures itself.
- Use files not read since the card went in, or the OS cache answers instead of
  the card.
- Do not time the card while the destination is still busy with an earlier copy
  (reads taken that way came out at 16–22 MB/s), and let the drive idle between
  runs.
