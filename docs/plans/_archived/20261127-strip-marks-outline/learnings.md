# Learnings: strip-marks-outline

## Step 1: the capture and the cause

Captured on Windows 11 on 2026-11-27 with a debug build of the
`screenshot` feature, on copies of `D:\Photos\tests\20261127-c5\marks` (8
Sony ARW files, 4 of them with `.xmp` sidecars carrying stars and labels) in
a scratch folder, with a fresh `--index`, `--settings` (seeded
`{"mcpEnabled": true}` through `--import-settings`) and `--log`, on
`--mcp-port 47870`. `set_judgment` gave `_DSC1616` and `_DSC1758` 4 stars
and `ILCE-6400_DSC01340` 2 stars; `wait_for_idle`, then `screenshot` with
`long_edge` 4096 returned the window's own 1600 x 1000, unscaled.

- `before.png`: the unchanged build. `before-face-icons-zoom.png`: the strip
  around `_DSC1616` (frowning gray Low icon over a dark area) and
  `_DSC1758` (smiling green Good icon over bright gravel), 6x nearest.
- `after.png`: the changed build, same folder and state.
  `after-face-icons-zoom.png`: the same crop; `after-stars-zoom.png`: the
  4 stars of `_DSC1616`, 12x; `after-closed-eyes-zoom.png`: the closed-eyes
  mark (slot 1) of `ILCE-6300_..._DSC03068`, 12x.

Root cause (the plan's inferred cause, confirmed): the eight black copies
at `overlay::HALO`'s offsets are drawn. Over the bright gravel of
`_DSC1758` they merge into a dark box behind the 12 px icon that also fills
its inside, so the icon reads as sitting on a dark square, not as having an
outline. Over a dark photo area or the black letterbox (the first cell,
`ILCE-1M2`, and the closed-eyes mark over the letterbox) they are not
visible at all. The stars' single copy 1 px down and right gives a filled
glyph a crisp dark edge on its lower-right, which is the "outline" the user
saw. GPUI did not drop any copy; no fallback means was needed.

What the after capture shows: the stars (unchanged), the face focus icon
(Good, green, over the bright `_DSC1758`: no dark box any more, a 1 px dark
edge down and right of each stroke; Low, gray, over `_DSC1616`), and the
closed-eyes mark (over the black letterbox, where before and after look the
same). The burst badge is not in the captures: no two of these photos are
within `BURST_GAP_MS`, so the strip had no burst. Its drawing changed the
same way (one `shape_line` copy at `MARK_SHADOW`), and the user's on-screen
check in `todo.md` covers it.

Decision: the three marks take the stars' method (one black copy at
`(+1, +1)`, the shared `MARK_SHADOW` constant next to `MARK_SIZE`, which the
stars' paint now reads too). The user wanted the marks to look like the
stars; the stars' copy keeps a mark readable over bright parts of a photo;
the halo did not show as an outline, so moving the stars to it would make
them less readable. `overlay::HALO` and `overlay.rs`'s `paint_label` are
unchanged: the `f` overlay's face labels keep the halo.

## Tripped over

- A stray `mise x -- python -I -` with an empty heredoc started Python's
  REPL with no console; under Git Bash it loops on `WinError 123` and spins a
  core until killed. Never pass `-` without real stdin; write the script to a
  file.
- `python -I` ignores `PYTHONIOENCODING`, so a reply holding `–` failed to
  print on cp932; the client calls `sys.stdout.reconfigure(encoding="utf-8")`.
- Git Bash's heredoc collapsed `\\?\` in a raw Python string; the client
  built the `\\?\` prefix with `chr(92)` and read the `set_judgment` paths
  from a JSON file.
- No Pillow on this host: the zoom crops came from a small pure-Python PNG
  reader / writer (`zlib`, the filters undone by hand).
- The cold `crates/app` build at `-j 2` took 12 min 43 s; the rebuild after
  the change 28 s.

## Deferred issues (todo candidates)

- Pending manual check (on screen, Windows first, then macOS and Linux when
  at hand): the strip's stars, face focus icon, closed-eyes mark and burst
  badge each have the same dark outline (one dark copy down and right) and
  read over both bright and dark parts of a photo; include a burst (frames
  within `BURST_GAP_MS`) so the badge is seen, and a closed-eyes mark over a
  bright area. The Step 1 checkbox was ticked on the automated criteria and
  the MCP capture; colors and legibility are the user's to judge. Basis: the
  plan's Done-when and the captures in this folder, which showed no burst
  badge. Files: `crates/app/src/main.rs` (`paint_face_icon`,
  `paint_burst_badge`, `paint_marks`, `MARK_SHADOW`). Already written into
  `todo.md` as the item replacing the closed one.
