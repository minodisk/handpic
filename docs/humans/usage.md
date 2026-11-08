<p align="center">English | <a href="./usage.ja.md">日本語</a></p>

# Using Riffle

The detailed behavior of Riffle. For a quick start, see [README.md](../../README.md).

## Features

Open a folder from the picker, or drop a folder or any file in it onto the
window. Riffle lists the RAW files (ARW, CR3, DNG, NEF, ORF and RAF) in it and
pages through their embedded previews, rotated by each file's Orientation. With no folder open the
viewer shows a prompt in its center; click it to open the folder picker.

- **Folders**: the left pane is a folder tree rooted at the home folder and
  the mounted volumes (`/Volumes/*` on macOS, the drive letters on Windows,
  `/mnt/*`, `/media/*/*` and `/run/media/*/*` on Linux; the list of these
  root volumes is read again each time the window gains focus, so one
  mounted after launch appears once you switch back to Riffle). A folder's
  arrow lists its subfolders (but for one already at the top level, so home
  is not repeated under the volume it lives on), and an expanded folder
  follows the disk: a subfolder created, deleted or renamed under it, by
  the file manager, another app or Riffle itself, appears or goes within
  about a second (a folder the app cannot watch, a network share say, is
  re-listed only when it is expanded again). On
  Windows, while a folder is expanded, the file manager cannot rename the
  folders that hold it (the folder itself and those under it stay free);
  collapse it first. An expanded
  folder shows how many RAW files it holds itself, or, for a folder with no
  RAW file, how many JPEGs (see **JPEG-only folders**). Clicking a folder's name
  opens it. `File > Open Folder…`, the `open` key (or the empty-state hint)
  and a drop still open anything the tree does not reach. The open folder
  is highlighted, and the tree expands down to it whenever a folder opens,
  however it was opened; one on a volume the tree does not list (a network
  share, say) adds that volume to the top level for the session.
  A click in the tree also gives it the keyboard, with a cursor on the open
  folder: `ArrowUp` / `ArrowDown` / `Home` / `End` move the cursor without
  opening anything, `ArrowRight` expands a folder or steps into its first
  subfolder, `ArrowLeft` collapses it or steps up to its parent, `Enter`
  opens the cursor's folder as a click does, and typing the start of a name
  jumps to the folder it matches (see the fixed keys under [Keys](#keys)).
  While the tree has the keyboard the culling keys are off: only
  `Open Folder`, the pane toggles (`Alt+Cmd+Arrow` / `Ctrl+Alt+Arrow` and
  `Tab` by default)
  and other native menu accelerators (`CmdOrCtrl+R` Reload Folder,
  `CmdOrCtrl+,` Settings) still work — `Edit > Undo` / `Redo` do not, since
  their keys are mirrored from the same rebindable `undo` / `redo` actions
  the tree gate turns off — until `Escape`, a click elsewhere or hiding the
  left pane hands the keys back.
  Right-clicking a folder opens a menu whose first item shows the folder
  selected in its parent in the OS file manager: `Reveal in Finder` on
  macOS, `Reveal in File Explorer` on Windows, `Open Containing Folder` on
  Linux. Below it, `Copy Path` puts the folder's absolute path, unquoted,
  on the clipboard and `Copy Folder Name` its name as the tree shows it;
  a refused clipboard write shows its error in the status line. Then
  `Refresh` lists the folder's subfolders and RAW count again, expanded or
  not, keeping the folders open under it, and retries the tree's folder
  watches; it is for a folder whose watch could not be set (a network share,
  a refused permission), which otherwise does not follow the disk, and a
  folder that can no longer be listed shows its error. On the open folder
  itself (not a folder above or below it), `Refresh` also reloads the
  folder the way `File > Reload Folder` does. Then
  `Rename…` (not offered on home or a volume at the top level) turns the
  folder's name into a text box, its name selected; a slow second click on
  the open folder's name (a click, then another about half a second later,
  not a double-click) does the same, and that click does not reopen the
  folder. `Enter` renames, `Escape` cancels, and a click anywhere else
  renames too; an empty or unchanged name just ends the edit, and while
  the edit is live every key goes to the text box. A rename confirmed
  while a scan runs waits for the scan, the status line saying so, and
  happens when it ends. Meanwhile the row shows the new name, muted and
  italic, after a clock icon ("Renames when the scan finishes"), and still
  opens the folder at its old path. Renaming it again edits the waiting
  name: a different name replaces it, the original name cancels the
  rename, and `Escape` or the unchanged name keeps it. Opening another
  folder, or another operation pressed during the scan, drops the waiting
  rename, the name going back and the status line saying
  `Rename… was canceled`, as it does when you cancel it yourself. A name
  that is invalid, already taken or refused by the OS shows its error in
  the status line with the folder left as it was. The folder index, ratings, flags, labels and the remembered file
  follow the folder, so nothing is re-extracted; the tree keeps its
  expansion; and when the open folder is the renamed one or under it, it
  reopens under its new path at the file it was on (a reopen, so the undo
  history and the selection start over). Then `Rename by Capture Date…`
  (not offered on home or a volume at the top level either) renames the
  folder after its photos' capture date, or splits it into one sibling
  folder per date when they span several (see **Rename by Capture
  Date…**). Then `Expand All` expands the
  folder and every folder below it, listing each in turn (a folder that
  fails to list shows its error and the rest still expand; a symlinked or
  junctioned folder is not descended into, since it can loop back on an
  ancestor), and
  `Collapse All` collapses every folder below it, leaving the folder itself
  open or closed as it was; neither is offered on home or a volume at the
  top level. Then `Rewrite Sidecars from Index…` writes
  the judgments Riffle holds for that folder back into its sidecars, and
  `Delete Sidecars…` moves its sidecars to the Trash (see
  **Rewrite Sidecars from Index… and Delete Sidecars…**). Then
  `Sequence JPEG Timestamps…` previews and sequences the capture times of
  the JPEGs in that folder (see
  **Sequence JPEG Timestamps…**). Last, `Import…`, offered on every folder,
  copies the photos under it into date folders (see **Import…**). The right-click neither opens the folder nor gives the tree the
  keyboard (or takes it away); `Escape` or a click elsewhere closes the menu.
  Several folders can be selected together: `Cmd+click` on macOS
  (`Ctrl+click` elsewhere) adds a folder to the selection or removes it, and
  `Shift+click` selects the visible folders between the last one clicked and
  this one; neither opens a folder. A plain click, or opening a folder any
  other way, selects that folder alone, and collapsing a folder drops the
  folders under it from the selection. A right-click selects the folder it
  is on alone first, even within a larger selection, and its menu acts on
  that folder.
- **Filmstrip**: thumbnails run along the bottom, under the viewer and the
  folder tree, follow paging and show the file you click. The mouse wheel
  scrolls it sideways. Its header bar holds the `N / M` counter and the
  filter and sort menus. While a folder is scanned (`scanning N / M`, then
  `analyzing N / M`), the scan takes the shown file and the strip's visible
  cells first, following paging and scrolling, so their thumbnails, then
  their focus marks, arrive before the rest. Both passes
  run below normal OS priority, the second lower still, so the viewer and the
  rest of the machine come first when they compete for the CPU.
  Several files can be selected: `Cmd+click` (`Ctrl+click` on Windows and
  Linux) adds or removes one file without changing the file shown (the shown
  file itself always stays selected), `Shift+click` selects every file from the
  last clicked one (the anchor) to the clicked one and shows the clicked file,
  and `Shift+ArrowLeft` / `Shift+ArrowRight` grow or shrink that range one file at
  a time. `CmdOrCtrl+A`, `Edit > Select All` or the right-click menu's
  `Select All` selects every file the strip shows (the filter's result); the
  shown file stays shown and becomes the anchor. A plain click, or any key that moves to another file (arrows, burst
  jumps), collapses the selection to the new file; a plain arrow at either end
  of the strip, which moves nowhere, keeps it. A right-click on a cell outside
  the selection collapses it to that cell; one inside keeps it, so the context
  menu items act on the whole selection. Files the filter hides leave the
  selection.
  Below `Select All` in the right-click menu come the reveal item, which
  shows the right-clicked file selected in the OS file manager
  (`Reveal in Finder` on macOS, `Reveal in File Explorer` on Windows,
  `Open Containing Folder` on Linux), then `Copy Path` and
  `Copy File Name` (the name with its extension), which copy the
  right-clicked file's full path or name, or, on a selection, every selected
  file's, one per line. All three are offered in a view-only JPEG folder too.
  The group before the right-click menu's last holds `Rename…` (not offered in a
  view-only JPEG folder), which acts on the shown file alone, whatever else is
  selected: its name turns into a text box with the part before the
  extension selected. A slow second click on the shown file's name (a
  click, then another about half a second later, not a double-click) does
  the same. `Enter` renames, `Escape` cancels, and a click anywhere else
  renames too; an empty or unchanged name just ends the edit, and while
  the edit is live every key goes to the text box. The new name must keep
  a RAW extension. The file's `.xmp` and `.dop` sidecars are renamed with
  it, and its ratings, flags, labels and cached thumbnail and metadata
  follow it, so the strip keeps its place. A rename confirmed while a scan
  runs waits for the scan, the status line saying so, and happens when it
  ends. Meanwhile the cell shows the new name, muted and italic (on a
  labeled cell, italic in the label's text color), after a clock icon
  ("Renames when the scan finishes"), and the file is still judged at its
  old name. Renaming it again edits the waiting name: a different name
  replaces it, the original name cancels the rename, and `Escape` or the
  unchanged name keeps it. Opening another folder, or another operation
  pressed during the scan, drops the waiting rename, the name going back
  and the status line saying `Rename… was canceled`, as it does when you
  cancel it yourself. A name that is invalid, already taken or refused by
  the OS shows its error in the status line with the file left as it
  was. The menu ends with `Move to Trash…` (`Move N Files to Trash…` when
  the right-clicked file is in a selection of N), offered in a view-only
  JPEG folder too, which moves the right-clicked file, or the whole
  selection, to the Trash (see **Move to Trash…**), and `Delete Permanently…`
  (`Delete N Files Permanently…`), offered there too, which deletes them
  instead (see **Delete Permanently…**).
- **Panels**: `Alt+Cmd+ArrowLeft` on macOS (`Ctrl+Alt+ArrowLeft` on
  Windows and Linux) hides and shows the left pane (the folder tree),
  `…ArrowRight` the right pane (the metadata), `…ArrowDown` the filmstrip,
  and `Tab` both side panes at once (hiding both when either is shown, as
  Lightroom does), so the viewer can take the whole height a landscape frame
  needs. The filmstrip's header bar goes with it, so the filter and sort
  menus and the `N / M` counter are hidden while the strip is. Which panels
  are hidden is remembered across restarts. The `View` menu's `Folders`,
  `Metadata` and `Filmstrip` do the same, and their check marks show which
  panes are shown. Earlier versions used `F7`, `F8` and `F6`; bind them
  back per action in Settings if you prefer them (a binding you already
  saved keeps working).
- **Focus mark**: `f` draws a crosshair at the camera's recorded focus point,
  inside a rectangle of the AF frame the camera used on Sony bodies that
  record it (hidden by default). A body that records only the point, such as
  the SIGMA BF, shows the crosshair alone; cameras that record none, such as
  the M11-P, and manual-focus shots show no mark (see
  [What the camera records](./cameras.md)). The mark's color says how the
  judged face came out: the face nearest the AF point, or, on a frame with no
  AF point or in manual focus, the largest face Riffle finds anywhere on the
  preview, so such a frame can be bright green or gray although it shows no crosshair
  (the strip's icon, the meta pane and the filter carry its state; a group
  photo without an AF point is judged on that one face). Bright green marks a good photo, a
  frame that is likely not a miss: the AF eyes in focus, the eyes open and
  the face toward the camera, all at once. The cuts: the
  AF eyes' in-focus probability is 90% or more; the eyes' openness (the meta
  pane's `Eyes` row) is 41 or more; the head is turned no more than 35° left
  or right; and it is tilted no more than 45° up or down (the roll is not
  looked at). A frame whose face mesh sits off the face (as on a face
  rotated far in-plane, such as a baby lying down) or whose face the frame's
  edge cuts is never good. The eyes of the judged face are also checked on the
  file's full-size embedded JPEG, which shows softness the smaller preview
  hides (the meta pane's `Eyes at full size` row): a frame whose eyes are soft
  there is not good and turns white (it is not rejected). A file with no JPEG
  larger than the preview (RAF, ORF, an HDR PQ CR3) skips the check. The
  threshold was checked on three folders from one body (ILCE-7M5) and on a
  separate hand-labeled set, where the author judged 5 of the 145 good frames
  sharp under it (a few sharp frames can leave good), and at high ISO it misses soft frames. It is selective by
  design, so most frames get no bright green: before the full-size check, on
  twelve of the author's folders it marked about 20% of the
  frames with a face at the AF point. On 180 frames the author rated (1 star
  for no subject, 2 for likely rejected, 3 or more for a pass), 54 of the 58
  good frames passed and none was a frame without a subject. Bright green
  means good only. Gray means a face was judged and its eyes are likely not
  in focus: their in-focus probability (a logistic combination of the
  Laplacian variance and the mean edge width) is under about 77%, the focus
  candidate threshold, including a window between the eyes with no clear
  edge, which counts as 0%. The
  probability is measured over each eye's eyelid region, which a face mesh
  model (MediaPipe Face Landmarker v2) finds, and the sharper of the two
  eyes counts; when neither eye's region counts (under 24 px, as on small
  faces, or without a clear edge), it is measured over the preview between
  the eyes instead. White is everything else: a judged face at about 77% or
  more that misses a good cut (under 90%, or the eyes, the pose, the mesh,
  the frame's edge or soft eyes at full size), no face found (near the AF point when there is one,
  anywhere on the preview when there is none), or not computed yet. A frame with no face found stays white rather
  than being judged on the sharpness score: on the author's 4361 face-free
  frames the score barely told picks from the rest (AUC 0.58). A frame with no face found or with a face
  under about 60 pixels on the embedded preview is never good,
  and neither is a strongly turned face, even when the turn was intended.
  The camera's face tracking no longer colors the mark: a Sony eye-AF frame
  is judged by the faces Riffle detects like any other. The state is computed
  in a second pass that starts right after the thumbnails and metadata of the
  folder are in, so the marks turn from white to their color, the files on
  screen first, while the status shows `analyzing N / M`. The strip marks
  each bright green or gray frame with an icon in the mark's color at the cell's
  bottom-left, above the file name, filling in as the pass runs: a smiling
  face (Lucide's `scan-face`) on a good frame and the same face with its mouth
  turned down on a gray one, both ISC license,
  text in `crates/app/ui/LICENSE-lucide`; white frames get none. On the 406 hand-labeled α7 V frames with a face the focus
  candidate cue was fitted on, 94% of the candidates were in focus and 91%
  of the in-focus frames were candidates; on 400 frames from other shoots it
  was not fitted on, 89% and 96%. It is a cue, not a verdict: AF on a person in the
  background gives a sharp face and a false candidate, and the back of a
  head or an upturned face finds no face and stays white. The mark also
  draws the faces Riffle detects near the AF point (anywhere on the preview
  when there is no AF point) as a cyan box with a cyan dot on each eye, the
  nose tip and the mouth corners, and a label beside the box with the
  detection score, the box size and the distance between the eyes, both in
  preview pixels (`0.93  212×254  eyes 61`). They
  appear a moment after `f`, because the detection runs when the frame is
  shown and is kept only for the session. The 1:1 view and Compare draw no
  faces. Whether the eyes of the judged face are open, and how open, shows in
  the meta pane's `Eyes` row (see **Meta pane**); that face also gets the
  outline of the face parts the judgment looked at (face oval, eyes, brows,
  nose, lips) and, when the eyes are judged open, the irises as rings with a
  dot at each center, drawn as thin magenta lines a moment after the `Eyes` row.
- **1:1 focus check**: `z` shows the full-resolution image at one pixel per
  screen pixel, centered on the focus point (or the frame center without one).
  Paging while zoomed stays zoomed and moves to the next file's focus point.
  There is no panning or free zoom.
- **Grayscale preview**: holding `g` shows the viewed image in grayscale to
  judge composition; releasing it restores color. It is momentary and
  display-only: nothing is written or remembered.
- **Focus peaking**: `e` lights the strong edges of the viewed image in red,
  in the fitted view and in the 1:1 focus check, so the in-focus plane stands
  out; pressing it again removes them. The edges are measured at the size the
  image is drawn, a moment after it appears. It is off at launch, not
  remembered, and Compare draws none. Holding `g` grays the red too.
- **Performance graph**: `d` shows a graph along the bottom of the viewer
  with the last 60 seconds of five series: a red bar per file switch, its
  height the time from the key or click to the preview being drawn (on a ms
  scale at the right of the legend, at least 500 ms tall), stacked from the
  bottom in shades of red by stage: `wait`, `read`, `ipc`, `post`, `worker`,
  `decode` and `draw` (a preview already read ahead has only `draw`: the
  files on both sides of the current one, the next in the paging direction
  first, are read and decoded ahead while no scan runs, so turning to one
  draws at once); Riffle's own CPU
  usage (the thick solid blue line) and the whole system's (the thin dashed
  blue line), both 0–100 % of all cores; Riffle's own reads (the thick solid
  green line, on an MB/s scale at the right of the legend, at least 10 MB/s
  tall); and how busy the busiest of the system's disks is (the thin dashed green
  line, 0–100 % of the time). When the bars grow, look at the lines: Riffle's CPU
  line high means Riffle itself is busy (a running scan, including its
  analysis pass, counts as Riffle's CPU); the system CPU line pegged while
  Riffle's stays low means another process is taking the CPU; both CPU lines
  low while the disk lines rise means the preview is waiting on the disk,
  often behind a scan reading RAW files. On Windows Riffle's read line counts
  every read Riffle makes, not only from disk, so it never sits quite at
  zero. The disk busy line is absent on macOS. Everything is sampled only
  while the graph is shown, so it starts empty each time; `d` again hides it,
  and the overlay is off at every launch.
- **Compare**: `v` lays 2–4 selected files out in the viewer. With only one
  file selected, it instead puts that file beside the first good frame (the
  focus mark's bright green) of its burst, or the burst's first frame when
  none is good, never the file itself. Each frame is labeled with its file
  name. Click a frame to make it
  `ACTIVE`; a star, pick, reject or label then applies only to that frame,
  regardless of the filmstrip selection. Press `v` again to return.
- **Judgments**: stars, a pick / reject flag and a color label, shown on
  the strip cell (the label tints the file-name band along the cell's bottom
  edge) and written to a sidecar; see
  [Ratings and sidecars](#ratings-and-sidecars).
  With several files selected, a judgment sets the same value, decided from
  the shown file, on every selected file; fields the judgment does not touch
  keep each file's own value (a star key leaves each label as it was, and a
  pick replaces a reject on every selected file but leaves each file's stars
  as they were). Hidden files
  are never judged.
- **Meta pane**: grouped by where each value comes from, in three groups.
  **EXIF** lists what the camera wrote in the standard tags (aperture,
  shutter, ISO, focal length, exposure, camera, lens and capture time).
  **Maker note** lists what it wrote in its vendor MakerNote: on Sony, the
  shutter type, the focus mode, the AF area (ILCE, NEX and ZV bodies only;
  not the FX cinema bodies, the compacts, or the A-mount bodies), the AF tracking, such as `Face
  tracking` or `Lock On AF`, the
  drive (the release mode, with the frame number within a burst, as in
  `Continuous, frame 2`), the stabilization, the exposure mode, the metering,
  the creative style (on current bodies the Creative Look, which may show as
  a two-letter code such as `ST`), the DRO and the RAW type; on Leica, the
  focus distance. The focus mode and AF tracking are left out on the older
  RX and HX compacts, where ExifTool reads them as not applying. The Sony
  MakerNote does not record the recognized subject type (human, animal, bird
  or vehicle) or whether the AF sat on an eye or a face, so Riffle does not
  show them, and enciphered values (shutter count, picture profile) are not
  read. When a lens reports no f-number (the M11-P with an M-mount
  lens), the aperture is the camera's estimate, marked `(est.)`. **Analysis**
  holds what Riffle computes itself: `AF eye in
  focus`, the in-focus probability (a percentage) of the eyes of the judged
  face (nearest the AF point; without an AF point or in manual focus, the
  largest face Riffle is confident of) that the focus candidate state is
  decided from (left out when there is none). It comes from the scan's second pass, so it
  appears a little after the file's thumbnail on a folder's first scan.
  `Eyes at full size` is the sharpness of those same eyes on the file's
  full-size embedded JPEG, as in `Sharp · 0.214` or `Soft · 0.143`; `Soft`
  is the verdict that takes a frame out of good. It is left out when nothing
  was measured (for example a file with no JPEG larger than the preview).
  `Eyes` says whether the eyes of the face nearest the AF point (without an
  AF point, the largest face Riffle is confident of) are open or closed and
  how open they are, from 0 to 100, as in `Open · 82` or `Closed · 0`. The
  openness is read from the eye aspect ratio (the eyelid gap over the eye's
  width) of the more closed eye: 0 at the ratio Riffle counts as closed and
  below, 100 at a wide-open eye (the 90th percentile of the author's faces)
  and above, linear between. It says how open the eyes are, not how sure the
  judgment is; an eye just past the closed point shows `Open · 0`. For that
  face it is stored by the scan's second pass, along with `Yaw`, `Pitch` and `Roll`; until
  the pass reaches the file, it is judged when the file is shown. Either way it
  comes from the eyelid
  points of the bundled
  [MediaPipe Face Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker)
  face mesh (Apache-2.0, text and provenance in
  `crates/core/models/LICENSE-mediapipe`, run locally with no network
  access), so a judged one appears a moment after the preview. It is left out when
  there is no face, when the face is under about 60 pixels on the embedded
  preview (too small to judge), and for JPEG files. A downcast eye shows no
  iris either, so it counts as closed like a blink. `Yaw`, `Pitch` and `Roll`
  are the head pose of that same face in whole degrees: yaw is positive when the
  face turns toward the image's right, pitch when it tilts up, and roll when
  the head tilts clockwise on screen. They come from the same face mesh, fitted
  onto a canonical face the way MediaPipe's face geometry does it (Apache-2.0,
  provenance in `crates/core/models/LICENSE-mediapipe`), and are left out
  whenever `Eyes` is, and also when the fit fails. Read them as a rough
  direction: on hand-labeled local faces the sign was right on about 94% of
  them for yaw and pitch, but a 15-25° turn still looks frontal to the eye,
  and far profiles (past about 70°, a yaw can read past 90°) and sports
  sunglasses are rough. While AI analysis is on, `Recommended`, `Reason`
  and `Confidence` follow for a file with an embedding (see
  **Recommended order**).
- **Filter menu**: narrows the strip by pick flag, stars, color label,
  orientation (`Portrait` / `Landscape`), the focus mark's state (the
  `Face focus` section, each item with the strip's icon of its state: `Good`
  for a good photo, the bright green mark and face icon, an in-focus
  probability of 90% or more among its cuts; `Neutral` for white, everything
  else: a judged face at about 77% or more that misses a good cut, no face
  found, a JPEG, and files the second pass has not reached; and `Low` for
  gray, a judged face with an in-focus probability under about 77%, a frame
  with no AF point included; each file falls under exactly one of them,
  checking several shows the files in any of them, the strip refills as
  the pass runs, and the `?` at the right of the heading shows each item's
  definition with its thresholds until the menu closes), the stored eye state of the judged face (the
  `Eyes` section: `Open`, `Closed`, or `Unknown` for a file with no stored
  state, such as one with no face found, with a face under about 60 pixels,
  or not reached by the second pass yet), whether the file was exported
  (the `Output` section, described below), the genre (the `Genre` section,
  shown while AI analysis is on; see **AI analysis and genres**),
  camera, lens, aperture, shutter speed, ISO and focal length (grouped into
  ranges such as `24–35 mm`). The color
  label group lists the seven colors and `No label`; a label outside those
  seven colors matches no color item (nor `No label`). The orientation is
  decided by the file's EXIF Orientation — a quarter turn is portrait, so a
  frame the camera did not tag as rotated counts as landscape.
  The `Output` section has `Output` and `No output`, so that the RAW files
  you never exported can be narrowed down and moved to the Trash with
  `Select All` and `Move to Trash…`. It shows while the open folder has an
  `Output` or `Output-sequenced` subfolder directly inside it, in any case:
  the folders PhotoLab exports to by default and **Sequence JPEG
  Timestamps…** writes. A file counts as exported when the stem (the name
  without its extension, compared case-insensitively) of a file directly in
  either folder equals the file's own stem, adds `_<n>` or `-<n>` to it (a
  PhotoLab virtual copy, such as `DSC0001_1.jpg` for `DSC0001.ARW`), or
  starts with its stem plus `-DxO` (a DeepPRIME DNG: `L1002140-DxO_DeepPRIME
  3.jpg` counts for that DNG and for the original `L1002140.DNG`). An
  empty export folder still shows the section, with every file as
  `No output`. With no export folder the section is hidden, and with one
  that cannot be read it is hidden too and the status line says why; the
  section's checks clear then, so a file is never shown as `No output`
  without its folder having been read. The folder watcher does not watch the
  export folders: an export made while the folder is open shows up when the
  window regains focus, on `File > Reload Folder` or when the folder is
  reopened. The re-read waits for a running scan to finish, and regaining
  focus within 5 seconds of the last rescan does not trigger one.
  The EXIF groups list only values present in the folder. Checks within a group
  are OR-ed, groups are AND-ed, and `Reset` clears them all. A judgment that drops
  the current file out of the filter hides it at once and moves to the next
  passing file after it, else the last one before it, else the empty view.
  The flag, star, label, orientation, `Face focus` and `Eyes` checks are
  remembered across restarts and apply to every folder; the EXIF checks are
  not, and clear whenever a folder opens. The `Genre` checks last until the
  app quits or AI analysis is turned off, and the `Output` checks until the
  app quits or the section is hidden. `Reset` clears the remembered filter
  too.
- **Sort menu**: orders the strip by file name, capture time or rating, or,
  while AI analysis is on, `Recommended` (see **Recommended order**);
  paging and `n / N` follow the chosen order. Capture time breaks ties by
  sub-second then file name, and files without a capture time come last.
  Rating puts higher stars first, then unrated files, then rejects, with file
  name breaking ties. With AI analysis off, `Recommended` reads as capture
  time.
  The order settles while a folder is scanned for the first time, and the
  choice is remembered across launches.
- **Open Folder…**: `File > Open Folder…` opens the folder picker, the same as
  the `open` key. It is rebindable: the menu shows the first key of `open`
  that can be an accelerator, and none when the action holds no such key.
- **Reload Folder**: the open folder keeps up with the disk on its own: a file
  copied in or deleted outside Riffle shows up in the strip about a second
  later. On volumes that send no change notifications, such as network shares,
  bring the window back to the front or choose `File > Reload Folder`
  (`CmdOrCtrl+R`) to do the same. Only what changed is read again; the stars,
  flags and color labels already given, the current file and the strip's
  position all stay as they were.
- **Move to Trash…**: right-click a file in the filmstrip and choose
  `Move to Trash…` to move it to the OS Trash, or, when it is part of a
  multi-selection, `Move N Files to Trash…` to move every selected file.
  Since `Select All` selects exactly what the filter shows, filtering to the
  rejects, pressing `CmdOrCtrl+A` and choosing it moves the folder's
  rejects. Each RAW goes with the sidecars sitting next to it — both `.xmp`
  and `.ARW.dop` when both are there, no matter which format is currently
  selected; in a view-only JPEG folder each JPEG goes alone. It asks first
  with the count alone ("Move 3 files to the Trash?"): `Move to Trash` moves
  them, `Cancel` or `Escape` leaves everything untouched. While they go,
  the status line counts them (`moving to Trash 12 / 300`; on Windows it
  moves up to 100 at a time, so the count goes in steps), the line stays
  if a folder is opened meanwhile, and the menu item is greyed out until the
  move ends. Once they went, the strip shows the neighbour of the shown file, the gone files leave the
  selection, and the status line says how many went ("Moved 3 files to the
  Trash", with how many failed); whatever could not be moved is listed as
  an error and stays in the folder. Nothing is deleted: the file and its
  sidecars all go to the Trash, so restoring them brings back the stars,
  the flag and the color label. Confirmed while a scan runs, it stops the
  scan, the status line saying it waits for it, moves the files as soon as
  the scan has stopped, and then scans the files left; opening another
  folder meanwhile drops it. One `Edit > Undo`
  (`CmdOrCtrl+Z`) takes the whole move back, whichever folder is open (none
  at all included): every file and sidecar it moved goes back where it came
  from, the status line says how many came back ("Restored 3 files from the
  Trash", with how many failed), and when the open folder is the one it
  moved from, the strip shows the files again with their judgments.
  Nothing is overwritten: a file with the same name back at the original
  location, a Trash emptied since, or a file already put back by hand is
  listed as an error and left as it is, and the sidecars of a file that
  could not come back stay in the Trash too. The move stays undoable after
  another folder opens; like the move, the undo waits for a running scan.
  `Edit > Redo` (`CmdOrCtrl+Shift+Z`) then moves the files that came back,
  with their sidecars, to the Trash again without the dialog (a file that
  could not come back is not touched, and one gone from its place since is
  listed as an error), and that move is undoable in turn.
- **Delete Permanently…**: the item below `Move to Trash…` in the strip's
  right-click menu (`Delete N Files Permanently…` for a selection of N) deletes
  the same files, each RAW with the sidecars next to it and a JPEG alone,
  without going through the Trash, so the disk space comes back at once. It
  cannot be undone: nothing goes to the Trash and `Edit > Undo` has no step for
  it. The dialog says so and opens on `Cancel`, and neither `Enter` nor `Space`
  confirms, so only a mouse click on `Delete Permanently` goes ahead; `Cancel`
  or `Escape` leaves everything untouched. The status line counts the files
  (`deleting 12 / 300`) and then says how many went ("Deleted 3 files", with
  how many failed); a file that could not be deleted is listed as an error and
  stays in the folder with its sidecars. Like a move, it stops a running scan,
  runs as soon as the scan has stopped and then scans the files left; opening
  another folder meanwhile drops it. The two are refused while either runs.
- **Import…**: right-click a folder in the folder tree and choose `Import…`
  to copy the photos under it, at any depth, into one folder per capture date
  under a destination root, skipping what is already there. It is offered on
  every folder, a volume at the top level included, since that is what a
  memory card is: `E:\DCIM\10061024\` lands in `D:\Photos\2026\2026-10-24\`
  with no file manager copy and no rename by hand. The date is the
  `DateTimeOriginal` the camera wrote, its local date with no time-zone
  handling, so a shoot across midnight goes to two folders. The source is only
  read: nothing on the card is changed or deleted.
  - **Settings**: the `Import` tab of `Riffle > Settings...`
    (`File > Settings...` on Windows and Linux) holds two values, both
    remembered across launches. `Destination root` is the folder imports go
    under, set with `Choose…`; until it is set, the dialog says so and `Run`
    is disabled. `Folder pattern` is the path under the root, in chrono's
    strftime syntax, `%Y/%F` by default: `%Y` the year (`2026`), `%y` its two
    digits (`26`), `%m` the month (`10`), `%d` the day (`24`), `%b` / `%B` the
    month's short / full name (`Oct` / `October`), `%a` / `%A` the weekday's
    short / full name (`Sat` / `Saturday`), `%j` the day of the year (`297`),
    `%F` the same as `%Y-%m-%d` (`2026-10-24`), and `/` separates folders.
    Other text is kept as it is. The tab lists these and offers `%Y/%F`, `%F`,
    `%Y/%m/%d` and `%Y/%m` as examples to click; when a pattern puts several
    dates in one folder, as `%Y/%m` does, they share it.
  - **Valid patterns**: the pattern is date-only. A time, zone or timestamp
    specifier (`%H`, `%M`, `%S`, `%z`, `%s` and the like) is refused, and so
    is an unknown specifier, a lone `%` and any folder name Windows or macOS
    could not hold: empty, `.`, `..`, with one of `< > : " | ? * \` or a
    control character, or ending in a space or a dot. The line under the input
    shows where today's photos would go, or the reason in red, in which case
    the stored pattern stays. Reserved names such as `CON` are not checked;
    one fails when the folder is created, with the OS's error.
  - **What is copied**: the RAW files Riffle supports and the JPEGs in the
    folder and every folder below it, hidden folders left out, and each RAW's
    existing `.xmp` and `.dop` sidecars. Videos and everything else stay
    behind. A JPEG next to a RAW is copied as a file of its own; the strip
    of the date folder then shows the RAW only, as in any folder of RAW and
    JPEG files. A file without a readable capture date is undated: the dialog
    lists it and it is not copied.
  - **Preview**: the status line shows `reading <folder>…` while every file's
    head is read, then a dialog shows the source, the total and one row per
    destination folder, in date order, with the files and bytes to copy and
    the number skipped, followed by the colliding and the undated names in
    lists of their own. The counts include the sidecars. `Run` is enabled
    when at least one file is to be copied, whatever the lists hold; else a
    note says why there is nothing to do. `Cancel` or `Escape` closes the
    dialog with nothing changed.
  - **Existing folders**: a date folder that already exists gets only what is
    missing. A file with the same name and the same size there counts as
    imported and is skipped, so a second import of the same card copies
    nothing. A file with the same name and another size is a collision: it is
    listed and skipped, and the file there is never overwritten. So is the
    second of two files with one name bound for one folder, as when the
    camera's numbering wraps across `100MSDCF` and `101MSDCF`. Only name and size are
    compared, with no hash, so a file edited in place to the same length
    counts as imported. A sidecar follows its RAW: next to a colliding RAW it
    collides too, and next to a skipped RAW it is copied when missing and left
    alone otherwise, whatever its size, since Riffle has likely rewritten it.
    The check is repeated just before each file is written: a file that
    appeared since the preview is skipped at the planned size, else that file
    fails, and either way what is there stays.
  - **Partial files**: each file is written to `<name>.riffle-tmp` in its date
    folder, flushed, given the source's modified time and only then renamed to
    its name, so neither the strip nor a scan ever sees a partial file.
  - **Cull while importing**: `Run` creates the date folders and opens the
    first one to receive files, the earliest, at once, even when another
    folder was open, whose scan stops; the folder tree reveals it. It starts
    with whatever the folder already holds (empty for a new one) and fills as
    files land: the strip grows, the current file stays
    current and the judgments made so far stay. Files landing in the other
    date folders leave the strip alone. With AI analysis on, the embeddings
    mostly fill in after the import, and the strip does not wait for them.
  - **Progress and cancel**: the dialog steps aside once the run starts, and
    the status line shows `importing N / M`. Choosing `Import…` again, on any
    folder, brings the dialog back rather than starting another import. Only
    its `Cancel` stops the run at once, dropping the file in flight (the next
    import copies it): the files copied so far stay, and the next import skips
    them. `Escape` just closes the dialog and
    the run goes on. At the end the status line shows
    `Imported N files (X MB)` (`X GB` from 1 GB up), or `Import canceled, N files copied`, followed
    by `, K already there` and `, J failed` when there are any. A file that
    fails is listed as `name: reason` in the error list at the bottom of the
    right pane, and the others are still copied. An import has no undo: its
    copies go like any other file, with `Move to Trash…`.
  - **While importing**: flags, ratings and color labels work, in the date
    folder as anywhere, except that a date folder holding only JPEGs is
    view-only (see **JPEG-only folders**), so they apply once it holds a RAW
    file. `Move to Trash…`, `Delete Permanently…`, Settings, `Rename…`,
    `Rename by Capture Date…`, `Rewrite Sidecars from Index…`,
    `Delete Sidecars…` and `Sequence JPEG Timestamps…` are refused until the
    run ends, a rename with `Rename… is unavailable while an import runs` on
    the status line.
  - **Throughput**: every run writes one line to `Riffle.log` (see
    **Open Log Folder**) with the throughput, MB being 10^6 bytes:
    `import: files=120 bytes=5280000000 mb_per_s=41.2 skipped=3 failed=0 canceled=false source=E:\DCIM root=D:\Photos in 128155ms`.
    Riffle does not claim to copy faster than the file manager; see
    [Copying from a memory card](./performance.md#copying-from-a-memory-card)
    for what was measured and how to compare the two.
- **Rename by Capture Date…**: right-click a folder in the folder tree (not
  home or a volume at the top level) and choose `Rename by Capture Date…` to
  name it after the capture date of its photos, the way a card's `100MSDCF`
  is usually renamed by hand before culling. The date is the
  `DateTimeOriginal` the camera wrote, its local date as `YYYY-MM-DD` (e.g.
  `2026-10-31`) with no time-zone handling, so a shoot across midnight spans
  two dates. Only the files directly in the folder count, its RAW files or,
  in a JPEG-only folder, its JPEGs; subfolders are not searched. A file
  without a readable capture date is undated. It asks first in a dialog:
  `Run` carries it out, `Cancel` or `Escape` leaves everything untouched,
  and the buttons are disabled while the files move.
  - **One date**: the dialog asks `Rename <name> to 2026-10-31?` with the
    file count, and `Run` renames the folder exactly as `Rename…` does: the
    undated files and the subfolders come along, and the index, the
    judgments and the remembered file follow it.
  - **Several dates**: the dialog lists one row per date folder with its
    file count. `Run` creates those folders next to the original folder, in
    its parent, and moves each dated file into its date's folder, a RAW
    file with its `.xmp` and `.dop` sidecars (whichever exist) and a JPEG
    alone. Its stars, flag, color label, thumbnail and AI analysis follow
    it, so nothing is re-extracted. The undated files stay in the original
    folder, and the dialog lists them; the subfolders stay too. A file that
    fails to move stays where it was, its error listed, and the others
    still move. When the original folder ends up empty it is removed, and
    if it was the open folder the first date folder opens instead;
    otherwise it is kept, and the status line says how many entries
    (files and subfolders) were left in it
    (`Split <name> into 2026-10-31 (120), 2026-11-01 (45); <name> kept (2 left)`).
  - **Refusals**: nothing is ever merged. When a folder or file with a
    target name already exists next to the folder, the dialog shows it in
    red and `Run` is disabled, as it is when the folder already bears its
    one date. In a split, a target named like the folder itself counts as
    taken too, so a folder `2026-10-31` holding photos of two dates is not
    split. A folder with no dated photo is refused on the status line, and
    so is the item chosen while a scan runs, with the scan message; choose
    it again once the scan ends. After a partial split, running it again on
    the kept folder is refused, since its date folders now exist.
  - **Undo**: the rename or split itself cannot be undone. A
    `Move to Trash…` made before a split still undoes, but its files go
    back to the original folder rather than their date folders, and when
    the original folder was removed the undo fails and lists them as errors,
    leaving them in the Trash.
- **Rewrite Sidecars from Index… and Delete Sidecars…**: right-click a
  folder in the folder tree to push Riffle's judgments of its files back out to their
  sidecars, or to remove the sidecars altogether, for instance after a write
  that failed, a sidecar another program overwrote, or a switch of sidecar
  format. Both act only on the sidecars of the currently selected format
  (both kinds under `XMP and .dop`) next to the folder's RAW files, not on
  its subfolders, and both ask first in a dialog: `Cancel` or `Escape`
  leaves everything untouched, and the buttons are disabled while the run
  goes on. A JPEG-only folder is refused on the status line. Chosen while a
  scan runs, either waits for the scan, the status line saying so, and asks
  when the scan ends.
  - `Rewrite Sidecars from Index…` writes what the folder index holds, so
    the folder must have been opened in Riffle once; on one never opened
    the status line says `No judgments indexed for <name>; open the folder
    first`. The dialog counts the files with a judgment, those without one
    (whose existing sidecar is cleared to unrated, unflagged and unlabeled;
    none is created for them) and the RAW files the index does not know
    yet (skipped), and asks `Rewrite the XMP sidecars of N files in
    <name>?`. An existing sidecar is patched, not regenerated: only the
    rating, the pick / reject flag and the color label (and the `.dop`'s
    two timestamps) change, so Lightroom's develop settings and PhotoLab's
    corrections are kept byte for byte; a file with a judgment and no
    sidecar gets a new one. The status line then says `Rewrote the
    sidecars of N files in <name>`, with how many failed; a failed write is
    listed in the meta pane's errors and retried the next time the folder
    opens. The strip does not change, since it already shows the index.
  - `Delete Sidecars…` works on any folder, opened or not. The dialog lists
    the sidecars per kind with their size (`12 XMP sidecars (48 KB)`) and
    asks `Move N sidecars (size) of <name> to the Trash?`; `Move to Trash`
    moves them to the OS Trash and the status line says `Moved N sidecars
    to the Trash`, with how many failed (each failure is listed in the meta
    pane's errors under its RAW file). A sidecar whose RAW file is gone is
    left alone. Riffle forgets the judgments of those files, so when the
    folder is open the strip drops their stars, flags and color labels at
    once. When the folder holds no sidecar of the format the status line
    says so (`No XMP sidecars in <name>`). Like **Move to Trash…**,
    one `Edit > Undo` puts the sidecars back (`Restored N sidecars from the
    Trash`) and, when the folder is open, the judgments come back with
    them; `Edit > Redo` moves them to the Trash again.
- **Sequence JPEG Timestamps…**: right-click a folder in the folder tree and
  choose `Sequence JPEG Timestamps…` to make the capture times of the exported
  JPEGs in it unique at second granularity, so Google Photos, which ignores
  `SubSecTimeOriginal`, keeps a burst in shooting order. Cull in Riffle,
  export the keepers as JPEGs from your RAW developer, then right-click the
  export folder in the tree (derived from
  [lapse](https://github.com/minodisk/lapse) v0.4.0).
  - **Files**: the `.jpg` / `.jpeg` files (any case) directly in the folder;
    subfolders are not searched. A folder without any is an error, shown on
    the status line.
  - **Order**: by `DateTimeOriginal`, then `SubSecTimeOriginal` (compared as
    a fraction, so `5` is later than `12`; a missing or malformed value counts
    as 0, so it sorts first within its second), then natural file name order
    (`DSC2` before `DSC10`). A folder exported from two bodies, or a Sony
    series that rolled over from `9999` to `0001`, therefore comes out in
    shooting order.
  - **New times**: the first file keeps its time; every next one gets
    `max(its own time, the previous file's new time + 1 s)`. Only frames
    collapsed into the same second are pushed, and a later scene keeps its
    time once the pushed times have caught up.
  - **Tags**: `DateTimeOriginal` (0x9003) is rewritten, and
    `DateTimeDigitized` (0x9004) and `DateTime` (0x0132) are set to the same
    time when the file has them; no tag is ever created. The 19-byte values
    are overwritten in place, so every other byte, including the image data,
    stays identical.
  - **Preview**: a dialog lists every file in the computed order as
    `old -> new`, the changed part of a new `DateTimeOriginal` in green and
    the files whose `DateTimeOriginal` stays dimmed, then two lines: how many
    files will be written to the output folder (every file that could be
    read, e.g. `247 files will be written to Output-sequenced`) and how many
    of them get a new capture time, the green ones (e.g. `68 of them get a
    new capture time`), then the files that could not be read and the output
    folder. When
    `<folder>-sequenced/` already exists, a notice says it will be rebuilt:
    its JPEG files are replaced. `Run` writes; `Cancel` or `Escape` closes
    without touching anything.
  - **Output**: the source files are never written. Every file is written
    into `<folder>-sequenced/`, the sibling of the right-clicked folder, under its
    own name, the unchanged ones too, so the output is a complete copy; each
    file is written to a temporary file and renamed into place. Running it
    again rebuilds the output from the original times: the `.jpg` / `.jpeg`
    files directly in `<folder>-sequenced/` are deleted first (other files and
    subfolders are left alone), so it always mirrors the source after files
    were added, removed or renamed. The run is refused when the output folder
    resolves to the source folder, for instance through a link. The copies do
    not keep the source's modification time or permissions.
  - **Progress and cancel**: the status line shows `sequencing done / total`
    while it runs. `Cancel` or `Escape` stops before the next file; the output
    folder then holds only the complete files written so far, and the next
    run rebuilds it. At the end the status line shows
    `Wrote N of M files to <folder>-sequenced`, or `canceled, N of M written`
    after a cancel, followed by `, K failed` when some files failed. After a
    run that wrote files, `<folder>-sequenced` is revealed in the OS file
    manager, selected in its parent folder like the folder tree's reveal item;
    nothing is revealed after a cancel or when no file was written.
  - **Failures**: a file whose `DateTimeOriginal` cannot be read gets no new
    time and no copy; the preview lists it below the order with the reason,
    as `name: reason`, and `Run` is disabled when no file could be read. A
    file that fails during the run, because it could not be read or written,
    is listed as `name: reason` in the error list at the bottom of the right
    pane once the dialog closes, and the other files are still written.
- **JPEG-only folders**: a folder that holds only JPEGs, such as a
  `<folder>-sequenced/` output of **Sequence JPEG Timestamps…**, opens in the
  strip view-only, so its order and capture times can be checked without
  another app.
  - **Which folders**: the `.jpg` / `.jpeg` files (any case) directly in the
    folder are listed only when it holds no RAW file at all; a folder with
    any RAW file lists the RAW files only, so a RAW+JPEG shooting folder does
    not double its strip. The folder tree's count follows the same rule: a
    JPEG-only folder shows how many JPEGs it holds.
  - **What shows**: the thumbnails, the preview (the whole JPEG, turned by
    its EXIF Orientation), `z` 1:1, `v` on 2–4 selected files, and the meta pane's **EXIF**
    rows read from the JPEG's Exif. There is no **Maker note** row, no
    sharpness score, and no focus mark or face detection:
    `f` draws nothing. The status line shows
    `JPEG folder: view only` for as long as the folder is open.
  - **Order**: always capture time (the sort menu's capture-time order:
    sub-second, then file name, break ties), and a JPEG without Exif or a
    capture time comes last by file name. The sort menu is disabled; the
    order chosen for RAW folders is neither applied nor changed, so the next
    RAW folder opens in it.
  - **What is off**: the star, flag and color label keys, `c` (clear all),
    `Shift+x` (reject the rest of a burst) and `Undo` / `Redo` of a judgment
    do nothing (a `Move to Trash…` and its undo and redo still work), and
    the strip's right-click menu holds only `Select All`, the reveal item,
    `Copy Path`, `Copy File Name`, `Move to Trash…` and
    `Delete Permanently…`. Navigation,
    selection, the filter menu and the panel toggles keep working, and the
    folder reopens at its last viewed file like any other. The MCP
    `set_judgment` tool is refused with
    `culling does not apply to a JPEG folder`; the other tools keep working.
  - **Sidecars**: none is read or written. A `foo.xmp` or `foo.jpg.dop` next
    to `foo.jpg` is ignored, and nothing is ever written next to a JPEG.
- **Undo**: `Edit > Undo` (the `undo` key, `CmdOrCtrl+Z` by default) restores the rating, flag and color
  label the last judged file had before, writes that to its sidecar and returns
  to the file (unless the filter now hides it, which the status line says).
  Repeated presses walk further back. A judgment on several selected files
  is undone as one. A reject-rest (`Shift+x`) is undone as
  one, restoring every frame it rejected and staying on the current file. The judgment history belongs to the open folder
  and is cleared when another folder opens or the sidecar format changes. A
  `Move to Trash…` is one step of the same
  history and outlives a folder switch: undoing it restores the files from the Trash (see
  **Move to Trash…**), after which further presses reach the
  judgments made before it.
- **Redo**: `Edit > Redo` (`CmdOrCtrl+Shift+Z`) re-applies the most recently
  undone judgment, the same way round. After an undone
  `Move to Trash…` it moves the files that came back to the Trash
  again, without the dialog. The redo history is forgotten as soon as you
  judge a file again, and like the undo history its judgments are cleared
  when another folder opens or the sidecar format changes.
- **Open Log Folder**: `Help > Open Log Folder` reveals the folder holding
  `Riffle.log`, the app's log file (capped at 1 MB; on rotation the previous
  contents are discarded, not kept as a separate file). It lives in the app
  log directory:
  `%LOCALAPPDATA%\com.minodisk.riffle\logs\` on Windows,
  `~/Library/Logs/com.minodisk.riffle/` on macOS and
  `~/.local/share/com.minodisk.riffle/logs/` on Linux.
- **Auto-advance**: when `Auto-advance after a star, reject or pick` is on in
  `Riffle > Settings...` (on by default), `1`-`5`, reject, pick and un-reject / un-pick move to the
  next file, also when the file already has the value pressed (which leaves
  it as it is and writes nothing), so a run of files that are already right
  can be confirmed with the same key. The last file stays selected. A file the judgment
  drops out of the active filter already hands the cursor to the next file, so
  it is not skipped twice. A judgment on more than one selected
  file does not advance.
- **Clear Cache**: the `Cache` tab of `Riffle > Settings...` shows how much disk
  the folder index takes, and `Clear Cache` empties it after a confirmation. It
  removes the cached thumbnails and metadata of every folder ever opened; your
  judgments are not touched, because they live in the sidecars. The size is in
  the same units your file manager uses (decimal on macOS, binary on Windows
  and Linux). A press while a scan is running waits for it: a note says the
  cache is cleared as soon as the scan finishes, and the clear then runs by
  itself. Closing the modal cancels a held clear, so it only runs while the
  modal stays open until the scan ends. The size figure refreshes each time a
  scan ends.
- **AI analysis and genres**: `AI analysis of what each photo shows` in the
  `Culling` tab of `Riffle > Settings...` (off by default) turns on a third
  background pass after a folder's scan. It embeds each RAW file's preview
  with SigLIP 2 ViT-B/32 (Apache-2.0, notice in
  `crates/core/models/LICENSE-siglip2`), locally; turning the setting on the
  first time downloads the 378 MB model from Hugging Face, checked against
  its SHA-256. The embeddings are cached and not recomputed unless a file
  changes, JPEG-only folders are skipped, and turning the setting off keeps
  them. While the setting is on, the filter menu shows a `Genre` section:
  `Sports` (with `Basketball`), `Portrait` (with `Group photo`),
  `Landscape`, `Birds / animals` (with `Pets`), `Snap` (with `Camping`) and
  `Unknown`. A file can have several genres, a sub-genre always brings its
  parent, and `Unknown` holds the files no genre fits plus those the pass
  has not reached yet. The genres are a rough sort, not a judgment, and
  never change a star or a flag. On 300 of the author's frames labeled by
  hand, about 9 in 10 of the files under `Sports` and `Basketball` were
  right, about 8 in 10 under `Birds / animals`, `Pets` and `Group photo`,
  about 6 in 10 under `Camping`, and only about half under `Portrait`,
  `Landscape` and `Snap`; `Unknown` mostly catches frames that do have a
  genre. The thresholds were calibrated on one photographer's folders and
  will fit other photography less well.
- **Recommended order**: while AI analysis is on, the sort menu gains
  `Recommended` and the meta pane's Analysis group gains three rows for a
  file with an embedding. The ranking compares the frames of one burst (see
  **Bursts**) and nothing across bursts: `Recommended` keeps the bursts in
  capture order and puts each burst's frames in the order of a score
  learned from picks, highest first, a frame without an embedding yet after
  the scored ones of its burst. Whatever the sort, the strip marks the
  frame the ranking puts first in its burst with Lucide's `sparkles` icon
  (ISC license, text in `crates/app/ui/LICENSE-lucide`) at the cell's
  bottom-left, right of the focus mark icon; frames tied for first are each
  marked, and a lone scored frame or a frame without an embedding gets
  none. The icon follows the model, so it can move after a refit, and it
  is the same modest suggestion as the order. The `Recommended` row shows the frame's
  place among the scored frames of its burst and its score, how far above
  (`+`) or below (`−`) the burst's average it sits in the model's units;
  `Reason` names up to three of what moved it most: `Look` (the image
  embedding: composition, expression, the moment), `Burst position`,
  `Sharpness`, `AF eye focus`, `Eyes open`, `Head pose` and `Good photo`.
  The model starts from a built-in one, learned from the author's picks on
  27 culled shooting days, and learns your taste from the frames you
  viewed: a frame you have shown in the preview and neither picked nor
  rejected counts as not chosen, in whatever folder it sits, and a reject
  counts as a stronger no whether you viewed it or not; a frame you never
  showed and did not judge never counts. A frame counts as viewed once its
  preview has been on screen for about half a second, so frames flipped past
  while holding an arrow key do not count, nor does a frame whose preview
  cannot be read. Which frames you viewed is recorded
  across sessions whether AI analysis is on or not (a JPEG-only folder
  records none), so there is nothing to mark. On top of what it learns for
  every photo, the model learns a correction per genre (`Sports`, `Portrait`,
  `Landscape`, `Birds / animals`, `Snap`; a burst counts as the genre most of its
  frames show, see **AI analysis and genres**): a genre you have judged
  little in is ranked as the rest are until it has enough judgments, and
  one you have judged a lot in can move away from them. When a burst has a
  genre, the `Recommended` row ends with the one it was ranked as
  (`2 of 5 in its burst · +0.31 · as sports`). While AI analysis is on, the
  model relearns from your judgments so far in the background at startup
  and each time you move to another folder; `Refit Ranking` in the
  `Culling` tab of `Riffle > Settings...` relearns it now, for example
  after a culling session. The fit is stored as `ranking.json` in the app's
  local data dir. `Confidence` reads
  `Built-in only, nothing learned yet` until a pick shares a burst with a
  viewed frame or a reject, then `Low`, `Medium` or
  `High` with the bursts and folders it learned from. It is a modest
  suggestion, not a best-shot pick: held out on the author's folders, the
  top-ranked frame of a burst was a pick in about 55% of the bursts, against
  39% for the burst's first frame. It never changes a star or a flag.
  The cache keeps a folder with a viewed frame or a reject (its thumbnails,
  metadata and embeddings) past the month after which it drops a folder not
  opened, so the model can keep learning from it. Which frames you viewed is
  only forgotten by `Clear Cache`, so such a folder is kept for good: only
  `Clear Cache`, or the cache's 1 GB size limit, which drops the folders
  opened longest ago first, forgets it (the fit already made stays until the
  next refit).
- **Sharpness score**: Riffle scores each RAW file's sharpness from the embedded
  preview on the camera's eye-AF frame when the camera recorded face
  tracking, else around the AF focus point when the camera recorded one, even
  when a face is found elsewhere in the frame. Only when there is no AF point
  (manual focus, or a camera that records none) does it use the eyes of a
  detected face, or, failing that, the sharpest region of the frame. Faces
  and eyes are found by the bundled
  [YuNet](https://github.com/opencv/opencv_zoo/tree/main/models/face_detection_yunet)
  model (MIT license, text in `crates/core/models/LICENSE`), run locally with
  no network access. The score is computed in the same second pass as the
  focus mark's state and reaches MCP clients only (`get_view` and
  `get_photo`): the app shows it nowhere, since it follows the texture under
  the AF point rather than the subject's focus. On the author's 1717 bursts
  that hold a good frame (the focus mark's bright green), the sharpest frame
  was a good one 47.6% of the time, a random frame 45.8% and the first frame
  55.0%. See
  [What the camera records](./cameras.md).
- **Bursts**: frames shot within 1 s of the previous frame form a burst. The
  grouping follows capture order whatever the chosen sort, and files from a
  camera that records no sub-second capture time are grouped by whole
  seconds. The grouping deliberately follows time rather than the camera's
  own per-press sequence numbering: a burst is one moment, and one moment
  often spans several presses, such as pre-capture frames followed by the
  full press, or a quick re-press. A tinted band behind the strip cells
  joins the frames of a burst of two or more. Every
  cell of the band shows the frame's position in the burst and the burst's
  size (`3/7`), counted over the whole burst in capture order, so a filter or
  sort that hides or separates frames leaves the numbers as they are. `ArrowDown` jumps to the first frame of the next burst and
  `ArrowUp` to the first frame of the current one, or of the previous one
  when already there. `Alt+ArrowLeft` / `Alt+ArrowRight` (`Option` on macOS) move
  one frame at a time within the current burst and stop at its first and last
  displayed frame. `Shift+x` rejects every other frame of the current burst,
  including frames the filter hides, and one `Edit > Undo` restores them all.
  See [What the camera records](./cameras.md).

## Keys

| Key | Action |
|-----|--------|
| `ArrowLeft` | previous file |
| `ArrowRight` | next file |
| `Shift+ArrowLeft` | extend the selection to the previous file (or shrink it back towards the anchor) |
| `Shift+ArrowRight` | extend the selection to the next file (or shrink it back towards the anchor) |
| `CmdOrCtrl+A` | select every file in the strip (also `Edit > Select All`, whose accelerator follows this key) |
| `ArrowUp` | first frame of the current burst, or of the previous burst when already on it |
| `ArrowDown` | first frame of the next burst |
| `Alt+ArrowLeft` | previous frame in the current burst (stops at its first frame) |
| `Alt+ArrowRight` | next frame in the current burst (stops at its last frame) |
| `Cmd+O` / `Ctrl+O` | open a folder (`File > Open Folder…`) |
| `f` | toggle the focus mark |
| `z` | toggle the 1:1 focus check |
| `e` | toggle focus peaking |
| `g` (hold) | grayscale preview |
| `v` | toggle comparison of selected files / the current file with its burst's good frame, else its first frame |
| `d` | toggle the performance graph |
| `1`-`5` | rate the current file that many stars |
| `x` | reject the current file (replaces a pick, keeps the stars) |
| `Shift+x` | reject every other frame of the current burst, including frames the filter hides (replaces their picks) |
| `p` | pick the current file (replaces a reject, keeps the stars) |
| `u` | un-reject or un-pick the current file |
| `0` | clear the stars |
| `c` | clear every flag of the current file: stars, reject, pick and color label |
| `CmdOrCtrl+Z` | undo the last judgment or `Move to Trash…` (also `Edit > Undo`, whose accelerator follows this key) |
| `CmdOrCtrl+Shift+Z` | redo the last undone judgment or `Move to Trash…` (also `Edit > Redo`, whose accelerator follows this key) |
| `Alt+Cmd+ArrowLeft` / `Ctrl+Alt+ArrowLeft` | show / hide the folder tree (also `View > Folders`, whose accelerator follows this key) |
| `Alt+Cmd+ArrowRight` / `Ctrl+Alt+ArrowRight` | show / hide the metadata pane (also `View > Metadata`, whose accelerator follows this key) |
| `Alt+Cmd+ArrowDown` / `Ctrl+Alt+ArrowDown` | show / hide the filmstrip (also `View > Filmstrip`, whose accelerator follows this key) |
| `Tab` | show / hide both side panes |

Pressing the key of the label the file already has clears it; the stars, the
flag and `0` leave the label alone, while `c` clears it along with
everything else.

| Label | Key |
|-------|-----|
| red | `Ctrl+Alt+1` |
| orange | `Ctrl+Alt+2` |
| yellow | `Ctrl+Alt+3` |
| green | `Ctrl+Alt+4` |
| blue | `Ctrl+Alt+5` |
| pink | `Ctrl+Alt+6` |
| purple | `Ctrl+Alt+7` |
| clear the label | `Ctrl+Alt+0` |

Keys can be changed from `Riffle > Settings...` (`CmdOrCtrl+,`):
click a row's `+` and press a key to add it (`Escape` cancels), or click the
`×` on a key to remove it. The last key of an action cannot be removed (use
Reset). A key already used by another action is refused, and a modifier pressed alone is ignored. Any combination of
Ctrl, Alt (Option), Shift and Cmd (Windows / Super) can be bound; it is stored
as `ctrl+alt+shift+meta+` with only the modifiers held, and the key named from
its physical key, so `ctrl+alt+1` stays `1` though Option changes the typed
character on macOS. Shift counts, so Shift+J is a different key from J.
Combinations the system or the app's menu already use (`Cmd+Q`, `Cmd+,`,
`Cmd+Tab`, `Ctrl+C` on Windows, any Windows-key combination, ...) are refused.
The File menu's `Open Folder…` accelerator, the Edit menu's Undo / Redo /
Select All and the View menu's items are the exception: they follow their own action's keys, so unlike `Cmd+,` they can be
rebound, and the
combination an action leaves behind is free for another action. A View item
shows its action's key as its accelerator only when that key includes Ctrl,
Alt or Cmd, which the defaults do; rebound to a key without one (`F7`, say),
the key still works but the item shows no accelerator.
`Reset all` restores the defaults.

While the folder tree has the keyboard, only `open`, `toggleStrip`,
`toggleLeft`, `toggleRight` and `toggleSides` (by default `Cmd+O` /
`Ctrl+O`, `Alt+Cmd+Arrow` / `Ctrl+Alt+Arrow` and `Tab`) still run, whatever keys they are bound
to; every other action is ignored until the tree lets go of the keyboard.
The tree's own keys below come first, and every printable character is
type-ahead there, so one of those five actions rebound to a plain character
(`l`, say) does not fire while the tree has the keyboard.

These keys are fixed and cannot be changed:

| Key | Action |
|-----|--------|
| `Escape` | close the filter, sort or right-click menu, or leave Compare; in Settings, cancel adding a key, or close the settings; in the folder tree, hand the keyboard back to culling |
| `Tab` / `Shift+Tab` | while the first-launch developing-software dialog or the settings are open, move between the dialog's visible buttons and language list or the settings' controls (otherwise `Tab` is the side-pane toggle above, which can be rebound) |
| `CmdOrCtrl+R` | reload the folder (`File > Reload Folder`) |
| `CmdOrCtrl+,` | open the settings (`Riffle > Settings...`, `File > Settings...` on Windows and Linux) |
| `ArrowLeft` / `ArrowRight` / `Home` / `End` | in the settings tab strip, the previous / next / first / last tab |
| `ArrowUp` / `ArrowDown` / `Home` / `End` | in the folder tree, move the cursor to the previous / next / first / last visible folder (without opening it) |
| `ArrowRight` | in the folder tree, expand the cursor's folder, or move to its first subfolder when already expanded |
| `ArrowLeft` | in the folder tree, collapse the cursor's folder, or move to its parent when it is collapsed or has no subfolders |
| `Enter` | in the folder tree, open the cursor's folder |
| typing (type-ahead) | in the folder tree, jump to the folder whose name starts with the typed characters, case-insensitively and wrapping around; the characters add up until a half-second pause, and repeating one character cycles through the folders starting with it |

## Ratings and sidecars

The RAW file is never written. Judgments go into a sidecar next to it, in one
of two formats or in both. Riffle asks which on its first launch, before any
folder can be opened, and the choice can be changed later in
`Riffle > Settings...` (`File > Settings...` on Windows and Linux):

- **Lightroom (XMP)**: `FOO.ARW` gets `FOO.xmp`, holding `xmp:Rating` (`0`-`5`),
  the pick / reject flag as `xmpDM:good` (`True` for a pick, `False` for a
  reject, absent for neither), and the color label as both
  `photoshop:LabelColor` and `xmp:Label`, the way Lightroom writes them.
  `xmp:Label` carries the name configured for the color in the settings
  (English by default: `Red`, `Yellow`, `Green`, `Blue`, `Purple`) and
  `photoshop:LabelColor` the lowercase English color (`red`, ...). Choosing
  Lightroom (XMP) or Both in the first-launch dialog also asks for the
  language Lightroom's menus are in and sets the names to that language's
  Lightroom defaults (`レッド` ... `パープル` for Japanese). A sidecar
  whose `xmp:Label` matches a configured name or the English name is shown in
  that color.
- **PhotoLab (.dop)**: `FOO.ARW` gets `FOO.ARW.dop` (and `FOO.NEF`
  `FOO.NEF.dop`, `FOO.ORF` `FOO.ORF.dop`, `FOO.RAF` `FOO.RAF.dop`, keeping
  the RAW extension),
  holding the stars, the pick / reject flag and the `ColorLabel` line (`Red`,
  `Orange`, `Yellow`, `Green`, `Blue`, `Pink`, `Purple`), which PhotoLab 10
  reads.
- **Both**: every judgment is written to `FOO.xmp` and to `FOO.ARW.dop`, each
  as above. When a folder opens, the one of the two modified last is read
  back (on a tie, the XMP), so an edit made later in either Lightroom or
  PhotoLab wins. A judgment made without knowing the file's label keeps the
  label of that newest sidecar and writes it to both. The two files are each
  written atomically, but not together: if one write fails, an in-session
  retry writes the judgment into both again; after every failed attempt, the
  sidecar that did get written is remembered, so a folder open during the
  retries or once they are exhausted still writes the judgment into both
  instead of mistaking that earlier write for an external edit.

Clearing a label removes `photoshop:LabelColor` and `xmp:Label`, or the
`ColorLabel` line; no label is the field being absent. The label is kept as
the exact string the sidecar holds: a name from the other tool's vocabulary is
written back unchanged and shown in its color, and any other name (say, a
custom Lightroom label) is shown gray.

A sidecar written by another tool is edited in place: only the rating, the
flag and the label change, and everything else — develop settings, keywords —
is kept byte for byte. Clearing a file that has no sidecar creates none.

Writes happen in the background and are atomic, so a crash never leaves a
half-written sidecar, and quitting finishes any pending write. A judgment that
could not be written (say, on a locked card) is retried a few times over the
next half minute, and if it still fails it is kept and retried the next time
the folder is opened; the error is shown at the bottom of the right pane until
you dismiss it, and so is any sidecar the app cannot read when a folder opens
(damaged, or larger than 4 MiB). Sidecars edited by another tool are picked up the next
time the folder is opened; when both changed, the other tool's edit wins.
Switching the format keeps unwritten judgments and writes them in the new
format (or both); the files of a format no longer selected are left alone,
and switching to Both rewrites nothing, so a file's two sidecars can disagree
until it is judged again.

The folder index is a cache, but it also holds unwritten judgments, so a new
index schema migrates the previous ones in place instead of discarding them;
only a version it cannot migrate is dropped and rebuilt from the sidecars.

## MCP companion

Riffle can serve an [MCP](https://modelcontextprotocol.io/) (Model Context
Protocol) endpoint, so an MCP client, such as an AI assistant, can follow
along while you cull: see what Riffle shows, look at a small preview, move the
view and record judgments. It is off by default; turn it on with
`Let MCP clients connect` in the `MCP` tab of `Riffle > Settings...`
(`File > Settings...` on Windows and Linux). The choice is remembered across
launches.

- **Endpoint**: `http://127.0.0.1:41917/mcp`, served over Streamable HTTP.
  The tab shows the URL, whether the server is listening, and the error when
  it could not start (say, another program holds port 41917); Riffle itself
  keeps running either way.
- **Local only**: the server listens on the loopback address, so only programs
  on this computer reach it, and it refuses any request carrying an `Origin`
  header, which is what a web page's request would carry. There is no token:
  anything running as you on this computer can connect while it is on.
- **Nothing is deleted**: no tool moves a file to the Trash or deletes one,
  and the RAW files are never written. The only writes are judgments, and they
  go to the sidecars.
- **The main window answers**: every tool reads or changes the state of the
  main window, so a folder has to be open there for most of them to be
  useful, and a call fails if the window does not answer within 5 seconds.

| Tool | What it does |
|------|--------------|
| `get_view` | What Riffle is showing: the open folder, how many photos the filter leaves, the current photo and its position, the selection, the view mode (`normal`, `zoom` or `compare`) and the active compare frame, the sort and whether a filter is on, the current photo's burst with each frame's sharpness score, stars, flag and label, and the current photo's `eyes`: whether its eyes are open or closed, the probability they are closed (0 to 1), and the head pose (yaw, pitch and roll in degrees, signed as the meta pane's `Yaw`, `Pitch` and `Roll`, null when it could not be fitted); `eyes` is null when there is no face to judge, and absent until the photo has been judged a moment after it is shown (never in a JPEG-only folder) |
| `get_photo` | One photo's stars, flag and label, sharpness score, AF point and frame, manual focus, orientation, capture time and shooting settings (camera, lens, aperture, shutter, ISO, focal length, exposure bias, focus distance); the current photo when no path is given |
| `get_preview` | A small upright JPEG of one photo, scaled from the embedded preview (never the RAW) to a long edge of 1024 pixels, or of the requested size from 256 to 1616 |
| `show_photo` | Show one photo, as clicking it in the filmstrip does |
| `select_photos` | Select a list of photos; the first one is shown |
| `set_view` | Switch to the fitted view, the 1:1 focus check or Compare, as `z` and `v` do; Compare needs two photos to compare, as with `v` |
| `set_judgment` | Set the stars (`0` clears them), the flag (`none`, `pick` or `reject`) and the color label (`Red`, `Orange`, `Yellow`, `Green`, `Blue`, `Pink`, `Purple`, or null to clear) of the given photos, or of the selection (the active frame in Compare) when none are given |

A photo has to be in the open folder, and the tools that show, select or
judge photos refuse one the filter hides. `set_judgment` goes through the
same path as the judgment keys: fields it is not given keep each photo's own
value, the strip updates at once, the change is one step for `Edit > Undo`,
and the sidecar is written as a key press writes it. Auto-advance does not
apply to it. The server's instructions ask the assistant to suggest
judgments and to write them only when you ask.

Any client that speaks MCP over Streamable HTTP connects to the endpoint
above. Two examples, which the `MCP` tab also shows with Copy buttons:

- **Claude Code**:
  `claude mcp add --transport http riffle http://127.0.0.1:41917/mcp`
- **Claude Desktop**: add the following to `claude_desktop_config.json`. It
  relays through `npx -y mcp-remote`, so it needs Node.js; whether Claude
  Desktop takes a direct `url` entry for a local server has not been
  verified.

  ```json
  {
    "mcpServers": {
      "riffle": {
        "command": "npx",
        "args": ["-y", "mcp-remote", "http://127.0.0.1:41917/mcp"]
      }
    }
  }
  ```

## Installing

Download the installer for your OS from the latest release on the
[Releases page](https://github.com/minodisk/riffle/releases):

| OS | File |
|----|------|
| macOS, Apple Silicon | `Riffle_<version>_aarch64.dmg` |
| macOS, Intel | `Riffle_<version>_x64.dmg` |
| Windows | `Riffle_<version>_x64-setup.exe` (or `Riffle_<version>_x64_en-US.msi`) |
| Linux | `Riffle_<version>_amd64.AppImage` (or `Riffle_<version>_amd64.deb` / `Riffle-<version>-1.x86_64.rpm`) |

The builds are not OS-signed (no Apple notarization, no Authenticode), so the
first launch needs one extra step:

- **macOS**: Gatekeeper blocks the first launch. Right-click `Riffle.app` →
  Open, or System Settings → Privacy & Security → Open Anyway, or run
  `xattr -d com.apple.quarantine /Applications/Riffle.app`.
- **Windows**: SmartScreen warns. More info → Run anyway.
- **Linux**: nothing extra.

Updating: the app checks for a newer release on launch and, when there is one,
downloads and installs it quietly in the background; the new version is used the
next time Riffle launches. On Windows the download still happens quietly in the
background, and the installer runs when Riffle quits, so the next launch is the
new version. **Check for Updates…** in the app menu (the File menu
on Windows and Linux) runs the same check by hand and reports the outcome; when
it finds a new version it offers **Restart Now**, which quits (saving pending
sidecar writes first) and relaunches into the new version, or **Later**, which
keeps the behavior above. The update itself is signed with the
project's updater key and verified before it is installed. On Linux only the
AppImage updates itself; a `.deb` / `.rpm` install is updated by installing the
newer package.

To build from source, see [CONTRIBUTING.md](../../CONTRIBUTING.md).
