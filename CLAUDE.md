# Handpic

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
holds new reads back, and pass 2 at its stage boundaries (`yield_to_pause`), while a `Paused` is held, `src/fullsharp.rs` the eye
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
formatting shared by the meta pane (`Metadata`, `metadata`, `read_metadata`) and the filter menu, `src/photolab.rs` the
PhotoLab database lookup that gives a fresh `.dop` the registered image's Source
and master Item Uuids so PhotoLab does not import it as a virtual copy (see
`docs/agents/photolab.md`), `src/shortcuts.rs` the keymap: the default keys
and the user's overrides, persisted in the `shortcuts` key, `src/store.rs` the native app's settings store (one key / JSON-value SQLite file, seeded once when new, never deleted or recreated), `src/index.rs` the SQLite folder index,
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
through the shared `ScanFocus` handle, `src/sidecar.rs` the coalescing sidecar writer thread and `SidecarFormat`, the
XMP, `.dop` or both setting chosen in the settings modal and persisted in the
`sidecarFormat` key of the settings store, a switch's drain, persist and index reset (`switch_to`), next to the configurable
`xmp:Label` names per color persisted in the `labelNames` key
(`label_names_setting`), and the folder-open sidecar pass (`reconcile_of`),
`src/listing.rs`
the one `read_dir` of a folder (its RAW files, or its JPEGs when it holds no
RAW, and the sidecars next to them) and `canonicalize`, `src/folders.rs` the
folder tree's listing (`roots`, the home directory given by the caller and the
volumes; `list`, one folder's RAW count and visible subfolders, each
`FolderNode` with its `is_link`; `REVEAL_LABEL`, `is_hidden`),
`src/treewatch.rs` the folder tree's watchers (`TreeWatch`, one
non-recursive watch per expanded folder, `set` making the watched set equal to
the given folders, each folder's burst debounced into one call of the `spawn`
callback, and `with_released`, which a folder rename runs under), `src/ondemand.rs`
the faces and the eye judgment of the shown file, read on demand rather than
at scan time and touching neither the index nor `ReadSlots` (`read_preview`, `read_faces`,
`read_eyes`, whose `current` check stops a superseded judgment, and the
`raw_only` guard), behind the app's `faces_of` and `eyes_of`, `src/trash.rs` the
move behind the strip's `Move to Trash…` (it groups the given files of one
folder, each RAW with the `.xmp` and `.dop` sidecars that exist for it and a
JPEG alone, and moves them to the OS trash, RAW first, in chunks on Windows
(`run_chunked`), reporting through a callback the app's `trash-progress`
events come from, recording every run that moved something in `Runs`, the
newest 100, which `trash_rejected_undo` takes one from to restore its files,
RAW before its sidecars, never overwriting, keeping what came back for
`trash_rejected_redo` to move to the Trash again), `src/output.rs` which of a
folder's files have an export in its `Output` or `Output-sequenced` subfolder,
by file stem, with PhotoLab's copy and DeepPRIME suffixes (no answer without
such a folder, an error when one cannot be read whole), `src/watch.rs` the open
folder's watcher, which reports a settled burst through a callback and names no
event, `src/perf.rs` the performance overlay's CPU and disk sampler (`Sampler`,
whose `start` runs one thread that hands a `Sample` every 500 ms to a callback:
Handpic's process CPU and the system's, each 0–100 %, Handpic's read
throughput in bytes per second and the system's disk busy time, 0–100 %, PDH
on Windows, `/sys/block` on Linux, `None` on macOS; `stop` only signals it),
`src/ai.rs` the `aiEnabled` state and the embedding model's download (`Ai`:
the switch, which starts the download on an enable with the model not in
place and cancels it on a disable, the leftover part files removed at
start, `fetch` and `install` with the size and SHA-256 check, each `AiState`
handed to a callback), `src/mcp.rs` the MCP companion's server (`Mcp`: the
loopback Streamable HTTP endpoint on `MCP_PORT` with its seven tools, started,
stopped and switched on the caller's tokio runtime, each switch returning its
`McpState`; `Bridge`, which hands each tool's `Request` to the app's send
callback and waits for its `reply`; the `get_photo` / `get_preview` reads;
`mcp_enabled_setting`, the stored `mcpEnabled` rule), `src/rename.rs` the
folder and RAW file rename (`check_name`, `folder_target`, `file_plan` /
`file_run`, and `rename_folder`, which releases the watcher and the tree
watches around the rename and carries the index rows and the recorded trash
runs along, given them in a `Context`), `src/datefolder.rs` the `Rename by
Capture Date…` plan (`plan_of` from the index's capture times, else the file
head; `preview`) and the split (`split_folder`, over the same `Context`),
`src/foldersidecars.rs` the counting, the rewrite and the delete of a
folder's sidecars (`preview`, `rewrite`, `doomed`, `delete_preview` and
`delete_run`, which records the run and clears the judgments), none of the
three checking for a scan, which is the caller's, `src/import.rs` the
import's settings rules (`format_date`, `validate_pattern`, `root_setting`,
`pattern_setting`, `example`, `today_example`) and its plan and run (`walk`, `plan`,
`plan_dir`, `summarize`, `refusal`, `prepare`, which creates the destination
folders, `run_plan`, with a cancel flag and a progress callback, and
`log_line`), `src/sequence.rs` the `Sequence JPEG Timestamps…` preview and
run over `riffle_core::sequence` (`preview`, `run`, with a cancel flag and a
`(done, total)` callback), neither touching the index, `src/text.rs` the
`SCAN_RUNNING` message and `format_bytes` / `SIZE_BASE`, and `src/diagnostics.rs` the panic hook and the `invariant!` helper, which put
those failures in `Riffle.log` under greppable prefixes (see
`docs/agents/app-log.md`)), `crates/app`
(the Tauri 2 desktop app, whose `src/ai.rs` is the `aiEnabled` setting
(persisted in the `aiEnabled` key, set by `set_ai_enabled`) over the
backend's `ai`, placing the embedding model in the app's local data dir and
emitting its state as `ai-state`, `src/rank.rs` the personal ranking's
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
held Move to Trash or Delete Permanently, `trash_files`, the move of the
backend's `trash` module behind the strip's `Move to Trash…`, with
`trash_rejected_undo` and `trash_rejected_redo`, streaming `trash-progress`
events, `delete_files`, the strip's
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
`rename_folder` and `rename_file` commands, wrappers over the backend's
`rename` that drain the sidecar writer, refuse a running scan under the
`Scans` lock and stop the embedding pass (the folder's: it checks the new
name, releases the folder watcher, renames the folder and carries its index
rows to the new path),
`src/datefolder.rs` the folder menu's `Rename by Capture Date…` commands,
wrappers over the backend's `datefolder` with the same guards
(`date_folder_preview` plans from the capture dates of the photos directly in
the folder a rename to its one `YYYY-MM-DD` or a split into one sibling folder
per date, with the undated files and any colliding target; `date_folder_run`
re-plans and either renames through `rename_folder`'s path or moves each dated
file with its sidecars and index rows into its date folder, leaving the
undated ones and removing the original folder only when it ends up empty),
`src/import.rs` the folder menu's `Import…`, over the backend's `import`:
the destination root and the
date-only folder pattern (chrono strftime, `%Y/%F` by default) of the
settings' `Import` tab, persisted in the `importRoot` and `importPattern`
keys, `import_preview`, which plans every RAW and JPEG under the folder, each
RAW with its sidecars, into date folders (copy, skip on the same name and
size, collision on another size, undated) and keeps the plan for the run, and
`import_run` / `import_cancel`, which create the folders, copy the plan through
`riffle_core::copy`, stream `import-progress` / `import-done` and log one
`import:` line per run,
`src/watch.rs`
the backend's watcher emitting `folder-changed` and `src/treewatch.rs` the folder tree's watchers on
its expanded folders (`set_tree_watches`, per-folder `tree-changed` events),
`src/foldersidecars.rs`
the folder menu's `Rewrite Sidecars from Index…` and `Delete Sidecars…`
commands, wrappers over the backend's `foldersidecars` with the writer drain
and the scan refusal (`rewrite_sidecars_preview` / `rewrite_sidecars_run` mark the
folder's index rows dirty and push them through the sidecar writer, patching
only the judgment fields of the current format's sidecars, and
`delete_sidecars_preview` / `delete_sidecars_run` move those sidecars to the
OS trash as one `Runs` run and clear the rows' judgments), `src/sequence.rs` the
`Sequence JPEG Timestamps…` commands over the backend's `sequence` (`sequence_preview`,
`sequence_run`, `sequence_cancel`) and their `sequence-progress` /
`sequence-done` events, `src/output.rs` the `outputs_of` command behind the
strip's `Output` filter, over the backend's `output`, and
`src/mcp.rs` the MCP
companion over the backend's `mcp`, turned on by the `mcpEnabled` key: the
bridge that asks the main window over the `mcp-request` event and the
`mcp_reply` command, and the `mcp-state` emit, and `src/diagnostics.rs`
the `log_frontend` command that writes the frontend's `uncaught-js:` and
`invariant:` lines to `Handpic.log` (see `docs/agents/app-log.md`), `src/migrate.rs` the move of the data of an earlier identifier into the
current one's folders, run first in `main()`, and `src/perf.rs` the performance overlay's
`set_perf_sampling`, which starts or stops the backend's `perf::Sampler` and
emits each sample as `perf-sample`, so nothing samples while the overlay is
off). `crates/native` (`riffle-native`)
is the GPUI port of the desktop app, a separate Cargo workspace with its own
`Cargo.lock` and Rust toolchain that uses `riffle-core` and `riffle-backend` and must not depend on
`crates/app` (see `docs/agents/native-app.md`); its `build.rs` embeds
`crates/app/icons/icon.ico` as the exe's icon resource ID 1 on Windows
(`winresource`), its window opens with `app_id` `IDENTIFIER`, and
`packaging/` holds the macOS `Info.plist` and the Linux
`dev.minodisk.handpic.desktop` the `native:bundle:macos` (`Handpic.app` under
`target/<profile>/`) and `native:install:linux` (the release binary, the
`.desktop` file and the icon under `~/.local`) mise tasks use; its `src/datadir.rs` places
its own index at `<cache dir>/dev.minodisk.handpic.native/index.sqlite` (or
`--index <path>`) and its settings store at `<config dir>/<IDENTIFIER>/settings.sqlite`
(or `--settings <path>`) and holds the product's identifiers (`IDENTIFIER`, the
Handpic one, and the earlier `LEGACY_IDENTIFIERS`), `src/logger.rs` writes the `log` lines to stderr
and to a `Handpic.log` in the Tauri app's line format under its own folder (or
`--log <path>`), starting it anew past 1 MB (this crate's `debug` lines too
under `RUST_LOG=debug`), `src/theme.rs` sets gpui-component's
global `Theme`, which `gpui_component::init` installs at startup, to dark
and maps it to the app's Neutral dark tokens from `main.rs`'s color
constants (`apply`, not following the OS appearance), `src/settings.rs`
reads the backend's settings store, a new one seeded once from a Tauri
`settings.json`, only read (`--import-settings <path>`, else, without
`--settings`, the `IDENTIFIER` folder's under the config dir, else a
`LEGACY_IDENTIFIERS` one's), into the sidecar format, the label names, the keymap, auto-advance, the
sort key, the remembered filter, `mcpEnabled`, the import root and pattern, the shown panes, `lastFolder` and `windowBounds`, and writes them back in order on one thread
(`Writes`, drained on quit), `src/session.rs` what a launch restores
(`startup_folder`: the command line's folder, else the remembered
`lastFolder` while still a directory; `Geometry`, the window's restore
bounds, maximized state and display UUID under `windowBounds`; `place`, the
saved display, else one the bounds intersect, else the centered default,
with the bounds clamped into its visible area; all off under `--auto`), the
folder written on every open, the geometry on quit, `src/sort.rs` the strip's order (`SortKey`: name by `natord`,
capture time then subsec, rating with a reject after the unrated;
`order_files`, a port of `sort.ts`, which the window's one `refilter` applies
to the listing `all` to give the `files` everything else indexes into),
`src/burst.rs` the bursts (a port of `burst.ts`: `group_bursts` over `all`
in capture order through `order_files`, frames no more than `BURST_GAP_MS`
apart, regrouped whenever the entries are set; `burst_marks` and
`burst_badge`, each strip cell's band and `position/size` badge; `burst_step`
and `burst_frame_step`, the `burstPrevious` / `burstNext` and
`burstFramePrevious` / `burstFrameNext` moves over the shown files' ids;
`reject_rest`, the `rejectRest` changes: the current file's other burst
members over `all` not rejected yet, stars and label kept),
`src/filter.rs` the strip's filter (`FilterState` and `passes`, a port of
`filter.ts` less the genres: flags, stars, labels,
orientation, `Face focus`, `Eyes`, `Output` and the per-folder EXIF groups
`src/exif.rs` builds (a port of `exif.ts`); `Stored`, the
`filter_setting` rule over the stored `filter`, and back (`stored`,
`setting`); `anchor_after_filter`, the
file a refilter or a re-listing lands on; the listed files' `Facts` and
`FILTERED_TEXT`), which the same `refilter` applies after the sort,
`src/menu.rs` the status row's `Sort ▾` and `Filter ▾` menus (`sort_rows` and
`filter_rows`, after `index.html`'s `#sort-menu` and `#filter-menu`
without the `Face focus` help, the filter menu heading every section, its
rating rows from 5 stars down and each kind its own lead; `apply`, what a
click on an item does, the window writing the changed sort or filter;
`Menus`, which one is open), which the window draws as
`deferred` / `anchored` popovers above their buttons, its leads Lucide icons
from `src/icons.rs` (the SVGs under their license header, served by `Icons`,
the application's one `AssetSource`, to `svg().path(..)`, falling back to
`gpui_kit_assets::Assets` for the icons gpui-component's controls draw),
`src/settingsmodal.rs` the settings modal's model (open state, tab, the key
rules of `modal.ts` and `tabs.ts`, the keymap edits refusing `APP_KEYS`) and `src/settingsview.rs`
its entity, opened on `ctrl+,` (`meta+,` on macOS): a deferred backdrop and box,
a hand-drawn vertical tab list beside gpui-component controls (`Button`,
`Checkbox`, `Radio`, `Input`, and the scrolling lists' scrollbars),
holding the Sidecar tab's format radios and label names (`Input` fields), the Culling tab's auto-advance toggle, the Keyboard Shortcuts tab, which replaces the keymap and `Bindings` live, and the Import tab's root (`Choose…`, `prompt_for_paths`) and validated pattern,
`src/keys.rs` names a GPUI keystroke the keymap's way (`key_name`, a port of
`keys.ts`'s `keyName`) and resolves it to its action (`Bindings`),
`src/judgment.rs` one file's judgment (`State`: stars, flag, label, from an
`IndexedFile`) and what a judgment key makes of it (`Command`, `apply`,
`label_known`, `advances_after`, `should_advance`, a port of the Tauri `judge` and `advance.ts`)
and `store`, the `ratings` row then the backend's sidecar `Writer`, which the
window spawns at startup and drains in `App::on_app_quit`, `src/history.rs`
the undo and redo of the judgments and the moves to the Trash (`History`, a
port of `undo.ts`'s bounded stack, its `Entry` a judgment `Batch` or a
`TrashEntry`, `step`, the Tauri `step` rule, `undone_trash`, and `Undo`, the
two histories, whose judgment batches go when another folder opens and whose
trash runs stay), `src/selection.rs` the strip's
multi-selection (`Selection`, a port of `selection.ts`: `click`, `extend`,
`all`, `prune`, `settle`, `restore`, `targets` and `judgments`, a command's
changes over the targets, its value decided from the current file), `src/scan.rs` lists a folder with its sidecars and reads which of its files have an export (`outputs`, for the filter's `Output` section), reconciles the
index with the whole listing, reads back the changed sidecars
(`sidecar::reconcile_of`, skipped in a view-only folder, its dirty rows handed
to the writer with no debounce) and
runs pass 1 (`index::run_scan`) and then pass 2 (`index::run_faces_scan`) on
a thread of its own, their progress reaching
the view through one channel and the status line's `Status` (cleared whenever
a re-listing stops the scan), the scan held in the `Running` the app's quit
handler shares, which stops it within `STOP_TIMEOUT` on every quit,
`src/strip.rs` is the thumbnail strip's pure
model (the cell pitch, the scroll offset, the range kept with `RANGE_MARGIN`,
hit testing and which cell to ask for next, with `strip.ts`'s `missing` /
`ready` / `failed` bookkeeping, and each cell's judgment `Marks`: the stars,
the flag dot, the dimmed reject and the label's `Tint`, and its hover texts'
`tip_regions`: the label's name over the name band, the scan's error over the
rest of the cell), which the window draws along its bottom edge on
a `canvas` with each cell's face focus icon (`MarkState::icon`), under
transparent region `div`s that show those texts as gpui-component `Tooltip`s, and `src/thumbs.rs` the thumbnails of the strip's range, read
`MAX_IN_FLIGHT` at a time through the index's read-only connection, decoded
and turned upright, released into the view's `retired` list when their cell
leaves the range, `src/focus.rs` the scan's hot list (`scan_focus_paths`, a
port of `scanfocus.ts`: the current file, then the strip's range nearest
first), handed to `ScanFocus` before the scan starts and `SCAN_FOCUS_DELAY`
after the current file or the range last changed, `src/hold.rs` the scan pause a preview holds from its read until its image is painted (`PaintHold`), `src/prefetch.rs` the ring of
decoded neighbour previews (`Ring`, a port of `prefetch.ts`: the current
file's two neighbours, the paging direction's first, under
`PREFETCH_PIXEL_CAP`, each image either in the ring or on screen, cleared on
an open or a re-listing and narrowed by `retain` on a `refilter`; off under
`--no-prefetch`), `src/mcp.rs` the MCP
companion's server (the backend's `Mcp` on a tokio runtime of its own on one
`mcp` thread, started only while `mcpEnabled` is on, on `--mcp-port` for a test
run, each tool call reaching the view as `scan::Event::Mcp`, stopped on quit),
`src/companion.rs` the answers to its tool calls (a port of `companion.ts`
over a snapshot of the view: the current file as the selection, its burst
(`burstOf`, each frame's sharpness from the `Facts`), compare and several
photos refused, and `set_judgment`'s changes, one undo
entry through the judgment keys' view, row and writer path), and `src/viewed.rs` the
pure parts of remembering the folder: the coalescing `set_last_viewed` writer
(`Latest`, which the next open resumes at), the per-folder `mark_viewed`
batches (`Batches`) and the `VIEW_DWELL_MS` dwell of a decoded preview
(`Dwell`), and `src/resync.rs` when a change the backend's folder watcher
(`watch::Watch`, its callback sending into the same channel) reports
re-lists the open folder (`Resync`: at once when idle, else once the listing
or scan under way ends, a burst collapsing into one), the current file
landing where `filter::anchor_after_filter` puts it, the strip keeping the same files in view (`strip.rs`'s
`carried_offset`) and the loaded thumbnails of files still listed
(`carried_indices`), `src/mark.rs` the port of `focus.ts`'s good-photo rule
(`photo_tier`, `af_eye_state`: Good / Neutral / Low, Low being Rust's
`not_candidate`; `MarkState`'s `color` and the strip cell's `icon`), `focus_mark`, `upright_point` and the face boxes' `face_marks`,
`face_label` and `face_label_at`, `src/faces.rs` the port of `faces.ts`'s
`FaceCache` (each path detected once, kept by path, a clear dropping the
answers issued before it), `src/eyes.rs` the port of `eyes.ts`'s `EyesCache`
(one judgment in flight, unknown eyes cached, a clear dropping the answers
issued before it, and the latest ticket's id in a shared atomic the
backend's `current` check reads, so a newer request or a clear supersedes the
running judgment), `src/facemesh.rs` the port of `facemesh.ts` (MediaPipe's
face parts outline and iris edges, `mesh_edges`, `mesh_points`), and
`src/overlay.rs` the
`f` overlay: the toggle, the current file's `Focus` read through the index's
read-only connection (again when a pass 1 or pass 2 message names the file),
the current RAW file's faces detected and its eyes judged once its image is
shown through the backend's `ondemand::read_faces` / `read_eyes` (one `eyes`
timing line per judgment, the file current by then asked for once one
settles; both cleared on every listing), drawn only on the current file's own
image, and the painting of the AF mark, the face boxes with their landmark
dots, the judged face's mesh and the upright labels over the upright image's
bounds, `src/meta.rs` the right-hand meta pane's rows (`groups`,
a port of `meta.ts`'s `metaGroups` over the backend's `exif::Metadata`, which
`load` builds next to the preview: the `EXIF` and `Maker note` groups and an
`Analysis` group from the rows the caller passes), `src/viewonly.rs`
whether a folder is JPEG-only and so view-only (`is_view_only`, a port of
`viewonly.ts`), whose `VIEW_ONLY_NOTE` the pane's notes area shows, and
which keeps such a folder in capture order (`sort_for`), `src/empty.rs` why
the preview area shows no photo (a port of `empty.ts`: `state`, no folder, no
listed file, no shown file, then a failed preview of the current file; `text`,
`NO_FILES_TEXT`, `filter::FILTERED_TEXT` and `PREVIEW_FAILED_TEXT`; and
`open_hint`, the no-folder message with the `open` action's keys, still
without the Tauri drop clause), which the window centers over the preview
canvas, the no-folder message opening the folder picker on a click, and `src/zoom.rs` the 1:1 view on `z` (`CROP_MAX`,
`crop_size` and the crop read, a port of the Tauri `focus_crop`, turned
upright on the CPU; the `Zoom` state machine of `toggleZoom` / `requestCrop`,
one read in flight, an earlier page's crop held on screen until the current
one arrives and a resize re-cut after `CROP_RESIZE_DEBOUNCE`; and where
the crop and the scaled-preview placeholder are painted, one JPEG pixel per
device pixel with the focus point at the canvas center), `src/tree.rs` the
left pane's folder tree (a port of the state half of `tree.ts`: the roots,
each folder's listed children, expansion, RAW count and failed listing, the
drawn `rows`, and the path spelling `ancestors_within`, `respell` and
`relation` compare by, case ignored on Windows and macOS), which the window
draws as a `uniform_list`, fills from the backend's `folders`, expands
down to the open folder on every open, keeps following the disk through the
backend's `treewatch` on its expanded, drawn folders (`watched_folders`) and
re-reads the roots when the window becomes active, and its keyboard (the
cursor's `step`, `tree_key`'s Left / Right / Enter, the type-ahead's
`append_typed` / `type_ahead`, and `collapse_all`), which the tree takes on a
click in its pane and hands back on `escape` or a press elsewhere,
`src/treekeys.rs` the keymap actions that still run while the tree has the
keyboard (`TREE_PASSTHROUGH`, `tree_gate`, a port of `treekeys.ts`),
`src/context.rs` the
right-click menus (a port of `context.ts`: the items with their action, label,
shortcut, checked and disabled state, `folder_groups`, the tree's reveal,
`Copy Path`, `Copy Folder Name`, `Refresh` and, off a root, `Expand All` /
`Collapse All`, `strip_groups`, the strip's `Select All`, reveal, `Copy Path`
/ `Copy File Name`, the judgment groups checked from the focused file (left
out in a view-only folder) and `Move to Trash…` / `Delete Permanently…`,
`shown_path`, a path without the Windows verbatim prefix the shell refuses,
and `Menu`, the open menu's
position and target), which the window draws as a `deferred` / `anchored`
popover at the pointer, `src/trash.rs` the strip's `Move to Trash…` /
`Delete Permanently…` (a port of `trash.ts`: `dialog_text`, the delete
dialog's `swallowed_by_delete` and `confirms_delete`, `progress_status`,
`trashed_status` and `TrashFlow`, idle / asking / running with the button
the dialog opens on and what a key does in it; for the undo,
`restored_status`, `restored_into`, `still_held` and `opens_target`), which
the window opens as a gpui-component `Dialog` in `Root`'s layer through
`src/modal.rs` (`Modal`, the open and close that follow a flow after the
current update, the shell with the component's own closing, backdrop and
animations off, and `backdrop`, the dimmed window the view draws under it
that keeps a press from moving the focus) and runs through the backend's `trash::trash_listed` /
`trash::delete_listed` on a background task, undoing a run with
`trash::undo_run` and redoing it with `trash::redo_run`, `src/idle.rs` the gate holding
a confirmed run, or a run's undo or redo, for the end of the listing or scan under way (`IdleGate`, one
held operation, a later request replacing it, and the in-flight count no
listing starts under; `cancels_scan`, a port of `idle.ts`), `src/dialog.rs` the
confirmation dialog model the folder operations share (the content, which
buttons are enabled, the one the focus goes to, and what Escape, Enter and
Space do), `src/folderops.rs` the
folder menu's operations (the flow under way, one at a time, their dialog
opened as a gpui-component `Dialog` through `modal.rs` like the trash
dialog's), `src/sidecars.rs` the port of `sidecars.ts`
(`Rewrite Sidecars from Index…` / `Delete Sidecars…`), `src/sequence.rs` the
port of `sequence.ts` (`Sequence JPEG Timestamps…`), `src/import.rs` the port
of `import.ts` (`Import…`), `src/datefolder.rs` the port of `datefolder.ts`
(`Rename by Capture Date…`), `src/rename.rs` the port of `rename.ts`'s folder
parts (the tree's inline `Rename…`), and `src/peaking.rs`
the focus peaking overlay on `e` (`PEAKING_THRESHOLD`, `PEAKING_COLOR` and the
Sobel of `peaking.ts`; one layer per view, the fit view and the 1:1 crop,
keyed by the source image and its drawn device size, one job in flight on the
background executor, the fit view's image resized to its drawn size first),
`src/perf.rs` the performance overlay's data (`Perf`, a port of `perf.ts`:
the backend's `perf::Sample`s stamped on arrival and the per-switch
`timing::Stages` of the last `WINDOW`, recording only while on, the latency
axis maximum never below `LATENCY_FLOOR` and the read axis maximum never
below `READ_FLOOR_BPS`; `latency_start`, the key else the selection change),
`src/perfgraph.rs` the overlay on `d` (`Overlay`: the backend's
`perf::Sampler`, whose samples reach the view as `Event::Perf`, and a 500 ms
redraw, both only while shown; the pure layout, `x_of`, `y_of`, `stack` over
the native `page` line's stages and `shown`; `paint`, a port of `drawPerf`
as a 120 px band along the bottom of the preview canvas over the fit view
and the 1:1 crop alike), `src/progress.rs` the window's one progress bar
(`fraction`, a port of `progress.ts`'s `progressWidth`, and `shown`, a Move to
Trash / Delete Permanently run before a folder operation's run before the
scan's running pass), which the window draws along the status row's top edge,
and `src/panels.rs` the panes shown (`Panels`, a
port of `panels.ts` and the Tauri `panels_setting`: `toggle`, `toggle_sides`,
`from_setting` and `setting`, the `panels` value of the settings store, a
missing or non-boolean entry meaning shown), which the window's
`change_panels` sets on `toggleLeft`, `toggleRight`, `toggleStrip` and
`toggleSides`, the left pane giving up the keyboard when hidden, a 1:1 crop
re-cut and `panels` written (neither read nor written under `--auto`), next to
`open`, the OS folder picker whose folder opens as a click in the tree does
unless another open came first, and `src/dropped.rs` the rules of a drop onto
the window (a port of the Tauri `dropped_dir`: `single`, the one path a drop
names, `dropped_dir`, a directory as is and a file by its parent, and
`DROP_TEXT`, the status line's refusal), which the window's root `div` takes
through `on_drop::<ExternalPaths>`, resolving off the UI thread, a later drop
winning, and opening the folder as a click in the tree does, while
`drop_outline` draws the Tauri drag outline over the main area.

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
