import { describe, expect, test } from "vitest";
import {
  FOCUS_RESCAN_INTERVAL,
  focusRescanDue,
  refreshOnFacesDone,
  refreshOnProgress,
  refreshOnScanDone,
  refreshTimingLine,
  rescanLine,
} from "./refresh.js";

describe("focusRescanDue", () => {
  test("never scanned", () => {
    expect(focusRescanDue(null, 1_000)).toBe(true);
  });

  test("inside the interval", () => {
    expect(focusRescanDue(10_000, 10_000 + FOCUS_RESCAN_INTERVAL - 1)).toBe(false);
  });

  test("exactly at the interval", () => {
    expect(focusRescanDue(10_000, 10_000 + FOCUS_RESCAN_INTERVAL)).toBe(true);
  });

  test("past the interval", () => {
    expect(focusRescanDue(10_000, 10_000 + FOCUS_RESCAN_INTERVAL + 1)).toBe(true);
  });
});

describe("refreshOnProgress", () => {
  test("no current file", () => {
    expect(refreshOnProgress(undefined, false, [], null)).toBe(false);
  });

  test("the current file already has its row", () => {
    expect(refreshOnProgress("/a.ARW", true, ["/a.ARW"], null)).toBe(false);
  });

  test("the first tick after an open", () => {
    expect(refreshOnProgress("/a.ARW", false, [], null)).toBe(true);
  });

  test("waits while the row has not landed", () => {
    expect(refreshOnProgress("/a.ARW", false, ["/b.ARW"], "/a.ARW")).toBe(false);
  });

  test("the tick that commits the current file", () => {
    expect(refreshOnProgress("/a.ARW", false, ["/b.ARW", "/a.ARW"], "/a.ARW")).toBe(true);
  });

  test("a file paged to after its row landed", () => {
    expect(refreshOnProgress("/c.ARW", false, [], "/a.ARW")).toBe(true);
  });
});

describe("refreshOnScanDone", () => {
  test("neither the reconcile nor the scan pass wrote anything", () => {
    expect(refreshOnScanDone({ scanId: 3, changed: 0 }, 3, 0)).toBe(false);
  });

  test("the scan pass wrote rows", () => {
    expect(refreshOnScanDone({ scanId: 3, changed: 0 }, 3, 5)).toBe(true);
  });

  test("the reconcile changed rows while the scan pass wrote none", () => {
    expect(refreshOnScanDone({ scanId: 3, changed: 1 }, 3, 0)).toBe(true);
  });

  test("the started record belongs to another scan", () => {
    expect(refreshOnScanDone({ scanId: 2, changed: 0 }, 3, 0)).toBe(true);
  });

  test("no started record", () => {
    expect(refreshOnScanDone(null, 3, 0)).toBe(true);
  });
});

describe("refreshOnFacesDone", () => {
  test("neither pass wrote anything", () => {
    expect(refreshOnFacesDone({ scanId: 3, total: 0 }, 3, 0)).toBe(false);
  });

  test("the faces pass wrote rows", () => {
    expect(refreshOnFacesDone({ scanId: 3, total: 0 }, 3, 2)).toBe(true);
  });

  test("the scan pass wrote rows", () => {
    expect(refreshOnFacesDone({ scanId: 3, total: 5 }, 3, 0)).toBe(true);
  });

  test("the scan-done belongs to another scan", () => {
    expect(refreshOnFacesDone({ scanId: 2, total: 0 }, 3, 0)).toBe(true);
  });

  test("no scan-done seen", () => {
    expect(refreshOnFacesDone(null, 3, 0)).toBe(true);
  });
});

describe("refreshTimingLine", () => {
  test("every phase", () => {
    const line = refreshTimingLine({
      rows: 3000,
      invoke: 41.25,
      entries: 3.5,
      bursts: 1,
      exif: 12,
      meta: 0.4,
      draw: 0.2,
      applyBursts: 1.5,
      candidates: 0.9,
      refilter: 7,
      setFiles: false,
      total: 70.04,
    });
    expect(line).toBe(
      "refresh entries: rows=3000 invoke=41.3ms entries=3.5ms bursts=1.0ms exif=12.0ms meta=0.4ms draw=0.2ms apply_bursts=1.5ms candidates=0.9ms refilter=7.0ms set_files=false total=70.0ms",
    );
    expect(line.length).toBeLessThan(220);
  });
});

describe("rescanLine", () => {
  test("deferred", () => {
    expect(rescanLine("focus", "defer")).toBe("rescan deferred: trigger=focus");
  });

  test("started at once", () => {
    expect(rescanLine("watch", "start")).toBe("rescan: trigger=watch deferred=false");
  });

  test("drained after a deferral", () => {
    expect(rescanLine("reload", "drained")).toBe("rescan: trigger=reload deferred=true");
  });

  test("names the tree's Refresh apart from Reload Folder", () => {
    expect(rescanLine("refresh", "start")).toBe("rescan: trigger=refresh deferred=false");
    expect(rescanLine("refresh", "defer")).toBe("rescan deferred: trigger=refresh");
  });

  test("names the rescan after a canceled scan", () => {
    expect(rescanLine("scan-canceled", "defer")).toBe("rescan deferred: trigger=scan-canceled");
    expect(rescanLine("scan-canceled", "drained")).toBe(
      "rescan: trigger=scan-canceled deferred=true",
    );
  });
});
