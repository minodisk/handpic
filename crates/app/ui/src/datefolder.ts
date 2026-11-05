// The folder tree's `Rename by Capture Date…`: the confirmation dialog's text
// and the flow from the menu item through the dialog to the run's end. See
// `crates/app/src/datefolder.rs` for the payloads.

// One target folder and how many files go into it.
export type DateTarget = { name: string; count: number };

// What `date_folder_preview` returns: a rename to the one date, a split into
// one sibling folder per date, or a folder already named its date; the
// folder's current name, the targets in date order, the undated files' names
// and the target names that already exist (any of which disables the run).
export type DatePreview = {
  kind: "rename" | "split" | "named";
  name: string;
  targets: DateTarget[];
  undated: string[];
  collisions: string[];
};

// One date folder a split moved files into.
export type DateMoved = { name: string; path: string; count: number };

// What `date_folder_run` returns: `rename_folder`'s result, or a split's
// date folders (only those that received files, in date order), the files
// that failed to move, the names left in the original folder and whether it
// was removed.
export type DateRan =
  | { kind: "renamed"; path: string; warning: string | null }
  | {
      kind: "split";
      moved: DateMoved[];
      failed: { path: string; message: string }[];
      left: string[];
      removed: boolean;
      warning: string | null;
    };

export const DATE_RUNNING_NOTE = "Moving the files; this closes once every file is moved.";

function files(count: number): string {
  return `${count} ${count === 1 ? "file" : "files"}`;
}

export function dateTitleLine(preview: DatePreview): string {
  const { kind, name, targets } = preview;
  switch (kind) {
    case "rename":
      return `Rename ${name} to ${targets[0]?.name ?? ""}?`;
    case "split":
      return `Split ${name} into ${targets.length} folders by capture date?`;
    case "named":
      return `${name} is already named its capture date.`;
  }
}

// One row per target folder with its file count.
export function dateRows(preview: DatePreview): string[] {
  return preview.targets.map(({ name, count }) => `${name}  ${files(count)}`);
}

// The undated files: they come along with a rename and stay in a split.
// Null when there are none.
export function undatedLine(preview: DatePreview): string | null {
  const { kind, name, undated } = preview;
  if (undated.length === 0) {
    return null;
  }
  const one = undated.length === 1;
  const where =
    kind === "split" ? `${one ? "stays" : "stay"} in ${name}` : `${one ? "comes" : "come"} along`;
  return `Without a capture date, ${files(undated.length)} ${where}: ${undated.join(", ")}`;
}

export function removalNote(preview: DatePreview): string | null {
  return preview.kind === "split" ? `${preview.name} is removed if it ends up empty.` : null;
}

// Why the run is disabled, or null when it can run.
export function disabledReason(preview: DatePreview): string | null {
  const { collisions } = preview;
  if (collisions.length > 0) {
    const verb = collisions.length === 1 ? "exists" : "exist";
    return `${collisions.join(", ")} already ${verb} next to ${preview.name}; nothing is merged.`;
  }
  if (preview.kind === "named") {
    return "Nothing to rename.";
  }
  return null;
}

// The status line after a split: the folders the files went to, and what
// failed or was left behind.
export function splitStatus(ran: Extract<DateRan, { kind: "split" }>, name: string): string {
  const into = ran.moved.map((moved) => `${moved.name} (${moved.count})`).join(", ");
  const parts = [
    ran.moved.length === 0 ? `Moved nothing out of ${name}` : `Split ${name} into ${into}`,
  ];
  if (ran.failed.length > 0) {
    parts.push(`${files(ran.failed.length)} failed`);
  }
  if (!ran.removed) {
    parts.push(`${name} kept (${ran.left.length} left)`);
  }
  if (ran.warning !== null) {
    parts.push(ran.warning);
  }
  return parts.join("; ");
}

// The folder `path` is in, as the tree keys it (a root keeps its separator).
export function parentOf(path: string): string {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (cut <= 0) {
    return path.slice(0, cut + 1) || path;
  }
  const parent = path.slice(0, cut);
  return /^[A-Za-z]:$/.test(parent) ? `${parent}${path[cut]}` : parent;
}

// The flow of one date rename or split, from the menu item through the
// dialog to the run's end. A second start while one is under way does
// nothing. The dialog is open while previewed and while running; a running
// move cannot be canceled.
export class DateFolderFlow {
  phase: "idle" | "previewing" | "previewed" | "running" = "idle";
  dir: string | null = null;

  get isOpen(): boolean {
    return this.phase === "previewed" || this.phase === "running";
  }

  get busy(): boolean {
    return this.phase !== "idle";
  }

  start(dir: string): boolean {
    if (this.phase !== "idle") {
      return false;
    }
    this.phase = "previewing";
    this.dir = dir;
    return true;
  }

  previewed(): boolean {
    if (this.phase !== "previewing") {
      return false;
    }
    this.phase = "previewed";
    return true;
  }

  // The folder to run on, or null when there is no preview to run.
  run(): string | null {
    if (this.phase !== "previewed") {
      return null;
    }
    this.phase = "running";
    return this.dir;
  }

  // True when the dialog should close: Cancel or Escape on a preview.
  dismiss(): boolean {
    if (this.phase !== "previewed") {
      return false;
    }
    this.end();
    return true;
  }

  end(): void {
    this.phase = "idle";
    this.dir = null;
  }
}
