import { describe, expect, test } from "vitest";
import { PEAKING_COLOR, PEAKING_THRESHOLD, peaking } from "./peaking.js";

function image(
  width: number,
  height: number,
  gray: (x: number, y: number) => number,
): Uint8ClampedArray {
  const pixels = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const o = (y * width + x) * 4;
      const value = gray(x, y);
      pixels[o] = value;
      pixels[o + 1] = value;
      pixels[o + 2] = value;
      pixels[o + 3] = 255;
    }
  }
  return pixels;
}

function lit(overlay: Uint8ClampedArray, width: number): [number, number][] {
  const out: [number, number][] = [];
  for (let i = 0; i < overlay.length / 4; i += 1) {
    if (overlay[i * 4 + 3] !== 0) {
      out.push([i % width, Math.floor(i / width)]);
    }
  }
  return out;
}

describe("peaking", () => {
  test("a flat image lights nothing", () => {
    const overlay = peaking(
      image(8, 8, () => 128),
      8,
      8,
    );
    expect(lit(overlay, 8)).toEqual([]);
  });

  test("a vertical step lights the columns on each side of it", () => {
    const overlay = peaking(
      image(8, 6, (x) => (x < 4 ? 0 : 255)),
      8,
      6,
    );
    const columns = new Set(lit(overlay, 8).map(([x]) => x));
    const rows = new Set(lit(overlay, 8).map(([, y]) => y));
    expect([...columns]).toEqual([3, 4]);
    expect([...rows]).toEqual([1, 2, 3, 4]);
    expect(lit(overlay, 8)).toHaveLength(8);
  });

  test("a horizontal step lights the rows on each side of it", () => {
    const overlay = peaking(
      image(6, 8, (_, y) => (y < 4 ? 255 : 0)),
      6,
      8,
    );
    const rows = new Set(lit(overlay, 6).map(([, y]) => y));
    const columns = new Set(lit(overlay, 6).map(([x]) => x));
    expect([...rows]).toEqual([3, 4]);
    expect([...columns]).toEqual([1, 2, 3, 4]);
    expect(lit(overlay, 6)).toHaveLength(8);
  });

  test("a step below the threshold lights nothing", () => {
    const step = Math.floor(PEAKING_THRESHOLD / 4) - 1;
    const overlay = peaking(
      image(8, 8, (x) => (x < 4 ? 100 : 100 + step)),
      8,
      8,
    );
    expect(lit(overlay, 8)).toEqual([]);
  });

  test("the one-pixel border stays unlit", () => {
    const overlay = peaking(
      image(8, 8, (x, y) => ((x + y) % 2 === 0 ? 0 : 255)),
      8,
      8,
    );
    for (const [x, y] of lit(overlay, 8)) {
      expect(x).toBeGreaterThan(0);
      expect(x).toBeLessThan(7);
      expect(y).toBeGreaterThan(0);
      expect(y).toBeLessThan(7);
    }
    const border = image(3, 3, (x) => (x === 0 ? 0 : 255));
    expect(lit(peaking(border, 3, 3), 3)).toEqual([[1, 1]]);
  });

  test("the overlay is RGBA of the image's size, lit pixels opaque in the peaking color", () => {
    const overlay = peaking(
      image(5, 7, (x) => (x < 2 ? 0 : 255)),
      5,
      7,
    );
    expect(overlay).toHaveLength(5 * 7 * 4);
    const pixels = lit(overlay, 5);
    expect(pixels.length).toBeGreaterThan(0);
    for (const [x, y] of pixels) {
      const o = (y * 5 + x) * 4;
      expect([overlay[o], overlay[o + 1], overlay[o + 2], overlay[o + 3]]).toEqual([
        ...PEAKING_COLOR,
        255,
      ]);
    }
  });

  test("tiny images do not throw", () => {
    expect(
      peaking(
        image(1, 1, () => 0),
        1,
        1,
      ),
    ).toHaveLength(4);
    expect(
      peaking(
        image(2, 2, (x) => x * 255),
        2,
        2,
      ),
    ).toHaveLength(16);
  });
});
