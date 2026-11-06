// Whether a `scan-progress` tick should re-read the folder's rows. Only while
// the current file's row is missing, and then only when this tick committed
// it, or when the current file is not the one the last progress-triggered
// refresh was for (a file paged to after its row landed still gets it).
export function refreshOnProgress(
  current: string | undefined,
  hasRow: boolean,
  ready: readonly string[],
  lastRefreshedFor: string | null,
): boolean {
  if (current === undefined || hasRow) {
    return false;
  }
  return ready.includes(current) || current !== lastRefreshedFor;
}

// The rows `scan_folder`'s reconcile changed for one scan, kept for its
// `scan-done`.
export interface ScanStarted {
  scanId: number;
  changed: number;
}

// Whether `scan-done` should re-read the folder's rows. Skipped only when
// neither the reconcile nor the scan pass of the same scan wrote anything.
// Assumes a `scan-done` total of zero means the scan pass wrote no row, and
// `changed` of zero means the reconcile deleted no `files` row and wrote or
// cleared no `ratings` row. Both are needed because the open-time
// `refreshEntries` read is not ordered against `scan_folder`'s reconcile; if
// either ever writes rows without counting them, this skip would hide them.
// `changed` also counts 1 when `scan_folder` joined a previous scan that was
// still running, whatever folder it was scanning (its directory is not
// compared): if that scan was of the same folder, its last in-flight batch
// can land after the open-time read and would otherwise go unseen here.
// Counting every joined scan, even one of a different folder, is a
// deliberate over-count that costs one extra refresh on a folder switch
// mid-scan. A scan that has ended is never joined, because `start_scan`'s
// task drops its entry before `faces-done`.
export function refreshOnScanDone(
  started: ScanStarted | null,
  scanId: number,
  total: number,
): boolean {
  return !(started?.scanId === scanId && started.changed === 0 && total === 0);
}

// The `scan-done` total of one scan, kept for its `faces-done`.
export interface ScanDone {
  scanId: number;
  total: number;
}

// Whether `faces-done` should re-read the folder's rows. Skipped only when
// neither pass of the same scan wrote anything. Assumes a `faces-done` total
// of zero means the faces pass made no `write_faces`; if it ever writes rows
// without counting them, this skip would hide them.
export function refreshOnFacesDone(
  scanDone: ScanDone | null,
  scanId: number,
  facesTotal: number,
): boolean {
  return !(scanDone?.scanId === scanId && scanDone.total === 0 && facesTotal === 0);
}

// The shortest gap, in milliseconds, between the start of the last scan and a
// main-window focus that rescans the open folder.
export const FOCUS_RESCAN_INTERVAL = 5_000;

// Whether a main-window focus should rescan the open folder: when no scan has
// started yet, or the last one started `FOCUS_RESCAN_INTERVAL` or more ago.
export function focusRescanDue(lastScanAt: number | null, now: number): boolean {
  return lastScanAt === null || now - lastScanAt >= FOCUS_RESCAN_INTERVAL;
}

export interface RefreshTiming {
  rows: number;
  invoke: number;
  entries: number;
  bursts: number;
  exif: number;
  meta: number;
  draw: number;
  applyBursts: number;
  candidates: number;
  refilter: number;
  setFiles: boolean;
  total: number;
}

// One `Riffle.log` line per `refreshEntries` run, times in milliseconds.
export function refreshTimingLine(t: RefreshTiming): string {
  const ms = (value: number): string => `${value.toFixed(1)}ms`;
  return [
    "refresh entries:",
    `rows=${t.rows}`,
    `invoke=${ms(t.invoke)}`,
    `entries=${ms(t.entries)}`,
    `bursts=${ms(t.bursts)}`,
    `exif=${ms(t.exif)}`,
    `meta=${ms(t.meta)}`,
    `draw=${ms(t.draw)}`,
    `apply_bursts=${ms(t.applyBursts)}`,
    `candidates=${ms(t.candidates)}`,
    `refilter=${ms(t.refilter)}`,
    `set_files=${t.setFiles}`,
    `total=${ms(t.total)}`,
  ].join(" ");
}

// What asked `resync()` for a rescan of the open folder.
export type RescanTrigger =
  | "focus"
  | "watch"
  | "reload"
  | "refresh"
  | "trash"
  | "restore"
  | "rename"
  | "ai"
  | "import"
  | "scan-canceled";

// One `Riffle.log` line per `resync()` call: `defer` when a scan (or a
// listing, or a deferred operation's invoke) holds it off, `start` when it
// lists the folder now, `drained` when a deferred one runs at last, still
// naming the trigger that was deferred.
export function rescanLine(trigger: RescanTrigger, phase: "defer" | "start" | "drained"): string {
  if (phase === "defer") {
    return `rescan deferred: trigger=${trigger}`;
  }
  return `rescan: trigger=${trigger} deferred=${phase === "drained"}`;
}
