# Riffle

A culling app for Sony ARW, Canon CR3, Nikon NEF, Fujifilm RAF, OM System / Olympus ORF and DNG files. See [README.md](./README.md)
for what it is and the current status.

## Layout

A Cargo workspace: `crates/core` (ARW, DNG, NEF, CR3, RAF and ORF parsing and JPEG decoding, `riffle-core`,
whose `src/xmp.rs` parses and patches XMP sidecar bytes (`xmp:Rating`, the
tri-state pick / reject flag as `xmpDM:good`, and the color label as
`photoshop:LabelColor` and `xmp:Label`) and `src/dop.rs` DxO PhotoLab `.dop`
sidecar bytes (rating, the tri-state pick / reject flag as `ShouldProcess`,
and `ColorLabel`), both sharing the `Flag` enum in `src/lib.rs`, `src/i18n.rs`
the per-language values, one JSON file per language in `crates/core/i18n/`
(so far the Lightroom color label presets) that `build.rs` embeds at build
time, `src/faces.rs` the YuNet face detector (each face's box, score and five
landmarks: the eyes, the nose tip and the mouth corners), whose ONNX model and license
live in `crates/core/models/`, `src/eyes.rs` the closed-eyes judgment of one
face (the eye aspect ratio of the MediaPipe Face Landmarker v2 face mesh,
its ONNX and `LICENSE-mediapipe` next to YuNet's; the scan runs only its
mesh, `mesh_of`, for the focus candidate cue), `src/pose.rs` the head pose (yaw / pitch / roll) of a face from its face mesh points, a port of MediaPipe's face geometry pipeline whose 33-point Procrustes basis is in `LICENSE-mediapipe`, `src/candidate.rs` the focus candidate cue
(the in-focus probability of the eyes of the face nearest the AF point,
a logistic combination of the Laplacian variance and mean edge width over
each eye's eyelid region from the face mesh, the sharper eye counting, or
over the window between the eyes when neither eye's region counts (under the floor or without a clear edge), and
whether it clears the threshold), `src/jpeg.rs` the Exif reader of a plain
JPEG file (orientation and the standard shooting tags into the same `Shot`
the RAW parsers fill), which `reader` and `scan` dispatch to for a `.jpg` /
`.jpeg`, `src/exif.rs` the crate-private IFD0 + Exif IFD reader `jpeg.rs`,
`nef.rs` and `cr3.rs` share, `src/nef.rs` the Nikon NEF parser (the full-size and
preview JPEGs from the SubIFDs), which `reader` dispatches to for a `.nef`,
`src/cr3.rs` the Canon CR3 parser (the ISOBMFF boxes: Exif from `CMT1` /
`CMT2`, the preview from `PRVW`, the full-size JPEG from its track), which
`reader` dispatches to for a `.cr3`, `src/hevc.rs` the decoder of the HEVC
`PRVW` / `THMB` of a CR3 shot with HDR PQ on (the pure-Rust `hpvcd` crate, then
a PQ-to-sRGB tone map and a JPEG encode), which `reader` hands those images to
so every consumer still gets a JPEG, `src/raf.rs` the Fujifilm RAF parser (the
fixed header's offset of the one embedded JPEG, whose Exif is the file's,
and the AF point from its Fujifilm MakerNote's `FocusPixel`), which `reader` dispatches to for a `.raf`, `src/orf.rs` the OM System /
Olympus ORF parser (IFD0 and the Exif IFD of its `IIRO` TIFF, the preview from
the MakerNote's CameraSettings, with note-relative offsets), which `reader`
dispatches to for a `.orf`, `src/sequence.rs` the JPEG timestamp sequencer ported
from lapse (orders a folder's JPEGs by capture time and writes copies with
unique `DateTimeOriginal` seconds into `<folder>-sequenced/`), `src/copy.rs`
the import's file copy (one sequential reader thread feeding one writer
thread through a bounded channel of buffers, each file written under
`TEMP_SUFFIX`, renamed into place and never overwriting), `src/embed.rs`
the SigLIP 2 image embedding of a preview (decoded near 404 px, 768-d,
L2-normalized; the model is downloaded, not bundled, its pinned URL, size and
SHA-256 here and its Apache-2.0 notice in `crates/core/models/LICENSE-siglip2`,
one plan shared through `embed::shared`, `EMBED_VERSION` naming the model and
its preprocessing), `src/genre.rs` the genre labels of an embedding (a cosine
per label against the text vectors and per-label calibrated thresholds of
`crates/core/genres/siglip2.json`, written offline by the image-embeddings
plan's `genre_vectors.py`, a sub-genre bringing its parent, `Unknown` when
none clears; `of_burst` the genre of a burst, the top-level label on for the
most of its frames, which the ranking keeps an offset per), `src/rank.rs` the personal best-shot ranking (the bursts as
`burst.ts` forms them, a pairwise logistic over each frame's embedding
projected to 32 whitened PCA dimensions, the pass-2 values and its position
in the burst, the prior in `crates/core/ranking/siglip2.json` written offline
by the plan's `rank_prior.py`, `fit` refitting the shared weights on the
user's judgments pulled toward the prior's plus one offset per top-level genre
pulled toward zero `OFFSET_SHRINK` times harder, `rank` the place, the score
and the reasons of each frame within its burst by the shared weights plus its
burst's genre offset), `src/scan.rs` the folder scan's worker pool and passes,
whose `ReadSlots` caps how many workers are inside a file read at once and
holds new reads back while a `Paused` is held, `src/fullsharp.rs` the eye
sharpness of the judged face on the full-size embedded JPEG (a crop around each
eye resized to 154 px, the gradient's 99th percentile over the contrast, the
sharper eye counting, a frame under `SOFT_BELOW` being soft), and
`src/sharpness.rs` the
sharpness score of the embedded preview, taken on the Sony eye-AF frame
when the camera tracked a face, else around the AF point, else between
the eyes of a detected face, else from the sharpest tile), `crates/cli` (the
benchmark CLI, including the `scan` folder-extraction benchmark, the
`copybench` timing of the import's copy pipeline and the
`facechoice` per-face `eye_focus` dump, with YuNet's nose, the face mesh's eye
centers and nose tip, their eye and nose offsets and the eye window's score, the mesh's edge gap and the full-size eye sharpness),
`crates/backend` (`riffle-backend`, the Tauri-free backend both apps depend on,
a root-workspace member that `crates/native` uses by path and so compiled twice
like `riffle-core`, whose `src/exif.rs` is the shooting-settings display
formatting shared by the meta pane and the filter menu, `src/photolab.rs` the
PhotoLab database lookup that gives a fresh `.dop` the registered image's Source
and master Item Uuids so PhotoLab does not import it as a virtual copy (see
`docs/agents/photolab.md`), and `src/shortcuts.rs` the keymap: the default keys
and the user's overrides, persisted in the `shortcuts` key), `crates/app`
(the Tauri 2 desktop app, whose `src/index.rs` is the SQLite folder index,
which also keeps each folder's last viewed file in `folders.last_viewed`
and the files shown in the preview in the separate `views` table (kept by
eviction, dropped by Clear Cache), which gives the ranking its negatives
(a viewed file left unflagged) and spares a folder holding a viewed file or
a reject from the 30-day eviction, not from the size cap or Clear Cache,
re-extracting rows written by an older `EXTRACTOR_VERSION` and filled in three
passes on the one scan task: `run_scan` (thumbnail, metadata), then
`run_faces_scan`, which fills the `eye_focus` / `sharpness` /
`faces_extractor` columns with the focus candidate cue the `f` focus mark is
colored by (YuNet, then the face mesh on the face nearest the AF point) and
the sharpness score, the `eyes_ear` / `pose_yaw` / `pose_pitch` /
`pose_roll` columns with the EAR of the more closed eye and the head pose
from that same mesh, and, on a frame the cue calls a candidate, the
`full_eye_sharp` column with the sharpness of its eyes on the full JPEG (a
second read), and streams them as `faces-progress` /
`faces-done` events, then, when the `aiEnabled` setting is on and its model
downloaded, `run_embed_scan`, which stores each RAW file's image embedding in
the separate `embeddings` table (valid while `EMBED_VERSION` and the file's
size / mtime match) and streams `embed-progress` / `embed-done`, without
counting as scanning; the passes run on worker threads below normal OS
priority (the second and third lower still) and take the on-screen files first
through the shared `ScanFocus` handle, `src/ai.rs` the `aiEnabled` setting
(persisted in the `aiEnabled` key, set by `set_ai_enabled`) and the download
of the embedding model into the app's local data dir on the first enable,
checked against its size and SHA-256, `src/rank.rs` the personal ranking's
commands (`ranking_of`, the rank of a folder's files from `Index::rank_rows`
and the weights in use; `refit_ranking`, the settings' refit on the
folders `Index::rank_dirs` lists, the picks against the rejects and the
viewed files left unflagged, also run in the background by
`refit_in_background` at startup and by `refit_ranking_in_background` when
the frontend switches to another folder (`viewed.ts`'s `switchesFolder`),
while `aiEnabled` is on, one at a time with the requests meanwhile
collapsed into one rerun; the shared weights and the per-genre offsets stored in
`ranking.json` under the app's local data dir, the prior's when none fits),
`src/commands.rs` the Tauri commands, including `set_scan_focus`, which hands
the running scan the current file and the strip's visible range, `preview`,
which pauses the scan's reads (`ReadSlots`, capped at `READ_SLOTS`) around its
own, `cancel_scan`, which stops the running scan short of its third pass for a
held Move to Trash or Delete Permanently, `delete_files`, the strip's
`Delete Permanently…`, `trash_files`'s guards and grouping with
`std::fs::remove_file` and no recorded run (`trash::delete_for_good`), `faces_of`, which detects the faces (box, score, five landmarks) the focus mark
draws on demand through the scan's `detect_around` without touching the index,
`genres_of`, which gives the folder's genre labels from its stored
embeddings, and `eyes_of`, which judges with `eyes` whether the eyes of the shown file's
AF face are closed, also on demand, a newer request superseding an older one,
and returns the face mesh points it judged on in stored preview coordinates,
`src/folders.rs` the folder tree's commands (the
home and volume roots, one folder's subfolders and RAW count, and
`reveal_folder`, the right-click item that reveals a folder, or from the
strip's menu a file, in the OS file manager under the per-platform
`REVEAL_LABEL`), `src/rename.rs` the
`rename_folder` command (it checks the new name, releases the folder watcher,
renames the folder and carries its index rows to the new path),
`src/datefolder.rs` the folder menu's `Rename by Capture Date…` commands
(`date_folder_preview` plans from the capture dates of the photos directly in
the folder a rename to its one `YYYY-MM-DD` or a split into one sibling folder
per date, with the undated files and any colliding target; `date_folder_run`
re-plans and either renames through `rename_folder`'s path or moves each dated
file with its sidecars and index rows into its date folder, leaving the
undated ones and removing the original folder only when it ends up empty),
`src/import.rs` the folder menu's `Import…`: the destination root and the
date-only folder pattern (chrono strftime, `%Y/%F` by default) of the
settings' `Import` tab, persisted in the `importRoot` and `importPattern`
keys, `import_preview`, which plans every RAW and JPEG under the folder, each
RAW with its sidecars, into date folders (copy, skip on the same name and
size, collision on another size, undated), and `import_run` /
`import_cancel`, which create the folders, copy the plan through
`riffle_core::copy`, stream `import-progress` / `import-done` and log one
`import:` line per run,
`src/watch.rs`
the open folder's watcher and `src/treewatch.rs` the folder tree's watchers on
its expanded folders (`set_tree_watches`, per-folder `tree-changed` events),
`src/trash.rs` the
move behind the strip's `Move to Trash…` and the `trash_files` command (it
groups the given files of one folder, each RAW with the `.xmp` and `.dop`
sidecars that exist for it and a JPEG alone, and moves them to the OS trash,
RAW first, in chunks on Windows (`run_chunked`), streaming `trash-progress`
events (as `trash_rejected_redo` does),
recording every run that moved something in `Runs`, the newest
100, which `trash_rejected_undo` takes one from to restore its files, RAW
before its sidecars, never overwriting, keeping what came back for
`trash_rejected_redo` to move to the Trash again), `src/foldersidecars.rs`
the folder menu's `Rewrite Sidecars from Index…` and `Delete Sidecars…`
commands (`rewrite_sidecars_preview` / `rewrite_sidecars_run` mark the
folder's index rows dirty and push them through the sidecar writer, patching
only the judgment fields of the current format's sidecars, and
`delete_sidecars_preview` / `delete_sidecars_run` move those sidecars to the
OS trash as one `Runs` run and clear the rows' judgments), `src/sequence.rs` the
`Sequence JPEG Timestamps…` commands (`sequence_preview`,
`sequence_run`, `sequence_cancel`) and their `sequence-progress` /
`sequence-done` events, `src/output.rs` the `outputs_of` command behind the
strip's `Output` filter (which of a folder's files have an export in its
`Output` or `Output-sequenced` subfolder, by file stem, with PhotoLab's copy
and DeepPRIME suffixes; no answer without such a folder, an error when one
cannot be read whole), and
`src/sidecar.rs` the coalescing sidecar writer thread and `SidecarFormat`, the
XMP, `.dop` or both setting chosen in the settings modal and persisted in the
`sidecarFormat` key of the settings store, next to the configurable
`xmp:Label` names per color persisted in the `labelNames` key, and `src/mcp.rs` the MCP
companion: the loopback Streamable HTTP server turned on by the `mcpEnabled`
key, its tools, and the bridge that asks the main window over the
`mcp-request` event and the `mcp_reply` command, and `src/diagnostics.rs`
the panic hook, the `log_frontend` command that writes the frontend's
`uncaught-js:` and `invariant:` lines, and the `invariant!` helper, which put
those failures in `Riffle.log` under greppable prefixes (see
`docs/agents/app-log.md`), `src/migrate.rs` the move of the data of an earlier identifier into the
current one's folders, run first in `main()`, and `src/perf.rs` the performance overlay's CPU
and disk sampler: `set_perf_sampling` starts or stops one thread that emits
Riffle's process CPU and the system's, each 0–100 %, Riffle's read
throughput in bytes per second and the system's disk busy time, 0–100 %
(PDH on Windows, `/sys/block` on Linux, `null` on macOS), as `perf-sample`
every 500 ms, so nothing samples while the overlay is off). `crates/native` (`riffle-native`)
is the GPUI port of the desktop app, a separate Cargo workspace with its own
`Cargo.lock` and Rust toolchain that uses `riffle-core` and `riffle-backend` and must not depend on
`crates/app` (see `docs/agents/native-app.md`).

The frontend lives under `crates/app/ui` (TypeScript built by Vite+, configured
in the root `vite.config.ts`; `pnpm exec vp {dev,build,check,fmt,test}`) and is
formatted, linted, type-checked and tested by `mise run ci`; its
`style.css` holds shadcn/ui's Neutral dark tokens and the shared component
classes every control uses (see `docs/agents/ui-styling.md`),
`src/icons.ts` the inlined Lucide icons,
`src/context.ts` builds the items of the strip's HTML right-click menu
(including the file's reveal item, `Copy Path` / `Copy File Name`, which
copy every targeted file, one per line, and `Move to Trash…` / `Delete
Permanently…`),
`src/trash.ts` the text and flow of the dialog those two share, a delete's
keys that do not confirm (`swallowedByDelete`) and their status lines,
`src/focus.ts` places the `f` focus mark and the face boxes' five landmark
dots and labels (score, box size and eye distance in stored preview px) and holds the good-photo rule
(`photoTier`: the AF eyes in focus, also on the full-size JPEG where measured
(`full_eye_sharp`), the eyes open and the face toward the camera, from the
values pass 2 stored for the AF face), which colors the
mark bright green and puts the strip's face icon on a good frame, `afEyeState`
(Good / Neutral / Low), the one state the mark's color, the strip's icon and the
filter menu read, `AF_EYE_HELP`, the definitions the `Face focus` section's `?` shows, and the eyes' openness the meta pane shows, and
`src/meta.ts` groups the meta pane rows by provenance (EXIF, Maker note
and Analysis, whose rows include the AF eye in-focus probability and the
`Eyes` judgment with its openness and the `Yaw`, `Pitch` and `Roll` of the head pose, from the values pass 2 stored for
the AF face, else the `eyes_of` judgment, and, while `aiEnabled` is on, the
personal ranking's place in the burst, reasons and confidence), `src/prefetch.ts` the ring of decoded neighbour previews (which files around the current one to hold, capped by total pixels, what to read next, and what a file list change, rename, delete or folder switch gives up), `src/recommend.ts` decides from a file's
ranking entry whether the strip marks it with the recommended icon (first
of two or more scored frames of its burst), `src/sort.ts`
orders the strip (file name, capture time, rating, or `recommended`: bursts in
capture order, each by the ranking's score), `src/eyes.ts` caches the `eyes_of` judgments per file with one
in flight at a time, `src/facemesh.ts` holds MediaPipe's face parts
outline and iris edges the focus mark draws over the judged face,
`src/peaking.ts` the focus peaking overlay (`e`): the 3x3 Sobel edges on
luminance over `PEAKING_THRESHOLD` lit in `PEAKING_COLOR`, which
`src/peakingworker.ts` computes off the main thread for the fit view and the
1:1 crop at the drawn size, `src/filter.ts` decides which files the strip's filter menu
lets through (including its `Face focus` section, whose `Good` item applies the good-photo rule and whose `Low` item is Rust's `not_candidate`, its `Eyes` section on the
stored eye state, and its `Genre` section on the `genres_of` labels, shown
while `aiEnabled` is on), `src/companion.ts` answers the MCP bridge's
requests over the main window's view state, `src/resume.ts` picks the file a
folder reopens at and coalesces the writes that remember it, `src/viewed.ts`
batches the `mark_viewed` writes of the files shown in the preview, counting
a file as viewed once its decoded preview has stayed on screen for
`VIEW_DWELL_MS` (500 ms, `dwellTimer`), and
tells a switch to another folder, which refits the ranking,
`src/idle.ts`
holds an operation pressed during a scan (Move to Trash, the
renames) until the scan ends and says which of them cancel it
(`cancelsScan`), `src/viewonly.ts`
decides from the listed paths whether a folder is JPEG-only and so opens
view-only (no judgment, capture-time order), `src/sidecars.ts` holds the
Rewrite Sidecars from Index / Delete Sidecars dialog's text and its flow,
`src/datefolder.ts` holds the Rename by Capture Date dialog's text (the title
line, the per-folder rows, the undated files, the removal note and the reason
the run is disabled) and its flow,
`src/import.ts` holds the Import dialog's text and `ImportFlow`, from the
folder tree's right-click through the preview to the run's end, the dialog
stepping aside during the run and coming back on `Import…`,
`src/importsettings.ts` the settings' `Import` tab's text (the pattern's
specifiers and examples, the note while no root is set),
`src/sequence.ts` holds the
Sequence JPEG Timestamps dialog's text and its flow from the folder tree's right-click
through the preview to the run's end, `src/uncaught.ts` builds the
`uncaught-js:` line of an uncaught error or unhandled rejection and the flood
guard that caps them per session, and `src/perf.ts` holds the performance
overlay's model (the CPU and disk samples and the per-switch preview
latencies of the last 60 s, the latency axis maximum and the read throughput
axis maximum, each with its floor, recording only while the overlay is on)
and the rule that a latency starts at the keypress, else at the selection
change.

## Language

**Write everything in the repository in English.** This covers documentation
(`README.md`, `CLAUDE.md`, `docs/**`, skill and agent definitions), code
comments, commit messages, PR titles and bodies, and any other text that lands
in the repository. The exceptions are `README.ja.md` and `docs/humans/*.ja.md`,
the Japanese translations of `README.md` and `docs/humans/<name>.md`; their
bodies are Japanese. Keep each pair in sync: a PR that changes `README.md` or
`docs/humans/<name>.md` updates `README.ja.md` or `docs/humans/<name>.ja.md` in
the same PR, and vice versa.

Conversation with the user stays in Japanese; only what gets committed is
English.

## Commits

Conventional Commits, in English. Example:
`feat(cli): add a partial decode benchmark`

## Development

Do all development through the `develop` skill (`/develop`), however small the
change. Do not edit code and open a PR by hand or through `/pr` alone.
