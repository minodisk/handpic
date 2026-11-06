import { describe, expect, test } from "vitest";
import { anchorAfterFilter, filterListChanged } from "./filter.js";
import {
  firstEntriesAnchor,
  type LastViewed,
  lastViewedWriter,
  mustReshow,
  resumeTarget,
} from "./resume.js";

describe("resumeTarget", () => {
  const all = ["/a.ARW", "/b.ARW", "/c.ARW"];

  test("nothing remembered", () => {
    expect(resumeTarget(null, all)).toBeUndefined();
  });

  test("the remembered file is gone", () => {
    expect(resumeTarget("/z.ARW", all)).toBeUndefined();
  });

  test("the remembered file is still listed", () => {
    expect(resumeTarget("/b.ARW", all)).toBe("/b.ARW");
  });
});

describe("firstEntriesAnchor", () => {
  test("a pending resume target takes over from the provisional anchor", () => {
    expect(firstEntriesAnchor("/b.ARW", "/a.ARW")).toBe("/b.ARW");
  });

  test("no pending resume target falls back to the provisional anchor", () => {
    expect(firstEntriesAnchor(undefined, "/a.ARW")).toBe("/a.ARW");
  });

  test("neither a pending target nor a provisional anchor", () => {
    expect(firstEntriesAnchor(undefined, undefined)).toBeUndefined();
  });
});

describe("mustReshow", () => {
  test("a pending resume always needs show(), even landing on its own anchor", () => {
    expect(mustReshow(true, "/a.ARW", "/a.ARW")).toBe(true);
  });

  test("a pending resume that moved off its anchor still needs show()", () => {
    expect(mustReshow(true, "/b.ARW", "/a.ARW")).toBe(true);
  });

  test("no pending resume and the current file stayed on its anchor skips show()", () => {
    expect(mustReshow(false, "/a.ARW", "/a.ARW")).toBe(false);
  });

  test("no pending resume but the current file moved off its anchor still needs show()", () => {
    expect(mustReshow(false, "/b.ARW", "/a.ARW")).toBe(true);
  });

  // An import's date folder opens empty: its first `resync` lists the first
  // file that landed, and `refilter` must rebuild the strip and show it.
  test("the first file to land in a folder opened empty is rebuilt and shown", () => {
    const empty: string[] = [];
    const landed = ["/d/DSC00001.ARW"];
    const anchor = empty[0];
    expect(filterListChanged(empty, landed, false)).toBe(true);
    const target = anchorAfterFilter(landed, () => true, anchor, empty);
    const current = target ?? landed[0];
    expect(current).toBe("/d/DSC00001.ARW");
    expect(mustReshow(false, current, anchor)).toBe(true);
  });

  // While the import goes on, each `resync` adds files after the current one:
  // the current file stays and is not shown again.
  test("files landing after the current one keep it current without a reshow", () => {
    const before = ["/d/DSC00001.ARW", "/d/DSC00002.ARW"];
    const after = [...before, "/d/DSC00003.ARW", "/d/DSC00004.ARW"];
    const anchor = before[1];
    expect(filterListChanged(before, after, false)).toBe(true);
    const target = anchorAfterFilter(after, () => true, anchor, before);
    expect(target).toBe(anchor);
    expect(mustReshow(false, target, anchor)).toBe(false);
  });
});

describe("lastViewedWriter", () => {
  function recorder(): {
    sent: LastViewed[];
    settle: () => Promise<void>;
    send: (value: LastViewed) => Promise<unknown>;
  } {
    const sent: LastViewed[] = [];
    const resolvers: (() => void)[] = [];
    return {
      sent,
      settle: async () => {
        resolvers.shift()?.();
        await new Promise((resolve) => setTimeout(resolve, 0));
      },
      send: (value) => {
        sent.push(value);
        return new Promise((resolve) => {
          resolvers.push(() => resolve(undefined));
        });
      },
    };
  }

  test("sends at once when idle", () => {
    const { sent, send } = recorder();
    const push = lastViewedWriter(send);
    push({ dir: "/d", path: "/d/a.ARW" });
    expect(sent).toEqual([{ dir: "/d", path: "/d/a.ARW" }]);
  });

  test("keeps only the latest value while one is in flight", async () => {
    const { sent, settle, send } = recorder();
    const push = lastViewedWriter(send);
    push({ dir: "/d", path: "/d/a.ARW" });
    push({ dir: "/d", path: "/d/b.ARW" });
    push({ dir: "/d", path: "/d/c.ARW" });
    expect(sent).toHaveLength(1);
    await settle();
    expect(sent).toEqual([
      { dir: "/d", path: "/d/a.ARW" },
      { dir: "/d", path: "/d/c.ARW" },
    ]);
    await settle();
    expect(sent).toHaveLength(2);
    push({ dir: "/e", path: "/e/a.ARW" });
    expect(sent[2]).toEqual({ dir: "/e", path: "/e/a.ARW" });
  });

  test("a failed send does not stall the next one", async () => {
    const sent: LastViewed[] = [];
    const push = lastViewedWriter((value) => {
      sent.push(value);
      return Promise.reject(new Error("failed"));
    });
    push({ dir: "/d", path: "/d/a.ARW" });
    push({ dir: "/d", path: "/d/b.ARW" });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(sent).toEqual([
      { dir: "/d", path: "/d/a.ARW" },
      { dir: "/d", path: "/d/b.ARW" },
    ]);
  });
});
