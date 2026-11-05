// The folder tree's `Import…`: the dialog's text and the flow from the menu
// item through the preview to the run's end. See `crates/app/src/import.rs`
// for the payloads.

// One destination folder of a preview.
export type ImportFolder = {
  path: string;
  copyCount: number;
  copyBytes: number;
  skipped: number;
  collisions: string[];
};

// What `import_preview` returns. `id` is null, and `refusal` says why, when
// the settings cannot take an import.
export type ImportPreview = {
  id: number | null;
  source: string;
  folders: ImportFolder[];
  undated: string[];
  totals: {
    copyCount: number;
    copyBytes: number;
    skipped: number;
    collisions: number;
    undated: number;
  };
  refusal: string | null;
};

export type ImportFailure = { path: string; message: string };

// The `import-progress` payload; `file` is the name of the file in flight.
export type ImportProgress = {
  runId: number;
  filesDone: number;
  filesTotal: number;
  bytesDone: number;
  bytesTotal: number;
  file: string;
};

// The `import-done` payload: `folders` are the destination folders that got
// files to copy, in date order.
export type ImportDone = {
  runId: number;
  copied: number;
  skipped: number;
  failed: ImportFailure[];
  canceled: boolean;
  bytes: number;
  elapsedMs: number;
  folders: string[];
};

export const IMPORT_RUNNING_NOTE =
  "Cancel stops before the next file; the files copied so far stay, and the next import skips them.";

export const COLLISIONS_HEADING = "Already at the destination with another size, not copied:";

export const UNDATED_HEADING = "Without a capture date, not copied:";

export const NOTHING_TO_COPY = "Nothing left to copy: every dated file is already there.";

function files(count: number): string {
  return `${count} ${count === 1 ? "file" : "files"}`;
}

function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

// Decimal units, as the `import:` log line's MB/s.
export function sizeText(bytes: number): string {
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(2)} GB` : `${(bytes / 1e6).toFixed(1)} MB`;
}

export function totalLine(preview: ImportPreview): string {
  const { copyCount, copyBytes, skipped } = preview.totals;
  const into = preview.folders.filter((folder) => folder.copyCount > 0).length;
  const head = `${files(copyCount)} (${sizeText(copyBytes)}) to copy into ${into} ${into === 1 ? "folder" : "folders"}`;
  return skipped === 0 ? head : `${head}, ${skipped} already there`;
}

// One row per destination folder.
export function folderRows(preview: ImportPreview): string[] {
  return preview.folders.map((folder) => {
    const parts = [files(folder.copyCount), sizeText(folder.copyBytes)];
    if (folder.skipped > 0) {
      parts.push(`${folder.skipped} skipped`);
    }
    return `${folder.path}  ${parts.join(", ")}`;
  });
}

// Each colliding file with the folder it would have gone to.
export function collisionItems(preview: ImportPreview): string[] {
  return preview.folders.flatMap((folder) =>
    folder.collisions.map((name) => `${name}  in ${baseName(folder.path)}`),
  );
}

// The note under the lists: why the import is refused, or that nothing is
// left to copy; null when there is something to run.
export function importNote(preview: ImportPreview): string | null {
  if (preview.refusal !== null) {
    return preview.refusal;
  }
  return preview.totals.copyCount === 0 ? NOTHING_TO_COPY : null;
}

// A collision or an undated file is informational: the run is enabled
// whenever a file is to be copied.
export function canRun(preview: ImportPreview): boolean {
  return preview.id !== null && preview.totals.copyCount > 0;
}

export function progressStatus(done: number, total: number): string {
  return `importing ${done} / ${total}`;
}

export function failureText(failure: ImportFailure): string {
  return `${baseName(failure.path)}: ${failure.message}`;
}

export function doneStatus(done: ImportDone): string {
  const head = done.canceled
    ? `Import canceled, ${files(done.copied)} copied`
    : `Imported ${files(done.copied)} (${sizeText(done.bytes)})`;
  const parts = [head];
  if (done.skipped > 0) {
    parts.push(`${done.skipped} already there`);
  }
  if (done.failed.length > 0) {
    parts.push(`${done.failed.length} failed`);
  }
  return parts.join(", ");
}

export type Phase = "idle" | "previewing" | "previewed" | "running" | "done";

// The flow of one import, from the tree's menu item to `import-done`. The
// dialog is open while previewed and while running.
//
// `import_run` returns the run id after the run has started, so its events
// can arrive before the id is known: an `import-done` that comes first is
// kept until `started`, and a cancel asked for then is passed on by `started`.
// Progress that comes first is dropped; the next tick catches up.
export class ImportFlow {
  phase: Phase = "idle";
  dir: string | null = null;
  previewId: number | null = null;
  runId: number | null = null;
  private early: ImportDone | null = null;
  private cancelAsked = false;

  get isOpen(): boolean {
    return this.phase === "previewed" || this.phase === "running";
  }

  // True from `start()` through `done()`/`fail()`: the flow is under way even
  // while its dialog is not shown yet (previewing).
  get busy(): boolean {
    return this.phase !== "idle" && this.phase !== "done";
  }

  // False while an import is already under way; else `dir` is to be
  // previewed.
  start(dir: string): boolean {
    if (this.busy) {
      return false;
    }
    this.phase = "previewing";
    this.dir = dir;
    this.previewId = null;
    this.runId = null;
    this.early = null;
    this.cancelAsked = false;
    return true;
  }

  // `previewId` is the preview's id, null for a refused one.
  previewed(previewId: number | null): boolean {
    if (this.phase !== "previewing") {
      return false;
    }
    this.previewId = previewId;
    this.phase = "previewed";
    return true;
  }

  // The preview or `import_run` failed: the flow ends and the dialog closes.
  fail(): void {
    this.phase = "idle";
  }

  // The preview id to run, or null when there is no preview to run.
  run(): number | null {
    if (this.phase !== "previewed" || this.previewId === null) {
      return null;
    }
    this.phase = "running";
    return this.previewId;
  }

  // `import_run` returned `runId`: the `import-done` that already came, if
  // any, and whether a cancel was asked for in the meantime.
  started(runId: number): { done: ImportDone | null; cancel: boolean } {
    if (this.phase !== "running") {
      return { done: null, cancel: false };
    }
    this.runId = runId;
    const done = this.early?.runId === runId ? this.early : null;
    this.early = null;
    return { done, cancel: done === null && this.cancelAsked };
  }

  accepts(runId: number): boolean {
    return this.phase === "running" && this.runId === runId;
  }

  // True when `payload` ends this flow's run; the dialog then closes.
  done(payload: ImportDone): boolean {
    if (this.phase !== "running") {
      return false;
    }
    if (this.runId === null) {
      this.early = payload;
      return false;
    }
    if (payload.runId !== this.runId) {
      return false;
    }
    this.phase = "done";
    return true;
  }

  // The dialog's Cancel or Escape: close a preview, or cancel the run, whose
  // `import-done` closes the dialog. `cancel` carries the run id, or null
  // when it is not known yet (`started` passes the cancel on).
  dismiss(): { kind: "close" } | { kind: "cancel"; runId: number | null } | { kind: "none" } {
    switch (this.phase) {
      case "previewed":
        this.phase = "idle";
        return { kind: "close" };
      case "running":
        if (this.runId === null) {
          this.cancelAsked = true;
        }
        return { kind: "cancel", runId: this.runId };
      default:
        return { kind: "none" };
    }
  }
}
