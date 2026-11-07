import { describe, expect, test } from "vitest";
import {
  TrashFlow,
  dialogText,
  filesLine,
  opensTarget,
  progressStatus,
  restoredInto,
  restoredStatus,
  shownPath,
  stillHeld,
  swallowedByDelete,
  trashedStatus,
} from "./trash.js";

describe("progressStatus", () => {
  test("counts the move", () => {
    expect(progressStatus("trash", 0, 2677)).toBe("moving to Trash 0 / 2677");
    expect(progressStatus("trash", 12, 2677)).toBe("moving to Trash 12 / 2677");
  });

  test("counts the delete", () => {
    expect(progressStatus("delete", 12, 2677)).toBe("deleting 12 / 2677");
  });
});

describe("trashedStatus", () => {
  test("counts the files it moved", () => {
    expect(trashedStatus({ moved: ["/a.ARW"], failed: [], run_id: null })).toBe(
      "Moved 1 file to the Trash",
    );
    expect(
      trashedStatus({
        moved: ["/a.ARW", "/b.ARW", "/c.ARW"],
        failed: [],
        run_id: null,
      }),
    ).toBe("Moved 3 files to the Trash");
  });

  test("appends the failures", () => {
    expect(
      trashedStatus({
        moved: ["/b.ARW", "/c.ARW"],
        failed: [{ path: "/a.ARW", message: "denied" }],
        run_id: null,
      }),
    ).toBe("Moved 2 files to the Trash, 1 failed");
  });
});

describe("sidecar runs", () => {
  test("trashedStatus counts the sidecars moved", () => {
    const summary = { moved: ["/a.xmp", "/b.xmp"], failed: [], run_id: 1 };
    expect(trashedStatus(summary, "sidecars")).toBe("Moved 2 sidecars to the Trash");
    expect(trashedStatus({ ...summary, moved: ["/a.xmp"] }, "sidecars")).toBe(
      "Moved 1 sidecar to the Trash",
    );
    expect(
      trashedStatus(
        { ...summary, failed: [{ path: "/c.ARW", message: "c.xmp: denied" }] },
        "sidecars",
      ),
    ).toBe("Moved 2 sidecars to the Trash, 1 failed");
  });

  test("restoredStatus counts the sidecars that did not fail", () => {
    const run = { what: "sidecars" as const, count: 3 };
    expect(restoredStatus({ restored: [], failed: [] }, run)).toBe(
      "Restored 3 sidecars from the Trash",
    );
    expect(
      restoredStatus({ restored: [], failed: [{ path: "/a.xmp", message: "gone" }] }, run),
    ).toBe("Restored 2 sidecars from the Trash, 1 failed");
    expect(restoredStatus({ restored: [], failed: [] }, { ...run, count: 1 })).toBe(
      "Restored 1 sidecar from the Trash",
    );
  });

  test("restoredStatus of a run of RAWs counts the RAWs whatever the count", () => {
    expect(restoredStatus({ restored: ["/a.ARW"], failed: [] }, { count: 5 })).toBe(
      "Restored 1 file from the Trash",
    );
  });
});

describe("restoredStatus", () => {
  test("counts the files that came back", () => {
    expect(restoredStatus({ restored: ["/a.ARW"], failed: [] })).toBe(
      "Restored 1 file from the Trash",
    );
    expect(restoredStatus({ restored: ["/a.ARW", "/b.ARW"], failed: [] })).toBe(
      "Restored 2 files from the Trash",
    );
  });

  test("appends the failures", () => {
    expect(
      restoredStatus({
        restored: ["/b.ARW"],
        failed: [
          { path: "/a.ARW", message: "already exists at the original location" },
          { path: "/a.xmp", message: "left in the Trash: its RAW could not be restored" },
        ],
      }),
    ).toBe("Restored 1 file from the Trash, 2 failed");
  });
});

describe("restoredInto", () => {
  test("keeps the RAWs directly in the open folder that are not listed yet", () => {
    expect(
      restoredInto(
        "/photos/a",
        ["/photos/a/1.ARW", "/photos/a/2.ARW", "/photos/a/sub/3.ARW", "/photos/b/4.ARW"],
        ["/photos/a/2.ARW"],
      ),
    ).toEqual(["/photos/a/1.ARW"]);
  });

  test("compares Windows paths the way the folder tree does", () => {
    expect(restoredInto("C:\\photos", ["c:\\photos\\1.ARW"], [])).toEqual(["c:\\photos\\1.ARW"]);
  });

  test("matches a canonical path against an open folder spelled without the verbatim prefix", () => {
    expect(restoredInto("C:\\photos", ["\\\\?\\C:\\photos\\1.ARW"], [])).toEqual([
      "\\\\?\\C:\\photos\\1.ARW",
    ]);
    expect(
      restoredInto("\\\\?\\C:\\photos", ["\\\\?\\C:\\photos\\1.ARW"], ["\\\\?\\C:\\photos\\1.ARW"]),
    ).toEqual([]);
  });

  test("matches an open folder spelled in another case only when the case is ignored", () => {
    expect(restoredInto("c:\\Photos", ["C:\\photos\\1.ARW"], [], true)).toEqual([
      "C:\\photos\\1.ARW",
    ]);
    expect(restoredInto("c:\\Photos", ["C:\\photos\\1.ARW"], [])).toEqual([]);
  });
});

describe("stillHeld", () => {
  test("keeps a run the backend refused while a scan ran", () => {
    expect(stillHeld("a scan is running; wait for it to finish")).toBe(true);
  });

  test("drops a run the backend no longer holds, to undo or to redo", () => {
    expect(stillHeld("this run can no longer be undone")).toBe(false);
    expect(stillHeld("this run can no longer be redone")).toBe(false);
  });
});

describe("opensTarget", () => {
  test("holds for a target equal to the open folder, however it is spelled", () => {
    expect(opensTarget("/photos/2026", ["/other", "/photos/2026"])).toBe(true);
    expect(opensTarget("C:\\Photos\\2026", ["c:/Photos/2026/"])).toBe(true);
  });

  test("does not hold for an unrelated folder or a sibling sharing a prefix", () => {
    expect(opensTarget("/photos/2025", ["/photos/2026"])).toBe(false);
    expect(opensTarget("/photos/2026b", ["/photos/2026"])).toBe(false);
  });

  test("does not hold for a folder above or below a target", () => {
    expect(opensTarget("/photos", ["/photos/2026"])).toBe(false);
    expect(opensTarget("/photos/2026/0101", ["/photos/2026"])).toBe(false);
  });

  test("holds for an open folder spelled in another case only when the case is ignored", () => {
    expect(opensTarget("d:\\photos\\2026", ["D:\\Photos\\2026"], true)).toBe(true);
    expect(opensTarget("d:\\photos\\2026", ["D:\\Photos\\2026"])).toBe(false);
  });
});

describe("the confirmation dialog", () => {
  test("shows a Windows folder without the verbatim prefix", () => {
    expect(shownPath("\\\\?\\C:\\Photos\\2026")).toBe("C:\\Photos\\2026");
    expect(shownPath("\\\\?\\UNC\\nas\\photos")).toBe("\\\\nas\\photos");
    expect(shownPath("/photos/2026")).toBe("/photos/2026");
  });

  test("asks with the count alone", () => {
    expect(filesLine(1)).toBe("Move 1 file to the Trash?");
    expect(filesLine(12)).toBe("Move 12 files to the Trash?");
  });
});

describe("trashedStatus of a delete", () => {
  test("says the files were deleted, and how many failed", () => {
    const summary = { moved: ["/a.ARW", "/b.ARW"], failed: [], run_id: null };
    expect(trashedStatus(summary, "files", "delete")).toBe("Deleted 2 files");
    expect(trashedStatus({ ...summary, moved: ["/a.ARW"] }, "files", "delete")).toBe(
      "Deleted 1 file",
    );
    expect(
      trashedStatus(
        { ...summary, failed: [{ path: "/c.ARW", message: "locked" }] },
        "files",
        "delete",
      ),
    ).toBe("Deleted 2 files, 1 failed");
  });
});

describe("dialogText", () => {
  test("asks a move with the count alone", () => {
    expect(dialogText("trash", 3)).toEqual({
      title: "Move to Trash",
      line: "Move 3 files to the Trash?",
      run: "Move to Trash",
    });
  });

  test("says a delete is permanent, singular for one file", () => {
    expect(dialogText("delete", 3)).toEqual({
      title: "Delete Permanently",
      line: "Permanently delete 3 files and their sidecars? They are not moved to the Trash and this cannot be undone.",
      run: "Delete Permanently",
    });
    expect(dialogText("delete", 1).line).toBe(
      "Permanently delete 1 file and its sidecars? It is not moved to the Trash and this cannot be undone.",
    );
  });
});

describe("swallowedByDelete", () => {
  test("takes the keys that click a button and any repeat", () => {
    expect(swallowedByDelete({ key: "Enter", repeat: false })).toBe(true);
    expect(swallowedByDelete({ key: " ", repeat: false })).toBe(true);
    expect(swallowedByDelete({ key: "Escape", repeat: true })).toBe(true);
    expect(swallowedByDelete({ key: "Tab", repeat: true })).toBe(true);
  });

  test("leaves Escape and Tab to the dialog", () => {
    expect(swallowedByDelete({ key: "Escape", repeat: false })).toBe(false);
    expect(swallowedByDelete({ key: "Tab", repeat: false })).toBe(false);
  });
});

describe("TrashFlow", () => {
  const target = { dir: "/photos", paths: ["/photos/a.ARW"], mode: "trash" as const };

  test("refuses a second start while one is under way", () => {
    const flow = new TrashFlow();
    expect(flow.start(target)).toBe(true);
    expect(flow.start(target)).toBe(false);
    expect(flow.busy).toBe(true);
    expect(flow.run()).toEqual(target);
    expect(flow.start(target)).toBe(false);
    flow.end();
    expect(flow.busy).toBe(false);
    expect(flow.start(target)).toBe(true);
  });

  test("closes the question on dismiss but not a running move", () => {
    const flow = new TrashFlow();
    flow.start(target);
    expect(flow.dismiss()).toBe(true);
    expect(flow.run()).toBeNull();
    flow.start(target);
    flow.run();
    expect(flow.dismiss()).toBe(false);
    expect(flow.busy).toBe(true);
  });
});
