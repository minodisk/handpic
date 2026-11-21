# Draws the app icon as an SVG, renders it into `crates/app/icons/source.png`
# and derives the bundled icon set from that render: `64x64.png`,
# `128x128@2x.png`, `icon.ico` and `icon.icns` next to it. Run by a developer
# only, never at build or run time, on any OS:
#
#     python -m pip install resvg-py pillow
#     python tools/app-icon/gen.py
#
# resvg (`resvg-py`) renders the SVG with only the vendored
# `BarlowCondensed-Bold.ttf` loaded, system fonts skipped, so a font mismatch
# shows as missing text rather than a fallback face. The font and its `OFL.txt`
# come from google/fonts at commit 06bfe0d62a98375cb09c942fa18e68bd32cff057
# (`ofl/barlowcondensed/`); the TTF's SHA-256 is
# e476562ec9c1e16cf16475895b511f08c804f438cc9a9f80a44ea50a0eeb5b65. It is a
# developer-time input and ships in no package.
#
# The script also writes `icon.svg` next to itself. It references `photo.png`
# by a relative path, so it opens in a browser as-is, with Barlow Condensed
# only where that font is installed. Font rasterization and resampling can
# change between versions, so the committed PNGs are the source of truth;
# rerun this only when the design changes.
#
# `photo.png` is the 3:2 gradient photo in the frame, generated with OpenAI
# gpt-image.
#
# `icon.ico` and `icon.icns` are written by hand rather than with Pillow's
# writers, to keep the entry set and order of the files `tauri icon` made:
# Pillow's ICNS writer would drop the legacy `il32` / `is32` entries, the only
# native 16 px representation.

import io
import struct
from pathlib import Path

import resvg_py
from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[2]
SVG_PATH = HERE / "icon.svg"
FONT_PATH = HERE / "BarlowCondensed-Bold.ttf"
ICONS = ROOT / "crates/app/icons"

# Canvas 1024, body 824 centered (100px margin), macOS-style corner radius.
BODY = (100, 100, 824, 824)
RADIUS = 185
BODY_FILL = "#141416"

# Photo window: exact 3:2, sharp corners.
PX, PW = 144, 736
PH = round(PW * 2 / 3)
PY = 512 - PH // 2

# Sprocket holes: HOLES per row, outer holes flush with the photo's edges.
HOLES, HW, HH, HGAP_Y = 10, 38, 54, 10
hstep = (PW - HW) / (HOLES - 1)
top_hy = PY - HGAP_Y - HH
bot_hy = PY + PH + HGAP_Y

# Edge print. FS lays it out; TEXT_FS, TRACK (the title) and NUM_TRACK (12
# and 12A) size and space the glyphs so Barlow Condensed matches the cap height
# and width of the DIN Condensed the icon was first drawn in.
INK = "#f2f2f2"
FONT = "Barlow Condensed"
FS = 26
TEXT_FS = 26.8
TRACK = -0.4
NUM_TRACK = 0.1
top_base = top_hy - 9
bot_base = bot_hy + HH + 9 + FS * 0.7

# Filled right-pointing triangle to the left of 12A, as in a film's frame-number edge print.
A_X = PX + PW * 0.89
A_FS = FS * 0.72
A_TEXT_FS = A_FS * TEXT_FS / FS
ARROW_TIP = A_X - A_FS * 0.96
A_LEN, A_HALF = A_FS * 1.15, A_FS * 0.23
ARROW_Y = bot_base - FS * 0.35  # vertical center of "12"
A_BASE = ARROW_Y + A_FS * 0.35
ARROW = (f'<polygon points="{ARROW_TIP - A_LEN:.1f},{ARROW_Y - A_HALF:.1f} {ARROW_TIP - A_LEN:.1f},{ARROW_Y + A_HALF:.1f} {ARROW_TIP:.1f},{ARROW_Y:.1f}" '
         'fill="currentColor"/>')

holes = []
for row_y in (top_hy, bot_hy):
    for i in range(HOLES):
        x = PX + i * hstep
        holes.append(f'<rect x="{x:.2f}" y="{row_y}" width="{HW}" height="{HH}" rx="4" fill="#000"/>')

TEXTS = f"""<text x="{PX + PW / 2}" y="{top_base:.1f}">HANDPIC 60FPS CULLER</text>
    <text x="{PX + PW / 2}" y="{bot_base:.1f}" letter-spacing="{NUM_TRACK}">12</text>
    <text x="{A_X:.1f}" y="{A_BASE:.1f}" font-size="{A_TEXT_FS:.1f}" letter-spacing="{NUM_TRACK}">12A</text>
    {ARROW}"""

svg = f"""<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <clipPath id="photo"><rect x="{PX}" y="{PY}" width="{PW}" height="{PH}"/></clipPath>
    <filter id="holeglow" x="-50%" y="-50%" width="200%" height="200%">
      <feMorphology operator="dilate" radius="1"/>
      <feGaussianBlur stdDeviation="2.5"/>
    </filter>
  </defs>
  <rect x="{BODY[0]}" y="{BODY[1]}" width="{BODY[2]}" height="{BODY[3]}" rx="{RADIUS}" fill="{BODY_FILL}"/>
  <image href="photo.png" x="{PX}" y="{PY}" width="{PW}" height="{PH}" preserveAspectRatio="xMidYMid slice" clip-path="url(#photo)"/>
  <g fill="#fff" opacity="0.07" filter="url(#holeglow)">{''.join(holes).replace(' fill="#000"', '')}</g>
  {''.join(holes)}
  <g font-family="{FONT}" font-weight="700" font-size="{TEXT_FS}" letter-spacing="{TRACK}" fill="#fff" color="#fff" text-anchor="middle" opacity="0.07" filter="url(#holeglow)">
    {TEXTS}
  </g>
  <g font-family="{FONT}" font-weight="700" font-size="{TEXT_FS}" letter-spacing="{TRACK}" fill="{INK}" color="{INK}" text-anchor="middle">
    {TEXTS}
  </g>
</svg>
"""
SVG_PATH.write_text(svg, encoding="utf-8", newline="\n")

source = bytes(resvg_py.svg_to_bytes(
    svg_string=svg,
    skip_system_fonts=True,
    font_files=[str(FONT_PATH)],
    resources_dir=str(HERE),
))
(ICONS / "source.png").write_bytes(source)

full = Image.open(io.BytesIO(source)).convert("RGBA")
images = {1024: full}
pngs = {1024: source}
for size in (512, 256, 128, 64, 48, 32, 24, 16):
    images[size] = full.resize((size, size), Image.LANCZOS)
    buf = io.BytesIO()
    images[size].save(buf, "PNG")
    pngs[size] = buf.getvalue()

(ICONS / "64x64.png").write_bytes(pngs[64])
(ICONS / "128x128@2x.png").write_bytes(pngs[256])

ico_sizes = (32, 16, 24, 48, 64, 256)
ico = struct.pack("<HHH", 0, 1, len(ico_sizes))
offset = 6 + 16 * len(ico_sizes)
for size in ico_sizes:
    ico += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 0, 32, len(pngs[size]), offset)
    offset += len(pngs[size])
ico += b"".join(pngs[size] for size in ico_sizes)
(ICONS / "icon.ico").write_bytes(ico)


def rle_literal(plane):
    out = bytearray()
    for i in range(0, len(plane), 128):
        chunk = plane[i:i + 128]
        out.append(len(chunk) - 1)
        out += chunk
    return bytes(out)


def rgb_planes(size):
    r, g, b, _ = images[size].split()
    return b"".join(rle_literal(c.tobytes()) for c in (r, g, b))


def alpha_plane(size):
    return images[size].getchannel("A").tobytes()


icns_entries = [
    (b"il32", rgb_planes(32)),
    (b"l8mk", alpha_plane(32)),
    (b"ic10", pngs[1024]),
    (b"ic07", pngs[128]),
    (b"ic14", pngs[512]),
    (b"ic12", pngs[64]),
    (b"ic11", pngs[32]),
    (b"ic08", pngs[256]),
    (b"ic13", pngs[256]),
    (b"ic09", pngs[512]),
    (b"is32", rgb_planes(16)),
    (b"s8mk", alpha_plane(16)),
]
body = b"".join(kind + struct.pack(">I", 8 + len(data)) + data for kind, data in icns_entries)
(ICONS / "icon.icns").write_bytes(b"icns" + struct.pack(">I", 8 + len(body)) + body)
