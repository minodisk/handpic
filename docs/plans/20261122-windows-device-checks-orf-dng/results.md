# Windows real-device checks, batch 8 (2026-11-22), ORF and big-endian DNG

Setup: Windows 11, debug build of `crates/app` with `--features screenshot` (origin/main `19bacdcf`). MCP on port 41991 (`--mcp-port`), seeded with `{"mcpEnabled": true, "sidecarFormat": "both"}` through `--import-settings` into a fresh `--settings` store; `mcpAllowDestructive` left off. Fresh `--index` / `--settings` / `--log` per run (the DNG run's index was the old build's, below). Material under `D:\Photos\tests\20261122-orf-dng\`: `orf\` (copies of all 169 files of `D:\Photos\samples\ORF\`: 167 ORFs and two `.xmp` that came with the samples) and `dng\` (copies of the eight `D:\Photos\samples\DNG\` files of the Pentax K-5 II, the iPhone 12 Pro / Pro Max and the Leica M (Typ 240)). Views were captured through the MCP `screenshot` tool and window captures; the 1:1 view was entered through MCP `set_view` `zoom`; judgments, the strip's right-click menu, the dialog and the undo used synthetic `keybd_event` / `mouse_event` input after checking the foreground window by PID. Each instance was closed through its window by its own PID; the log showed the quit handlers' drains.

Incidents: none. Git Bash mangled `\\` inside the MCP JSON argument (the rule in `docs/agents/app.md`); the client expanded placeholders for the test folder and the backslash instead.

## The pre-bump build (EXTRACTOR_VERSION 11)

- Commit `425667a2`, the parent of `a6382dbc` (`fix(core): open big-endian (MM) DNGs`), the last tree with `EXTRACTOR_VERSION` 11 (index schema 17). That tree is the Tauri app (`riffle-app`).
- Built in a temporary `git worktree` under the session's scratchpad (outside this worktree, its own `target`) with `pnpm tauri build --debug --no-bundle`, after changing only `tauri.conf.json`'s `identifier` to `com.minodisk.riffle-devicecheck`, so its settings store, window state and index (`%APPDATA%` / `%LOCALAPPDATA%\com.minodisk.riffle-devicecheck`) were separate from any real Riffle or Handpic data. The old app takes no `--index`; its first-run sidecar dialog was answered (Lightroom, then Continue) and it opened `dng\` from a seeded `lastFolder`.
- It showed every cell of `dng\` as `not a little-endian TIFF/ARW`, the status `8 failed`. The index (copied after a clean close) held 8 `files` rows at `extractor` 11, each `error` `not a little-endian TIFF/ARW`, no thumbnail, no make or model; `user_version` 17.
- The temporary worktree and the test identifier's data folders were deleted afterwards.

## Big-endian DNGs after the EXTRACTOR_VERSION bump

- The current build opened `dng\` on a copy of that index. The log: `scan reconcile: ... todo=8 removed=8`, `scan extract: ... files=8 done=8 errors=1`, then pass 2 on 7 files. The index came out at schema 27, every row at `extractor` 13.
- Re-extraction: PASS. 7 of the 8 rows lost the cached error and gained a thumbnail, make, model and orientation (K-5 II x2, Leica M (Typ 240) x3, iPhone 12 Pro, iPhone 12 Pro Max). The eighth, `iPhone_12_Pro_Max_thing.dng`, now reads `invalid TIFF byte order`. Its first bytes are `FF D8 FF E0 ... JFIF`, a JPEG named `.dng`, the case `docs/plans/_archived/20261002-big-endian-dng/learnings.md` already records.
- Thumbnails, preview and 1:1 view: PASS on all seven. The strip shows each thumbnail. Each fit view and each `zoom` view was captured; the meta pane held the EXIF rows. The 1:1 views of the K-5 II (4928x3264 preview) and the iPhone 12 Pro (4032x3024, orientation 6, shown upright) were sharp crops.
- FINDING: the three Leica M (Typ 240) DNGs hold only a 160x120 JPEG (`page` lines: `jpegKB=9..13 px=160x120`). The fit view is that image upscaled, visibly blocky, and the 1:1 view shows it as a small 160x120 picture in the middle of the canvas. `LEICA_M_(Typ_240)_MP006646.DNG` (orientation 8) is upright in both views.
- `iPhone_12_Pro_Max_thing.dng`: the fit view shows "The preview of this file could not be shown.", the meta pane `invalid TIFF byte order` in red, and the status line `error: iPhone_12_Pro_Max_thing.dng: invalid TIFF byte order  1 failed`.

## An ORF folder in the strip, sidecars and Move to Trash

Fresh index on `orf\`. Pass 1 took 2.2 s on 167 files (`errors=2`) and pass 2 took 29 s on 165 files.

- (1) Strip and tree count: PASS. All 167 ORFs are listed, 165 with a thumbnail. The two that fail, `Unknown_image.orf` and `Unknown_image_2.orf`, show `no embedded preview` in their cells, and the status line reads `2 failed`. These two failed the same way in the 2026-10-02 `check` run (`docs/plans/_archived/20261002-big-endian-dng/check-after.txt`). The tree's `orf` row reads 167 (the two `.xmp` not counted).
- (2) Preview and 1:1: PASS. `E-M1MarkII_olympus_om_d_e_m1_mark_ii_01.orf`: the `page` line reads `px=3200x2400 orientation=8`, and the fit view shows the portrait upright. The 1:1 (`zoom`) view is a sharp crop of the face, upright. Across the folder, 23 files are orientation 8 and 6 are orientation 6.
- (3) Meta pane: PASS. The meta pane shows Aperture f/4, Shutter 1/2500, ISO 800, Focal length 100 mm, Exposure 0 EV, Camera `OLYMPUS CORPORATION E-M1MarkII`, Lens `OLYMPUS M.12-100mm F4.0` and Captured `2016:11:05 15:31:16`. No `make` or `model` in the index ends in a space. Every opened file has a capture time and an exposure; 104 of 165 have a lens.
- FINDING (outside the box): the lens of `E-M5MarkII__1230049.ORF` and `E-PM2_P1010001.ORF` is stored as `LEICA DG SUMMILUX 25/F1.4 `, with a trailing space.
- (4) Sidecars: PASS, with the sidecar format set to both. `3` (auto-advance on, so the file was shown again over MCP after each key), `ctrl+alt+1` and `p` on `E-M1MarkII_olympus_om_d_e_m1_mark_ii_01.orf` wrote two sidecars next to it.
  - `E-M1MarkII_olympus_om_d_e_m1_mark_ii_01.xmp`: `xmp:Rating="3"`, `xmpDM:good="True"`, `xmp:Label="Red"`, `photoshop:LabelColor="red"`.
  - `E-M1MarkII_olympus_om_d_e_m1_mark_ii_01.orf.dop`: `Rating = 3`, `ShouldProcess = 0`, `ColorLabel = "Red"`.
  - The cell showed the stars, the pick dot and the red name band. MCP `get_photo` read rating 3, flag `pick`, label `Red`.
- (5) Move to Trash and Undo: PASS.
  - A right-click on that file's strip cell opened the strip menu on that file: Pick, three stars and Red checked. `Move to Trash…` opened the dialog `Move 1 file to the Trash?`. `Move to Trash` moved the ORF, its `.xmp` and its `.orf.dop` to the Recycle Bin, all three with the original location `D:\Photos\tests\20261122-orf-dng\orf`. The strip went to `27 / 166`, the tree row to 166, and the pane note read `Moved 1 file to the Trash`. Log: `trash: moved=1 failed=0`.
  - `ctrl+z` restored all three files (the ORF at 18000300 bytes, the sidecars at 477 / 478 bytes), and none were left in the Recycle Bin. The cell came back with its marks; the strip read `28 / 167`, the tree row 167, and the note `Restored 1 file from the Trash`. Log: `trash: undo of run 1 in 112ms: Ok((1, 0))`.
