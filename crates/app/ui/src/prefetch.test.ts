import { describe, expect, test } from "vitest";
import { type Held, PREFETCH_PIXEL_CAP, PrefetchRing, neighbours } from "./prefetch.js";

interface Fake {
  width: number;
  height: number;
  name: string;
}

function held(name: string, pixels = 1): Held<Fake> {
  return { bitmap: { width: pixels, height: 1, name }, orientation: 1, jpegKB: 1 };
}

const FILES = ["a", "b", "c", "d"];

function names(bitmaps: Fake[]): string[] {
  return bitmaps.map((bitmap) => bitmap.name);
}

// Reads and stores one file.
function fill(ring: PrefetchRing<Fake>): void {
  const path = ring.next();
  if (path !== null) ring.settle(held(path));
}

describe("neighbours", () => {
  test("puts the next file in the paging direction first", () => {
    expect(neighbours(FILES, 1, 1)).toEqual(["c", "a"]);
    expect(neighbours(FILES, 1, -1)).toEqual(["a", "c"]);
  });

  test("stops at the ends of the list", () => {
    expect(neighbours(FILES, 0, 1)).toEqual(["b"]);
    expect(neighbours(FILES, 3, 1)).toEqual(["c"]);
  });
});

describe("PrefetchRing", () => {
  test("reads the wanted files in order, one at a time", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    expect(ring.next()).toBe("c");
    expect(ring.next()).toBeNull();
    ring.settle(held("c"));
    expect(ring.next()).toBe("a");
    ring.settle(held("a"));
    expect(ring.next()).toBeNull();
  });

  test("take hands the bitmap over once", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    fill(ring);
    expect(ring.take("c")?.bitmap.name).toBe("c");
    expect(ring.take("c")).toBeUndefined();
  });

  test("a page turn gives up what is no longer a neighbour", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    fill(ring);
    fill(ring);
    ring.take("c");
    expect(names(ring.plan(FILES, 2, 1))).toEqual(["a"]);
  });

  test("keep takes back a neighbour and closes anything else", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 2, 1);
    expect(ring.keep("b", held("b"))).toEqual([]);
    expect(names(ring.keep("a", held("a")))).toEqual(["a"]);
    expect(ring.take("b")?.bitmap.name).toBe("b");
  });

  test("the file on screen is never read or kept", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(["a"], 0, 1);
    expect(ring.next()).toBeNull();
    expect(names(ring.keep("a", held("a")))).toEqual(["a"]);
  });

  test("over the cap the lowest priority entry goes first", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    expect(ring.keep("a", held("a", PREFETCH_PIXEL_CAP))).toEqual([]);
    expect(names(ring.keep("c", held("c", PREFETCH_PIXEL_CAP)))).toEqual(["a"]);
    expect(ring.take("c")?.bitmap.name).toBe("c");
    expect(ring.take("a")).toBeUndefined();
  });

  test("a preview over the cap is not kept and not read again", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 2, 1);
    expect(ring.next()).toBe("d");
    expect(names(ring.settle(held("d", PREFETCH_PIXEL_CAP + 1)))).toEqual(["d"]);
    expect(ring.next()).toBe("b");
    ring.settle(held("b"));
    ring.plan(FILES, 2, 1);
    expect(ring.next()).toBeNull();
  });

  test("a neighbour evicted for space is not re-read while it stays wanted", () => {
    const big = PREFETCH_PIXEL_CAP * 0.75;
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    ring.keep("a", held("a", big));
    expect(ring.next()).toBe("c");
    expect(names(ring.settle(held("c", big)))).toEqual(["a"]);
    expect(ring.next()).toBeNull();
    // Turning on to "c" makes "a" unwanted, which ends its exclusion.
    ring.plan(FILES, 2, 1);
    ring.plan(FILES, 1, 1);
    expect(ring.next()).toBe("c");
    ring.settle(held("c"));
    expect(ring.next()).toBe("a");
  });

  test("a failed read is not retried until the file is forgotten", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(["a", "b"], 0, 1);
    expect(ring.next()).toBe("b");
    expect(ring.settle(null)).toEqual([]);
    ring.plan(["a", "b"], 0, 1);
    expect(ring.next()).toBeNull();
    ring.forget(["b"]);
    expect(ring.next()).toBe("b");
  });

  test("a reply for a file the page has moved past is closed", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    expect(ring.next()).toBe("c");
    ring.plan(FILES, 0, 1);
    expect(ring.stillWanted()).toBe(false);
    expect(names(ring.settle(held("c")))).toEqual(["c"]);
  });

  test("one prefetch stays in flight until its reply, even when stale", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    ring.next();
    ring.clear();
    ring.plan(FILES, 1, 1);
    expect(ring.next()).toBeNull();
    ring.settle(held("c"));
    expect(ring.next()).toBe("c");
  });

  test("a folder switch closes everything and voids the prefetch in flight", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    fill(ring);
    ring.next();
    expect(names(ring.clear())).toEqual(["c"]);
    ring.plan(FILES, 1, 1);
    expect(names(ring.settle(held("a")))).toEqual(["a"]);
    expect(ring.take("a")).toBeUndefined();
  });

  test("a file list change drops only the files that left", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    fill(ring);
    fill(ring);
    expect(names(ring.retain(["a", "b", "d"]))).toEqual(["c"]);
    expect(ring.take("a")?.bitmap.name).toBe("a");
  });

  test("a rename or delete forgets the paths and voids a read of them", () => {
    const ring = new PrefetchRing<Fake>();
    ring.plan(FILES, 1, 1);
    fill(ring);
    ring.next();
    expect(names(ring.forget(["c", "x"]))).toEqual(["c"]);
    expect(ring.take("c")).toBeUndefined();
    expect(ring.stillWanted()).toBe(true);
    expect(names(ring.forget(["a"]))).toEqual([]);
    expect(ring.stillWanted()).toBe(false);
    expect(names(ring.settle(held("a")))).toEqual(["a"]);
  });
});
