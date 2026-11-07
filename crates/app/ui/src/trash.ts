import { relation } from "./tree.js";

// The strip's `Move to Trash…` and `Delete Permanently…`: the confirmation
// dialog's text and flow, what the status line says during the run and once it
// comes back, and whether the open folder was among the folders it ran in.

export type TrashFailure = { path: string; message: string };

// The `Summary` that `trash_files` and `delete_files` return: the RAWs (or
// JPEGs) that went, which the frontend prunes its state by, every file that
// failed, and the recorded run `trash_rejected_undo` takes back (null when
// nothing moved, and always for a delete, which nothing takes back).
export type TrashSummary = {
  moved: string[];
  failed: TrashFailure[];
  run_id: number | null;
};

// What `trash_rejected_undo` returns: the RAWs and JPEGs that came back, and every
// file (RAW or sidecar) that did not, with the reason. The backend keeps
// what came back, sidecars included, for `trash_rejected_redo`.
export type TrashRestored = {
  restored: string[];
  failed: TrashFailure[];
};

function files(count: number): string {
  return count === 1 ? "file" : "files";
}

// What a recorded run moved: RAWs with their sidecars or JPEGs (`Move to
// Trash…`), or sidecars alone (`Delete Sidecars…`).
export type TrashWhat = "files" | "sidecars";

function noun(count: number, what: TrashWhat): string {
  return what === "sidecars" ? (count === 1 ? "sidecar" : "sidecars") : files(count);
}

// A folder as the user knows it: without the verbatim `\\?\` prefix the
// backend's canonical spelling carries on Windows.
export function shownPath(dir: string): string {
  if (dir.startsWith("\\\\?\\UNC\\")) {
    return `\\\\${dir.slice(8)}`;
  }
  return dir.startsWith("\\\\?\\") ? dir.slice(4) : dir;
}

// The strip's `Move to Trash…` asks with the count alone.
export function filesLine(count: number): string {
  return `Move ${count} ${files(count)} to the Trash?`;
}

// The strip's `Delete Permanently…` asks in the plainest words: this one has no
// way back.
function deleteLine(count: number): string {
  return count === 1
    ? "Permanently delete 1 file and its sidecars? It is not moved to the Trash and this cannot be undone."
    : `Permanently delete ${count} files and their sidecars? They are not moved to the Trash and this cannot be undone.`;
}

// The operation a `trash-progress` event reports on, and what the dialog asks.
export type TrashOp = "trash" | "delete";

export function dialogText(
  mode: TrashOp,
  count: number,
): { title: string; line: string; run: string } {
  return mode === "delete"
    ? { title: "Delete Permanently", line: deleteLine(count), run: "Delete Permanently" }
    : { title: "Move to Trash", line: filesLine(count), run: "Move to Trash" };
}

// The keys the delete dialog swallows, so only a mouse click confirms: Enter
// and Space, with or without a modifier, click the focused button, and a repeat
// is a key still held from before the dialog opened.
export function swallowedByDelete(event: { key: string; repeat: boolean }): boolean {
  return event.repeat || event.key === "Enter" || event.key === " ";
}

export type TrashProgress = { op: TrashOp; dir: string; done: number; total: number };

const PROGRESS_LABEL: Record<TrashOp, string> = { trash: "moving to Trash", delete: "deleting" };

export function progressStatus(op: TrashOp, done: number, total: number): string {
  return `${PROGRESS_LABEL[op]} ${done} / ${total}`;
}

export function trashedStatus(
  summary: TrashSummary,
  what: TrashWhat = "files",
  mode: TrashOp = "trash",
): string {
  const count = summary.moved.length;
  const went =
    mode === "delete"
      ? `Deleted ${count} ${noun(count, what)}`
      : `Moved ${count} ${noun(count, what)} to the Trash`;
  return summary.failed.length === 0 ? went : `${went}, ${summary.failed.length} failed`;
}

// The run an undo restored: what it moved and how many files. Its `count`
// is read only for sidecars, since `restored` lists the RAWs and JPEGs that
// came back, never a sidecar.
export type UndoneRun = { what?: TrashWhat; count: number };

// How many of the run's files came back: the RAWs and JPEGs `restored`
// lists, or, for
// a run of sidecars, those that did not fail.
export function restoredCount(result: TrashRestored, run?: UndoneRun): number {
  if (run?.what === "sidecars") {
    return Math.max(0, run.count - result.failed.length);
  }
  return result.restored.length;
}

export function restoredStatus(result: TrashRestored, run?: UndoneRun): string {
  const count = restoredCount(result, run);
  const restored = `Restored ${count} ${noun(count, run?.what ?? "files")} from the Trash`;
  return result.failed.length === 0 ? restored : `${restored}, ${result.failed.length} failed`;
}

// The restored files that sit directly in `openDir`, not yet in `allFiles`.
// The backend spells them canonically (with the verbatim prefix on Windows),
// as `list_arw` spells `allFiles`, while `openDir` may lack the prefix.
export function restoredInto(
  openDir: string,
  restored: string[],
  allFiles: string[],
  ignoreCase = false,
): string[] {
  return restored.filter(
    (path) =>
      relation(shownPath(path.replace(/[\\/][^\\/]*$/, "")), shownPath(openDir), ignoreCase) ===
        "same" && !allFiles.includes(path),
  );
}

// The backend's refusals of a run it no longer holds, to undo or to redo;
// any other failure (a scan that slipped in) leaves the run undoable or
// redoable later.
const RUN_GONE = ["this run can no longer be undone", "this run can no longer be redone"];

export function stillHeld(err: string): boolean {
  return !RUN_GONE.includes(err);
}

// Whether `openDir` is one of `dirs`.
export function opensTarget(openDir: string, dirs: string[], ignoreCase = false): boolean {
  return dirs.some((dir) => relation(openDir, dir, ignoreCase) === "same");
}

// The files of the folder `dir` the strip's `Move to Trash…` moves, or its
// `Delete Permanently…` deletes.
export type TrashTarget = { dir: string; paths: string[]; mode: TrashOp };

// The flow of one trashing or delete, from the menu item through the dialog to
// the run's end. A second start while one is under way does nothing. The dialog
// is open while asking and while running; a running move cannot be
// canceled.
export class TrashFlow {
  phase: "idle" | "asking" | "running" = "idle";
  target: TrashTarget | null = null;

  get busy(): boolean {
    return this.phase !== "idle";
  }

  start(target: TrashTarget): boolean {
    if (this.phase !== "idle") {
      return false;
    }
    this.phase = "asking";
    this.target = target;
    return true;
  }

  // The files to move, or null when nothing is being asked.
  run(): TrashTarget | null {
    if (this.phase !== "asking") {
      return null;
    }
    this.phase = "running";
    return this.target;
  }

  // True when the dialog should close: Cancel or Escape on the question.
  dismiss(): boolean {
    if (this.phase !== "asking") {
      return false;
    }
    this.end();
    return true;
  }

  // The run came back.
  end(): void {
    this.phase = "idle";
    this.target = null;
  }
}
