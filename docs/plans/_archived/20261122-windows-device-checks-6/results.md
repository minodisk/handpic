# Windows real-device checks, batch 6 (2026-11-22), strip, scan progress, resume, resync and the 1:1 view

Setup: Windows 11, debug build of `crates/app` with `--features screenshot` (origin/main `2d319f96`), window 1616x1008, synthetic mouse / keys (`keybd_event`, `mouse_event`), window captures and burst captures of the status line or the strip (one region every 80 to 200 ms, stacked with the ms offset), MCP on port 41951 seeded with `{"mcpEnabled": true}` through `--import-settings`, fresh `--index` / `--settings` / `--log` per run (except the resume, below). Material under `D:\Photos\tests\20261122-strip6\`: `few\` (300 hardlinked ARWs of three `2026-*-focus-sample` folders), `big\` (1100 hardlinked ARWs of the eleven `2026-*-focus-sample` folders, distinct files, renamed with a date prefix), `corrupt\` (copies of `D:\Photos\tests\20261110-gpui-strip-index\ARW\`, 21 files including `DSC-RX100M3_corrupt.ARW`), `manual4\` (5 ARW copies of `2026-08-22-focus-sample`), `zoom\` (copies of `NIKON_D70_Nikon.nef` and `CGO3P_YUN00007.dng`), `zoom-arw\` (copies of `ILCE-9M2_sony_a9_ii_10.arw`, `ILCE-7RM4_DSC00395.ARW`, `ILCE-7RM4A_DSC00551.ARW`, `ILCE-7C_DSC00018.ARW`, `ILCE-6400_DSC00087.ARW`, `ILCE-6600__DSC0268.ARW`, `ZV-E10_DSC00002.ARW`). Every instance was quit with `q` (the default keymap still quits on `q`; the log showed the quit handlers' drains) or through its window by its own PID.

Incidents: none. The `page` / `zoom` timing lines go to stdout, not to `Handpic.log`; they were read from the redirected stdout.

## The strip, the scan progress, the resume and the resync (gpui-strip-index)

### Box 1: `scanning N/total` on a few hundred files
- On `few\` (fresh index) MCP `get_status` polls about 0.4 s apart read `scanning 155/300`, `179`, `208`, `240`, `273`, `290`, then `analyzing 0/0` and `analyzing N/300`; a relaunch with another fresh index came up after pass 1 had ended (the files cached by then), so the burst capture missed it there.
- On `big\` (fresh index), the status line burst at 100 ms from the window's first frame: `scanning 146/1100`, `152`, `158`, `165`, `165`, `171`, `178`, … `397/1100` at 2.9 s; N changed on almost every 100 ms frame. PASS (cadence on a debug build).
- End of the scan: `scanning` gave way to `analyzing N/total` (pass 2, added after the item was written), and at pass 2's end the status line was empty. On `corrupt\` it ended as `1 failed`. PASS.

### Box 2: the 1000+ folder
- (1) Thumbnails from the current cell outward: on every capture of `big\` the cells in view already held thumbnails, names under each, the current cell outlined. The gray placeholder phase was not seen (the window took longer to come up than the cells in view took to fill). Not exercised.
- (2) A held `→`: 90 key-downs at 33 ms with one key-up (a synthetic auto-repeat). After release the preview, the outlined cell (at the right edge, the strip scrolled just enough), the meta pane and the status line all named `2026-06-05-f__DSC1673.ARW`, `91 / 1100`. `End`: `1100 / 1100`, the last cell outlined; `Home`: `1 / 1100`, the first cell. PASS.
- (3) Wheeling toward the user scrolled right; a click on a far cell (`_DSC0889`) showed that file and moved the outline (`28 / 1100`); a click in the 8 px gap after it changed nothing. PASS.
- (4) Memory, as `PrivateMemorySize64` sampled from PowerShell (not Task Manager): 200 MB before; three round trips of 1600 wheel events each way (not quite end to end): during 204–216 MB, settled 209–211 MB, then 166–169 MB once pass 2 ended; two full end-to-end round trips (2600 events each way, the last cell and the first reached): 166, 167, 169, 172, 171 MB. Rose a little while scrolling and settled at about the same level, with no climb from one run to the next. PASS.
- (5) On `corrupt\` the `DSC-RX100M3_corrupt.ARW` cell showed `invalid TIFF byte order` in its gray box, the status line `1 failed`. PASS.

### Box 3: the resume and the steered scan
- (a) `big\` with a fresh `manual3.sqlite`. Waited for both passes to end (about 280 s), pressed `End`, then `←` three times, waited a second and pressed `q` (the instance exited, the quit handlers' drains logged). Relaunched with the same arguments: the preview, the outlined cell (the strip scrolled to it), the meta pane and the status line opened at `2026-09-19-f__DSC3921.ARW`, `1097 / 1100`. PASS.
- (b) `big\` with a fresh `manual3b.sqlite`, `End` sent on the window's first frame: pass 1 had already ended (`analyzing 1/1100`, every file cached by the earlier runs), so the steering of pass 1 toward the last cells and the fill of the middle after a wheel could not be seen. Not exercised; needs a folder whose files are not in the OS cache.

### Box 4: the resync
- `manual4\` with a fresh index, paged to the second file (`_DSC0786.ARW`, `2 / 5`). The copies and deletes were made from PowerShell (`Copy-Item` / `Remove-Item`), not from Explorer.
- (1) A copy of another ARW named `_DSC0800.ARW` (sorting between `_DSC0788` and `_DSC0825`): in the capture 300 ms later its cell was in place with its thumbnail; the outline, the preview and the status line still named `_DSC0786.ARW`; the total went `2 / 5` → `2 / 6`; the other thumbnails stayed on every capture (300 ms, 1 s, 2.5 s). PASS.
- (2) `Remove-Item` of `_DSC0825.ARW` (not current): its cell went; the current file stayed `_DSC0786.ARW`; no thumbnail went gray in a 80 ms burst. PASS.
- (3) `Remove-Item` of the current `_DSC0786.ARW`: the preview, the outline and the status line moved to `_DSC0787.ARW`, `2 / 4`. With the last file current (`_DSC0800.ARW`, after `End`), deleting it moved to `_DSC0788.ARW`, `3 / 3`. PASS.

## The 1:1 view on files with a malformed full-size JPEG
- `zoom\` (`CGO3P_YUN00007.dng`, `NIKON_D70_Nikon.nef`), fresh index. The app stayed up through `z` on both files and paging between them.
- Fit view: "The preview of this file could not be shown." centered; the meta pane showed the file name and, in red, `libjpeg fatal error: Not a JPEG file: starts with 0x80 0x03` (DNG) / `... 0x3c 0x44` (NEF); the status line `error: CGO3P_YUN00007.dng: libjpeg fatal error: Not a JPEG file: starts with 0x80 0x03  2 failed`.
- 1:1 (`z`): the status line added `1:1`; the preview canvas was plain black (the failed-preview message hidden, no crop, no text of its own). The error stayed in the meta pane and the status line. stdout held one `error <file>: libjpeg fatal error: Not a JPEG file ...` line per read (two per file), no `zoom` line. PASS for staying up and surfacing the error; FINDING: the 1:1 view itself shows nothing for a failed crop.
- FINDING: both files' strip cells read `panic while encoding the thumbnail of \\?\D:\Photos\tests\20261122-strip6\zoom\CGO3P_YUN00007.dng` (and the NEF's), the internal panic wording with the Windows verbatim path, from `crates/core/src/scan.rs` (`catch_unwind` around the thumbnail encode), where the corrupt ARW's cell reads a plain reason (`invalid TIFF byte order`).

## The 1:1 view on older Sony ARW bodies (no full-size JPEG)
- `zoom-arw\`, fresh index, one fit capture and one `z` capture per file, all seven bodies (`ILCE-7C`, `ILCE-7RM4A`, `ILCE-7RM4`, `ILCE-9M2`, `ILCE-6400`, `ILCE-6600`, `ZV-E10`).
- Each 1:1 view was a sharp crop (fine texture, crisp edges, e.g. the knit of `ZV-E10_DSC00002`, the stone of `ILCE-6400_DSC00087`), at a modest magnification over the fit view, never a blurry upscale. The `page` lines gave the sources as `px=1616x1080`, `px=1920x1080` (ZV-E10) and `px=1440x1080` (α7C); each `z` printed one `zoom read=0.73–1.20ms decode=24–27ms` line. The `ILCE-6400` crop showed black beyond the preview's edge on two sides (its AF point near the corner), observation, not judged against a rule. PASS.
