import { describe, expect, test } from "vitest";
import {
  type ImportDone,
  ImportFlow,
  type ImportPreview,
  NOTHING_NEW,
  NOTHING_TO_COPY,
  NO_PHOTOS,
  canRun,
  collisionItems,
  doneStatus,
  failureText,
  folderRows,
  importNote,
  progressStatus,
  sizeText,
  totalLine,
} from "./import.js";

const preview = (over: Partial<ImportPreview> = {}): ImportPreview => ({
  id: 1,
  source: "E:\\",
  folders: [
    {
      path: "D:\\Photos\\2026\\2026-10-24",
      copyCount: 2,
      copyBytes: 88_000_000,
      skipped: 0,
      collisions: [],
    },
    {
      path: "D:\\Photos\\2026\\2026-10-25",
      copyCount: 1,
      copyBytes: 44_000_000,
      skipped: 3,
      collisions: ["DSC00009.ARW"],
    },
  ],
  undated: [],
  totals: { copyCount: 3, copyBytes: 132_000_000, skipped: 3, collisions: 1, undated: 0 },
  refusal: null,
  ...over,
});

const nothing = (): ImportPreview =>
  preview({
    folders: [
      {
        path: "D:\\Photos\\2026\\2026-10-24",
        copyCount: 0,
        copyBytes: 0,
        skipped: 2,
        collisions: [],
      },
    ],
    totals: { copyCount: 0, copyBytes: 0, skipped: 2, collisions: 0, undated: 0 },
  });

const done = (over: Partial<ImportDone> = {}): ImportDone => ({
  runId: 1,
  copied: 3,
  skipped: 0,
  failed: [],
  canceled: false,
  bytes: 132_000_000,
  elapsedMs: 5000,
  folders: ["D:\\Photos\\2026\\2026-10-24"],
  ...over,
});

describe("text", () => {
  test("sizes in decimal units", () => {
    expect(sizeText(0)).toBe("0.0 MB");
    expect(sizeText(44_000_000)).toBe("44.0 MB");
    expect(sizeText(2_500_000_000)).toBe("2.50 GB");
  });

  test("the total line counts the folders that get files", () => {
    expect(totalLine(preview())).toBe("3 files (132.0 MB) to copy into 2 folders, 3 already there");
    expect(totalLine(nothing())).toBe("0 files (0.0 MB) to copy into 0 folders, 2 already there");
  });

  test("a row per destination folder", () => {
    expect(folderRows(preview())).toEqual([
      "D:\\Photos\\2026\\2026-10-24  2 files, 88.0 MB",
      "D:\\Photos\\2026\\2026-10-25  1 file, 44.0 MB, 3 skipped",
    ]);
  });

  test("each collision names its folder", () => {
    expect(collisionItems(preview())).toEqual(["DSC00009.ARW  in 2026-10-25"]);
  });

  test("the note: a refusal, nothing to copy, or none", () => {
    expect(importNote(preview())).toBeNull();
    expect(importNote(nothing())).toBe(NOTHING_TO_COPY);
    expect(importNote(preview({ id: null, refusal: "not set" }))).toBe("not set");
  });

  test("the note: no photos under the folder", () => {
    const empty = preview({
      folders: [],
      totals: { copyCount: 0, copyBytes: 0, skipped: 0, collisions: 0, undated: 0 },
    });
    expect(importNote(empty)).toBe(NO_PHOTOS);
  });

  test("the note: collisions or undated files are not 'already there'", () => {
    const collided = preview({
      folders: [
        {
          path: "D:\\Photos\\2026\\2026-10-24",
          copyCount: 0,
          copyBytes: 0,
          skipped: 0,
          collisions: ["DSC00001.ARW"],
        },
      ],
      totals: { copyCount: 0, copyBytes: 0, skipped: 0, collisions: 1, undated: 0 },
    });
    expect(importNote(collided)).toBe(NOTHING_NEW);
    const undated = preview({
      folders: [],
      undated: ["a.ARW"],
      totals: { copyCount: 0, copyBytes: 0, skipped: 0, collisions: 0, undated: 1 },
    });
    expect(importNote(undated)).toBe(NOTHING_NEW);
  });

  test("runs whenever a file is to be copied", () => {
    expect(canRun(preview())).toBe(true);
    expect(canRun(preview({ undated: ["a.ARW"] }))).toBe(true);
    expect(canRun(nothing())).toBe(false);
    expect(canRun(preview({ id: null, refusal: "not set" }))).toBe(false);
  });

  test("progress", () => {
    expect(progressStatus(2, 5)).toBe("importing 2 / 5");
  });

  test("a failure names the file", () => {
    expect(failureText({ path: "E:\\DCIM\\DSC00001.ARW", message: "denied" })).toBe(
      "DSC00001.ARW: denied",
    );
  });

  test("the done status", () => {
    expect(doneStatus(done())).toBe("Imported 3 files (132.0 MB)");
    expect(
      doneStatus(
        done({ copied: 1, skipped: 2, failed: [{ path: "/a.ARW", message: "x" }], bytes: 1e6 }),
      ),
    ).toBe("Imported 1 file (1.0 MB), 2 already there, 1 failed");
    expect(doneStatus(done({ copied: 1, canceled: true }))).toBe("Import canceled, 1 file copied");
  });
});

function running(): ImportFlow {
  const flow = new ImportFlow();
  flow.start("E:\\");
  flow.previewed(7);
  flow.run();
  return flow;
}

describe("ImportFlow", () => {
  test("walks previewing, previewed, running and done", () => {
    const flow = new ImportFlow();
    expect(flow.start("E:\\")).toBe(true);
    expect(flow.phase).toBe("previewing");
    expect(flow.busy).toBe(true);
    expect(flow.isOpen).toBe(false);
    expect(flow.previewed(7)).toBe(true);
    expect(flow.isOpen).toBe(true);
    expect(flow.run()).toBe(7);
    expect(flow.phase).toBe("running");
    expect(flow.isOpen).toBe(true);
    expect(flow.started(1)).toEqual({ done: null, cancel: false });
    expect(flow.isOpen).toBe(false);
    expect(flow.busy).toBe(true);
    expect(flow.accepts(1)).toBe(true);
    expect(flow.done(done())).toBe(true);
    expect(flow.phase).toBe("done");
    expect(flow.isOpen).toBe(false);
    expect(flow.busy).toBe(false);
    expect(flow.start("E:\\")).toBe(true);
  });

  test("does not start again while under way", () => {
    const flow = new ImportFlow();
    flow.start("E:\\");
    expect(flow.start("F:\\")).toBe(false);
    expect(running().start("F:\\")).toBe(false);
  });

  test("a refused preview has nothing to run", () => {
    const flow = new ImportFlow();
    flow.start("E:\\");
    flow.previewed(null);
    expect(flow.isOpen).toBe(true);
    expect(flow.run()).toBeNull();
    expect(flow.dismiss()).toEqual({ kind: "close" });
    expect(flow.busy).toBe(false);
  });

  test("a failed preview ends the flow", () => {
    const flow = new ImportFlow();
    flow.start("E:\\");
    flow.fail();
    expect(flow.phase).toBe("idle");
    expect(flow.previewed(1)).toBe(false);
  });

  test("a started run's dialog steps aside and comes back to cancel", () => {
    const flow = running();
    flow.started(4);
    expect(flow.isOpen).toBe(false);
    expect(flow.reopen()).toBe(true);
    expect(flow.isOpen).toBe(true);
    expect(flow.reopen()).toBe(false);
    expect(flow.dismiss()).toEqual({ kind: "cancel", runId: 4 });
    expect(flow.isOpen).toBe(true);
    expect(flow.done(done({ runId: 4, canceled: true }))).toBe(true);
    expect(flow.isOpen).toBe(false);
    expect(flow.reopen()).toBe(false);
  });

  test("only a run under way with its dialog aside reopens", () => {
    const flow = new ImportFlow();
    expect(flow.reopen()).toBe(false);
    flow.start("E:\\");
    expect(flow.reopen()).toBe(false);
    flow.previewed(7);
    expect(flow.reopen()).toBe(false);
    flow.run();
    expect(flow.reopen()).toBe(false);
    flow.started(1);
    expect(flow.reopen()).toBe(true);
  });

  test("ignores the events of another run", () => {
    const flow = running();
    flow.started(4);
    expect(flow.accepts(3)).toBe(false);
    expect(flow.done(done({ runId: 3 }))).toBe(false);
    expect(flow.phase).toBe("running");
  });

  test("a done that comes before the run id is kept until started", () => {
    const flow = running();
    expect(flow.accepts(1)).toBe(false);
    expect(flow.done(done())).toBe(false);
    expect(flow.started(1)).toEqual({ done: done(), cancel: false });
  });

  test("a cancel asked for before the run id is passed on by started", () => {
    const flow = running();
    expect(flow.dismiss()).toEqual({ kind: "cancel", runId: null });
    expect(flow.started(1)).toEqual({ done: null, cancel: true });
    expect(flow.isOpen).toBe(true);
  });
});
