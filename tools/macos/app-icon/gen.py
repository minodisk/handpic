# Draws the app icon (its edge print still reads RIFFLE) as an SVG and renders it into
# `crates/app/icons/source.png`, the 1024x1024 image `tauri icon` derives the
# bundled icon set from. Run by a developer only, never at build or run time:
#
#     python3 tools/macos/app-icon/gen.py
#
# macOS only: the edge print uses DIN Condensed Bold, a macOS system font, and
# headless Chrome at `/Applications/Google Chrome.app/...` renders the SVG.
# The script writes `icon.svg` next to itself (it references `photo.png` by a
# relative path, so it opens in a browser as-is) and then the render. Chrome
# and font rasterization can change between versions, so the committed PNGs are
# the source of truth; rerun this only when the design changes.
#
# `photo.png` is the 3:2 gradient photo in the frame, generated with OpenAI
# gpt-image.
#
# After a rerun, regenerate the bundled icons from the repository root and drop
# the mobile icons the app has no use for:
#
#     pnpm exec tauri icon crates/app/icons/source.png -o crates/app/icons
#     rm -r crates/app/icons/ios crates/app/icons/android

import math
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[3]
SVG_PATH = HERE / "icon.svg"
PNG_PATH = ROOT / "crates/app/icons/source.png"
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

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

# Edge print.
INK = "#f2f2f2"
FONT = "'DIN Condensed'"
FS = 26
top_base = top_hy - 9
bot_base = bot_hy + HH + 9 + FS * 0.7

# Filled right-pointing triangle to the left of 12A, as in a film's frame-number edge print.
A_X = PX + PW * 0.89
A_FS = FS * 0.72
ARROW_TIP = A_X - A_FS * 0.96
A_LEN, A_HALF = A_FS * 1.15, A_FS * 0.23
ARROW_Y = bot_base - FS * 0.35  # vertical center of "12"
A_BASE = ARROW_Y + A_FS * 0.35
ARROW = (f'<polygon points="{ARROW_TIP - A_LEN:.1f},{ARROW_Y - A_HALF:.1f} {ARROW_TIP - A_LEN:.1f},{ARROW_Y + A_HALF:.1f} {ARROW_TIP:.1f},{ARROW_Y:.1f}" '
         'fill="currentColor"/>')

# Sticker: green circle near the photo's top-left, peeled just a little at the lower right.
GREEN = "#5cbe58"
BACKING = "#e6ebe3"
cx, cy, r = PX + 34, PY + 30, 54
theta = math.radians(40)
ux, uy = math.cos(theta), math.sin(theta)
d = 0.93 * r
fx, fy = cx + 2 * d * ux, cy + 2 * d * uy  # flap = circle reflected across the fold


def halfplane_clip(cid):
    # A large square covering everything on the center's side of the fold line.
    ang = math.degrees(theta)
    mx, my = cx + d * ux, cy + d * uy
    return (
        f'<clipPath id="{cid}"><rect x="-2000" y="-1000" width="2000" height="2000" '
        f'transform="translate({mx:.2f} {my:.2f}) rotate({ang:.2f})"/></clipPath>'
    )


holes = []
for row_y in (top_hy, bot_hy):
    for i in range(HOLES):
        x = PX + i * hstep
        holes.append(f'<rect x="{x:.2f}" y="{row_y}" width="{HW}" height="{HH}" rx="4" fill="#000"/>')

svg = f"""<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <clipPath id="photo"><rect x="{PX}" y="{PY}" width="{PW}" height="{PH}"/></clipPath>
    {halfplane_clip("keep")}
    <filter id="holeglow" x="-50%" y="-50%" width="200%" height="200%">
      <feMorphology operator="dilate" radius="1"/>
      <feGaussianBlur stdDeviation="2.5"/>
    </filter>
    <clipPath id="stickerclip"><circle cx="{cx}" cy="{cy}" r="{r}"/></clipPath>
    <filter id="innerglow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="1.5"/>
    </filter>
    <filter id="flapshadow" x="-50%" y="-50%" width="200%" height="200%">
      <feDropShadow dx="1.5" dy="2" stdDeviation="2" flood-color="#000" flood-opacity="0.35"/>
    </filter>
  </defs>
  <rect x="{BODY[0]}" y="{BODY[1]}" width="{BODY[2]}" height="{BODY[3]}" rx="{RADIUS}" fill="{BODY_FILL}"/>
  <image href="photo.png" x="{PX}" y="{PY}" width="{PW}" height="{PH}" preserveAspectRatio="xMidYMid slice" clip-path="url(#photo)"/>
  <g fill="#fff" opacity="0.07" filter="url(#holeglow)">{''.join(holes).replace(' fill="#000"', '')}</g>
  {''.join(holes)}
  <g font-family="{FONT}" font-weight="700" font-size="{FS}" letter-spacing="1" fill="#fff" color="#fff" text-anchor="middle" opacity="0.07" filter="url(#holeglow)">
    <text x="{PX + PW / 2}" y="{top_base:.1f}">RIFFLE 60FPS CULLER</text>
    <text x="{PX + PW / 2}" y="{bot_base:.1f}">12</text>
    <text x="{A_X:.1f}" y="{A_BASE:.1f}" font-size="{A_FS:.1f}">12A</text>
    {ARROW}
  </g>
  <g font-family="{FONT}" font-weight="700" font-size="{FS}" letter-spacing="1" fill="{INK}" color="{INK}" text-anchor="middle">
    <text x="{PX + PW / 2}" y="{top_base:.1f}">RIFFLE 60FPS CULLER</text>
    <text x="{PX + PW / 2}" y="{bot_base:.1f}">12</text>
    <text x="{A_X:.1f}" y="{A_BASE:.1f}" font-size="{A_FS:.1f}">12A</text>
    {ARROW}
  </g>
  <g>
    <g clip-path="url(#keep)">
      <circle cx="{cx}" cy="{cy}" r="{r}" fill="{GREEN}"/>
      <circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="#fff" stroke-opacity="0.3" stroke-width="4" filter="url(#innerglow)" clip-path="url(#stickerclip)"/>
    </g>
    <circle cx="{fx:.2f}" cy="{fy:.2f}" r="{r}" fill="{BACKING}" clip-path="url(#keep)" filter="url(#flapshadow)"/>
  </g>
</svg>
"""
SVG_PATH.write_text(svg)
subprocess.run(
    [
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--default-background-color=00000000",
        "--window-size=1024,1024",
        f"--screenshot={PNG_PATH}",
        SVG_PATH.as_uri(),
    ],
    check=True,
)
