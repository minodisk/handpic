import { describe, expect, test } from "vitest";
import { type ViewApi, getView, handleRequest, respond } from "./companion.js";
import { COMPARE_NEEDS_FRAMES } from "./compare.js";
import type { Eyes } from "./eyes.js";
import { type Command, type PickFlag, judgments } from "./selection.js";

function view(overrides: Partial<ViewApi> = {}): ViewApi {
  return {
    folder: null,
    files: [],
    index: 0,
    selection: new Set(),
    bursts: new Map(),
    sharpness: new Map(),
    ratings: new Map(),
    flags: new Map(),
    labels: new Map(),
    zoomed: false,
    comparing: false,
    compareActive: null,
    sort: "name",
    filtered: false,
    viewOnly: false,
    eyes: () => undefined,
    showPhoto: () => {},
    selectPhotos: () => {},
    setMode: () => {},
    judge: () => {},
    ...overrides,
  };
}

// A view over `files` whose actions record their calls and apply them the
// way `main.ts` does; `compare` is whether compare can start.
function driven(files: string[], compare = true) {
  const calls: unknown[][] = [];
  const state = view({ folder: "/d", files, selection: new Set(files.slice(0, 1)) });
  return {
    calls,
    view: Object.assign(state, {
      showPhoto(path: string) {
        calls.push(["showPhoto", path]);
        Object.assign(state, { index: files.indexOf(path), selection: new Set([path]) });
      },
      selectPhotos(paths: readonly string[]) {
        calls.push(["selectPhotos", paths]);
        Object.assign(state, { index: files.indexOf(paths[0]), selection: new Set(paths) });
      },
      setMode(mode: string) {
        calls.push(["setMode", mode]);
        Object.assign(state, {
          comparing: mode === "compare" && compare,
          zoomed: mode === "zoom",
        });
      },
    }),
  };
}

describe("getView", () => {
  test("reports an empty view with no folder open", () => {
    expect(getView(view())).toEqual({
      folder: null,
      count: 0,
      current: null,
      selected: [],
      mode: "normal",
      compare_active: null,
      burst: [],
      sort: "name",
      filtered: false,
    });
  });

  test("reports a single file with no burst", () => {
    const state = getView(
      view({
        folder: "/d",
        files: ["/d/a.ARW"],
        selection: new Set(["/d/a.ARW"]),
        bursts: new Map([["/d/a.ARW", { burst: 0, position: 0, size: 1 }]]),
        zoomed: true,
        sort: "capture",
        filtered: true,
      }),
    );
    expect(state).toEqual({
      folder: "/d",
      count: 1,
      current: { path: "/d/a.ARW", position: 1 },
      selected: ["/d/a.ARW"],
      mode: "zoom",
      compare_active: null,
      burst: [],
      sort: "capture",
      filtered: true,
    });
  });

  test("lists the current file's burst in capture order with its judgments", () => {
    const state = getView(
      view({
        folder: "/d",
        files: ["/d/a", "/d/b", "/d/c", "/d/x"],
        index: 1,
        selection: new Set(["/d/c", "/d/b"]),
        bursts: new Map([
          ["/d/c", { burst: 0, position: 2, size: 3 }],
          ["/d/a", { burst: 0, position: 0, size: 3 }],
          ["/d/b", { burst: 0, position: 1, size: 3 }],
          ["/d/x", { burst: 1, position: 0, size: 1 }],
        ]),
        sharpness: new Map([
          ["/d/a", 0.5],
          ["/d/b", 0.8],
        ]),
        ratings: new Map([["/d/b", 3]]),
        flags: new Map([["/d/c", "reject"]]),
        labels: new Map([["/d/a", "Red"]]),
      }),
    );
    expect(state.current).toEqual({ path: "/d/b", position: 2 });
    expect(state.selected).toEqual(["/d/b", "/d/c"]);
    expect(state.burst).toEqual([
      {
        path: "/d/a",
        position: 1,
        sharpness: 0.5,
        rating: null,
        flag: "none",
        label: "Red",
        visible: true,
      },
      {
        path: "/d/b",
        position: 2,
        sharpness: 0.8,
        rating: 3,
        flag: "none",
        label: null,
        visible: true,
      },
      {
        path: "/d/c",
        position: 3,
        sharpness: null,
        rating: null,
        flag: "reject",
        label: null,
        visible: true,
      },
    ]);
  });

  test("marks a burst frame the filter hides as not visible", () => {
    const state = getView(
      view({
        folder: "/d",
        // The filter hides "/d/c", so it is missing from `files` even
        // though `bursts` still groups it with "/d/a" and "/d/b".
        files: ["/d/a", "/d/b"],
        bursts: new Map([
          ["/d/a", { burst: 0, position: 0, size: 3 }],
          ["/d/b", { burst: 0, position: 1, size: 3 }],
          ["/d/c", { burst: 0, position: 2, size: 3 }],
        ]),
      }),
    );
    expect(state.burst.map(({ path, visible }) => ({ path, visible }))).toEqual([
      { path: "/d/a", visible: true },
      { path: "/d/b", visible: true },
      { path: "/d/c", visible: false },
    ]);
  });

  test("reports compare mode and its active pane", () => {
    const state = getView(
      view({
        folder: "/d",
        files: ["/d/a", "/d/b"],
        selection: new Set(["/d/a", "/d/b"]),
        comparing: true,
        compareActive: "/d/b",
      }),
    );
    expect(state.mode).toBe("compare");
    expect(state.compare_active).toBe("/d/b");
  });

  const judged: Eyes = {
    state: "closed",
    probability: 0.9,
    ear: 0.1,
    pose: { yaw: 40, pitch: -5, roll: 2 },
    mesh: { width: 1616, height: 1080, points: [[1, 2]] },
  };

  function eyesView(eyes: Eyes | null | undefined): ViewApi {
    return view({
      folder: "/d",
      files: ["/d/a", "/d/b"],
      index: 1,
      eyes: (path) => (path === "/d/b" ? eyes : undefined),
    });
  }

  test("reports the current file's eyes judgment and head pose without the mesh", () => {
    expect(getView(eyesView(judged)).current).toEqual({
      path: "/d/b",
      position: 2,
      eyes: { state: "closed", probability: 0.9, pose: { yaw: 40, pitch: -5, roll: 2 } },
    });
  });

  test("reports a null pose when the fit failed", () => {
    expect(getView(eyesView({ ...judged, state: "open", pose: null })).current?.eyes).toEqual({
      state: "open",
      probability: 0.9,
      pose: null,
    });
  });

  test("reports null eyes when the file was judged without a face", () => {
    const current = getView(eyesView(null)).current;
    expect(current).not.toBeNull();
    expect(current?.eyes).toBeNull();
  });

  test("leaves the eyes key out until the file has been judged", () => {
    const current = getView(eyesView(undefined)).current;
    expect(current).not.toBeNull();
    expect("eyes" in (current ?? {})).toBe(false);
  });
});

describe("respond", () => {
  test("answers get_view", async () => {
    const reply = await respond({ id: 7, kind: "get_view", args: {} }, view());
    expect(reply.id).toBe(7);
    expect(reply.ok).toBe(true);
    expect(reply.value).toMatchObject({ folder: null, count: 0 });
  });

  test("answers get_view without the eyes key for a file not judged yet", async () => {
    const reply = await respond(
      { id: 9, kind: "get_view", args: {} },
      view({ folder: "/d", files: ["/d/a"] }),
    );
    const sent = JSON.parse(JSON.stringify(reply.value)) as { current: object };
    expect(sent.current).toEqual({ path: "/d/a", position: 1 });
    expect("eyes" in sent.current).toBe(false);
  });

  test("turns an unknown request into an error reply", async () => {
    expect(await respond({ id: 8, kind: "nope", args: {} }, view())).toEqual({
      id: 8,
      ok: false,
      value: "unknown request: nope",
    });
  });
});

describe("show_photo", () => {
  test("makes a visible path current", async () => {
    const { calls, view } = driven(["/d/a", "/d/b"]);
    const state = await handleRequest("show_photo", { path: "/d/b" }, view);
    expect(calls).toEqual([["showPhoto", "/d/b"]]);
    expect(state).toMatchObject({ current: { path: "/d/b", position: 2 }, selected: ["/d/b"] });
  });

  test("refuses a path the strip does not show", async () => {
    const { calls, view } = driven(["/d/a"]);
    await expect(handleRequest("show_photo", { path: "/d/x" }, view)).rejects.toThrow(
      "/d/x is not shown in Handpic: not in the open folder, or hidden by the filter",
    );
    await expect(handleRequest("show_photo", {}, view)).rejects.toThrow("path must be a string");
    expect(calls).toEqual([]);
  });
});

describe("select_photos", () => {
  test("selects the paths without duplicates and makes the first current", async () => {
    const { calls, view } = driven(["/d/a", "/d/b", "/d/c"]);
    const state = await handleRequest("select_photos", { paths: ["/d/c", "/d/a", "/d/c"] }, view);
    expect(calls).toEqual([["selectPhotos", ["/d/c", "/d/a"]]]);
    expect(state).toMatchObject({
      current: { path: "/d/c", position: 3 },
      selected: ["/d/a", "/d/c"],
    });
  });

  test("refuses an empty list, a non-list and a hidden path", async () => {
    const { calls, view } = driven(["/d/a", "/d/b"]);
    await expect(handleRequest("select_photos", { paths: [] }, view)).rejects.toThrow(
      "paths must name at least one photo",
    );
    await expect(handleRequest("select_photos", { paths: "/d/a" }, view)).rejects.toThrow(
      "paths must be an array of strings",
    );
    await expect(handleRequest("select_photos", { paths: ["/d/a", "/d/x"] }, view)).rejects.toThrow(
      "/d/x is not shown in Handpic",
    );
    expect(calls).toEqual([]);
  });
});

describe("set_view", () => {
  test("asks for the requested mode and reports it", async () => {
    const { calls, view } = driven(["/d/a", "/d/b"]);
    expect(await handleRequest("set_view", { mode: "zoom" }, view)).toMatchObject({
      mode: "zoom",
    });
    expect(await handleRequest("set_view", { mode: "compare" }, view)).toMatchObject({
      mode: "compare",
    });
    expect(await handleRequest("set_view", { mode: "normal" }, view)).toMatchObject({
      mode: "normal",
    });
    expect(calls).toEqual([
      ["setMode", "zoom"],
      ["setMode", "compare"],
      ["setMode", "normal"],
    ]);
  });

  test("fails with the UI's message when compare cannot start", async () => {
    const { view } = driven(["/d/a"], false);
    await expect(handleRequest("set_view", { mode: "compare" }, view)).rejects.toThrow(
      COMPARE_NEEDS_FRAMES,
    );
  });

  test("refuses an unknown mode, and zoom or compare with nothing shown", async () => {
    const { calls, view } = driven([]);
    await expect(handleRequest("set_view", { mode: "grid" }, view)).rejects.toThrow(
      "mode must be one of normal, zoom, compare",
    );
    await expect(handleRequest("set_view", { mode: "zoom" }, view)).rejects.toThrow(
      "no photo is shown in Handpic",
    );
    expect(calls).toEqual([]);
  });
});

// A view over `files` whose `judge` builds the changes with `judgments`, as
// `main.ts` does, records them and applies them to the maps.
function judging(files: string[], overrides: Partial<ViewApi> = {}) {
  const changes: unknown[][] = [];
  const ratings = new Map<string, number>();
  const flags = new Map<string, "pick" | "reject">();
  const labels = new Map<string, string>();
  const state = view({
    folder: "/d",
    files,
    selection: new Set(files.slice(0, 1)),
    ratings,
    flags,
    labels,
    ...overrides,
  });
  state.judge = (paths: readonly string[], command: Command) => {
    const built = judgments(
      paths,
      paths[0],
      (path) => ({
        rating: ratings.get(path) ?? null,
        flag: flags.get(path) ?? ("none" as PickFlag),
        label: labels.get(path) ?? null,
      }),
      command,
    );
    changes.push(built);
    for (const { before, after } of built) {
      if (after.rating === null) ratings.delete(before.path);
      else ratings.set(before.path, after.rating);
      if (after.flag === "none") flags.delete(before.path);
      else flags.set(before.path, after.flag);
      if (after.label === null) labels.delete(before.path);
      else labels.set(before.path, after.label);
    }
  };
  return { changes, view: state, ratings, flags, labels };
}

describe("set_judgment", () => {
  test("changes only the given fields and reports every file", async () => {
    const { changes, view, flags, labels } = judging(["/d/a", "/d/b"]);
    flags.set("/d/a", "pick");
    labels.set("/d/b", "Red");
    expect(
      await handleRequest("set_judgment", { paths: ["/d/a", "/d/b"], rating: 3 }, view),
    ).toEqual([
      { path: "/d/a", rating: 3, flag: "pick", label: null },
      { path: "/d/b", rating: 3, flag: "none", label: "Red" },
    ]);
    expect(changes).toEqual([
      [
        {
          before: { path: "/d/a", rating: null, flag: "pick", label: null },
          after: { rating: 3, flag: "pick", label: null },
        },
        {
          before: { path: "/d/b", rating: null, flag: "none", label: "Red" },
          after: { rating: 3, flag: "none", label: "Red" },
        },
      ],
    ]);
  });

  test("clears the stars with 0 and the label with null", async () => {
    const { view, ratings, labels } = judging(["/d/a"]);
    ratings.set("/d/a", 4);
    labels.set("/d/a", "Blue");
    expect(
      await handleRequest("set_judgment", { rating: 0, flag: "reject", label: null }, view),
    ).toEqual([{ path: "/d/a", rating: null, flag: "reject", label: null }]);
  });

  test("builds no change when every file already has the value", async () => {
    const { changes, view, ratings } = judging(["/d/a"]);
    ratings.set("/d/a", 2);
    expect(await handleRequest("set_judgment", { rating: 2 }, view)).toEqual([
      { path: "/d/a", rating: 2, flag: "none", label: null },
    ]);
    expect(changes).toEqual([[]]);
  });

  test("defaults to the selection, or the active pane in compare", async () => {
    const selected = judging(["/d/a", "/d/b", "/d/c"], {
      index: 1,
      selection: new Set(["/d/b", "/d/c"]),
    });
    expect(await handleRequest("set_judgment", { flag: "pick" }, selected.view)).toMatchObject([
      { path: "/d/b" },
      { path: "/d/c" },
    ]);
    const comparing = judging(["/d/a", "/d/b"], {
      selection: new Set(["/d/a", "/d/b"]),
      comparing: true,
      compareActive: "/d/b",
    });
    expect(await handleRequest("set_judgment", { flag: "pick" }, comparing.view)).toMatchObject([
      { path: "/d/b" },
    ]);
  });

  test("refuses a hidden path and bad values without judging", async () => {
    const { changes, view } = judging(["/d/a"]);
    await expect(
      handleRequest("set_judgment", { paths: ["/d/hidden"], rating: 1 }, view),
    ).rejects.toThrow("/d/hidden is not shown in Handpic");
    await expect(handleRequest("set_judgment", { rating: 6 }, view)).rejects.toThrow(
      "rating must be an integer from 0 to 5",
    );
    await expect(handleRequest("set_judgment", { flag: "maybe" }, view)).rejects.toThrow(
      "flag must be one of none, pick, reject",
    );
    await expect(handleRequest("set_judgment", { label: "Teal" }, view)).rejects.toThrow(
      "label must be one of Red, Orange, Yellow, Green, Blue, Pink, Purple, or null",
    );
    await expect(handleRequest("set_judgment", {}, view)).rejects.toThrow(
      "set at least one of rating, flag, label",
    );
    await expect(handleRequest("set_judgment", { rating: 1 }, judging([]).view)).rejects.toThrow(
      "no photo is shown in Handpic",
    );
    expect(changes).toEqual([]);
  });

  test("refuses any judgment in a view-only folder", async () => {
    const { changes, view } = judging(["/d/a.jpg"], { viewOnly: true });
    await expect(
      handleRequest("set_judgment", { paths: ["/d/a.jpg"], rating: 3 }, view),
    ).rejects.toThrow("culling does not apply to a JPEG folder");
    await expect(handleRequest("set_judgment", { flag: "pick" }, view)).rejects.toThrow(
      "culling does not apply to a JPEG folder",
    );
    expect(changes).toEqual([]);
  });

  test("leaves the other requests working in a view-only folder", async () => {
    const view = Object.assign(driven(["/d/a.jpg", "/d/b.jpg"]).view, { viewOnly: true });
    expect(await handleRequest("get_view", {}, view)).toMatchObject({ count: 2 });
    expect(await handleRequest("show_photo", { path: "/d/b.jpg" }, view)).toMatchObject({
      current: { path: "/d/b.jpg", position: 2 },
    });
    expect(
      await handleRequest("select_photos", { paths: ["/d/a.jpg", "/d/b.jpg"] }, view),
    ).toMatchObject({ selected: ["/d/a.jpg", "/d/b.jpg"] });
    expect(await handleRequest("set_view", { mode: "zoom" }, view)).toMatchObject({
      mode: "zoom",
    });
  });
});
