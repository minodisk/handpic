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
mesh, `mesh_of`, for the face cue), `src/pose.rs` the head pose (yaw / pitch / roll) of a face from its face mesh points, a port of MediaPipe's face geometry pipeline whose 33-point Procrustes basis is in `LICENSE-mediapipe`, `src/candidate.rs` the face cue
(the face nearest the AF point, else the largest confident face of the
whole preview, its face mesh and what the mesh gives: the EAR, the head
pose, the eye offset, the edge gap and the eye centers `fullsharp.rs`
measures around, YuNet's eye landmarks without a mesh), `src/jpeg.rs` the Exif reader of a plain
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
`facechoice` per-face dump of the face choice, with YuNet's nose, the face mesh's eye
centers and nose tip, their eye and nose offsets, the mesh's edge gap and the full-size eye sharpness),
`crates/backend` (`riffle-backend`, the GUI-free backend the app depends on,
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
and in the separate `folder_counts` table each folder's file count as the
native app's tree last listed it, keyed by the tree's spelling of the path
(`set_folder_counts`, `folder_counts`, `rename_folder_counts`,
`prune_folder_counts`, which drops the rows under a listed folder outside
the child folders it keeps; kept by the eviction, emptied by Clear Cache),
re-extracting rows written by an older `EXTRACTOR_VERSION` and filled in three
passes on the one scan task: `run_scan` (thumbnail, metadata), then
`run_faces_scan`, which runs the face cue (YuNet, then the face mesh on the face nearest the AF point) and fills the `sharpness` /
`faces_extractor` columns with the sharpness score, the `eyes_ear` / `pose_yaw` / `pose_pitch` /
`pose_roll` columns with the EAR of the more closed eye and the head pose
from that same mesh, and, on every frame whose cue judged a face, the
`full_eye_sharp` column with the sharpness of its eyes on the full JPEG (a
second read; none when the file holds no JPEG larger than the preview), and streams them as `faces-progress` /
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
loopback Streamable HTTP endpoint on `MCP_PORT` with the seven companion
tools, or every tool after `with_all_tools` (the native app's), started,
stopped and switched on the caller's tokio runtime, each switch returning its
`McpState`; `Bridge`, which hands each tool's `Request` to the app's send
callback and waits for its `reply`, `wait_for_idle`'s call waiting up to
`IDLE_WAIT_MAX`; the `get_photo` / `get_preview` reads and `list_folders`,
over `folders` from the home directory `with_home` gives;
`mcp_enabled_setting`, the stored `mcpEnabled` rule), `src/rename.rs` the
folder and RAW file rename (`check_name`, `folder_target`, `file_plan` /
`file_run`, and `rename_folder`, which releases the watcher and the tree
watches around the rename and carries the index rows, the folder count rows
and the recorded trash runs along, given them in a `Context`),
`src/datefolder.rs` the `Rename by
Capture Date…` plan (`plan_of` from the index's capture times, else the file
head; `preview`) and the split (`split_folder`, over the same `Context`,
writing each date folder's count row and dropping the original's),
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
`docs/agents/app-log.md`)), `crates/native` (`riffle-native`)
is the desktop app, built on GPUI, a separate Cargo workspace with its own
`Cargo.lock` and Rust toolchain that uses `riffle-core` and `riffle-backend`, takes GPUI and
gpui-component through the one `gpui-kit` umbrella crate (`gpui_kit::*` is GPUI,
`gpui_kit::component` gpui-component, started by `gpui_kit::init` and
`gpui_kit::open_window`, see `docs/agents/native-app.md`); its `build.rs`
embeds `icons/icon.ico` as the exe's icon resource ID 1 on Windows
(`winresource`), it sets the app identity (`IDENTIFIER`, "Handpic") at
startup so its OS notifications can be delivered, its window opens with
`app_id` `IDENTIFIER` under
gpui-component's `TitleBar` (the app icon and "Handpic", and at its right end
a gear `Button` whose click opens the settings as `ctrl+,` does, its press
kept from the bar's window drag) in place of the OS title bar, and
`packaging/` holds the macOS `Info.plist` and the Linux
`dev.minodisk.handpic.desktop` the `native:bundle:macos` (`Handpic.app` under
`target/<profile>/`, its `Info.plist` given the version and
`Contents/Resources/licenses/` the notices) and `native:install:linux` (the release binary, the
`.desktop` file and the icon under `~/.local`) mise tasks use, and
`NOTICE-gpui`, the GPUI crates' Apache-2.0 notice; the manual
`.github/workflows/package.yml` (`Package`, `workflow_dispatch` only) builds the
release binary per platform and packages it with the scripts of
`tools/package/` (`version.sh`, the crate's `[package].version`; `notices.sh`,
the license notices every package ships into one `licenses/` folder;
`macos.sh`, the ad hoc signed `Handpic-<version>-macos-<arch>.dmg`, also
`mise run native:package:macos`; `windows.sh`, the per-user Inno Setup
`Handpic-<version>-windows-x64-setup.exe` from `packaging/windows/handpic.iss`
(`AppId` `dev.minodisk.handpic`, for good) and the portable
`Handpic-<version>-windows-x64.zip`, also `mise run native:package:windows`;
`linux.sh`, the `Handpic-<version>-linux-x64.tar.gz`, beside it
`packaging/linux/install.sh` (the per-user install of that tarball under
`~/.local` after a SHA-256 check, adapted from gpui-kit's guide) and
`handpic_<version>_amd64.deb` from the `packaging/linux/control` template
(`Depends` from `dpkg-shlibdeps` plus `libvulkan1` and `libwayland-client0`), also
`mise run native:package:linux`), uploading each package and one
`SHA256SUMS` as workflow artifacts; `crates/native`'s `src/datadir.rs` places
its own index at `<cache dir>/dev.minodisk.handpic.native/index.sqlite` (or
`--index <path>`) and its settings store at `<config dir>/<IDENTIFIER>/settings.sqlite`
(or `--settings <path>`) and holds the product's identifiers (`IDENTIFIER`, the
Handpic one, and the earlier `LEGACY_IDENTIFIERS`), `src/logger.rs` writes the `log` lines to stderr
and to a `Handpic.log` in the line format `docs/agents/app-log.md` describes under its own folder (or
`--log <path>`), starting it anew past 1 MB (this crate's `debug` lines too
under `RUST_LOG=debug`), `src/theme.rs` sets gpui-component's
global `Theme`, which `gpui_kit::init` installs at startup, to the mode
the stored `theme` setting resolves to (`Setting`, `{"mode": "system" |
"light" | "dark", "light": <name>, "dark": <name>}` read field by field, a
missing or invalid mode Dark, a name that is not a shipped theme of its
mode `NEUTRAL`; `resolve`, System following the OS appearance) and that
mode's chosen theme: Neutral, mapped to the app's shadcn/ui Neutral dark or
light tokens, its own `DARK` / `LIGHT` palettes, `title_bar` and
`title_bar_border` included, or one of gpui-kit's theme files at the 0.9.1
release, vendored unchanged under `crates/native/themes/` with their
`LICENSE`, embedded and parsed into `ThemeConfig`s (`names`, each mode's
list) and installed unmapped through `Theme`'s `light_theme` / `dark_theme`
slots and `Theme::change` (`apply`, at startup, on a choice
in the Appearance page and, in System mode, on a change of the window's
appearance, which also sets the theme's scrollbar mode to always shown),
and sets beside it the `Roles` global, the colors the app paints that equal
no token (the window's surface and text, the strip's names, failures, burst
band and reject dim; today's values for Neutral dark, else derived from the
tokens by `Roles::of`), which the panes, the strip, the status row, the
menus and the dialogs read with the theme (`cx.theme()`, `roles`), the
judgment, label band and over-the-photo colors staying fixed, and
`SCROLLBAR_TRACK`, the scrollbar track's 16 px the tree's rows and the meta
pane's body keep clear, `src/settings.rs`
reads the backend's settings store, a new one seeded once from a Tauri
`settings.json`, only read (`--import-settings <path>`, else, without
`--settings`, the `IDENTIFIER` folder's under the config dir, else a
`LEGACY_IDENTIFIERS` one's), into the sidecar format, the label names, the keymap, auto-advance, the
sort key, the remembered filter, `mcpEnabled`, `mcpAllowDestructive`, the import root and pattern, the shown panes, their widths (`panelWidths`), the strip's height (`stripHeight`), the theme (`theme`), `lastFolder` and `windowBounds`, and writes them back in order on one thread
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
`src/menu.rs` the strip header's `Sort ▾` and `Filter ▾` menus (`sort_rows` and
`filter_rows`, after `index.html`'s `#sort-menu` and `#filter-menu`
without the `Face focus` help, the filter menu heading every section, its
rating rows from 5 stars down and each kind its own lead; `apply`, what a
click on an item does, the window writing the changed sort or filter;
`Menus`, which one is open), which the window draws as
`deferred` / `anchored` popovers above their gpui-component `Button`s at
the right end of the header row on the strip's top edge, the
color label rows gpui-component `Tag`s, the other leads Lucide icons
from `src/icons.rs` (served by `Icons`, the application's one
`AssetSource`, to `svg().path(..)`: the two own derivatives, the filled
star and the frowning `scan-face`, under their license header,
then the `gpui-kit-assets` catalog icons selected with `icon_assets!`, then
`gpui_kit::assets::Assets`, which serves `star` and the icons
gpui-component's controls draw, with the title bar's
`icons/64x64.png` for `img(..)`),
`src/settingsmodal.rs` the settings modal's model (open state, the pages' order and titles, the key
rules of `modal.ts` less Tab, which `Root` takes, the keymap edits refusing `APP_KEYS`, the Keyboard Shortcuts page's groups: `ShortcutGroup`, `SHORTCUT_GROUPS` and `shortcut_group`, an unknown action under `General`) and `src/settingsview.rs`
its entity, opened on `ctrl+,` (`meta+,` on macOS) as a full-window, opaque, occluding layer the window draws beside its own root `div` over the still-painted main layout, its box under a focus trap (closed by Escape or its `×`; `tab` / `shift+tab` captured as a shortcut through a `NoAction` key context),
its body gpui-component's `Settings` (a searchable sidebar of pages, a text field letting its keys through by the `Input` key context, a pointer press cancelling a capture) over its controls (`Button`,
`Switch`, `Radio`, `Select`, `Kbd`, `Input`, `InputGroup`),
holding the Sidecar page's format radios and label names (`Input` fields, the Lightroom language a `Select` showing the preset in force, whose choice applies that preset's names), the Culling page's auto-advance switch, the Keyboard Shortcuts page (its rows grouped under `Filmstrip`, `Judgment`, `Preview`, `Panes` and `General`, each key a `Kbd`), which replaces the keymap and `Bindings` live, the Cache page's index size and `Clear Cache`, the Import page's root (a read-only `Input` with `Choose…` in one `InputGroup`, `prompt_for_paths`) and validated pattern, the MCP page's `mcpAllowDestructive` switch, which only the user flips, and the Appearance page's System / Light / Dark radios (`MODES`) and its light and dark theme `Select`s (`theme_change`), applied live and written to `theme`,
`src/cache.rs` that Cache page's pure parts (its texts and confirmation, `on_disk_bytes`, the index file with its `-wal` and `-shm`, and `ClearFlow`, a press held for the view's idle point, asked, re-checked and run, dropped when the settings close, or an MCP confirm run or held without asking, `confirmed`), whose clear the window runs as `Index::clear` off the UI thread under the gate's in-flight count, then re-lists the open folder and re-reads the tree's cached counts,
`src/keys.rs` names a GPUI keystroke the keymap's way (`key_name`, a port of
`keys.ts`'s `keyName`), gives a key name's keystroke back for `Kbd` to draw
(`keystroke_of`) and resolves it to its action (`Bindings`),
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
model (the cells' `Geometry`, the image box, cell and pitch `Geometry::of`
derives from the strip's height, the scroll offset, the range kept with `RANGE_MARGIN`,
hit testing and which cell to ask for next, with `strip.ts`'s `missing` /
`ready` / `failed` bookkeeping, and each cell's judgment `Marks`: the stars,
the flag dot, the dimmed reject and the label's `Tint`, and its hover texts'
`tip_regions`: the label's name over the name band, the scan's error over the
rest of the cell), which the window draws along its bottom edge on
a `canvas` with each cell's face focus icon (`MarkState::icon`), under
transparent region `div`s that show those texts as gpui-component `Tooltip`s
and under gpui-component's horizontal scrollbar, whose handle is
`src/stripbar.rs`'s `StripBar` (a `ScrollbarHandle` over the strip's offset
and content width, synced on every render, the offset a drag or a track click
sets taken once at the top of `render`), and `src/thumbs.rs` the thumbnails of the strip's range, read
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
companion's server (the backend's `Mcp` with all its tools and the tree's home directory on a tokio runtime of its own on one
`mcp` thread, started only while `mcpEnabled` is on, on `--mcp-port` for a test
run, each tool call reaching the view as `scan::Event::Mcp`, stopped on quit),
`src/companion.rs` the answers to its tool calls (a port of `companion.ts`
over a snapshot of the view: the strip's selection, its targets as
`selected`, its burst
(`burstOf`, each frame's sharpness from the `Facts`), the coverage table
(`Coverage`: every keymap action, right-click item, menu item, folder
operation and settings page mapped to its tool, to a tool refused with
`not_in_native` until the feature lands (compare, the `recommended` sort,
the Genre filter, `rename_file`, `refit_ranking`, the AI switch) or to the
reason it has none, tested against the keymap's actions),
`set_judgment`'s changes, one undo
entry through the judgment keys' view, row and writer path, the `Key` a
keymap action and its tool both run through the view's `perform` (the moves,
`select_all`, `reject_rest`, `undo` and `redo`, a trash run on top stepped
as `StepTrash`, its redo refused unless `mcpAllowDestructive`), `set_sort` and `set_filter` as the menu clicks the view's
`apply_menu` applies (a view-only folder
refusing the sort), `set_panes` through `change_panels` and `set_overlays`
flipping only the `f`, `e` and `d` overlays and the grayscale latch that differ, each reported back
by `get_view` (`filter`, `filter_offers`, `panes`, `overlays`), `open_folder`
through `open_from_tree` (the open folder not reopened), `tree_folder` through
`tree_toggle` and the tree menu's `folder_action` on the drawn row of the
path, `reveal` through `folder_action` / `files_action`, `Status`, the
`get_status` answer of the status line, the scan pass, a listing or run under
way, the pane's notes (`pane_notes`, `pane_errors`) and the `Operation` under
way or last ended (asking, held, running with done / total, finished,
failed), `trash_photos`, the strip menu's Move to Trash / Delete Permanently
on the given photos or the targets, a move previewed as an `Op` the view keeps
as the `Pending` its id names and run on a confirming call while
`mcpAllowDestructive` is on (through `request_run`, no dialog), else, like
every delete, the app's dialog opened through `confirm_trash`,
`rewrite_sidecars`, `delete_sidecars`, `sequence_timestamps`,
`import_folder` and `rename_by_capture_date`, the tree menu's flows started
through `folder_op` (`FolderOp`), their dialog opened while
`mcpAllowDestructive` is off, else their preview's dialog content kept as the
`Pending` (`mcp_folder` marking the flow, `get_status`'s `pending`; an
import's naming the plan its run copies) and run on a confirming call through
`run_confirmed`, `rename_folder`, the tree's `Rename…` previewed as a
`Pending` and run through `request_rename` while `mcpAllowDestructive` is on,
else the tree's inline rename opened with the name filled in (`ask_rename`)
for the user's Enter, `clear_cache`, the Cache page's `Clear Cache` previewed
and run through `ClearFlow::confirmed` while `mcpAllowDestructive` is on,
else its confirmation opened through `press_clear`, `cancel_operation`, the dialog's Cancel, a sequencing
or import run's cancel or the `Pending` forgotten,
answered while the view drops keys, and `wait_for_idle`,
answered by the view's `observe_self` once `Status::idle`, else at its
timeout, `get_settings`, what the settings modal shows, `set_settings`, the
modal's own `settingsview::Change`s by its rules, which the view's
`apply_setting` applies as it does the modal's (never `mcpAllowDestructive`,
refused while a folder operation runs), and `Busy`, which refuses every request but the reads (`READS`) while
`key_down` drops keys), and `src/viewed.rs` the
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
(`photo_tier`, `af_eye_state`: Good / Neutral / Low, Low being
`full_eye_sharp` under `LOW_FULL_EYE_SHARP`; `MarkState`'s `color` and the strip cell's `icon`), `focus_mark`, `upright_point` and the face boxes' `face_marks`,
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
`load` builds next to the preview: the `EXIF` and `Maker note` groups, the
latter also carrying the index's AF point and frame (`af_rows`) after
`AF tracking`, and an `Analysis` group (`analysis`) of every value pass 1 and
pass 2 stored for the shown file: `metaGroups`' rows with its labels and
strings, the full-size eye score named `Eye focus`, the closed probability, the EAR, the mesh's eye
offset, the edge gap and the preview's sharpness; both from the window's
`Facts` of the file, never the on-demand eyes judgment), `src/viewonly.rs`
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
cached counts of the index's `folder_counts` table (`set_cached_counts`,
keyed by the tree's own spelling, which an unlisted row shows until a listing
or an open's `set_listed_count` gives its own; `CountWrites`, the coalescing
of the counts the tree learns, and of the prunes of the child folders a
listing no longer holds (`prune_keep`), on their way to that table in push
order), the
drawn `rows`, and the path spelling `ancestors_within`, `respell` and
`relation` compare by, case ignored on Windows and macOS), which the window
draws as a `uniform_list`, fills from the backend's `folders`, expands
down to the open folder on every open, keeps following the disk through the
backend's `treewatch` on its expanded, drawn folders (`watched_folders`) and
re-reads the roots when the window becomes active, and its keyboard (the
cursor's `step`, `tree_key`'s Space / Enter, the type-ahead's
`append_typed` / `type_ahead`, and `collapse_all`), which the tree takes on a
click in its pane and hands back on `escape` or a press elsewhere,
`src/treekeys.rs` the keymap actions that still run while the tree has the
keyboard (`TREE_PASSTHROUGH`, `tree_gate`, after `treekeys.ts` but also
letting the strip's `previous` / `next` through, so Left / Right page the
strip, unlike the Tauri app),
`src/context.rs` the
right-click menus (a port of `context.ts`: the items with their action, label,
shortcut (the keymap's key name, which the window draws as `Kbd` text), checked and disabled state, `folder_groups`, the tree's reveal,
`Copy Path`, `Copy Folder Name`, `Refresh` and, off a root, `Expand All` /
`Collapse All`, `strip_groups`, the strip's `Select All`, reveal, `Copy Path`
/ `Copy File Name`, the judgment groups checked from the focused file (left
out in a view-only folder; in the filter menu's order, the rating rows its
stars) and `Move to Trash…` / `Delete Permanently…`,
`shown_path`, a path without the Windows verbatim prefix the shell refuses,
and `Target`, what an open menu acts on), which the window shows as
gpui-component's `PopupMenu` in a `deferred` / `anchored` layer at the
pointer, `src/trash.rs` the strip's `Move to Trash…` /
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
of `import.ts` (`Import…`, and `done_notification`, the OS notification of a
run's end posted only while the window is not active: `Import finished` or
`Import canceled` over the status line's `done_status`), `src/datefolder.rs` the port of `datefolder.ts`
(`Rename by Capture Date…`), `src/rename.rs` the port of `rename.ts`'s folder
parts (the tree's inline `Rename…`), and `src/peaking.rs`
the focus peaking overlay on `e` (`PEAKING_THRESHOLD`, `PEAKING_COLOR` and the
Sobel of `peaking.ts`; one layer per view, the fit view and the 1:1 crop,
keyed by the source image and its drawn device size, one job in flight on the
background executor, the fit view's image resized to its drawn size first),
`src/grayscale.rs` the momentary grayscale preview on a held `g` (`Hold`:
`press`, `key_up` of the held key, `modifiers_changed`, a modifier going up
ending it, and `release`, on the window's deactivation and the view's blur;
`set`, the MCP `set_overlays` latch, which only `set` or the key's own
release ends;
only the photo, the fit view and the 1:1 placeholder and crop, is drawn with
the image sprite's grayscale flag, the overlays keeping their colors),
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
and the 1:1 crop alike), `src/progress.rs` the window's one progress indicator
(`fraction`, a port of `progress.ts`'s `progressWidth`, and `shown`, a Move to
Trash / Delete Permanently run before a folder operation's run before the
scan's running pass), which the window draws as a gpui-component
`ProgressCircle` in the status row's `StatusBar`, right of the position,
and `src/panels.rs` the panes shown (`Panels`, a
port of `panels.ts` and the Tauri `panels_setting`: `toggle`, `toggle_sides`,
`from_setting` and `setting`, the `panels` value of the settings store, a
missing or non-boolean entry meaning shown), which the window's
`change_panels` sets on `toggleLeft`, `toggleRight`, `toggleStrip` and
`toggleSides`, the left pane giving up the keyboard when hidden, a 1:1 crop
re-cut and `panels` written (neither read nor written under `--auto`), and
`src/panewidths.rs` the side panes' widths (`PaneWidths`, `DEFAULT` 220 px
within `MIN` and `MAX`, the preview kept `PREVIEW_MIN`; `from_setting` and
`setting`, the `panelWidths` value; `dragged`, the widths a drag of a pane's
boundary gives; `drawn`, the panes narrowed while the window cannot hold
them), which the window sets from gpui-component's standalone
`resize_handle` on the preview's edges and writes at the drag's end (neither
read nor written under `--auto`), and `src/stripheight.rs` the strip's
height, its cells' (`DEFAULT` 128 px within `MIN` and `MAX`, the preview kept
`PREVIEW_MIN`; `HEADER`, the header row over the cells, and `GUTTER`, the
9 px band under them holding the horizontal scrollbar's track, which the
drawn pane adds; `from_setting` and `setting`, the `stripHeight` value;
`dragged`, the cells' height a drag of the pane's top edge gives, the header
and the gutter counted), which the window sets from a vertical `resize_handle` on the
header's top edge, the cells
resized around those in view (`Strip::set_geometry`), and writes at the
drag's end (neither read nor written under `--auto`), next to
`open`, the OS folder picker whose folder opens as a click in the tree does
unless another open came first, and `src/dropped.rs` the rules of a drop onto
the window (a port of the Tauri `dropped_dir`: `single`, the one path a drop
names, `dropped_dir`, a directory as is and a file by its parent, and
`DROP_TEXT`, the status line's refusal), which the window's root `div` takes
through `on_drop::<ExternalPaths>`, resolving off the UI thread, a later drop
winning, and opening the folder as a click in the tree does, while
`drop_outline` draws the Tauri drag outline over the main area.

The `.ts` files a `crates/native` module is called a port of (`sort.ts`,
`filter.ts`, `focus.ts` and the rest), and the Tauri commands some are ported
from, were the removed Tauri app's frontend and commands; they are in git
history.

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
