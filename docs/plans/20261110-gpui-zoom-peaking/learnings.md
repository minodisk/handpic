# Learnings

## Step 1

- The canvas's device-pixel size reaches the view through an
  `Rc<Cell<(u32, u32)>>` the paint closure writes, the smaller of the two
  options the plan named. A resize is seen by `observe_window_bounds` before
  the new size is painted, so the cell is read only when the 150 ms debounce
  fires (and when a crop arrives, to notice a resize during its read).
- `Zoom::page` releases the held crop on every page and listing, zoomed or not.
  The Tauri app keeps it until the next crop replaces it, but it is never drawn
  again (its `seq` is gone), so releasing it at once is the same on screen and
  frees the texture sooner.
- The point of interest maps through 6 / 8 by pixel index (tested against
  `orient::rotate_bgra`); the placeholder rectangle turns by continuous
  coordinates as the Tauri canvas rotation does. The two differ by at most one
  JPEG pixel on a quarter turn, under the placeholder's own scale error.
- `unsized` is a reserved word in Rust; a test variable named so does not
  compile.
- Key injection: `keybd_event` needs the window in the foreground for every
  key. In one long injected sequence most keys went missing (presumably focus
  moved away mid-run); the script was changed to re-assert and check the
  foreground before each key and skip otherwise, and to pick the process by
  its path in this worktree (another `riffle-native.exe` was exiting at the
  time). With that, every injected key arrived.
- Checked by running the release build at scale 1.0 (96 dpi, 1600x1000 client)
  on `D:\Photos\tests\20261110-gpui-zoom-peaking\` with
  `--index D:\Photos\tests\20261110-gpui-zoom-peaking\index.sqlite`:
  - every `z` that asked for a crop printed one `zoom` line (25 over ARW, RAF
    and the `jpeg` subfolder), none printed an error, and `z` off / on on the
    same file printed nothing (the held crop reused);
  - 25 `→` at 30 ms while zoomed, then `z` off / on: no error and no new
    `zoom` line, so the crop held at the end was the current file's;
  - the measured figures are in `docs/agents/native-app.md` ("A crop is on
    screen 8-112 ms after `z`"); the machine was not idle.
- Test folder: `ILCE-7M3_PPR10K_1277_4.ARW` (focus point 458 px from the top
  of 4000, orientation 1, the edge case), `ILCE-7M3_PPR10K_1283_0.ARW` (an
  off-center focus point), `ILCE-7M4_DSC07652.ARW` (orientation 6),
  `ILCE-7M3_sony_a7_iii_36.arw` (orientation 8), `X-E5_DSCF1008.RAF`,
  `X-H2S_fujifilm_x_h2s_30.raf` (RAF, orientation 8), and `jpeg\` with three
  JPEGs (orientations 1, 6 and 8), all copies from `D:\photos\samples\`.

## Deferred issues (todo candidates)

- **Pending manual check (Step 1, by eye, Windows):** the step's checkbox was
  ticked on the automated criteria; these need the user's eyes (no
  screenshots). Launch the release build:
  `C:\Users\daisu\.herdr\worktrees\riffle\worktree-green-cloud-634b\crates\native\target\release\riffle-native.exe D:\Photos\tests\20261110-gpui-zoom-peaking --index D:\Photos\tests\20261110-gpui-zoom-peaking\index.sqlite`
  and check, recording the window's scale factor (Windows display scale):
  1. On `ILCE-7M3_PPR10K_1283_0.ARW`, `z` shows the crop sharp (no resampling
     blur, single pixels crisp) centered on the focus point, with the scaled
     preview around it beyond the 1024 px box, and `1:1` on the status line.
  2. On `ILCE-7M4_DSC07652.ARW` (orientation 6) and
     `ILCE-7M3_sony_a7_iii_36.arw` (orientation 8), the crop is upright and
     centered on the same subject the fit view shows at the focus point.
  3. On `ILCE-7M3_PPR10K_1277_4.ARW` (focus point near the top edge), the focus
     point still sits at the window center and the image's top edge is visible
     with black above it.
  4. Holding `→` while zoomed keeps up and ends on the file the status line
     names, showing that file's crop.
  5. Dragging the window edge while zoomed re-cuts the crop to the new size
     shortly after the drag stops.
  6. `z` again returns to the fit view; the `jpeg` subfolder (open it as the
     folder argument) behaves the same.
  Expected: all hold. Basis: plan Step 1's "Manual check on the test folder";
  files `crates/native/src/zoom.rs`, `crates/native/src/main.rs`.
