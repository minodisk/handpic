# Working on `crates/native`

Read this before touching the GPUI app in `crates/native`. It lists what the
port has met so far, each with the reason.

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something here,
**Measured** steered a design decision, **Inferred** comes from sources or docs
only.

Source: [the gpui-port plan](../plans/20261109-gpui-port/plan.md), its spike
and Step 1's [learnings](../plans/20261109-gpui-port/learnings.md).

## Toolchain

### GPUI comes from one pinned Zed rev (Inferred)

`gpui` and `gpui_platform` are git dependencies on Zed's repository at
`b5cd310b3b54dfc659fcc06453e6358843dfe216`; crates.io's `gpui 0.2.2` is stale.
Zed pins Rust 1.98.1, and so does `crates/native/mise.toml`.

- Rule: bump the rev only in a deliberate chore PR, and record API changes met
  at the bump here.

### Only two of Zed's patches are needed (Inferred)

`[patch.crates-io]` carries Zed's `async-task` and `async-process` forks. Zed's
other patches (`cpal`, `notify`, `calloop`, `khronos-egl`, ...) are not needed
by `gpui` / `gpui_platform`.

- Rule: do not copy the others. Do not move these two to the root workspace
  while the Tauri app lives there: both crates are in its lock, so a workspace
  patch would swap them for Zed's forks in the shipped app.

### Linux needs system libraries, macOS and Windows do not (Inferred)

Per Zed's `script/linux`: `libfontconfig-dev`, `libwayland-dev`,
`libx11-xcb-dev`, `libxkbcommon-x11-dev`, `libvulkan1`, plus a Vulkan-capable
driver at run time. macOS needs the Xcode CLT (Metal), Windows only MSVC.

### Without NASM, mozjpeg silently loses SIMD (Measured)

`mozjpeg-sys`'s build script runs `nasm -v`, prints a `cargo:warning` and
continues without SIMD when that fails, and cargo hides build-script warnings
of non-path dependencies. RAF decode measured about 99 ms without SIMD and
46 ms with it. `crates/native/build.rs` therefore fails the build, and its own
`cargo:warning` is the only place the SIMD state is visible.

### winget's NASM is per-user and not on `PATH` (Hit)

`winget install NASM.NASM` installs to `%LOCALAPPDATA%\bin\NASM`, not
`C:\Program Files\NASM`, and adds nothing to `PATH`.

- Rule: put that directory on `PATH` for every build.

### `mozjpeg-sys` ignores the `NASM` variable (Inferred)

`nasm-rs` honors `NASM` as the binary path, but the `nasm_supported()` probe of
`mozjpeg-sys` 2.2.3 runs plain `nasm` from `PATH`. `build.rs` therefore runs
`nasm -v` from `PATH` only and fails when it does not run, whatever `NASM` holds.

- Rule: put `nasm` on `PATH`; setting `NASM` does not satisfy the build.

## Backend

### The backend names no event and no `AppHandle` (Inferred)

`riffle-backend` serves both apps, so it reports through callbacks:
`watch::Watch::spawn` takes `on_changed`, and `watch::set` / `release_under`
take the `&Watch`. Each app's own thin module owns the event name and payload;
in the Tauri app that is `crates/app/src/watch.rs` with `folder-changed`.

- Rule: never add `tauri`, an event name or an `AppHandle` to `crates/backend`;
  take a callback or the state itself.

### The backend is compiled twice (Inferred)

`crates/backend` is a member of the root workspace, and `crates/native`, a
separate workspace with its own `Cargo.lock` and Rust 1.98.1, uses it by path,
as it does `riffle-core`. A dependency added to the backend also lands in
`crates/native/Cargo.lock`, and clippy lints the backend only from the root.

- Rule: check a backend change with both `mise run ci` and
  `mise run native:check`.
- Rule: `#[cfg(test)]` does not cross a crate boundary. When code moves into the
  backend, a helper the app's tests still use (for example `Index::row_flags`)
  must lose its `#[cfg(test)]` or be copied into the app's test module.
- Rule: `mise run native:check` runs clippy and tests but does not build
  `riffle-native.exe`. Run `cargo build` before launching the exe, or you run a
  stale binary. A still-running older `riffle-native.exe` locks
  `target/debug/riffle-native.exe` and `cargo build` fails with `os error 5`;
  rename the running exe aside (for example `riffle-native.stale-<pid>.exe`)
  instead of killing it, or ask the user to close it.
- Rule: `crates/native/Cargo.lock` resolves its own versions (`notify` 8.2.2
  against 8.2.0 in the root lock). `watch::set` (watch, replace, the debounce)
  works on 8.2.2: copying into and deleting from the open folder resynced it.
  The native app does not call `release_under` / `settle` yet, so that
  `configure` still queues behind `Unwatch` on 8.2.2 (see `tauri-app.md`) is
  unverified; re-check it when the folder rename (port Step 8) needs it.

### The judgments' Tauri-free pieces live in the backend (Inferred)

The folder-open sidecar pass (`sidecar::reconcile_of`, with `SidecarError`
and `MAX_SIDECAR_BYTES`), the meta pane's formatting (`exif::Metadata` and
`exif::metadata(name, &shot)`) and the stored `labelNames` value
(`sidecar::label_names_setting` / `label_names_value`) moved out of
`crates/app/src/commands.rs` so the native app can call them; the Tauri
commands call them from there. `raw_only` stayed in `crates/app`: the native
app's view-only rule is the frontend's `viewonly.ts`, ported on its own.

- Rule: a Tauri-free helper the native app needs moves into the backend, it is
  not copied, so the two apps cannot drift in what they read and show.

### The settings are the Tauri app's, read-only (Inferred)

`settings.rs` reads the Tauri app's `settings.json` (`sidecarFormat`,
`labelNames`, `shortcuts`, `autoAdvance`) with the backend's rules
(`SidecarFormat::from_setting`, `sidecar::label_names_setting`,
`Keymap::from_overrides`, `autoAdvance` on unless a boolean says off), so a
native judgment writes the same sidecars with the same label names and keys
as the Tauri app on the same folder (the user's store says `"both"`). The
file is `--settings <path>`, else the first that exists of
`<config dir>/<IDENTIFIER>/settings.json` and each
`<config dir>/<LEGACY_IDENTIFIERS[i]>/settings.json`, so it is found before
and after the Handpic rename's migration. A missing, unreadable or non-object
file is logged once with the paths tried and gives the defaults (`Xmp`, the
English names, the default keymap, auto-advance on). The native app never
writes it; its settings UI is port Step 9.

- Rule: every debug or hand-check run passes `--settings` with a test file,
  or it picks up the user's real format.

### The folder open reads the sidecars before pass 1 (Measured)

`scan::list` lists with `read_listing(dir, Some(format))`, and
`Scan::prepare` runs `stat_sidecars` and `sidecar::reconcile_of` right after
`reconcile`, on the open's background task and before the scan thread starts,
as the Tauri `scan_folder` orders them, logging `scan sidecars: dir=…
dirty=N changed=N in …ms`; a view-only folder skips it. The sidecars it could
not read go to the pane's notes as `<name>: <message>`. On a copied folder of
8 DNGs with XMP and `.dop` sidecars and a fresh index, the first open logged
`changed=8` and the second `changed=0`, under `"xmp"` and under `"both"`, and
no sidecar's mtime moved.

- Rule: `Index::entries` joins `files`, so on a fresh index it returns nothing
  until pass 1 has written the files' rows; the view reads the judgments at
  the open and again at pass 1's `Done`. The `ratings` rows exist right after
  `reconcile_of`, so a ratings-only read (`Index::rows_of`) would show the
  marks before pass 1, at the cost of the "entry seen" set Step 4's
  `label_known` follows.

### A folder change re-lists, never mid-scan (Measured)

`main.rs` holds one `watch::Watch` whose callback sends `Event::Changed` into
the scan's channel; each listing calls `watch::set` before `read_listing`, so a
change during the listing is not missed. `resync.rs` defers a change that
arrives while a listing or its scan runs to that scan's end, one pending at
most, as the Tauri frontend's `resyncPending`: a re-listing mid-scan would
cancel and restart it. A re-listing carries the current file by path, the
strip's offset (`carried_offset`) and the loaded thumbnails
(`carried_indices`), bumps a listing counter that drops the previous listing's
preview reads, and always reads the current preview again (it also refreshes
the status line's `n/total`).

- Rule: a file still being copied in changes size between events, so each
  debounced event re-extracts it (`todo=1 removed=1`) until the copy ends;
  that is expected, not a loop.

### The native app keeps its own index (Inferred)

The index lives at `<dirs::cache_dir()>/com.minodisk.handpic.native/index.sqlite`,
apart from the Tauri app's `com.minodisk.riffle/index.sqlite` (or
`com.minodisk.handpic/` after the Handpic rename); `--index <path>`
overrides it. The folder is named after the Handpic identifier
(`datadir.rs`'s `IDENTIFIER`, next to `LEGACY_IDENTIFIERS`) and nothing is
migrated from the earlier `com.minodisk.riffle.native`: the native app never
shipped, so an old developer cache is simply rebuilt by pass 1. Sharing one file would put two processes on it (the Tauri
launch's `VACUUM` against a native scan's writes), make two
`EXTRACTOR_VERSION`s re-extract each other's rows, and let a native run's
`reconcile` drop the user's rows. The cost is a cold pass 1 on each folder's
first native open. An index that cannot be opened leaves the pager working
with `no index: …` on the status line.

- Rule: never point a debug or measurement run at the Tauri app's index; use
  `--index` with a fresh file.
- Rule: `reconcile` drops every row of the folder it was not given, so it
  always gets the whole listing; `--size` / `--max` narrow only what is shown.
- Rule: on Windows `canonicalize` gives `\\?\D:\...`, and every index key and
  listed path in the native app carries that prefix, as in the Tauri app. Key
  anything compared with the index or the scan's `ready` paths (strip cells,
  thumbnails, the watcher's owner string) on `FileStat.path.to_string_lossy()`
  or the canonical `PathBuf`, never on a path rebuilt from the user's argument.

### `run_scan` runs on a `std::thread` (Inferred)

`index::run_scan` blocks for the whole pass and has its own rayon pool. On
`cx.background_spawn` it would hold one of GPUI's background threads, which
the preview decode needs, for as long as the folder takes. `scan.rs` spawns a
thread per scan; a new open cancels and joins the previous one, and the first
scan joins the launch's eviction thread before it reconciles, so the
`VACUUM` never races it.

### The hot list is set before the scan's workers start (Measured)

`run_scan`'s workers take the first file of the hot list not yet taken, else
the next in listing order. With the hot list sent right after `Scan::start`
(as the Tauri frontend does after `start_scan`), the first wave of the 22
workers had already taken files 0-10 of a 373-file folder opened at file 199;
with `ScanFocus::set` called on the handle before `Scan::start`, the first
committed batch was the 20 files of the strip's range around 199 plus 0 and 1
(the two workers left over). Later updates are debounced by
`SCAN_FOCUS_DELAY` (a `cx.spawn` timer task replaced on each change; dropping
a GPUI `Task` cancels it).

- Rule: an index write from the view (`set_last_viewed`, `mark_viewed`) goes
  through the writer on `cx.background_spawn`, one in flight (`viewed.rs`);
  a failure is logged, never shown.
- Rule: `Index::evict` drops a folder whose `opened_at` is over 30 days old
  with its `last_viewed`; seeding a test index by hand needs a current
  `opened_at`.

### Pass 2 runs on pass 1's thread (Inferred)

`Scan::start`'s thread runs `index::run_faces_scan` over `faces_todo` right
after `run_scan`, with the same `ScanFocus`, cancel flag and
`scan_threads()`, as the Tauri app's `run_faces_pass` does on its one task.
One thread and one flag mean `Scan::stop` cancels and joins whichever pass is
running, and the hot list reaches both passes through the one handle
(`focus_scan` gates on either pass running). The thread starts on every open
with an index, even when pass 1 has nothing to do, so rows below
`FACES_VERSION` still get pass 2; a no-op pass costs one `faces_todo` query.
`Event::FacesDone` is sent also when the pass was canceled, had nothing to do
or `faces_todo` failed, so the view always learns the scan is over. The first
non-empty batch's file names go to the log as `scan faces first:`.

- Rule: the deferred resync (`finished`) runs at `FacesDone`, not at pass 1's
  `Done`, as the Tauri frontend drains its deferred rescan at `faces-done`. A
  re-listing during pass 2 would be harmless (a canceled file keeps its old
  `faces_extractor`, so the next scan resumes it) but would restart the pass;
  waiting for its end keeps a burst of changes from restarting it over and
  over.

### The backend logs through `log`, printed by `logger.rs` (Hit)

Without a logger the backend's `scan extract` lines went nowhere. `logger.rs`
prints `[target][LEVEL] message` to stderr at `Info`, which also shows GPUI's
own lines (the UI font, the GPU, and an `ERROR` for the missing DXGI debug
interface on a machine without the Graphics Tools, which is harmless).
`RUST_LOG=debug` (or `trace`) adds this crate's own lines of that level, such
as `key: <name> action=…` for every keystroke, and not GPUI's.

## GPUI

### Callbacks from other threads go through one channel (Inferred)

`AsyncApp` is not `Send`, and a `BackgroundExecutor` task cannot update an
entity, so a callback on a rayon or watcher thread cannot touch the view. The
view owns one `futures::channel::mpsc::unbounded` sender; the callback only
sends, and a `cx.spawn` task drains the receiver with `this.update(...)` per
message. Each scan message carries the scan's id, so a stopped scan's late
messages are dropped.

- Rule: a callback from a non-GPUI thread only sends into that channel.

### `RenderImage` holds BGRA and must be released (Inferred)

`RenderImage` holds BGRA in an `image::RgbaImage`. Every image handed to the
renderer must be released with `cx.drop_image(image, None)` or it leaks on the
GPU.

### Textures have one mip level (Inferred)

A large downscale (a RAF fit to a 1600 px window) may alias. Not settled: one
captured 1:1 crop of a 4416x2944 RAF fit view, a clapboard wall without fine
repeating texture, shows no moire, but no comparison with the Tauri app or a
hard-case image has been made. Three native-side RAF frames showed no moire;
the comparison with the Tauri app is pending (see the pending manual check in
the gpui-port plan's `learnings.md`).

### The image sprite cannot rotate (Inferred)

`PolychromeSprite` has no `transformation`; only the monochrome SVG sprite
does. A rotation has to happen on the pixels before painting.

### There is no after-`Present` hook (Measured)

`cx.defer` from the paint closure runs after the frame's `draw()` and
`present()`, which is the stamp the spike's timings used. It is a workaround
and may change with a GPUI bump. `crates/native` follows that pattern: the
paint closure stamps `paint_started` and `paint_finished`, then the deferred
callback stamps `presented`, prints the `page` line and releases the retired
images with `cx.drop_image`. Doing the release earlier than that deferred
callback could pull a texture out from under the frame being presented.

### Rotating on the CPU needs threads for a RAF's budget (Measured)

Windows, release, idle, 31 runs of synthetic BGRA buffers, p50 in ms, 90 degree
turns (orientation 6 / 8): 1616x1080 plain loop 3.2, 64-pixel tiles 1.6, tiles
over 8 threads 0.9; 4000x3000 plain 62, tiles 21, tiles over 8 threads 9-11;
4416x2944 plain 70, tiles 23, tiles over 8 threads 12-13. Allocating the output
alone costs about 8 ms at 4000x3000 (a plain copy of the buffer takes that long).
In the app on real portrait files: 1616x1080 ARW `orient` 1.3-1.5 ms, 1920x1280
RAF about 2.2 ms, 4000x3000 RAF (GFX 100) 7.6-8.5 ms.

- Rule: `orient.rs` tiles over 8 scoped threads because the tiled loop alone
  misses the plan's 15 ms budget on a 4000x3000 RAF; orientation 1 returns the
  buffer without a copy and 3 reverses in place.

### mozjpeg reports corrupt input by unwinding (Inferred)

`mozjpeg` 0.10's decoder calls `resume_unwind` on a libjpeg fatal error, so a
corrupt file would otherwise end the background task and leave the pager's
request in flight forever.

- Rule: wrap every decode in `catch_unwind`, as `load` does through `guarded`.

### The strip is a `canvas` over a pure model, not a `uniform_list` (Inferred)

`uniform_list` is vertical-first; its horizontal mode
(`ListHorizontalSizingBehavior::Unconstrained`) is about item width, not a
horizontal virtual list. A flex `div` with one child per file lays out every
cell each frame. The strip is one `canvas` that paints only the cells on
screen, fed by `strip.rs` (offset, range, hit test, request order), whose
range can go to `ScanFocus` as it is. Its thumbnails come from
`thumbs.rs`, four reads at a time through the index's read-only connection,
and every image a cell drops goes through `retired`, like the preview's.

### A `canvas` gets no input; its wrapping `div` does (Inferred)

`canvas` registers no hitbox and takes no listeners. The strip's canvas sits in
a `div` with `on_mouse_down` and `on_scroll` (`on_scroll_wheel` is a deprecated
alias at this rev, and `ScrollWheelEvent` of `ScrollEvent`). The events carry
window coordinates; the strip spans the window from its left edge, so the
event's `x` is the strip's `x` and `strip::hit_test` takes it as is. The div's
hitbox already keeps clicks and wheels above the strip out.

- Rule: a canvas that needs input sits in a `div` that owns the listeners, and
  maps the event position against where the canvas is laid out.

### A Windows mouse wheel arrives in lines (Inferred)

`gpui_windows` turns `WM_MOUSEWHEEL` into `ScrollDelta::Lines`, the notch
times the system's wheel-scroll lines (3 by default), positive away from the
user; Shift moves it to `x` (`wheel_scroll_chars`). A precision touchpad
through Direct Manipulation gives `ScrollDelta::Pixels`. `pixel_delta(window.line_height())`
turns either into pixels; the strip subtracts it from its offset, `x` when
non-zero, else `y`, so a wheel toward the user scrolls right.

### A stroked path has butt caps, and no cap option is reachable (Inferred)

`PathBuilder::stroke(width)` builds lyon's `StrokeOptions::default()`, whose
caps are `LineCap::Butt` and joins `Miter`. `with_style(PathStyle::Stroke(..))`
could set square caps, but gpui re-exports `StrokeOptions` and not lyon's
`LineCap`, so naming one would need a direct `lyon` dependency. The `f` mark's
crosshair arms are therefore lengthened by half the stroke width at both ends
(`overlay.rs`'s `segments`), where the Tauri mark's `lineCap = "square"`
reaches, so each arm end keeps the 1 px dark outline. The AF frame is a closed
polygon (`add_polygon(.., true)`) and needs no caps.

- Rule: stroke an open path that must look like a canvas square cap by
  extending its ends by half the width; do not add `lyon` for it.

### A canvas paints text through the window's text system (Inferred)

A `canvas` has no child elements, so the file names are shaped with
`window.text_system().shape_line` (one line, centered with
`TextAlign::Center` over the cell width) and the failure text with
`shape_text` and a wrap width, each clipped with `with_content_mask` to its
cell. The run's font comes from `window.text_style().to_run(len)`.

### Text has no stroke; a label's halo is dark copies (Inferred)

GPUI shapes and paints glyphs filled only; there is no text stroke like the
canvas `strokeText` the Tauri face labels use for their 3 px dark halo. A
dark quad behind the label was the other option; it hides more of the face
than the Tauri halo does. `overlay.rs`'s `paint_label` instead paints the
label in `rgba(0, 0, 0, 0.8)` at the eight 1 px offsets and then in the
label color on top, a ring about as wide as the Tauri stroke's outer
1.5 px, which keeps the look close to the Tauri app's. That it reads on a
bright face is not yet checked by eye. A dot is a `paint_quad` whose corner radii are
half its side, with no `PathBuilder` arc. The label's line height is the
shaped line's `ascent + descent`, so its origin is the text's top, as the
canvas's `textBaseline = "top"`.

- Rule: halo text over a picture with offset dark copies; do not paint a box
  behind it.

### An on-demand judgment is superseded through a shared atomic (Measured)

`ondemand::read_eyes` runs on `cx.background_spawn` and asks its `current`
closure before the detection and before the model. The view cannot answer
from another thread, so `eyes.rs`'s `EyesCache` keeps the latest ticket's id
in an `Arc<AtomicU64>` the closure compares its own id with; a newer request
stores its id and a clear (another folder, a re-listing) stores `0`, which no
ticket has. Opening another folder while a judgment was in its detection
logged `eyes ... model=0.0ms superseded` on the `marks` hand-check folder,
twice out of two runs. Only one judgment runs at a time and the file current
when it settles is asked for next, so 60 presses 30 ms apart over a 300-ARW
folder logged 7 judgments, not 60.

- Rule: hand a background job a cheap `Fn() -> bool` over a shared atomic to
  stop it; do not try to reach the view from the worker thread.

### Key names on Windows are lowercase (Measured)

`KeyDownEvent.keystroke.key` is `"right"`, `"left"`, `"home"`, `"end"` and `"q"`,
checked with injected `keybd_event` key events in the running app. A non-extended
`VK_RIGHT` / `VK_LEFT`, the keypad's, gives the same names.

### Every keystroke goes through the keymap (Inferred)

`keys.rs`'s `key_name` names a keystroke as the keymap does: the modifiers
held in `ctrl+alt+shift+meta+` order (`platform` is `meta`), then the key
lower-cased, GPUI's `left` / `right` / `up` / `down` as `arrowleft` ...
`arrowdown`; a lone modifier has no name. `Bindings` maps each bound key to
its action once at startup. The window's handler takes `home`, `end` and `q`
literally first (the native app's own keys, bound to no action), then the
actions: `previous` / `next` page and `focus` toggles the `f` overlay and `zoom` the `z` 1:1 view, so a
user's override of those keys applies; any other action is logged at `debug`
until its step lands. A later step's key (zoom `z`, peaking `e`) is added as
its keymap action in that `match`, not as a literal key.

Read from `gpui_windows` at the pinned rev, not yet seen in the running app
(the pending manual check in the gpui-judgments plan's `learnings.md`):

- A shifted letter arrives as the lower-case letter with `shift` set
  (`shift+x`). A shifted digit or punctuation key arrives as the character it
  types with `shift` cleared (`!` for Shift+1, from `get_shifted_key`),
  where `keys.ts` names it from `event.code` (`shift+1`); no default key is
  one, but a user override such as `shift+1` would not match.
- With Ctrl or Alt held, a digit is `MapVirtualKeyW`'s unshifted character,
  `1` on a US or JIS layout (`ctrl+alt+1`), but `&` on AZERTY, where the
  Tauri app's `event.code` still says `1`. Windows reports AltGr as Ctrl+Alt,
  so on a layout with AltGr `ctrl+alt+<digit>` is also what AltGr+digit
  sends; the default color keys are the Tauri app's, so both apps behave the
  same there.

### The canvas's size reaches the view through a shared cell (Inferred)

The preview canvas's bounds are known only inside its paint closure. The
closure writes the size in device pixels (`bounds.size * window.scale_factor()`,
rounded) into an `Rc<Cell<(u32, u32)>>` the view also holds, as `retired` and
`pending` are shared; the 1:1 view reads it when it asks for a crop. A deferred
entity update from the closure was the other option and needs more of
`main.rs`. The value is the last painted frame's: a resize is seen by
`observe_window_bounds` before the next paint, so the crop is re-cut only
after `zoom::CROP_RESIZE_DEBOUNCE` (150 ms), by when the new size has been
painted.

- Rule: read the cell after at least one paint; before that it is `(0, 0)`
  (`crop_size` clamps it to 1).

### 1:1 is one JPEG pixel per device pixel (Inferred)

`paint_image` snaps the visible bounds to device pixels (`snap_bounds`) and,
when the image runs past the clip bounds, cuts the atlas sub-tile by the
visible fraction, rounded. `zoom::Paint` therefore places the upright crop with
its point of interest on a whole device pixel (`canvas_center` floors the
canvas center in device pixels) and gives it its own size divided by
`scale_factor`, so both edges land on whole device pixels and the sub-tile is
cut on whole texels. The window ran at scale 1.0 (96 dpi, a 1600x1000 client)
during the check; that the crop shows without resampling blur by eye is
pending the user (see the zoom-peaking plan's `learnings.md`).

- Rule: anything meant to be 1:1 is placed in device pixels and divided by
  `scale_factor` at the end, never laid out in logical pixels.

### A crop is on screen 8-112 ms after `z` on a busy machine (Measured)

Release build, `zoom` lines of `z` presses on the zoom-peaking test folder,
not idle (other sessions ran), the file cache warm, 25 lines at scale 1.0
(crops of 1024 px per axis at most): four ARW files `read` 0.7-4.7 ms,
`decode` 3-27 ms, `orient` 0-20 ms on the portrait ones, `keypressToPixels`
8-43 ms (112 ms with the outlier below); two RAF files (X-E5, X-H2S) `read`
1.6-5.2 ms, `decode` 14-33 ms, `keypressToPixels` 19-89 ms; two JPEG-only
files `decode` 21 and 47 ms.
`decode` includes the RGBA to BGRA swap. Two `orient` outliers of 36 and 76 ms
(a 1024 px crop over 8 scoped threads, 1-2 ms otherwise) are attributed to the
load, not measured apart.

### IME handling is unverified (Inferred)

It exists in `gpui_windows`; nothing has exercised it yet.

### The native folder picker is `cx.prompt_for_paths` (Measured)

It takes `PathPromptOptions`. With `directories: true` and `files: false` it
is the OS folder dialog; Enter on a typed path navigates into the folder, and a
second Enter selects it.

## Timings

### `keypressToPixels` differs from the Tauri app's (Inferred)

The `page` line uses the Tauri line's `wait` / `read` / `decode` meanings,
collapses `ipc + post + worker` into `hop` (the time from the background result
to the update on the main thread), and splits `draw` into
`frameWait + paint + present`. `keypressToPixels` includes `present`; Tauri's
stops at `drawImage`. It starts at the latest press of the file that ended up
shown.

### The meta pane is `div` rows, and its cost lands in `present` (Measured)

The right-hand pane (220 px, the Tauri `#info` width) lays out the file
name, `n / total`, the `meta.rs` groups and the notes as `div`s, built from
the `Metadata` that `load` formats on the background thread next to the
preview, so paging reads nothing new. Release, `--auto 20` on the 20-file ARW
compare folder, a fresh `--index` per run, before and after interleaved in one
session with other sessions building on the machine (CPU 12-31 % before each
run, so noisy): `paint` p50 0.70 / 0.82 ms before against 0.90 / 0.76 ms after,
`present` p50 0.19 / 0.22 against 0.44 / 0.41 ms, `keypressToPixels` p50
17.97 / 18.65 against 18.02 / 18.52 ms. `paint` times only the preview
canvas's closure; the pane's `div`s paint after it in the same frame, so their
cost shows as about +0.2 ms in `present`. The `div` rows were kept (the plan's
rule: a canvas only when `paint` grows by more than 1 ms).

- Rule: the view-only flag comes from the whole listing (`Listed.files`), not
  the `--size` / `--max` subset, and is set on every open and re-listing
  before the strip is built.
- Rule: a stage that times one canvas's closure does not see the rest of the
  element tree; look at `present` (and `frameWait` for layout) when adding
  elements beside it.

### The strip costs the pager under 1 ms (Measured)

Same-session A/B, the pager with the strip, the index and pass 1 against the
pre-strip pager (`fb7fcdf4`), three runs each, 60 presses, warm index: median
p50 `keypressToPixels` +0.80 ms on ARW and +0.51 ms on RAF. `paint` did not
move; `present`'s p95 rose from about 0.2 to 0.55-0.85 ms on ARW. See
[the plan's results](../plans/_archived/20261110-gpui-strip-index/results.md).

- Rule: compare against a binary built in the same session, alternating; the
  pre-strip pager's ARW p50 was 3.5 ms lower in this session than the pager's
  in the port's Step 3 session, and `frameWait`'s p50 ranges over 6-10 ms across runs of one binary.

### During pass 1 the preview reads what the scan already read (Measured)

Stepping with the arrow through 520 cold ARW files while pass 1 ran (27.7 s),
the preview `read` was 0.96 ms p50 and 1.22 ms p95, against Tauri's 14-16 ms
p50 at `READ_SLOTS` 2. The hot list puts the current file and the strip's range
first, so the next file's head is usually cached before the press. A build
without the pause read the same at p50 / p95; its one 336 ms read came on a
slower disk and is not attributed.

- Rule: copying test files with `cp` leaves them in the OS cache (the second
  pass 1 over a 520-file copy took 0.41 s). For a cold pass 1 copy them with
  `robocopy /J` (unbuffered) and use each copy once.
- Rule: to measure the pause itself, press onto files the scan has not read
  (a jump, not the next file); `--auto` only steps to the next file.
