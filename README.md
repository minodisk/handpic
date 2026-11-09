<p align="center"><img src="./crates/app/icons/128x128@2x.png" alt="Handpic" width="128"></p>

<p align="center">English | <a href="./README.ja.md">日本語</a></p>

# Handpic

A culling app for going through the RAW files you shot, fast, and deciding what
to keep and what to throw away.

- It reads only the JPEG previews embedded in the RAW files, so paging through a
  folder of thousands of shots never keeps you waiting.
- Stars, picks / rejects and color labels are written to sidecar files that
  tools that read XMP sidecars, such as Lightroom, and DxO PhotoLab can read.
  The RAW files themselves are never written.
- There is no developing and no editing. It does one thing: choosing.

For the supported formats, see [Compatibility](#compatibility); for the
verified cameras, see [docs/humans/cameras.md](./docs/humans/cameras.md).

## Getting started

### Install

Download the file for your OS from the latest release on the
[Releases page](https://github.com/minodisk/handpic/releases):

| OS | File |
|----|------|
| macOS, Apple Silicon | `Handpic_<version>_aarch64.dmg` |
| macOS, Intel | `Handpic_<version>_x64.dmg` |
| Windows | `Handpic_<version>_x64-setup.exe` |
| Linux | `Handpic_<version>_amd64.AppImage` |

The app is not signed by the OS vendors, so the first launch needs your
permission once:

- **macOS**: on macOS 15 and later, after the first launch is blocked, open
  System Settings → Privacy & Security → Open Anyway. On macOS 14 and earlier,
  right-click `Handpic.app` → Open.
- **Windows**: in the SmartScreen warning, More info → Run anyway.

After that, updates are downloaded automatically on launch and take effect the
next time Handpic starts. For the other packages and the details, see
[docs/humans/usage.md](./docs/humans/usage.md#installing).

### First steps

1. Choose your developing software (Lightroom, DxO PhotoLab, or both) in the
   dialog Handpic shows on its first launch, and for Lightroom also the
   language its menus are in, so the color labels match its names; both can
   be changed later in Settings.
2. Drop a folder onto the window (or open one with `Cmd+O` / `Ctrl+O`). To
   start from a memory card, set the destination in the settings' `Import`
   tab, right-click the card in the folder tree and choose `Import…`; the
   first date folder opens while the copy runs.
3. Page through the shots with `←` `→`, give stars with `1`-`5` and reject with
   `x`.
4. When the focus is in doubt, press `z` for the 1:1 view and check the focus
   point.

Judgments are saved to the sidecars as you go, so there is nothing to save.
When you are done, they are picked up by tools that read XMP sidecars and by
DxO PhotoLab.

### Example workflow

1. Open the camera's RAW folder and cull it: stars for the keepers, `x` for
   the rest ([Key features](#key-features)).
2. Open the folder in Lightroom or DxO PhotoLab, which read the judgments from
   the sidecars ([Working with other software](#working-with-other-software));
   develop the keepers and export them as JPEGs.
3. Right-click the export folder in the folder tree and choose
   `Sequence JPEG Timestamps…` to make `<folder>-sequenced/`
   ([Key features](#key-features)).
4. Open `<folder>-sequenced/` in Handpic; it opens view-only, so you can check
   the order ([Key features](#key-features)).
5. Upload `<folder>-sequenced/` to Google Photos.

## Key features

- **Folder tree**: the left pane browses home and the mounted volumes; click
  a folder to open it. The click also gives the tree the keyboard: the
  arrows, `Home` / `End`, `Enter` and typing a name move through and open
  folders, with the culling keys off until `Escape`. Right-click a folder
  to reveal it in Finder / File Explorer / the file manager, copy its
  path or name, rename it with `Rename…` or after its photos' capture date
  with `Rename by Capture Date…` (one `YYYY-MM-DD` folder, or one sibling
  folder per date when they span several), rewrite its sidecars from
  Handpic's index with `Rewrite Sidecars from Index…` (patching only the
  judgments, so Lightroom's and PhotoLab's edits stay) or move them to the
  Trash with `Delete Sidecars…`; a slow second click on the
  open folder's name renames it too. The name is edited in place: `Enter`
  or a click away renames, `Escape` cancels, and the index, the ratings
  and the remembered position follow the folder, which reopens under its
  new name when it was the open one.
- **Import from a card**: right-click a memory card, or any folder, in the
  folder tree and choose `Import…` to copy its RAW files, its JPEGs and the
  sidecars next to the RAW files into one folder per capture date, under the
  root and the pattern set in the settings' `Import` tab (`%Y/%F` by default,
  so `2026/2026-10-24`). Videos stay behind and the card is never modified.
  A preview lists what will be copied; what is already there is skipped,
  a same-named file of another size is listed and never overwritten, and
  files without a capture date are listed and skipped. The first date folder
  opens as the run starts and fills as files land, so culling starts while
  the copy runs ([details](./docs/humans/usage.md#features)).
- **Filmstrip**: thumbnails run along the bottom; click one to show it.
  `Cmd+click` / `Ctrl+click` adds or removes one, `Shift+click` and
  `Shift+←` / `Shift+→` select a range, `Cmd+A` / `Ctrl+A` selects every
  file the filter shows, and every judgment applies to the whole selection.
  Right-click a file and choose `Rename…`, or click the shown file's name
  again after a moment, to rename it in place: `Enter` or a click away
  renames, `Escape` cancels, and its XMP and `.dop` sidecars and its
  ratings move with it. Right-click and choose `Move to Trash…` to move the
  file, or the whole selection, to the Trash with its sidecars after a
  count-only confirmation (filter to the rejects and select all to clear
  them out). Nothing is deleted: one `Edit > Undo` brings the whole move
  back with the judgments, and `Edit > Redo` moves it to the Trash again.
  When the disk is full, `Delete Permanently…` removes them with their sidecars
  instead of moving them: there is no Undo, and only a mouse click on its
  confirmation goes ahead.
  The right-click menu also reveals the file in Finder / File Explorer and
  copies its path or file name (every selected file's, one per line).
  `Ctrl+Alt+ArrowDown` (`Alt+Cmd+ArrowDown` on macOS) hides the filmstrip,
  `…ArrowLeft` the folder tree, `…ArrowRight` the meta pane and `Tab` both
  side panes, to give the viewer more room (the first three are also check
  items in the `View` menu). The meta
  pane's `Maker note` group shows the Sony AF, drive, stabilization and
  picture settings.
- **1:1 focus check**: `z` shows the image at 1:1, centered on the focus point.
  Paging keeps the zoom.
- **Focus peaking**: `e` lights the image's strong edges in red over the
  preview and the 1:1 view, so the in-focus plane stands out at a glance.
- **Focus mark**: `f` draws the AF frame the camera used around a crosshair on
  its focus point when the camera records the frame, the crosshair alone when
  it records only a point, and nothing without an AF point or on manual-focus
  shots (see [What the camera records](./docs/humans/cameras.md)). The mark's
  color judges the face nearest the AF point, or, without an AF point or on
  manual focus, the largest face Handpic finds anywhere on the preview, so
  such a frame can be bright green or gray although it shows no crosshair. The mark is
  bright green for a good photo, one likely not a miss: the AF eyes in focus
  (an in-focus probability of 90% or more), the eyes open (an openness of 41
  or more) and the face toward the camera (turned no more than 35° and
  tilted no more than 45°), with a face mesh that sits on the face and a
  face the frame's edge does not cut. The eyes are also checked on the file's
  full-size embedded JPEG: a frame whose eyes are soft there is not good and
  turns white (it is not rejected), and a file with no JPEG larger than the
  preview (RAF, ORF, an HDR PQ CR3) skips the check. The threshold was checked
  on three folders from one body (ILCE-7M5) and on a separate hand-labeled
  set, where the author judged 5 of the 145 good frames sharp under it (a few
  sharp frames can leave good), and it misses soft frames at high ISO. It is selective by design, on about
  20% of the author's frames with a face at the AF point before the full-size
  check, and bright green means good only. The mark is gray when a face was judged and its eyes are
  likely not in focus (a probability combining their sharpness and edge
  width, under about 77%); and white for everything else (a judged face at
  about 77% or more that misses a good cut, no face found near the AF point,
  or anywhere without one, or not computed yet). A frame with no face stays white, since on the author's folders the
  sharpness score barely told picks from the rest on such frames.
  The camera's face tracking no longer colors the mark. The cue is computed
  in a second pass right after the thumbnails, so the marks turn from white
  as it runs, the files on screen first (both passes run below normal
  priority, so the viewer comes first when they compete for the CPU); the strip marks each bright green or gray file
  with an icon at the cell's bottom-left in the mark's color (a smiling face
  on a good photo, a frowning face on gray, none on white), the meta pane
  shows the probability as `AF eye in focus` (a percentage), whether that
  face's eyes are open and how open (0 to 100) as `Eyes` and the face's
  yaw, pitch and roll as `Yaw`, `Pitch` and `Roll`, both stored by the same second pass
  for the judged face and judged when the file is shown until then, and the filter menu's `Face focus` section (`Good` / `Neutral` /
  `Low`, with a `?` that shows their definitions) narrows the strip by the state, its `Eyes` section (`Open` / `Closed` / `Unknown`) by the stored eye
  state (a file without one, such as a shot with no face found, is
  `Unknown`). The mark also draws the faces
  Handpic detects near the AF point as a cyan box with a cyan dot on each eye,
  the nose tip and the mouth corners and a label beside it (the detection
  score, the box size and the distance between the eyes, in preview pixels),
  a moment after `f`, since the detection runs when the frame is shown, and
  the face the `Eyes` row judged also gets the outline of its face parts (face
  oval, eyes, brows, nose, lips) and, when the eyes are judged open, the
  irises, in magenta, a moment after that row.
- **Offline face detection**: faces and eyes are found by the bundled
  [YuNet](https://github.com/opencv/opencv_zoo/tree/main/models/face_detection_yunet)
  model (MIT license); whether the eyes of the shown file's subject are
  closed is judged by the bundled
  [MediaPipe Face Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker)
  face mesh (Apache-2.0, converted to ONNX from Google's TFLite). Both run
  locally with no network access. The strip's
  focus mark icons, also on the filter menu's `Good` and `Low` items, are
  [Lucide](https://lucide.dev)'s `scan-face` and the same with its mouth
  turned down, and its
  recommended icon is Lucide's `sparkles` (ISC license).
- **HDR PQ (HEIF) CR3 previews**: the HEVC previews of Canon CR3 files shot
  with HDR PQ on are decoded by the bundled
  [hpvcd](https://github.com/awxkee/hpvcd) (BSD-3-Clause OR Apache-2.0).
- **Bursts**: frames shot within 1 s of each other share a band and a
  count badge on the strip; `ArrowUp` / `ArrowDown` jump between bursts,
  `Alt+ArrowLeft` / `Alt+ArrowRight` step through the frames of one and stop at
  its ends, and `Shift+x` rejects the rest of one. Cameras that record no
  sub-second capture time are grouped by whole seconds (see
  [What the camera records](./docs/humans/cameras.md)).
- **Compare**: `v` shows 2–4 selected shots together, or the current shot
  beside its burst's first good frame (else the burst's first frame). Click a frame to rate, pick or
  reject only that one.
- **Sequence JPEG Timestamps**: after culling in Handpic and exporting the keepers
  as JPEGs from your RAW developer, right-click the export folder in the
  folder tree and choose `Sequence JPEG Timestamps…` to space the capture
  times of a burst one second apart, so Google
  Photos, which ignores sub-second times, keeps the frames in shooting order.
  Files are ordered by capture time, then sub-second time, then file name, so a
  folder exported from two bodies interleaves correctly. A preview shows the
  new times first. The result is a complete copy in `<folder>-sequenced/` next
  to the export folder, which is shown in your file manager when the run ends;
  the originals are never touched, and running it again rebuilds the copy from
  the original times.
- **JPEG-only folders**: a folder that holds only JPEGs, such as a
  `<folder>-sequenced/` output, opens view-only, so its order and capture
  times can be checked in Handpic: thumbnails, the preview and the EXIF rows,
  in capture-time order. Stars, flags, color labels, sidecars and the focus
  cue do not apply there.
- **Genres (AI analysis)**: with `AI analysis of what each photo shows` on
  in the settings (off by default; the first enable downloads a 378 MB model
  from Hugging Face), a background pass embeds each RAW file's preview
  locally with SigLIP 2, and the filter menu's `Genre` section narrows the
  strip to sports (basketball), portrait (group photo), landscape,
  birds / animals (pets), snap (camping) or `Unknown`. It is a rough sort,
  calibrated on the author's photos: reliable for sports, right about half
  the time for portrait, landscape and snap, and it never changes a judgment
  (see [AI analysis and genres](./docs/humans/usage.md#features)).
- **Recommended order (AI analysis)**: with AI analysis on, the sort menu's
  `Recommended` keeps the bursts in capture order and puts each burst's
  frames in the order of a small model learned from picks, and the meta
  pane shows the frame's place in its burst, the reasons and a confidence,
  and the strip marks each burst's first-ranked frame with an icon.
  It starts from a model learned on the author's culling and learns yours
  from the frames you viewed (on screen for about half a second) and did not
  pick, and from your rejects, with a correction per genre once you have
  judged enough of it. It is a
  modest suggestion (held out on the author's folders, the top-ranked frame
  was a pick in about 55% of the bursts, against 39% for the first
  frame), and it never changes a judgment (see
  [Recommended order](./docs/humans/usage.md#features)).
- **MCP companion**: an MCP client, such as an AI assistant, can follow along
  while you cull: read what Handpic shows, look at a preview, move the view and
  record stars, picks / rejects and labels the way the keys do. It is off by
  default, listens on this computer only and cannot move anything to the
  Trash (see [MCP companion](#mcp-companion)).

Every feature, and the full key reference, is described in
[docs/humans/usage.md](./docs/humans/usage.md) ([Keys](./docs/humans/usage.md#keys)).

## Working with other software

Judgments are saved in a sidecar file next to the RAW. The format is chosen on
the first launch and can be changed in the settings:

- **Lightroom (XMP)**: `FOO.ARW` gets `FOO.xmp`, the format Lightroom
  and others read. Picks and rejects are written as `xmpDM:good` and color
  labels as both `photoshop:LabelColor` and `xmp:Label`, the way Lightroom
  writes them. The name written to `xmp:Label` can be set per color in the
  settings.
- **PhotoLab (.dop)**: `FOO.ARW` gets `FOO.ARW.dop`, which holds picks and rejects too.
- **Both**: every judgment goes to `FOO.xmp` and `FOO.ARW.dop` alike, for
  developing in Lightroom and PhotoLab from one culling pass. When a folder
  opens, whichever of the two was modified last is read back, so a later
  edit in either tool is picked up. Switching to Both rewrites nothing: a
  file's two sidecars can disagree until it is judged again.

A sidecar created by other software is edited in place: everything except the
stars, the flag and the label (develop settings, keywords, ...) is left
untouched.

Two RAWs of the same stem with different extensions in one folder (`FOO.ARW`
and `FOO.DNG`) share `FOO.xmp` and are not supported: renaming one of them
or moving it to the Trash takes the shared `FOO.xmp` with it, and the other's
judgment along. Deleting one permanently keeps the shared `FOO.xmp`. Use the PhotoLab (.dop) format alone from the start, which writes one
`FOO.ARW.dop` per RAW, with no `FOO.xmp` in the folder (**Both** writes the
shared `FOO.xmp` too, and Handpic moves an existing `FOO.xmp` along whichever
format is chosen), or keep such RAWs in separate folders.

### Lightroom

- Lightroom (not Classic) reads the XMP Handpic wrote when the photos are
  imported.
- A rating, flag or color label changed in Lightroom is written back to the
  `.xmp` next to the RAW by Lightroom itself; unlike Lightroom Classic, no
  `Ctrl+S` or auto-write setting is needed. Handpic picks the change up when
  the folder is opened.
- Verified with Lightroom 9.5.1 on Windows.

### Lightroom Classic

- Lightroom Classic reads the XMP Handpic wrote when the photos are first
  imported.
- After import it does not re-read a sidecar Handpic changed, not even on
  restart. For the selected photos, right-click them in the Library grid and
  choose `Metadata > Read Metadata from File`. For a whole folder, right-click
  the folder, choose `Synchronize Folder...`, check
  `Scan for metadata updates` and click `Synchronize`.
- Lightroom Classic does not write XMP by default. `Ctrl+S`
  (`Metadata > Save Metadata to File`) writes it for the selected photos, or
  turn on `Catalog Settings > Metadata > Automatically write changes into XMP`.
- Color labels are matched by name against Lightroom Classic's color label
  set (`Metadata > Color Label Set > Edit...`). Either rename that set's labels
  to `Red`, `Yellow`, `Green`, `Blue` and `Purple`, or set Handpic's label names
  in the settings to the names in the set. Presets for Lightroom's localized
  default sets are provided (every UI language Lightroom Classic ships);
  adding a language is [one JSON file](./crates/core/i18n/README.md).

### DxO PhotoLab

- PhotoLab recognizes an image by the identifiers in its `.dop`. Once
  PhotoLab has seen an image (opening its folder is enough, even if no `.dop`
  was written), a new `.dop` with identifiers it does not know is imported as
  a virtual copy that carries Handpic's judgment, while the master keeps its
  own.
- So when Handpic creates a `.dop`, it reads the image's identifiers from
  PhotoLab's database (on Windows the newest
  `%APPDATA%\DxO\DxO PhotoLab N\Database\PhotoLab.db`, on macOS the newest
  `~/Library/DxO PhotoLab vN/DOPDatabaseVN.dopdata`) and writes them into it,
  and PhotoLab applies the judgment to the master. On macOS, folders under
  `/Volumes` are not looked up yet and so still get random identifiers.
- When the database is not found (without PhotoLab installed) the new `.dop` still works: PhotoLab imports it as the master, unless it
  had already seen the image, in which case the judgment lands on a virtual
  copy.
- A `.dop` that already exists is edited in place as before.
- Verified with PhotoLab 10.0.1 on Windows.

### MCP companion

Handpic can serve an [MCP](https://modelcontextprotocol.io/) endpoint for an
MCP client (an AI assistant, say) to sit beside you while you cull. Turn on
`Let MCP clients connect` in the `MCP` tab of the settings (it is off by
default); the server then listens on this computer only, at:

```text
http://127.0.0.1:41917/mcp
```

That URL is all a client that speaks MCP over Streamable HTTP needs. Two
examples:

- **Claude Code**:
  `claude mcp add --transport http handpic http://127.0.0.1:41917/mcp`
- **Claude Desktop**: add this to `claude_desktop_config.json`. It relays
  through `npx -y mcp-remote`, so it needs Node.js; a direct `url` entry has
  not been verified.

  ```json
  {
    "mcpServers": {
      "handpic": {
        "command": "npx",
        "args": ["-y", "mcp-remote", "http://127.0.0.1:41917/mcp"]
      }
    }
  }
  ```

The `MCP` tab shows the URL and these examples with Copy buttons. What the
tools do is described in [docs/humans/usage.md](./docs/humans/usage.md#mcp-companion).

## Compatibility

Anything unchecked has not been verified yet. Reports of how it went for you
are very welcome.

### OS

- macOS (Apple Silicon)
  - [x] 26
- Windows
  - [x] 11
- Linux
  - [x] Ubuntu

If it works, post in the
[OS works report thread](https://github.com/minodisk/handpic/discussions/286) in
Discussions. If it does not, open an issue from the
[OS issue template](https://github.com/minodisk/handpic/issues/new?template=os.yml).

### RAW formats

- ARW
- CR3
- DNG
- NEF
- ORF
- RAF

A folder that holds only JPEGs (`.jpg` / `.jpeg`) and no RAW file opens
view-only: thumbnails, the preview and the EXIF rows, in capture-time order,
with no stars, flags, color labels, sidecars or focus cue.

The cameras verified on a real file, what each records, and which features
that affects are listed in [docs/humans/cameras.md](./docs/humans/cameras.md). Why support is
listed per camera rather than per format is explained in
[docs/humans/raw-formats.md](./docs/humans/raw-formats.md).

If a camera not listed in [docs/humans/cameras.md](./docs/humans/cameras.md) works, post in
the
[camera works report thread](https://github.com/minodisk/handpic/discussions/287)
in Discussions. If it does not, open an issue from the
[camera issue template](https://github.com/minodisk/handpic/issues/new?template=camera.yml).
A sample file is needed to look into it, so please attach one (or link to it).
One file is enough; a landscape and a portrait shot, if possible, also let the
rotation and the focus mark be checked.

### Sidecar formats and software

- XMP
  - [x] Adobe Lightroom
  - [x] Adobe Lightroom Classic
- DOP
  - [x] DxO PhotoLab

If the stars, flags and color labels given in Handpic show up correctly in the
software, post in the
[software works report thread](https://github.com/minodisk/handpic/discussions/288)
in Discussions. If they do not, open an issue from the
[software issue template](https://github.com/minodisk/handpic/issues/new?template=software.yml).

## For developers

- Building from source: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Performance measurements: [docs/humans/performance.md](./docs/humans/performance.md)
