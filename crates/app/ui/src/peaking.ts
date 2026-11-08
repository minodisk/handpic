// Focus peaking: the edges whose 3x3 Sobel magnitude on Rec. 601 luminance
// clears `PEAKING_THRESHOLD`, lit in `PEAKING_COLOR` over a transparent
// overlay of the same size. The magnitude is `|gx| + |gy|` rather than
// `hypot`, which spares a square root per pixel over millions of pixels.
export const PEAKING_THRESHOLD = 160;
export const PEAKING_COLOR: readonly [number, number, number] = [255, 32, 32];

export function peaking(
  pixels: Uint8ClampedArray,
  width: number,
  height: number,
): Uint8ClampedArray {
  const overlay = new Uint8ClampedArray(width * height * 4);
  if (width < 3 || height < 3) {
    return overlay;
  }
  const luma = new Float32Array(width * height);
  for (let i = 0, p = 0; i < luma.length; i += 1, p += 4) {
    luma[i] = 0.299 * pixels[p] + 0.587 * pixels[p + 1] + 0.114 * pixels[p + 2];
  }
  const [red, green, blue] = PEAKING_COLOR;
  for (let y = 1; y < height - 1; y += 1) {
    const row = y * width;
    for (let x = 1; x < width - 1; x += 1) {
      const above = row - width + x;
      const here = row + x;
      const below = row + width + x;
      const gx =
        luma[above + 1] -
        luma[above - 1] +
        2 * (luma[here + 1] - luma[here - 1]) +
        luma[below + 1] -
        luma[below - 1];
      const gy =
        luma[below - 1] +
        2 * luma[below] +
        luma[below + 1] -
        luma[above - 1] -
        2 * luma[above] -
        luma[above + 1];
      if (Math.abs(gx) + Math.abs(gy) >= PEAKING_THRESHOLD) {
        const o = here * 4;
        overlay[o] = red;
        overlay[o + 1] = green;
        overlay[o + 2] = blue;
        overlay[o + 3] = 255;
      }
    }
  }
  return overlay;
}
