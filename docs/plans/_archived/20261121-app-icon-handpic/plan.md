<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# App Icon: Handpic Edge Print, No Sticker

## Purpose

The app icon's film-border edge print still reads `RIFFLE 60FPS CULLER`, the
product's old name, and the icon carries a green peeling sticker the user no
longer wants. Regenerating was deferred because `gen.py` needed macOS (DIN
Condensed Bold, a macOS system font, and Chrome at a macOS path), and because
no checked-in command derived `icon.ico`, `icon.icns` and the PNG sizes from
`source.png` after `tauri icon` left with the Tauri app.

After this work the icon reads `HANDPIC 60FPS CULLER` in Barlow Condensed
Bold (SIL OFL, redistributable), has no sticker, and is otherwise unchanged.
One checked-in script, runnable on Windows, regenerates `source.png` and every
derived icon file.

## Steps

- [x] Step 1: Redraw the icon (HANDPIC edge print in Barlow Condensed Bold, no sticker) and regenerate every icon file from one script that runs on Windows
  - Done when:
    - `gen.py` runs to completion on this Windows 11 machine with only its
      documented Python dependencies installed. It writes `icon.svg`,
      `crates/app/icons/source.png`, `64x64.png`, `128x128@2x.png`, `icon.ico`
      and `icon.icns`. A second run changes nothing: `git status` is clean
      after rerunning on the committed state.
    - `icon.svg` and `source.png` show `HANDPIC 60FPS CULLER`, `12`, `12A` and
      the arrow in Barlow Condensed Bold, in both the glow group and the main
      group. No sticker element is left in `gen.py` or `icon.svg`: `GREEN`,
      `BACKING`, `cx, cy, r`, `theta` / `d` / `fx, fy`, `halfplane_clip`, the
      `keep` and `stickerclip` clip paths, the `innerglow` and `flapshadow`
      filters, the trailing sticker `<g>`, and the `math` import if nothing
      else uses it.
    - The derived files have the formats found on `main`:
      - `source.png`: 1024x1024 RGBA, 8-bit, non-interlaced, 100 px
        transparent margin.
      - `64x64.png`: 64x64 RGBA.
      - `128x128@2x.png`: 256x256 RGBA.
      - `icon.ico`: 6 PNG-compressed 32-bit entries in the order 32, 16, 24,
        48, 64, 256.
      - `icon.icns`: no TOC, entries in the order `il32`, `l8mk`, `ic10`
        (1024), `ic07` (128), `ic14` (512), `ic12` (64), `ic11` (32), `ic08`
        (256), `ic13` (256), `ic09` (512), `is32`, `s8mk`.
      - A header dump of the new files against `main`'s shows the same entry
        types, sizes and order. Every entry opens in Pillow, including the
        `il32` / `is32` entries with their masks.
    - A side-by-side of `main`'s `source.png` and the new one shows the body,
      the photo, the sprocket holes and their glow, and the position and
      apparent size and weight of `12`, `12A` and the arrow unchanged. Only
      the title's wording, the typeface and the missing sticker differ. The
      title's cap height matches the old render within about 1 px at 1024.
    - A contact sheet of the new icon at 16, 24, 32, 48, 64, 128 and 256 px
      next to `main`'s, on a light and a dark background, shows it as
      recognizable at 16–32 px as before, with no artifact where the sticker
      was.
    - The side-by-side and the contact sheet are shown to the user in the PR
      as images, not committed.
    - The `todo.md` item "Icon: the app icon still reads RIFFLE" is removed.
    - The paths `crates/app/build.rs`, `crates/app/src/icons.rs`, `mise.toml`,
      `tools/package/linux.sh`, `crates/app/packaging/windows/handpic.iss`,
      `README.md` and `README.ja.md` read are unchanged.
    - `mise run ci:local` passes locally. It runs the app check for a
      `crates/app/*` change, which compiles the new `icon.ico` into the exe
      through `winresource` on Windows. The PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files that change:
      - the generator folder: `gen.py`, `icon.svg`, plus the new
        `BarlowCondensed-Bold.ttf` and `OFL.txt`;
      - `crates/app/icons/`: `source.png`, `64x64.png`, `128x128@2x.png`,
        `icon.ico`, `icon.icns`;
      - `todo.md`.
      - No Rust, `mise.toml`, workflow or packaging script changes.
    - Folder: `git mv tools/macos/app-icon tools/app-icon`, since nothing in
      it is macOS-specific any more and `tools/macos/` holds nothing else.
      `ROOT` in `gen.py` becomes `parents[2]`. Do not rewrite the old path in
      archived plans, review history or the other in-flight plans; they are
      history. The `todo.md` item that names the path is removed anyway.
    - Font: vendor `BarlowCondensed-Bold.ttf` from
      `https://raw.githubusercontent.com/google/fonts/main/ofl/barlowcondensed/BarlowCondensed-Bold.ttf`
      and that directory's `OFL.txt` next to `gen.py`. Record the upstream
      commit and the file's SHA-256 in `gen.py`'s header comment. The font is
      a developer-time input only and ships in no package, so
      `tools/package/notices.sh` does not change.
    - Renderer: `resvg-py` (0.5.0 on PyPI) renders the SVG string to PNG bytes
      in place of the headless Chrome subprocess.
      - Load only the vendored font (`font_files`) with system fonts skipped,
        so a family-name mismatch shows as missing text instead of a silent
        fallback.
      - Pass the script's folder as the resources directory, so the relative
        `photo.png` href resolves.
      - Check the exact keyword names against the installed version.
      - The SVG's `font-family` is the TTF's family name, expected
        `Barlow Condensed` with `font-weight="700"`; confirm from the render.
      - `icon.svg` keeps opening in a browser as-is; whether it shows Barlow
        there depends on the font being installed. Say so in the header
        comment rather than adding an `@font-face`.
      - What remains after the sticker is removed is within resvg's feature
        set: `clipPath`, `feMorphology` + `feGaussianBlur`, `letter-spacing`,
        `currentColor`, `text-anchor`.
    - Matching the look of DIN Condensed:
      - `FS` drives layout as well as the font size (`bot_base`, `A_FS`, the
        arrow geometry). Keep `FS = 26` as the layout constant and introduce
        separate values for the text's `font-size` and `letter-spacing`, so
        the arrow, the baselines and `12A`'s position do not move.
      - Measure, do not assume. Take the cap height of the title and of `12`
        from `main`'s `source.png` and pick the Barlow `font-size` that
        reproduces it (the `12A` size scales by the same factor).
      - Barlow is reported about 6% wider at equal cap height. Reduce
        `letter-spacing` from 1 until the rendered width of a reference
        string (`60FPS CULLER`, or `12`) matches the old render. A first
        guess is roughly 0.4, to be confirmed by measurement.
      - The whole title gets wider regardless (`HANDPIC` has one more letter
        than `RIFFLE`). It stays centered at `PX + PW / 2`; check that it
        stays well inside the body's rounded corners.
    - Windows: write the SVG with `encoding="utf-8", newline="\n"`. The
      default `write_text` uses the locale encoding (cp932 here) and CRLF.
      Remove the `CHROME` constant and the macOS-only comment.
    - Derivation, in `gen.py` after the render, so one command regenerates
      everything:
      - Pillow LANCZOS downscales of the 1024 render to 16, 24, 32, 48, 64,
        128, 256 and 512, each encoded once as PNG.
      - `64x64.png` and `128x128@2x.png` are the 64 and 256 PNGs.
      - `icon.ico` and `icon.icns` are written by hand with `struct`, so the
        entry set and order match the current files (see Trade-offs for why
        not Pillow's writers).
        - ICO: `ICONDIR`, six `ICONDIRENTRY`s (width and height byte 0 for
          256, planes 0, bit count 32), then the PNG bytes. On `main` the 256
          entry is byte-identical to `128x128@2x.png`; reuse the same bytes.
        - ICNS: `icns` + total length, then each entry as type + length +
          data. The `ic*` entries are the PNG bytes. `il32` (32 px) and
          `is32` (16 px) are the R, G, B planes, each ICNS-RLE encoded; a
          literal-only encoding (control byte `n < 128` followed by `n + 1`
          bytes) is valid. `l8mk` and `s8mk` are the raw 8-bit alpha planes.
      - Verify by reopening both files with Pillow at every size.
    - Dependencies: `resvg-py` and `pillow`, installed with
      `python -m pip install`. Neither is installed on this machine today and
      `uv` is absent. Document the install line in `gen.py`'s header. The
      repository has no Python dependency manifest; do not add one.
    - Header comment of `gen.py`: rewrite it for the new facts. It is no
      longer macOS-only, uses Barlow Condensed Bold and resvg, and derives
      the icon set itself. The committed PNGs stay the source of truth, and
      the script is rerun only when the design changes.
    - Verification artifacts go in a scratch folder. The old images come from
      `git show origin/main:crates/app/icons/<file>`.
    - If a numeric diff is used (old against new outside the title band and
      the sticker's area), mind the two pitfalls `todo.md` records under
      "Docs: consider a guide for verifying Pillow pixel edits":
      `getbbox()` is alpha-only by default on RGBA, and a measurement window
      must be wider than the crop that produced the mark. That item asks to
      note whether they recur; record it in `learnings.md`.
    - Do not edit the icon rows in
      `docs/plans/20261109-rename-to-handpic/plan.md` or its `learnings.md`;
      they belong to another in-flight plan and describe the state at their
      time.

## Trade-offs and risks

- **Renderer: resvg or headless Chrome on Windows.** The old `source.png` was
  rendered by Chrome. resvg scales `photo.png` and blurs the glow with its own
  code, so pixels differ subtly everywhere, and "everything else looks the
  same" can only be judged visually. If the resvg side-by-side shows a visible
  change in the photo or the glow, fall back to Chrome
  (`C:\Program Files\Google\Chrome\Application\chrome.exe` on this machine)
  with an `@font-face` in the SVG pointing at the vendored TTF.
- **Font: vendored, not downloaded at render time.** Vendoring adds one TTF
  and `OFL.txt` next to the 1.8 MB `photo.png` already there, and needs no
  network, hash or cache code.
- **Folder renamed to `tools/app-icon/`.** History files keep naming the old
  path.
- **ICO / ICNS: hand-written containers, not Pillow's writers.** Pillow's ICNS
  writer emits only the `ic07`–`ic14` PNG entries and would drop the legacy
  `il32` / `l8mk` / `is32` / `s8mk` entries, the only native 16 px
  representation. Pillow's ICO writer is believed to sort the sizes ascending
  instead of keeping 32 first (not verified).
- **Font matching is by measurement, not exact.** Cap height and tracking can
  be matched; the letterforms cannot. The 6% width figure was not measured
  during planning.
- **The legacy ICNS entries cannot be checked on macOS from this machine.**
  The check is a Pillow round trip. The PR's macOS job only tests that the
  bundled `icon.icns` is non-empty.
- **Resampling differs from `tauri icon`.** The current small sizes came from
  the Rust `image` crate's Lanczos3; Pillow's LANCZOS is close but not
  byte-identical. The contact sheet is the check.

## Progress

- 2026-11-21 Step 1 done: the generator moved to `tools/app-icon/`, the SVG
  is rendered with resvg, and the ICO / ICNS are written by hand. The title's
  `TRACK = -0.4` leaves it about 13% wider than the old render instead of
  matching its width (matching needs about -1.1 and cramps the letters); the
  judgment call is recorded in `learnings.md`.
