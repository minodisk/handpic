import { describe, expect, test } from "vitest";
import {
  DateFolderFlow,
  type DatePreview,
  dateRows,
  dateTitleLine,
  disabledReason,
  parentOf,
  removalNote,
  splitStatus,
  undatedLine,
} from "./datefolder.js";

const rename: DatePreview = {
  kind: "rename",
  name: "100MSDCF",
  targets: [{ name: "2026-10-31", count: 12 }],
  undated: [],
  collisions: [],
};

const split: DatePreview = {
  kind: "split",
  name: "100MSDCF",
  targets: [
    { name: "2026-10-31", count: 1 },
    { name: "2026-11-01", count: 4 },
  ],
  undated: ["DSC00001.ARW", "DSC00009.ARW"],
  collisions: [],
};

describe("dateTitleLine", () => {
  test("names the one date of a rename", () => {
    expect(dateTitleLine(rename)).toBe("Rename 100MSDCF to 2026-10-31?");
  });

  test("counts the folders of a split", () => {
    expect(dateTitleLine(split)).toBe("Split 100MSDCF into 2 folders by capture date?");
  });

  test("says a folder is already named its date", () => {
    const named = { ...rename, kind: "named" as const, name: "2026-10-31" };
    expect(dateTitleLine(named)).toBe("2026-10-31 is already named its capture date.");
  });
});

describe("dateRows", () => {
  test("gives one row per target with its file count", () => {
    expect(dateRows(rename)).toEqual(["2026-10-31  12 files"]);
    expect(dateRows(split)).toEqual(["2026-10-31  1 file", "2026-11-01  4 files"]);
  });
});

describe("undatedLine", () => {
  test("lists the undated files that stay in a split", () => {
    expect(undatedLine(split)).toBe(
      "Without a capture date, 2 files stay in 100MSDCF: DSC00001.ARW, DSC00009.ARW",
    );
  });

  test("says the undated files come along with a rename", () => {
    expect(undatedLine({ ...rename, undated: ["a.ARW"] })).toBe(
      "Without a capture date, 1 file comes along: a.ARW",
    );
  });

  test("is null when every file is dated", () => {
    expect(undatedLine(rename)).toBeNull();
  });
});

describe("removalNote", () => {
  test("says a split's original folder is removed once empty", () => {
    expect(removalNote(split)).toBe("100MSDCF is removed if it ends up empty.");
    expect(removalNote(rename)).toBeNull();
  });
});

describe("disabledReason", () => {
  test("names the colliding targets", () => {
    expect(disabledReason({ ...rename, collisions: ["2026-10-31"] })).toBe(
      "2026-10-31 already exists next to 100MSDCF; nothing is merged.",
    );
    expect(disabledReason({ ...split, collisions: ["2026-10-31", "2026-11-01"] })).toBe(
      "2026-10-31, 2026-11-01 already exist next to 100MSDCF; nothing is merged.",
    );
  });

  test("disables an already named folder", () => {
    expect(disabledReason({ ...rename, kind: "named" })).toBe("Nothing to rename.");
  });

  test("is null when the run can go", () => {
    expect(disabledReason(rename)).toBeNull();
    expect(disabledReason(split)).toBeNull();
  });
});

describe("splitStatus", () => {
  const moved = [
    { name: "2026-10-31", path: "/p/2026-10-31", count: 1 },
    { name: "2026-11-01", path: "/p/2026-11-01", count: 4 },
  ];

  test("names the date folders of a clean split", () => {
    const ran = {
      kind: "split" as const,
      moved,
      failed: [],
      left: [],
      removed: true,
      warning: null,
    };
    expect(splitStatus(ran, "100MSDCF")).toBe("Split 100MSDCF into 2026-10-31 (1), 2026-11-01 (4)");
  });

  test("adds the failures, the folder kept and the warning", () => {
    const ran = {
      kind: "split" as const,
      moved,
      failed: [{ path: "/p/100MSDCF/a.ARW", message: "denied" }],
      left: ["a.ARW", "b.ARW"],
      removed: false,
      warning: "cache not moved",
    };
    expect(splitStatus(ran, "100MSDCF")).toBe(
      "Split 100MSDCF into 2026-10-31 (1), 2026-11-01 (4); 1 file failed; 100MSDCF kept (2 left); cache not moved",
    );
  });

  test("says when nothing moved", () => {
    const ran = {
      kind: "split" as const,
      moved: [],
      failed: [{ path: "/p/a/x.ARW", message: "denied" }],
      left: ["x.ARW"],
      removed: false,
      warning: null,
    };
    expect(splitStatus(ran, "a")).toBe("Moved nothing out of a; 1 file failed; a kept (1 left)");
  });
});

describe("parentOf", () => {
  test("drops the last component", () => {
    expect(parentOf("/home/me/100MSDCF")).toBe("/home/me");
    expect(parentOf("D:\\photos\\100MSDCF")).toBe("D:\\photos");
  });

  test("keeps a root's separator", () => {
    expect(parentOf("/100MSDCF")).toBe("/");
    expect(parentOf("E:\\DCIM")).toBe("E:\\");
  });
});

describe("DateFolderFlow", () => {
  test("goes from start through the preview to the run", () => {
    const flow = new DateFolderFlow();
    expect(flow.start("/p/a")).toBe(true);
    expect(flow.busy).toBe(true);
    expect(flow.isOpen).toBe(false);
    expect(flow.start("/p/b")).toBe(false);
    expect(flow.run()).toBeNull();
    expect(flow.previewed()).toBe(true);
    expect(flow.isOpen).toBe(true);
    expect(flow.run()).toBe("/p/a");
    expect(flow.dismiss()).toBe(false);
    flow.end();
    expect(flow.busy).toBe(false);
  });

  test("dismisses a preview but not a running move", () => {
    const flow = new DateFolderFlow();
    flow.start("/p/a");
    flow.previewed();
    expect(flow.dismiss()).toBe(true);
    expect(flow.busy).toBe(false);
  });
});
