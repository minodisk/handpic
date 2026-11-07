import { type Binding, isModifierCode, keyName } from "./keys.js";
import * as strip from "./strip.js";
import * as folders from "./folders.js";
import { type Exif, type ExifGroup, exifKey } from "./exif.js";
import { shouldAdvance } from "./advance.js";
import {
  type Entry,
  type TrashEntry,
  History,
  isEmptyBatch,
  isJudgments,
  mapJudgments,
  withoutPaths,
  mapTrashDirs,
  undoneTrash,
} from "./undo.js";
import { ErrorList } from "./errors.js";
import { token } from "./theme.js";
import {
  type AfEye,
  type FilterState,
  type Flag,
  type Orientation,
  type OutputState,
  type StoredFilter,
  anchorAfterFilter,
  applyStored,
  filterListChanged,
  outputOf,
  passes as filterPasses,
  toStored,
} from "./filter.js";
import { type SortKey, orderFiles } from "./sort.js";
import { burstFrameStep, burstMarks, burstStep, groupBursts, type BurstMember } from "./burst.js";
import { placeholderRect } from "./zoom.js";
import {
  FOCUS_MARK_COLORS,
  FOCUS_MARK_ICONS,
  type FaceReady,
  type Faces,
  type FocusPoint,
  applyFaceReady,
  afEyeState,
  applySharpnessReady,
  faceLabelAt,
  faceMarks,
  focusMark,
  photoTier,
  stripState,
} from "./focus.js";
import { type EyeState, type Eyes, EyesCache, type Pose } from "./eyes.js";
import { FaceCache, NO_FACES } from "./faces.js";
import { meshEdges, meshPoints } from "./facemesh.js";
import {
  type TrashOp,
  type TrashProgress,
  type TrashRestored,
  type TrashSummary,
  type TrashWhat,
  TrashFlow,
  dialogText,
  opensTarget,
  progressStatus as trashProgressStatus,
  restoredInto,
  restoredStatus,
  stillHeld,
  swallowedByDelete,
  confirmsDelete,
  trashedStatus,
} from "./trash.js";
import {
  RUNNING_NOTE,
  type SequenceDone,
  SequenceFlow,
  type SequencePreview,
  captureTimeLine,
  doneStatus,
  failureText,
  progressStatus,
  rebuildNotice,
  revealAfter,
  rowParts,
  writtenLine,
} from "./sequence.js";
import {
  FILTERED_TEXT,
  NO_FILES_TEXT,
  PREVIEW_FAILED_TEXT,
  emptyState,
  openHint,
} from "./empty.js";
import { type MenuItem, contextMenuGroups, folderMenuGroups, menuPosition } from "./context.js";
import { type Metadata, type RankModel, type Ranked, metaGroups } from "./meta.js";
import { FormatGate, asksLanguage, defaultPreset } from "./firstrun.js";
import type { LabelNames, LabelPreset } from "./labels.js";
import { type McpRequest, type ViewApi, respond } from "./companion.js";
import { VIEW_ONLY_NOTE, isViewOnly, sortFor } from "./viewonly.js";
import { initSettings } from "./settings.js";
import { type AiState, aiActive } from "./ai.js";
import { CHECK_SVG } from "./icons.js";
import { SettingsModal, cycleFocus } from "./modal.js";
import { type Panels, toggle, toggleSides } from "./panels.js";
import { treeGate } from "./treekeys.js";
import { rebase } from "./tree.js";
import { type Pending, editKey } from "./rename.js";
import { IdleGate, cancelsScan } from "./idle.js";
import { scanFocusPaths } from "./scanfocus.js";
import {
  type RescanTrigger,
  type ScanDone,
  type ScanStarted,
  focusRescanDue,
  refreshOnFacesDone,
  refreshOnProgress,
  refreshOnScanDone,
  refreshTimingLine,
  rescanLine,
} from "./refresh.js";
import {
  COMPARE_NEEDS_FRAMES,
  comparePaneAt,
  comparisonCandidates,
  loadComparisonFrames,
  reconcileActive,
} from "./compare.js";
import {
  type Command,
  type PickFlag,
  type Selection,
  all,
  click,
  extend,
  judgments,
  prune,
  restore,
  selectionOf,
  settle,
  single,
  targets,
} from "./selection.js";
import { progressWidth } from "./progress.js";
import { recommended } from "./recommend.js";
import { firstEntriesAnchor, lastViewedWriter, mustReshow, resumeTarget } from "./resume.js";
import {
  DELETE_RUNNING_NOTE,
  type DeletePreview,
  REWRITE_RUNNING_NOTE,
  type RewritePreview,
  type RewriteSummary,
  SidecarFlow,
  deleteRows,
  deleteTotalLine,
  rewriteRows,
  rewriteTotalLine,
  rewrittenStatus,
} from "./sidecars.js";
import {
  DATE_RUNNING_NOTE,
  DateFolderFlow,
  type DatePreview,
  type DateRan,
  dateRows,
  dateTitleLine,
  disabledReason,
  parentOf,
  removalNote,
  splitStatus,
  undatedLine,
} from "./datefolder.js";
import {
  COLLISIONS_HEADING,
  IMPORT_RUNNING_NOTE,
  type ImportDone,
  ImportFlow,
  type ImportPreview,
  type ImportProgress,
  type ImportStarted,
  UNDATED_HEADING,
  canRun,
  collisionItems,
  doneStatus as importDoneStatus,
  failureText as importFailureText,
  folderRows,
  importNote,
  progressStatus as importProgressStatus,
  totalLine,
} from "./import.js";
import { UncaughtGate, uncaughtLine } from "./uncaught.js";
import { dwellTimer, switchesFolder, viewedWriter } from "./viewed.js";
import { type PerfSample, PerfModel, WINDOW_MS, latencyStart } from "./perf.js";

// Registered before anything else runs so an early throw still reaches
// `Riffle.log`. Not gated by `debugLogging`, and the default console output is
// left alone.
const uncaughtGate = new UncaughtGate();

function forwardUncaught(line: string): void {
  const passed = uncaughtGate.pass(line);
  if (passed === null) return;
  void window.__TAURI__.core
    .invoke("log_frontend", { kind: "uncaught-js", line: passed })
    .catch(() => {});
}

window.addEventListener("error", (event) => {
  // A resource load error is a plain `Event` with no message.
  if (!(event instanceof ErrorEvent) || !event.message) return;
  const stack = event.error instanceof Error ? event.error.stack : undefined;
  forwardUncaught(
    uncaughtLine("error", event.message, event.filename, event.lineno, event.colno, stack),
  );
});

window.addEventListener("unhandledrejection", (event) => {
  const reason: unknown = event.reason;
  const message = reason instanceof Error ? reason.message : String(reason);
  const stack = reason instanceof Error ? reason.stack : undefined;
  forwardUncaught(
    uncaughtLine("unhandledrejection", message, undefined, undefined, undefined, stack),
  );
});

// Header layout of a `preview` payload, see `crates/app/src/commands.rs`.
const PREVIEW_HEADER_LEN = 8;
const PREVIEW_KIND_JPEG_V1 = 1;
// Header layout of a `focus_crop` payload, see `crates/app/src/commands.rs`.
const CROP_HEADER_LEN = 32;
const CROP_KIND_RGBA_V3 = 5;
// Turned on by the settings modal's `Timing logs` item. That item only
// shows in a development build, so elsewhere this stays off.
let debugLogging = false;

function debugLog(...args: unknown[]): void {
  if (debugLogging) {
    console.debug(...args);
    void window.__TAURI__.core.invoke("log_timing", { line: args.map(String).join(" ") });
  }
}

interface Focus {
  point: FocusPoint | null;
  manual_focus: boolean;
  candidate: "candidate" | "not_candidate" | "unknown";
  eye_focus: number | null;
  eyes_ear: number | null;
  eyes: EyeState;
  eyes_closed: number | null;
  pose: Pose | null;
  eye_offset: number | null;
  edge_gap: number | null;
}

interface IndexedFile {
  path: string;
  orientation: number;
  capture_time: string | null;
  subsec: string | null;
  focus: Focus | null;
  has_thumb: boolean;
  rating: number | null;
  flag: PickFlag;
  label: string | null;
  has_sidecar: boolean;
  sharpness: number | null;
  exif: Exif | null;
  error: string | null;
}

interface DecodeResponse {
  seq: number;
  bitmap?: ImageBitmap;
  error?: string;
}

const canvas = document.getElementById("canvas") as HTMLCanvasElement;
const context = canvas.getContext("2d") as CanvasRenderingContext2D;
const perfCanvas = document.getElementById("perf") as HTMLCanvasElement;
const perfContext = perfCanvas.getContext("2d") as CanvasRenderingContext2D;
const metaEl = document.getElementById("meta") as HTMLDivElement;
const metaStatusEl = document.getElementById("meta-status") as HTMLDivElement;
const positionEl = document.getElementById("position") as HTMLDivElement;
const emptyEl = document.getElementById("empty") as HTMLDivElement;
const scanProgressEl = document.getElementById("scan-progress") as HTMLDivElement;
const scanProgressIndicator = scanProgressEl.firstElementChild as HTMLDivElement;
const formatDialog = document.getElementById("format-dialog") as HTMLDivElement;
const formatError = document.getElementById("format-error") as HTMLParagraphElement;
const settings = initSettings({
  applyKeymap,
  setAutoAdvance: (enabled) => {
    autoAdvance = enabled;
  },
  setDebugLogging: (enabled) => {
    debugLogging = enabled;
  },
});
const formatButtons = [...formatDialog.querySelectorAll<HTMLButtonElement>("button[data-format]")];
const formatLanguage = document.getElementById("format-language") as HTMLDivElement;
const formatLanguageSelect = document.getElementById("format-language-select") as HTMLSelectElement;
const formatContinue = document.getElementById("format-continue") as HTMLButtonElement;
// Lightroom (XMP) or Both, chosen in the dialog and saved once its language is.
let chosenFormat: string | null = null;
let formatPresets: LabelPreset[] = [];

// Closed until a sidecar format is saved; no folder opens before that.
const formatGate = new FormatGate();

const worker = new Worker(new URL("./worker.js", import.meta.url), {
  type: "module",
});

// Linux's WebKitGTK only; resolved once, awaited before every post so the
// first preview cannot race it.
const previewPixelLimit = window.__TAURI__.core
  .invoke<number | null>("preview_pixel_limit")
  .catch(() => null);

// Every RAW file in the open folder, in `list_arw` order.
let allFiles: string[] = [];
// The strip order, kept across folder opens within the session.
let sortKey: SortKey = "name";
// A JPEG-only folder is open: no judgment, no faces, capture order.
let viewOnly = false;
// The files that pass the filter, in `sortKey` order. `index`, the strip and
// paging all work on this view.
let files: string[] = [];
let index = 0;
// Incremented on every page turn; a response tagged with an older sequence
// belongs to a file that is no longer current and is dropped.
let seq = 0;
const orientations = new Map<number, number>();
// `performance.now()` marks of the preview request for each `seq` awaiting
// its decoded bitmap, for the `page …` timing line.
const pageTimings = new Map<
  number,
  { startedAt: number; invokeMs: number; postedAt: number; keypressAt: number | null }
>();
// `performance.now()` at the key that turned the page, cleared by the preview
// request that page turn produced. A strip click or a folder open starts its
// own request, which is not timed from a keypress.
let pageKeypressAt: number | null = null;
// `performance.now()` at the last `show()`, where a preview's latency starts
// when no keypress timed it (a strip click, a folder open, or a key whose
// request was queued behind an in-flight one).
let switchAt = 0;
// The performance overlay's series; it records only while the overlay is on.
const perf = new PerfModel();
let shown: { bitmap: ImageBitmap; orientation: number; seq: number } | null = null;
// The `seq` whose preview failed to load or decode, so the empty overlay says
// so over the cleared canvas until the next page turn.
let previewFailedSeq: number | null = null;
// True while a `preview` invoke is outstanding. Keeps at most one request in
// flight; when it settles, if `index` moved on in the meantime, exactly one
// follow-up request is issued for the latest index.
let inFlight = false;
// The metadata last read. While the current file's read is outstanding it
// still holds the previous file's (`metaStale`), so the rows keep their place
// instead of collapsing and reappearing on every step while paging quickly.
let meta: Metadata | null = null;
let metaStale = false;
// The same one-in-flight, re-request-if-stale pattern as `inFlight`, so
// holding a paging key down does not queue up a read per file passed.
let metaInFlight = false;
// A transient line in the status block at the bottom of the right pane: an
// error.
let note: string | undefined;
// How far the current scan got, or null when nothing is scanning. Events
// carry the id of the scan that emitted them; only events whose id matches
// `scanId` are applied, so the stragglers of a canceled scan (including one
// canceled by reopening the very same folder) are ignored even though they
// carry the same `dir`.
let scanId: number | null = null;
let scanning: string | null = null;
// How far the sequencing run got, or null when none runs.
let sequencing: string | null = null;
// How far the move to the Trash or the delete in flight got, or null when none
// runs. Also what refuses a second one: `trashFlow` ended already when the run
// was held.
let trashing: string | null = null;
// The import's status line: the preview being read, then how far the run got;
// null when neither is under way.
let importing: string | null = null;
// The last import's done line, kept until the next import starts or a folder
// opens, so paging and decoded previews (`setStatus()`) do not wipe it.
let imported: string | null = null;
// True between `start_scan` and its `faces-done`, i.e. through both scan
// passes. A rescan asked for while it is true is deferred (`resyncPending`)
// rather than canceling the scan.
let scanRunning = false;
// The files the first pass failed on, kept for the final status after the
// second pass.
let scanErrors = 0;
// The first pass's `scan-done` total, so its `faces-done` can skip a re-read
// when neither pass wrote anything.
let scanDone: ScanDone | null = null;
// The rows `scan_folder`'s reconcile changed, so its `scan-done` can skip a
// re-read when neither it nor the scan pass wrote anything.
let scanStarted: ScanStarted | null = null;

// The first pass's bar over the strip; `null` hides it.
function setScanProgress(width: string | null): void {
  scanProgressEl.hidden = width === null;
  scanProgressIndicator.style.width = width ?? "0%";
}

function setScanRunning(running: boolean): void {
  scanRunning = running;
  settings.setScanRunning(running);
}
// How long the current file and the strip's range must settle before the
// running scan is told to take them first, so a held arrow key or a wheel
// scroll sends one `set_scan_focus` per stop rather than one per step.
const SCAN_FOCUS_DELAY = 100;
let scanFocusTimer: ReturnType<typeof setTimeout> | undefined;

function sendScanFocus(): void {
  if (!scanRunning) {
    return;
  }
  clearTimeout(scanFocusTimer);
  scanFocusTimer = setTimeout(() => {
    if (!scanRunning || scanId === null) {
      return;
    }
    const { first, last } = strip.visibleRange();
    const paths = scanFocusPaths(files, index, first, last);
    if (paths.length > 0) {
      void window.__TAURI__.core.invoke<void>("set_scan_focus", { scanId, paths }).catch(() => {});
    }
  }, SCAN_FOCUS_DELAY);
}

// An operation pressed while a scan runs waits here for the scan's end; a
// folder switch drops it.
const idle = new IdleGate(() => scanRunning);

// Stops the current scan, if `scan_folder` has given it an id. A stale id is
// ignored by the backend, so `startScan` asks again for the scan it starts.
function cancelScan(): void {
  if (scanId !== null) {
    void window.__TAURI__.core.invoke<void>("cancel_scan", { scanId }).catch(() => {});
  }
}

// Whether `run` was held for the scan's end rather than run at once.
function whenIdle(label: string, run: () => void, cancel?: () => void): boolean {
  const held = idle.request(label, run, cancel);
  renderMeta();
  return held;
}

const RENAME_CANCELED = "Rename… was canceled";
const RENAME_REFUSED = "Rename… is unavailable while an import runs";
// The path of the rename being requested, so the held rename of the same
// folder or file it replaces (a re-edit of the pending name) is not reported
// as canceled.
let renaming: string | null = null;
// The path of the rename the idle gate holds, if any.
let heldRename: string | null = null;

// Holds a confirmed rename for the scan's end, its new name shown pending in
// the tree or the strip until it runs, or until it is replaced or dropped,
// which reverts the name. Only this rename's own mark is cleared, through
// the `clearPending` handed to `run` (a no-op when it ran without being
// held), so a later rename of the same path keeps its pending name.
function holdRename(
  path: string,
  name: string,
  view: { markPending(path: string, name: string): Pending; clearPending(mark: Pending): void },
  run: (clearPending: () => void) => void,
): void {
  let mark: Pending | null = null;
  const clearPending = (): void => {
    if (mark !== null) {
      view.clearPending(mark);
    }
  };
  if (importFlow.busy) {
    setStatus(RENAME_REFUSED);
    return;
  }
  renaming = path;
  const held = whenIdle(
    "Rename…",
    () => {
      heldRename = null;
      // A held rename can outlive the check above: an import may have begun.
      if (importFlow.busy) {
        clearPending();
        setStatus(RENAME_REFUSED);
        return;
      }
      run(clearPending);
    },
    () => {
      heldRename = null;
      clearPending();
      if (renaming !== path) {
        setStatus(RENAME_CANCELED);
      }
    },
  );
  renaming = null;
  if (held) {
    heldRename = path;
    mark = view.markPending(path, name);
  }
}

// The tree or the strip confirmed a pending name back to the real one: the
// held rename is dropped, which reverts the name and reports it.
// Only when that path is the one still held: once the rename has drained its
// view stays pending until the invoke settles, and the slot may by then hold
// another rename that must not be dropped.
function cancelPendingRename(path: string): void {
  if (heldRename !== path) {
    return;
  }
  idle.discard();
  renderMeta();
}

// Marks a deferred operation's invoke, or the folder picker through its
// picked folder's open, as in flight so `resync()` waits for it.
function settleIdle<T>(promise: Promise<T>): void {
  idle.enterInFlight();
  void promise.finally(() => {
    idle.leaveInFlight();
    drainResync();
  });
}
// Mints a per-call id for `startScan` so its `.then`/`.catch` can tell
// whether a later call (a re-open of the same folder included) has already
// superseded it, since `folder !== openDir` can't detect that case.
let scanSeq = 0;
let currentScan = 0;
// When the last `startScan` began or the last focus rescan was triggered,
// in `Date.now()` milliseconds, so a focus rescan inside
// `FOCUS_RESCAN_INTERVAL` of it is skipped. The focus listener stamps it too,
// since `resync()` reaches `startScan` only after its `list_arw` resolves.
let lastScanAt: number | null = null;
// True while a rescan's `list_arw` is outstanding, and the trigger that
// arrived while one was (or while a scan ran), the way `refreshEntries` keeps
// one read in flight. The first deferred trigger is kept, so the drained
// rescan's log line names what caused it.
let resyncInFlight = false;
let resyncPending: RescanTrigger | null = null;
// Reserved by the picker before its dialog opens, and minted by a drop only
// once its dropped path has resolved (see `newFolderToken` and `dropCounter`
// below). When two folder opens race, the `list_arw` result of the one whose
// token is no longer current (e.g. A resolves after B was opened) is dropped
// instead of overwriting `files` with a stale folder's contents.
let folderToken = 0;
// The indexed rows of the open folder, keyed by the path `list_arw` returned.
// Fills in as the scan progresses; the focus mark needs nothing else from it.
const entries = new Map<string, IndexedFile>();
// The folder the entries belong to, so `scan-progress` can ask for them again.
let openDir: string | null = null;
// The resume target queued by `openDirectory`, still waiting on entries
// (capture time, ratings, flags, labels) to resolve where it lands once the
// filter and sort apply. Consumed, and cleared, by the first `refreshEntries`
// of the open; cleared again at the start of the next `openDirectory`.
let pendingResume: string | undefined;
// Records the current file of the open folder, so the next open resumes there.
const rememberViewed = lastViewedWriter(({ dir, path }) =>
  window.__TAURI__.core.invoke("set_last_viewed", { dir, path }),
);
// The open folder's files shown in the preview at least once, across
// sessions, loaded by `openDirectory` and grown by `dwell`.
let viewedFiles = new Set<string>();
const recordViewed = viewedWriter(({ dir, paths }) =>
  window.__TAURI__.core.invoke("mark_viewed", { dir, paths }),
);
// Records the file on screen as viewed once its preview has stayed up for
// `VIEW_DWELL_MS`; `show`, `openDirectory` and an emptied list cancel it.
const dwell = dwellTimer(
  (path) => viewedFiles.has(path),
  (path) => {
    if (openDir === null) {
      return;
    }
    viewedFiles.add(path);
    recordViewed(openDir, path);
  },
);
// True while a `folder_entries` invoke is outstanding. Keeps at most one
// request in flight, so a 10/s `scan-progress` stream while the user is
// paged ahead of the scan does not queue up a full re-read on every tick,
// and two overlapping reads cannot land out of order and clobber `entries`
// with a stale snapshot.
let entriesInFlight = false;
// True when a refresh was requested while one was already in flight. Re-run
// once the in-flight one settles, so the authoritative `scan-done` refresh
// is never silently dropped just because a `scan-progress` refresh happened
// to be outstanding at that moment.
let entriesPending = false;
// The current file the last `scan-progress`-triggered refresh was for, so a
// tick re-reads only when that file's row lands or the current file changes.
let progressRefreshedFor: string | null = null;
// The stars of every file that has some: `1`-`5`, and a missing entry is
// unrated. Filled from `folder_entries` and then owned by the keyboard until
// the next folder open.
const ratings = new Map<string, number>();
// The pick / reject of every flagged file, owned the same way as `ratings`.
// It coexists with the stars under both sidecar formats.
const flags = new Map<string, "pick" | "reject">();
// The color label of every file that has one, owned the same way as
// `ratings`.
const labels = new Map<string, string>();
// The sharpness score of every file the index has one for, from
// `folder_entries`, for the MCP companion; cleared with `labels`.
const sharpness = new Map<string, number>();
// The faces the focus mark draws, detected per file when first shown with
// the mark on; cleared with `sharpness`.
const faceCache = new FaceCache();
// Whether the eyes of each shown file's AF face are closed, judged per file
// when it is shown; cleared with `faceCache`.
const eyesCache = new EyesCache();
// Every file's burst, from `groupBursts` over `allFiles` in capture order
// whatever the sort; recomputed whenever `entries` is refreshed.
let bursts = new Map<string, BurstMember>();
// The paths judged through the keyboard in this session, so a refresh from
// `folder_entries` (which may predate the pending sidecar write) does not
// undo what the user just pressed.
const touched = new Set<string>();
// The strip's multi-selection; `files[index]` is its focused member.
let selection: Selection = single(undefined);
// The index of each path in `files`, for handing a rating to the strip.
const fileIndex = new Map<string, number>();
// A judgment's file and its state before it, for `Edit > Undo`. An entry is
// a batch, undone as one: a single key judges one file, reject-rest several;
// or a `Move to Trash` run, undone by restoring it from the Trash.
// Judgments are per folder: `openDirectory` drops them. A trash run stays,
// since it needs no folder open.
type Judgment = { path: string; rating: number | null; flag: PickFlag; label: string | null };
const history = new History<Entry<Judgment>>(100);
// The pre-undo state of each undone batch, for `Edit > Redo`, and each
// undone trash run, redone by moving what came back to the Trash again. A
// new judgment forgets it, as every editor does.
const redoable = new History<Entry<Judgment>>(100);
// The trash run whose restore or redo is out, so a second undo does not pop
// the judgments beneath it before its files are back.
let trashStep: TrashEntry | null = null;
// Sidecar problems, kept until dismissed rather than in the transient `note`.
const errors = new ErrorList();
const shownFlags = new Set<Flag>();
const shownStars = new Set<number>();
const shownLabels = new Set<string>();
const shownOrientations = new Set<Orientation>();
const shownCandidates = new Set<AfEye>();
const shownEyes = new Set<EyeState>();
// The checked `Genre` items, per session like the EXIF groups.
const shownGenres = new Set<string>();
// The genre labels of the open folder's files with an embedding
// (`genres_of`); a file absent from it counts as `Unknown`. Empty while AI
// analysis is off.
let fileGenres = new Map<string, string[]>();
let genresInFlight = false;
let genresPending = false;
// The checked `Output` items, per session like the genres.
const shownOutputs = new Set<OutputState>();
// The open folder's files with an export (`outputs_of`) out of
// `outputsListing`, the files it was asked about; null while no export folder
// is known, so no file counts as not exported.
let fileOutputs: Set<string> | null = null;
let outputsListing = new Set<string>();
// Why the last `outputs_of` failed, kept apart from the transient `note`: the
// decoded preview's `setStatus()` would wipe it at once, and it stays true
// until a read succeeds.
let outputsFailure: string | null = null;
// The personal ranking of the open folder's files with an embedding
// (`ranking_of`) and the model it ran, while AI analysis is on.
let rankingOn = false;
let fileRanks = new Map<string, Ranked>();
let rankModel: RankModel | null = null;
let ranksInFlight = false;
let ranksPending = false;
// The EXIF groups, keyed by label (two estimated apertures with one label can
// differ in value). Focal length is keyed by the range's label instead.
const exifGroups: { group: ExifGroup; heading: string }[] = [
  { group: "camera", heading: "Camera" },
  { group: "lens", heading: "Lens" },
  { group: "aperture", heading: "Aperture" },
  { group: "shutter", heading: "Shutter speed" },
  { group: "iso", heading: "ISO" },
  { group: "focal", heading: "Focal length" },
];
const shownExif = new Map<ExifGroup, Set<string>>(
  exifGroups.map(({ group }) => [group, new Set<string>()]),
);
const filterState: FilterState = {
  flags: shownFlags,
  stars: shownStars,
  labels: shownLabels,
  orientations: shownOrientations,
  candidates: shownCandidates,
  eyes: shownEyes,
  genres: shownGenres,
  outputs: shownOutputs,
  exif: shownExif,
};
let showFocus = false;
// The event.code of the key holding the grayscale preview, or null when off.
let grayscaleHeld: string | null = null;
// True while the 1:1 focus check is showing instead of the fitted preview.
let zoomed = false;
// Side-by-side culling view. With a multi-selection it compares up to four
// selected files; otherwise it compares the current file with the first good
// frame (`photoTier`) of its burst, else the burst's first frame. The bitmaps
// are independent of `shown`, which remains ready for an immediate return to
// the single-image view.
let comparing = false;
let compareSeq = 0;
let compareFrames: { path: string; bitmap: ImageBitmap; orientation: number }[] = [];
let compareActivePath: string | null = null;
// The crop of the file that `cropSeq` identifies, at one JPEG pixel per
// device pixel, with its point of interest in crop pixels. Kept while the
// view is toggled off so toggling back on redraws without a round trip.
let crop: {
  bitmap: ImageBitmap;
  pointX: number;
  pointY: number;
  cropSeq: number;
  orientation: number;
  // The full JPEG size the crop was cut from, for placing the placeholder.
  fullWidth: number;
  fullHeight: number;
  // The device-pixel viewport size (`canvas.client*` × dpr) at request time,
  // so a resize that lands while a request is already in flight (silently
  // dropped, since `requestCrop` no-ops on `cropInFlight`) is still noticed
  // once this crop settles and can be re-requested.
  requestedWidth: number;
  requestedHeight: number;
} | null = null;
// The same one-in-flight, re-request-if-stale pattern as `inFlight`.
let cropInFlight = false;
// `performance.now()` at the zoom key that asked for a crop, cleared by the
// crop that keypress produced. Only that one crop can be timed from the
// keypress; a resize or a page turn starts its own request.
let zoomKeypressAt: number | null = null;
// A resize fires continuously while the window edge is dragged, and each
// crop costs a partial decode plus a multi-megabyte IPC payload, so the
// re-request waits for the drag to settle. 150ms is long enough to swallow a
// drag's stream of events and short enough to feel immediate once released;
// until it fires, the held crop keeps being drawn into the new viewport.
const CROP_RESIZE_DEBOUNCE_MS = 150;
let cropResizeTimer: number | null = null;

// Re-requests the crop once the viewport has stopped changing. The `seq` of
// the file the resize was seen for is captured here rather than re-read in
// the callback, so a timer that outlives a page turn or a folder change is
// dropped instead of applied.
function scheduleCropForResize(): void {
  const current = seq;
  if (cropResizeTimer !== null) {
    window.clearTimeout(cropResizeTimer);
  }
  cropResizeTimer = window.setTimeout(() => {
    cropResizeTimer = null;
    if (!zoomed || current !== seq) {
      return;
    }
    if (crop !== null && !cropViewportStale()) {
      return;
    }
    requestCrop();
  }, CROP_RESIZE_DEBOUNCE_MS);
}

// The crosshair's arm length and the gap left open around the point itself,
// in CSS pixels. The mark only points, so it keeps one size like a cursor
// instead of growing over the subject with the window. The gap keeps the
// lines off the focus point, which is the one pixel the mark exists to show.
const FOCUS_MARK_ARM = 8;
const FOCUS_MARK_GAP = 4;
// The detected faces, apart from every mark color above, and the judged
// face's mesh, apart from the faces so YuNet's output and the mesh's differ.
const FACE_MARK_COLOR = "#3ff";
const FACE_MESH_COLOR = "#f3f";
const FACE_MARK_EYE_RADIUS = 2.5;
// The judged face's parts outline, lighter than the boxes so the box stays
// the outer frame.
const FACE_MESH_OUTLINE_WIDTH = 3;
const FACE_MESH_LINE_WIDTH = 1.5;

// The neutral colors the canvas overlays draw with, read from the stylesheet's
// tokens once: the module runs after the `<head>` stylesheet has applied.
const rootStyle = getComputedStyle(document.documentElement);
const THEME = {
  card: token(rootStyle, "--card", "black"),
  foreground: token(rootStyle, "--foreground", "white"),
  border: token(rootStyle, "--border", "gray"),
  primary: token(rootStyle, "--primary", "white"),
  mutedForeground: token(rootStyle, "--muted-foreground", "gray"),
  chartCpu: token(rootStyle, "--chart-cpu", "blue"),
  chartDisk: token(rootStyle, "--chart-disk", "green"),
  chartLatency: token(rootStyle, "--chart-latency", "red"),
};

function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

function row(list: HTMLDListElement, label: string, value: string): void {
  const dt = document.createElement("dt");
  dt.textContent = label;
  const dd = document.createElement("dd");
  dd.textContent = value;
  list.append(dt, dd);
}

function line(className: string, text: string): HTMLDivElement {
  const el = document.createElement("div");
  el.className = className;
  el.textContent = text;
  return el;
}

// The keymap last applied, so the empty state can name the `open` key.
// Empty until the `shortcuts` invoke resolves.
let keyBindings: Binding[] = [];

// The centered message over the viewer: the clickable opening hint when no
// folder is open, or why an open folder shows nothing.
function renderEmpty(): void {
  const state = emptyState(
    openDir,
    allFiles.length,
    files.length,
    previewFailedSeq === seq && !comparing && !zoomed,
  );
  emptyEl.hidden = state === "none";
  if (state === "none") {
    emptyEl.removeAttribute("data-state");
    emptyEl.textContent = "";
    return;
  }
  emptyEl.dataset.state = state;
  emptyEl.textContent =
    state === "no-folder"
      ? openHint(keyBindings)
      : state === "no-files"
        ? NO_FILES_TEXT
        : state === "filtered"
          ? FILTERED_TEXT
          : PREVIEW_FAILED_TEXT;
}

emptyEl.addEventListener("click", () => {
  if (emptyEl.dataset.state === "no-folder") {
    openFolder();
  }
});

// Redraw the right pane: the current file's name and its shooting settings
// in the scrolling metadata block, and any note (an error, the scan's
// progress, the 1:1 indicator, the sticky errors) in the status block pinned
// to the bottom. Also refreshes the strip pane's `N / M` counter.
function renderMeta(): void {
  renderTitle();
  renderEmpty();
  positionEl.textContent = files.length === 0 ? "" : `${index + 1} / ${files.length}`;
  if (selection.selected.size > 1) {
    positionEl.textContent += ` \u00B7 ${selection.selected.size} selected`;
  }
  metaEl.replaceChildren();
  metaStatusEl.replaceChildren();
  if (files.length > 0) {
    metaEl.append(line("name", meta === null || metaStale ? baseName(files[index]) : meta.name));
    const ranked = fileRanks.get(files[index]);
    for (const group of metaGroups(
      meta,
      entries.get(files[index])?.focus,
      eyesCache.get(files[index]),
      ranked === undefined || rankModel === null ? null : { ranked, model: rankModel },
    )) {
      const block = document.createElement("div");
      block.className = "group";
      const list = document.createElement("dl");
      for (const { label, value } of group.rows) {
        row(list, label, value);
      }
      block.append(line("group-title", group.heading), list);
      metaEl.append(block);
    }
  }
  if (viewOnly) {
    metaStatusEl.append(line("note", VIEW_ONLY_NOTE));
  }
  if (note !== undefined) {
    metaStatusEl.append(line("note", note));
  }
  if (outputsFailure !== null) {
    metaStatusEl.append(line("note", outputsFailure));
  }
  if (scanning !== null) {
    metaStatusEl.append(line("note", scanning));
  }
  if (sequencing !== null) {
    metaStatusEl.append(line("note", sequencing));
  }
  if (trashing !== null) {
    metaStatusEl.append(line("note", trashing));
  }
  if (importing !== null) {
    metaStatusEl.append(line("note", importing));
  }
  if (imported !== null) {
    metaStatusEl.append(line("note", imported));
  }
  if (idle.waiting !== null) {
    metaStatusEl.append(line("note", `${idle.waiting}: waiting for the scan to finish`));
  }
  // Driven by `zoomed` rather than `note`, so paging or an error does not
  // erase the mode indicator while the 1:1 view is still showing.
  if (zoomed) {
    metaStatusEl.append(line("note", "1:1"));
  }
  if (comparing) {
    metaStatusEl.append(line("note", `Compare · ${compareCandidates().length} frames`));
  }
  for (const { key, message } of errors.list()) {
    const el = line("error", message);
    const dismiss = document.createElement("button");
    dismiss.className = "button ghost";
    dismiss.textContent = "\u00d7";
    dismiss.title = "Dismiss";
    dismiss.addEventListener("click", () => {
      errors.dismiss(key);
      renderMeta();
    });
    el.append(dismiss);
    metaStatusEl.append(el);
  }
}

// The window title last sent, so paging only crosses IPC when it changes.
let shownTitle = "Riffle";

// Show the open folder and the current file in the title bar.
function renderTitle(): void {
  const parts = ["Riffle"];
  if (openDir !== null) {
    parts.push(baseName(openDir));
  }
  if (files.length > 0) {
    parts.push(baseName(files[index]));
  }
  const title = parts.join(" \u2014 ");
  if (title === shownTitle) {
    return;
  }
  shownTitle = title;
  void window.__TAURI__.window.getCurrentWindow().setTitle(title);
}

// The confirmation of the strip's `Move to Trash…`, asking with the count
// alone, and of its `Delete Permanently…` (`confirmTrashFiles`).
const trashFlow = new TrashFlow();
// Only its key decisions are used: Escape and Tab, as in the settings modal.
const trashKeys = new SettingsModal();
const trashDialog = document.getElementById("trash-dialog") as HTMLDivElement;
const trashTitle = document.getElementById("trash-title") as HTMLHeadingElement;
const trashTotal = document.getElementById("trash-total") as HTMLParagraphElement;
const trashRunButton = document.getElementById("trash-run") as HTMLButtonElement;
const trashCancelButton = document.getElementById("trash-cancel") as HTMLButtonElement;

function listItem(text: string): HTMLLIElement {
  const item = document.createElement("li");
  item.textContent = text;
  return item;
}

// Ask whether to move `paths`, files of the open folder taken when the strip's
// `Move to Trash…` was chosen, to the Trash, or for `Delete Permanently…` to
// delete them. Refused while either is in flight. A delete has no way back, so
// its dialog opens on Cancel and takes no key as a yes (`trashKeydown`).
function confirmTrashFiles(paths: string[], mode: TrashOp): void {
  if (
    trashing !== null ||
    openDir === null ||
    paths.length === 0 ||
    !formatDialog.hidden ||
    settings.isOpen ||
    sequenceFlow.busy ||
    sidecarFlow.busy ||
    dateFolderFlow.busy ||
    importFlow.busy ||
    !trashFlow.start({ dir: openDir, paths, mode })
  ) {
    return;
  }
  setFilterMenuOpen(false);
  setSortMenuOpen(false);
  closeContextMenu();
  const text = dialogText(mode, paths.length);
  trashTitle.textContent = text.title;
  trashTotal.textContent = text.line;
  trashRunButton.textContent = text.run;
  trashRunButton.disabled = false;
  trashCancelButton.disabled = false;
  trashDialog.hidden = false;
  (mode === "delete" ? trashCancelButton : trashRunButton).focus();
}

// Move `paths` of `dir` and their sidecars to the Trash through
// `trash_files`, or in the "delete" mode delete them through `delete_files`,
// at once when no scan runs, else as soon as the scan this
// cancels has ended: the backend refuses the move until then. The canceled
// scan leaves files unscanned, so a rescan is queued whatever becomes of the
// held move. Dropped when another folder was opened meanwhile; the run's result is reported even if one was opened while it
// ran, since the backend recorded it for undo. The confirm dialog stays open
// in its running phase until the invoke settles, so the strip does not get
// the keyboard back mid-move. When the idle gate holds the move for a scan's
// end, the dialog closes at once instead and the held closure leaves the
// dialog alone, since it may belong to another trashing by then; it also
// waits out any trash dialog that is open or running.
function trashFilesIn(dir: string, paths: string[], mode: TrashOp): void {
  const token = folderToken;
  let sync = true;
  const held = whenIdle(dialogText(mode, paths.length).title, () => {
    const own = sync;
    const done = (): void => {
      if (own) {
        closeTrashDialog();
      }
    };
    if (
      dir !== openDir ||
      token !== folderToken ||
      sidecarFlow.busy ||
      sequenceFlow.busy ||
      dateFolderFlow.busy ||
      importFlow.busy ||
      trashing !== null ||
      (!own && trashFlow.busy)
    ) {
      done();
      return;
    }
    trashing = trashProgressStatus(mode, 0, paths.length);
    renderMeta();
    settleIdle(
      window.__TAURI__.core
        .invoke<TrashSummary>(mode === "delete" ? "delete_files" : "trash_files", { dir, paths })
        .then((summary) => {
          trashing = null;
          done();
          if (summary.run_id !== null) {
            redoable.clear();
          }
          trashed(summary, [dir], "files", true, mode);
        })
        .catch((err: unknown) => {
          trashing = null;
          done();
          setStatus(String(err));
        }),
    );
  });
  sync = false;
  if (held) {
    closeTrashDialog();
    cancelScan();
    resyncPending ??= "scan-canceled";
  }
}

// The last event can land after the invoke settled, and must not bring the
// line and the guard back.
void window.__TAURI__.event.listen<TrashProgress>("trash-progress", ({ payload }) => {
  if (trashing === null) {
    return;
  }
  trashing = trashProgressStatus(payload.op, payload.done, payload.total);
  renderMeta();
});

function closeTrashDialog(): void {
  trashFlow.end();
  trashDialog.hidden = true;
}

// The confirmed files go to `trashFilesIn`, which closes the dialog once
// the run comes back. A delete takes a single mouse click only (see
// `confirmsDelete`).
function runTrash(event: MouseEvent): void {
  if (trashFlow.target?.mode === "delete" && !confirmsDelete(event)) {
    return;
  }
  const target = trashFlow.run();
  if (target === null) {
    return;
  }
  trashRunButton.disabled = true;
  trashCancelButton.disabled = true;
  trashFilesIn(target.dir, target.paths, target.mode);
}

// What a run or a redo does once its files went to the Trash, or a delete
// (`mode` "delete", which recorded no run) once they are gone: prune the open
// folder's state when it was among `dirs`, push the recorded run for undo,
// report the failures, and re-list the folder. A run of sidecars (`what` is
// "sidecars") drops the judgments the strip shows instead, and `rescan`
// false re-reads `folder_entries` rather than re-listing: `Delete Sidecars…`
// cleared the rows already, so a rescan would find nothing changed.
function trashed(
  summary: TrashSummary,
  dirs: string[],
  what: TrashWhat = "files",
  rescan = true,
  mode: TrashOp = "trash",
): void {
  const refresh = openDir !== null && opensTarget(openDir, dirs, folders.ignoreCase);
  if (refresh && what === "sidecars") {
    if (summary.moved.length > 0) {
      clearShownJudgments();
    }
  } else if (refresh) {
    for (const path of summary.moved) {
      // `ratings` / `flags` / `labels` are kept: they are keyed by
      // path and never iterated, so leaving them stale does no harm
      // while the file is out of `allFiles`, and `undoTrash` needs
      // them intact for an undo landing before the rescan repopulates
      // them from the restored sidecar.
      sharpness.delete(path);
      touched.delete(path);
    }
  }
  // The judgments of the trashed files stay: this entry sits above
  // them, so undo reaches them only once the files are back, and
  // `step` skips a file that did not come back. A delete pushes no such
  // entry and the files never come back, so their judgments go.
  if (mode === "delete" && summary.moved.length > 0) {
    const gone = new Set(summary.moved);
    for (const stack of [history, redoable]) {
      stack.map(withoutPaths(gone));
      stack.removeWhere(isEmptyBatch);
    }
  }
  if (summary.run_id !== null) {
    history.push({
      kind: "trash",
      runId: summary.run_id,
      count: summary.moved.length,
      dirs,
      ...(what === "sidecars" ? { what } : {}),
    });
  }
  for (const { path, message } of summary.failed) {
    const failure = mode === "delete" ? "could not delete" : "could not move to the Trash";
    errors.add(path, `${baseName(path)}: ${failure}: ${message}`);
  }
  setStatus(trashedStatus(summary, what, mode));
  if (refresh && rescan) {
    resync("trash");
  } else if (refresh) {
    refreshEntries();
  }
}

// Drop the judgments the strip shows for every file of the open folder, and
// forget which were judged this session, or the next `folder_entries` read
// would keep what the keyboard set.
function clearShownJudgments(): void {
  for (const path of allFiles) {
    touched.delete(path);
    applyRating(path, null, "none", null);
  }
}

function dismissTrash(): void {
  if (trashFlow.dismiss()) {
    trashDialog.hidden = true;
  }
}

// Every key stops here while the dialog is open, so none reaches the strip.
function trashKeydown(event: KeyboardEvent): void {
  if (trashFlow.target?.mode === "delete" && swallowedByDelete(event)) {
    event.preventDefault();
    return;
  }
  const decision = trashKeys.key(keyName(event));
  if (decision.kind === "close") {
    event.preventDefault();
    dismissTrash();
  } else if (decision.kind === "focus") {
    event.preventDefault();
    const buttons = [trashRunButton, trashCancelButton].filter((b) => !b.disabled);
    if (buttons.length === 0) {
      return;
    }
    const from = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[cycleFocus(buttons.length, from, decision.step)].focus();
  }
}

trashRunButton.addEventListener("click", runTrash);
trashCancelButton.addEventListener("click", dismissTrash);

// The folder tree's `Sequence JPEG Timestamps…`: take the export folder
// right-clicked in the tree, preview the times a run would write, then run it
// with progress and cancel. Its errors join the sticky `errors` list,
// keyed by path.
const sequenceFlow = new SequenceFlow();
// Only its key decisions are used: Escape and Tab, as in the settings modal.
const sequenceKeys = new SettingsModal();
const sequenceDialog = document.getElementById("sequence-dialog") as HTMLDivElement;
const sequenceSource = document.getElementById("sequence-source") as HTMLSpanElement;
const sequenceOutput = document.getElementById("sequence-output") as HTMLSpanElement;
const sequenceRebuild = document.getElementById("sequence-rebuild") as HTMLParagraphElement;
const sequenceRows = document.getElementById("sequence-rows") as HTMLOListElement;
const sequenceWritten = document.getElementById("sequence-written") as HTMLParagraphElement;
const sequenceCount = document.getElementById("sequence-count") as HTMLParagraphElement;
const sequenceFailed = document.getElementById("sequence-failed") as HTMLUListElement;
const sequenceRunning = document.getElementById("sequence-running") as HTMLParagraphElement;
const sequenceRunButton = document.getElementById("sequence-run") as HTMLButtonElement;
const sequenceCancelButton = document.getElementById("sequence-cancel") as HTMLButtonElement;
sequenceRunning.textContent = RUNNING_NOTE;

function startSequence(): boolean {
  return (
    formatDialog.hidden &&
    !settings.isOpen &&
    !trashFlow.busy &&
    !sidecarFlow.busy &&
    !dateFolderFlow.busy &&
    !importFlow.busy &&
    sequenceFlow.start()
  );
}

function sequenceTimestampsOf(dir: string): void {
  if (!startSequence() || !sequenceFlow.picked(dir)) {
    return;
  }
  previewSequence(dir).catch(failSequence);
}

function previewSequence(dir: string): Promise<void> {
  return window.__TAURI__.core
    .invoke<SequencePreview>("sequence_preview", { dir })
    .then((preview) => {
      if (sequenceFlow.previewed()) {
        showSequencePreview(dir, preview);
      }
    });
}

function failSequence(err: unknown): void {
  const dir = sequenceFlow.dir;
  sequenceFlow.fail();
  errors.add(
    dir ?? "sequence",
    dir === null ? String(err) : failureText({ path: dir, message: String(err) }),
  );
  setStatus();
}

function showSequencePreview(dir: string, preview: SequencePreview): void {
  setFilterMenuOpen(false);
  setSortMenuOpen(false);
  closeContextMenu();
  sequenceSource.textContent = dir;
  sequenceOutput.textContent = preview.output_dir;
  const notice = rebuildNotice(preview);
  sequenceRebuild.textContent = notice ?? "";
  sequenceRebuild.hidden = notice === null;
  sequenceRows.replaceChildren(
    ...preview.rows.map((row) => {
      const item = document.createElement("li");
      const parts = rowParts(row);
      item.textContent = parts.head + parts.same;
      if (parts.diff !== "") {
        const diff = document.createElement("span");
        diff.className = "changed";
        diff.textContent = parts.diff;
        item.append(diff);
      }
      item.classList.toggle("unchanged", row.old === row.new);
      return item;
    }),
  );
  sequenceWritten.textContent = writtenLine(preview.rows, preview.output_dir);
  sequenceCount.textContent = captureTimeLine(preview.rows);
  sequenceFailed.replaceChildren(
    ...preview.failed.map((failure) => {
      const item = document.createElement("li");
      item.textContent = failureText(failure);
      return item;
    }),
  );
  sequenceRunning.hidden = true;
  sequenceRunButton.disabled = preview.rows.length === 0;
  sequenceDialog.hidden = false;
  (sequenceRunButton.disabled ? sequenceCancelButton : sequenceRunButton).focus();
}

function closeSequenceDialog(): void {
  sequenceDialog.hidden = true;
}

function runSequence(): void {
  const dir = sequenceFlow.run();
  if (dir === null) {
    return;
  }
  sequenceRunButton.disabled = true;
  sequenceRunning.hidden = false;
  sequenceCancelButton.focus();
  sequencing = progressStatus(0, 0);
  renderMeta();
  window.__TAURI__.core
    .invoke<number>("sequence_run", { dir })
    .then((runId) => {
      const { done, cancel } = sequenceFlow.started(runId);
      if (done !== null) {
        finishSequence(done);
      } else if (cancel) {
        void window.__TAURI__.core.invoke("sequence_cancel", { runId });
      }
    })
    .catch((err: unknown) => {
      closeSequenceDialog();
      sequencing = null;
      failSequence(err);
    });
}

function finishSequence(payload: SequenceDone): void {
  if (!sequenceFlow.done(payload)) {
    return;
  }
  closeSequenceDialog();
  for (const failure of payload.failed) {
    errors.add(failure.path, failureText(failure));
  }
  sequencing = null;
  setStatus(doneStatus(payload) ?? undefined);
  const out = revealAfter(payload);
  if (out !== null) {
    window.__TAURI__.core.invoke("reveal_folder", { path: out }).catch((err: unknown) => {
      setStatus(String(err));
    });
  }
}

function dismissSequence(): void {
  const decision = sequenceFlow.dismiss();
  if (decision.kind === "close") {
    closeSequenceDialog();
  } else if (decision.kind === "cancel" && decision.runId !== null) {
    void window.__TAURI__.core.invoke("sequence_cancel", { runId: decision.runId });
  }
}

// Every key stops here while the dialog is open, so none reaches the strip.
function sequenceKeydown(event: KeyboardEvent): void {
  const decision = sequenceKeys.key(keyName(event));
  if (decision.kind === "close") {
    event.preventDefault();
    dismissSequence();
  } else if (decision.kind === "focus") {
    event.preventDefault();
    const buttons = [sequenceRunButton, sequenceCancelButton].filter((b) => !b.disabled);
    const from = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[cycleFocus(buttons.length, from, decision.step)].focus();
  }
}

sequenceRunButton.addEventListener("click", runSequence);
sequenceCancelButton.addEventListener("click", dismissSequence);

void window.__TAURI__.event.listen<{ run_id: number; done: number; total: number }>(
  "sequence-progress",
  ({ payload }) => {
    if (!sequenceFlow.accepts(payload.run_id)) {
      return;
    }
    sequencing = progressStatus(payload.done, payload.total);
    renderMeta();
  },
);

void window.__TAURI__.event.listen<SequenceDone>("sequence-done", ({ payload }) => {
  finishSequence(payload);
});

// The folder tree's `Rewrite Sidecars from Index…` and `Delete Sidecars…`:
// count what the index holds for the folder (or the sidecars on disk),
// confirm, then push it out to the sidecars (or move them to the Trash). The
// run cannot be canceled, so its buttons are disabled and Escape does nothing
// while it runs.
const sidecarFlow = new SidecarFlow();
// Only its key decisions are used: Escape and Tab, as in the settings modal.
const sidecarKeys = new SettingsModal();
const sidecarDialog = document.getElementById("sidecar-dialog") as HTMLDivElement;
const sidecarTitle = document.getElementById("sidecar-title") as HTMLHeadingElement;
const sidecarRows = document.getElementById("sidecar-rows") as HTMLUListElement;
const sidecarTotal = document.getElementById("sidecar-total") as HTMLParagraphElement;
const sidecarRunning = document.getElementById("sidecar-running") as HTMLParagraphElement;
const sidecarRunButton = document.getElementById("sidecar-run") as HTMLButtonElement;
const sidecarCancelButton = document.getElementById("sidecar-cancel") as HTMLButtonElement;

function rewriteSidecarsIn(dir: string): void {
  whenIdle("Rewrite Sidecars from Index", () => {
    if (
      !formatDialog.hidden ||
      settings.isOpen ||
      trashFlow.busy ||
      sequenceFlow.busy ||
      dateFolderFlow.busy ||
      importFlow.busy ||
      !sidecarFlow.start({ kind: "rewrite", dir })
    ) {
      return;
    }
    window.__TAURI__.core
      .invoke<RewritePreview>("rewrite_sidecars_preview", { dir })
      .then((preview) => {
        if (sidecarFlow.previewed()) {
          showRewritePreview(dir, preview);
        }
      })
      .catch((err: unknown) => {
        sidecarFlow.end();
        setStatus(String(err));
      });
  });
}

function deleteSidecarsIn(dir: string): void {
  whenIdle("Delete Sidecars", () => {
    if (
      !formatDialog.hidden ||
      settings.isOpen ||
      trashFlow.busy ||
      sequenceFlow.busy ||
      dateFolderFlow.busy ||
      importFlow.busy ||
      !sidecarFlow.start({ kind: "delete", dir })
    ) {
      return;
    }
    window.__TAURI__.core
      .invoke<DeletePreview>("delete_sidecars_preview", { dir })
      .then((preview) => {
        if (sidecarFlow.previewed()) {
          showSidecarDialog(
            "Delete Sidecars",
            deleteRows(preview),
            deleteTotalLine(preview, baseName(dir) || dir),
            { note: DELETE_RUNNING_NOTE, label: "Move to Trash", style: "destructive" },
          );
        }
      })
      .catch((err: unknown) => {
        sidecarFlow.end();
        setStatus(String(err));
      });
  });
}

function showRewritePreview(dir: string, preview: RewritePreview): void {
  showSidecarDialog(
    "Rewrite Sidecars from Index",
    rewriteRows(preview),
    rewriteTotalLine(preview, baseName(dir) || dir),
    { note: REWRITE_RUNNING_NOTE, label: "Rewrite", style: "primary" },
  );
}

function showSidecarDialog(
  title: string,
  rows: string[],
  total: string,
  run: { note: string; label: string; style: "primary" | "destructive" },
): void {
  setFilterMenuOpen(false);
  setSortMenuOpen(false);
  closeContextMenu();
  sidecarTitle.textContent = title;
  sidecarRows.replaceChildren(...rows.map(listItem));
  sidecarTotal.textContent = total;
  sidecarRunning.textContent = run.note;
  sidecarRunning.hidden = true;
  sidecarRunButton.textContent = run.label;
  sidecarRunButton.className = `button ${run.style}`;
  sidecarRunButton.disabled = false;
  sidecarCancelButton.disabled = false;
  sidecarDialog.hidden = false;
  sidecarRunButton.focus();
}

function closeSidecarDialog(): void {
  sidecarFlow.end();
  sidecarDialog.hidden = true;
}

// Rewriting the open folder leaves the strip alone: the index it writes from
// is what the strip already shows. Deleting its sidecars drops the strip's
// judgments (see `trashed`), and the run goes onto the undo history with the
// trash runs.
function runSidecars(): void {
  const target = sidecarFlow.run();
  if (target === null) {
    return;
  }
  const { dir } = target;
  sidecarRunButton.disabled = true;
  sidecarCancelButton.disabled = true;
  sidecarRunning.hidden = false;
  if (target.kind === "delete") {
    settleIdle(
      window.__TAURI__.core
        .invoke<TrashSummary>("delete_sidecars_run", { dir })
        .then((summary) => {
          closeSidecarDialog();
          if (summary.run_id !== null) {
            redoable.clear();
          }
          trashed(summary, [dir], "sidecars", false);
        })
        .catch((err: unknown) => {
          closeSidecarDialog();
          setStatus(String(err));
        }),
    );
    return;
  }
  settleIdle(
    window.__TAURI__.core
      .invoke<RewriteSummary>("rewrite_sidecars_run", { dir })
      .then((summary) => {
        closeSidecarDialog();
        setStatus(rewrittenStatus(summary, baseName(dir) || dir));
      })
      .catch((err: unknown) => {
        closeSidecarDialog();
        setStatus(String(err));
      }),
  );
}

function dismissSidecars(): void {
  if (sidecarFlow.dismiss()) {
    sidecarDialog.hidden = true;
  }
}

// Every key stops here while the dialog is open, so none reaches the strip.
function sidecarKeydown(event: KeyboardEvent): void {
  const decision = sidecarKeys.key(keyName(event));
  if (decision.kind === "close") {
    event.preventDefault();
    dismissSidecars();
  } else if (decision.kind === "focus") {
    event.preventDefault();
    const buttons = [sidecarRunButton, sidecarCancelButton].filter((b) => !b.disabled);
    if (buttons.length === 0) {
      return;
    }
    const from = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[cycleFocus(buttons.length, from, decision.step)].focus();
  }
}

sidecarRunButton.addEventListener("click", runSidecars);
sidecarCancelButton.addEventListener("click", dismissSidecars);

// The folder tree's `Rename by Capture Date…`: plan the rename or split of
// the folder from its photos' capture dates, confirm, then carry it out. A
// press during a scan is refused by the backend with the scan message rather
// than held for the scan's end, since the plan could be stale by then. The
// run cannot be canceled, so its buttons are disabled and Escape does nothing
// while it runs.
const dateFolderFlow = new DateFolderFlow();
// Only its key decisions are used: Escape and Tab, as in the settings modal.
const dateFolderKeys = new SettingsModal();
const dateFolderDialog = document.getElementById("datefolder-dialog") as HTMLDivElement;
const dateFolderTotal = document.getElementById("datefolder-total") as HTMLParagraphElement;
const dateFolderRows = document.getElementById("datefolder-rows") as HTMLUListElement;
const dateFolderUndated = document.getElementById("datefolder-undated") as HTMLParagraphElement;
const dateFolderNote = document.getElementById("datefolder-note") as HTMLParagraphElement;
const dateFolderReason = document.getElementById("datefolder-reason") as HTMLParagraphElement;
const dateFolderRunning = document.getElementById("datefolder-running") as HTMLParagraphElement;
const dateFolderRunButton = document.getElementById("datefolder-run") as HTMLButtonElement;
const dateFolderCancelButton = document.getElementById("datefolder-cancel") as HTMLButtonElement;
dateFolderRunning.textContent = DATE_RUNNING_NOTE;

function renameByDateIn(dir: string): void {
  if (
    !formatDialog.hidden ||
    settings.isOpen ||
    trashFlow.busy ||
    sequenceFlow.busy ||
    sidecarFlow.busy ||
    importFlow.busy ||
    !dateFolderFlow.start(dir)
  ) {
    return;
  }
  window.__TAURI__.core
    .invoke<DatePreview>("date_folder_preview", { dir })
    .then((preview) => {
      if (dateFolderFlow.previewed()) {
        showDateFolderPreview(preview);
      }
    })
    .catch((err: unknown) => {
      dateFolderFlow.end();
      setStatus(String(err));
    });
}

function showDateFolderPreview(preview: DatePreview): void {
  setFilterMenuOpen(false);
  setSortMenuOpen(false);
  closeContextMenu();
  const reason = disabledReason(preview);
  dateFolderTotal.textContent = dateTitleLine(preview);
  dateFolderRows.replaceChildren(...dateRows(preview).map(listItem));
  dateFolderUndated.textContent = undatedLine(preview) ?? "";
  dateFolderNote.textContent = removalNote(preview) ?? "";
  dateFolderReason.textContent = reason ?? "";
  dateFolderRunning.hidden = true;
  dateFolderRunButton.disabled = reason !== null;
  dateFolderCancelButton.disabled = false;
  dateFolderDialog.hidden = false;
  (reason === null ? dateFolderRunButton : dateFolderCancelButton).focus();
}

function closeDateFolderDialog(): void {
  dateFolderFlow.end();
  dateFolderDialog.hidden = true;
}

// A rename follows as the tree's inline rename does. A split re-lists the
// parent in the tree; when the split folder was the open one, it is re-listed
// if it remains, else the first date folder opens.
function runDateFolder(): void {
  const dir = dateFolderFlow.run();
  if (dir === null) {
    return;
  }
  dateFolderRunButton.disabled = true;
  dateFolderCancelButton.disabled = true;
  dateFolderRunning.hidden = false;
  settleIdle(
    window.__TAURI__.core.invoke<DateRan>("date_folder_run", { dir }).then(
      (ran) => {
        closeDateFolderDialog();
        if (ran.kind === "renamed") {
          return folderRenamed(dir, ran.path, baseName(ran.path), ran.warning);
        }
        for (const { path, message } of ran.failed) {
          errors.add(path, `${baseName(path)}: could not move: ${message}`);
        }
        setStatus(splitStatus(ran, baseName(dir) || dir));
        folders.refresh(parentOf(dir));
        if (openDir === null || !opensTarget(openDir, [dir], folders.ignoreCase)) {
          return;
        }
        const first = ran.moved[0];
        if (!ran.removed) {
          resync("rename");
        } else if (first !== undefined) {
          return openDirectory(first.path, newFolderToken()).catch((err: unknown) => {
            setStatus(String(err));
          });
        }
      },
      (err: unknown) => {
        closeDateFolderDialog();
        setStatus(String(err));
      },
    ),
  );
}

function dismissDateFolder(): void {
  if (dateFolderFlow.dismiss()) {
    dateFolderDialog.hidden = true;
  }
}

// Every key stops here while the dialog is open, so none reaches the strip.
function dateFolderKeydown(event: KeyboardEvent): void {
  const decision = dateFolderKeys.key(keyName(event));
  if (decision.kind === "close") {
    event.preventDefault();
    dismissDateFolder();
  } else if (decision.kind === "focus") {
    event.preventDefault();
    const buttons = [dateFolderRunButton, dateFolderCancelButton].filter((b) => !b.disabled);
    if (buttons.length === 0) {
      return;
    }
    const from = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[cycleFocus(buttons.length, from, decision.step)].focus();
  }
}

dateFolderRunButton.addEventListener("click", runDateFolder);
dateFolderCancelButton.addEventListener("click", dismissDateFolder);

// The folder tree's `Import…`: plan copying every photo under the
// right-clicked folder (a memory card's root included) into the date folders
// of the import settings, then run the copy with progress and cancel. The run
// does not touch the index nor wait for a scan. Once it has started, its first
// date folder opens and the dialog steps aside, so the strip fills through the
// folder watcher and can be culled meanwhile; `Import…` again brings the
// dialog back to cancel. Its errors join the sticky `errors` list, keyed by
// path.
const importFlow = new ImportFlow();
// Only its key decisions are used: Escape and Tab, as in the settings modal.
const importKeys = new SettingsModal();
const importDialog = document.getElementById("import-dialog") as HTMLDivElement;
const importSource = document.getElementById("import-source") as HTMLSpanElement;
const importTotal = document.getElementById("import-total") as HTMLParagraphElement;
const importRows = document.getElementById("import-rows") as HTMLUListElement;
const importCollisionsHeading = document.getElementById(
  "import-collisions-heading",
) as HTMLParagraphElement;
const importCollisions = document.getElementById("import-collisions") as HTMLUListElement;
const importUndatedHeading = document.getElementById(
  "import-undated-heading",
) as HTMLParagraphElement;
const importUndated = document.getElementById("import-undated") as HTMLUListElement;
const importNoteEl = document.getElementById("import-note") as HTMLParagraphElement;
const importRunning = document.getElementById("import-running") as HTMLParagraphElement;
const importRunButton = document.getElementById("import-run") as HTMLButtonElement;
const importCancelButton = document.getElementById("import-cancel") as HTMLButtonElement;
importRunning.textContent = IMPORT_RUNNING_NOTE;

function importFrom(dir: string): void {
  if (
    !formatDialog.hidden ||
    settings.isOpen ||
    trashFlow.busy ||
    sequenceFlow.busy ||
    sidecarFlow.busy ||
    dateFolderFlow.busy
  ) {
    return;
  }
  if (importFlow.reopen()) {
    setFilterMenuOpen(false);
    setSortMenuOpen(false);
    closeContextMenu();
    importDialog.hidden = false;
    importCancelButton.focus();
    return;
  }
  if (!importFlow.start(dir)) {
    return;
  }
  imported = null;
  importing = `reading ${dir}…`;
  renderMeta();
  window.__TAURI__.core
    .invoke<ImportPreview>("import_preview", { dir })
    .then((preview) => {
      importing = null;
      renderMeta();
      if (importFlow.previewed(preview.id)) {
        showImportPreview(preview);
      }
    })
    .catch(failImport);
}

function failImport(err: unknown): void {
  importing = null;
  renderMeta();
  const dir = importFlow.dir;
  importFlow.fail();
  errors.add(
    dir ?? "import",
    dir === null ? String(err) : importFailureText({ path: dir, message: String(err) }),
  );
  setStatus();
}

function showImportPreview(preview: ImportPreview): void {
  setFilterMenuOpen(false);
  setSortMenuOpen(false);
  closeContextMenu();
  importSource.textContent = preview.source;
  importTotal.textContent = preview.refusal === null ? totalLine(preview) : "";
  importRows.replaceChildren(...folderRows(preview).map(listItem));
  const collisions = collisionItems(preview);
  importCollisionsHeading.textContent = collisions.length === 0 ? "" : COLLISIONS_HEADING;
  importCollisions.replaceChildren(...collisions.map(listItem));
  importUndatedHeading.textContent = preview.undated.length === 0 ? "" : UNDATED_HEADING;
  importUndated.replaceChildren(...preview.undated.map(listItem));
  importNoteEl.textContent = importNote(preview) ?? "";
  importRunning.hidden = true;
  importRunButton.disabled = !canRun(preview);
  importDialog.hidden = false;
  (importRunButton.disabled ? importCancelButton : importRunButton).focus();
}

function closeImportDialog(): void {
  importDialog.hidden = true;
}

function runImport(): void {
  const previewId = importFlow.run();
  if (previewId === null) {
    return;
  }
  importRunButton.disabled = true;
  importRunning.hidden = false;
  importCancelButton.focus();
  importing = importProgressStatus(0, 0);
  renderMeta();
  window.__TAURI__.core
    .invoke<ImportStarted>("import_run", { previewId })
    .then(({ runId, folders: created }) => {
      const { done, cancel } = importFlow.started(runId);
      if (!importFlow.isOpen) {
        closeImportDialog();
      }
      const first = created[0];
      // The body of `finishImport` waits for this open, whose own `errors.clear()`
      // and status would otherwise wipe the failures it lists, and which must
      // have `openDir` set for the resync check.
      const opening =
        first === undefined
          ? Promise.resolve()
          : openDirectory(first, newFolderToken()).catch((err: unknown) => {
              setStatus(String(err));
            });
      importOpening = opening;
      if (done !== null) {
        finishImport(done);
      } else if (cancel) {
        void window.__TAURI__.core.invoke("import_cancel", { runId });
      }
    })
    .catch((err: unknown) => {
      closeImportDialog();
      importing = null;
      failImport(err);
    });
}

// The open of the running import's first date folder, if any.
let importOpening: Promise<void> = Promise.resolve();

function finishImport(payload: ImportDone): void {
  if (!importFlow.done(payload)) {
    return;
  }
  closeImportDialog();
  void importOpening.then(() => {
    reportImport(payload);
  });
}

function reportImport(payload: ImportDone): void {
  for (const failure of payload.failed) {
    errors.add(failure.path, importFailureText(failure));
  }
  importing = null;
  imported = importDoneStatus(payload);
  renderMeta();
  // A file that landed between the open folder's listing and its watch
  // being set reaches the strip here at the latest.
  if (openDir !== null && opensTarget(openDir, payload.folders, folders.ignoreCase)) {
    resync("import");
  }
}

function dismissImport(key = false): void {
  const decision = importFlow.dismiss(key);
  if (decision.kind === "close" || decision.kind === "aside") {
    closeImportDialog();
  } else if (decision.kind === "cancel" && decision.runId !== null) {
    void window.__TAURI__.core.invoke("import_cancel", { runId: decision.runId });
  }
}

// Every key stops here while the dialog is open, so none reaches the strip.
function importKeydown(event: KeyboardEvent): void {
  const decision = importKeys.key(keyName(event));
  if (decision.kind === "close") {
    event.preventDefault();
    dismissImport(true);
  } else if (decision.kind === "focus") {
    event.preventDefault();
    const buttons = [importRunButton, importCancelButton].filter((b) => !b.disabled);
    const from = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[cycleFocus(buttons.length, from, decision.step)].focus();
  }
}

importRunButton.addEventListener("click", runImport);
importCancelButton.addEventListener("click", () => {
  dismissImport();
});

void window.__TAURI__.event.listen<ImportProgress>("import-progress", ({ payload }) => {
  if (!importFlow.accepts(payload.runId)) {
    return;
  }
  importing = importProgressStatus(payload.filesDone, payload.filesTotal);
  renderMeta();
});

void window.__TAURI__.event.listen<ImportDone>("import-done", ({ payload }) => {
  finishImport(payload);
});

// Set the transient note, or clear it when called with no argument.
function setStatus(extra?: string): void {
  note = extra;
  renderMeta();
}

function draw(): void {
  if (comparing) {
    drawCompare();
    return;
  }
  if (zoomed) {
    drawZoom();
    return;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const dpr = window.devicePixelRatio;
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  context.clearRect(0, 0, canvas.width, canvas.height);
  if (shown === null) {
    return;
  }
  const { bitmap, orientation } = shown;
  const quarterTurn = orientation === 6 || orientation === 8;
  const uprightWidth = quarterTurn ? bitmap.height : bitmap.width;
  const uprightHeight = quarterTurn ? bitmap.width : bitmap.height;
  const scale = Math.min(width / uprightWidth, height / uprightHeight);
  const drawWidth = bitmap.width * scale;
  const drawHeight = bitmap.height * scale;
  context.save();
  context.translate(canvas.width / 2, canvas.height / 2);
  context.scale(dpr, dpr);
  if (orientation === 6) {
    context.rotate(Math.PI / 2);
  } else if (orientation === 8) {
    context.rotate(-Math.PI / 2);
  } else if (orientation === 3) {
    context.rotate(Math.PI);
  }
  context.drawImage(bitmap, -drawWidth / 2, -drawHeight / 2, drawWidth, drawHeight);
  drawFocusMark(drawWidth, drawHeight);
  drawFaceMarks(drawWidth, drawHeight);
  drawFaceMesh(drawWidth, drawHeight);
  context.restore();
  drawFaceLabels(drawWidth, drawHeight, orientation);
}

// The scrolling time axis redraws on this timer too, so the graph moves
// between samples and latency points.
const PERF_REDRAW_MS = 500;
const PERF_HEADER = 16;
const PERF_SWATCH = 14;
// CPU lines are blue and disk lines green; within each pair Riffle's is thick
// and solid, the system's thin and dashed, and the legend draws each label's
// line so the series can be told apart. `value` is the series' height as a
// fraction of the plot, null where it has no reading.
interface PerfLine {
  label: string;
  color: string;
  width: number;
  dash: readonly number[];
  value: (sample: PerfSample, readMax: number) => number | null;
}
const PERF_LINES: readonly PerfLine[] = [
  {
    label: "Riffle CPU",
    color: THEME.chartCpu,
    width: 2,
    dash: [],
    value: (sample) => sample.process / 100,
  },
  {
    label: "System CPU",
    color: THEME.chartCpu,
    width: 1,
    dash: [4, 3],
    value: (sample) => sample.system / 100,
  },
  {
    label: "Riffle disk",
    color: THEME.chartDisk,
    width: 2,
    dash: [],
    value: (sample, readMax) => Math.min(sample.read, readMax) / readMax,
  },
  {
    label: "System disk",
    color: THEME.chartDisk,
    width: 1,
    dash: [4, 3],
    value: (sample) => (sample.disk === null ? null : sample.disk / 100),
  },
];
let perfTimer: number | null = null;
let perfUnlisten: Promise<() => void> | null = null;

// The sampler runs only while the overlay is shown; the startup call stops one
// a reloaded window left running.
void window.__TAURI__.core.invoke("set_perf_sampling", { enabled: false }).catch(() => {});

function togglePerf(): void {
  if (perf.on) {
    perf.stop();
    perfCanvas.hidden = true;
    if (perfTimer !== null) {
      window.clearInterval(perfTimer);
      perfTimer = null;
    }
    void perfUnlisten?.then((unlisten) => unlisten());
    perfUnlisten = null;
    void window.__TAURI__.core.invoke("set_perf_sampling", { enabled: false }).catch(() => {});
    return;
  }
  perf.start();
  perfCanvas.hidden = false;
  perfUnlisten = window.__TAURI__.event.listen<Omit<PerfSample, "at">>(
    "perf-sample",
    ({ payload }) => {
      perf.addSample({ at: performance.now(), ...payload });
      drawPerf();
    },
  );
  perfTimer = window.setInterval(drawPerf, PERF_REDRAW_MS);
  void window.__TAURI__.core.invoke("set_perf_sampling", { enabled: true }).catch(() => {});
  drawPerf();
}

// The CPU and disk busy series on a 0-100 % scale, Riffle's reads on a
// 0-`readMax` scale and the latency bars on a 0-`latencyMax` scale share the
// plot below a one-line legend, the newest at the right edge.
function drawPerf(): void {
  if (perfCanvas.hidden) {
    return;
  }
  const width = perfCanvas.clientWidth;
  const height = perfCanvas.clientHeight;
  const dpr = window.devicePixelRatio;
  perfCanvas.width = Math.round(width * dpr);
  perfCanvas.height = Math.round(height * dpr);
  const ctx = perfContext;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 0.85;
  ctx.fillStyle = THEME.card;
  ctx.fillRect(0, 0, width, height);
  ctx.globalAlpha = 1;
  const now = performance.now();
  const { samples, latency, latencyMax, readMax } = perf.view(now);
  const bottom = height - 4;
  const plot = bottom - PERF_HEADER - 4;
  const x = (at: number): number => width - ((now - at) / WINDOW_MS) * width;
  ctx.strokeStyle = THEME.border;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (const fraction of [0, 0.5, 1]) {
    const y = Math.round(bottom - fraction * plot) + 0.5;
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
  }
  ctx.stroke();
  ctx.fillStyle = THEME.chartLatency;
  for (const point of latency) {
    const barHeight = (Math.min(point.ms, latencyMax) / latencyMax) * plot;
    ctx.fillRect(x(point.at) - 1, bottom - barHeight, 2, barHeight);
  }
  ctx.lineJoin = "round";
  // A series without a reading (the system disk on macOS) has no line and no
  // legend entry.
  const shown = PERF_LINES.filter(
    (line) =>
      line.label !== "System disk" ||
      samples.some((sample) => line.value(sample, readMax) !== null),
  );
  for (const line of shown) {
    ctx.strokeStyle = line.color;
    ctx.lineWidth = line.width;
    ctx.setLineDash([...line.dash]);
    ctx.beginPath();
    let drawing = false;
    for (const sample of samples) {
      const value = line.value(sample, readMax);
      if (value === null) {
        drawing = false;
        continue;
      }
      const y = bottom - value * plot;
      if (drawing) ctx.lineTo(x(sample.at), y);
      else ctx.moveTo(x(sample.at), y);
      drawing = true;
    }
    ctx.stroke();
  }
  ctx.font = "11px system-ui, sans-serif";
  ctx.textBaseline = "top";
  ctx.textAlign = "left";
  let left = 6;
  for (const [text, color, line] of [
    ["100%  60 s", THEME.mutedForeground, null],
    ...shown.map((line) => [line.label, line.color, line] as const),
    ["Latency", THEME.chartLatency, null],
  ] as const) {
    if (line !== null) {
      const y = 9.5;
      ctx.strokeStyle = color;
      ctx.lineWidth = line.width;
      ctx.setLineDash([...line.dash]);
      ctx.beginPath();
      ctx.moveTo(left, y);
      ctx.lineTo(left + PERF_SWATCH, y);
      ctx.stroke();
      left += PERF_SWATCH + 4;
    }
    ctx.fillStyle = color;
    ctx.fillText(text, left, 3);
    left += ctx.measureText(text).width + 12;
  }
  ctx.setLineDash([]);
  ctx.textAlign = "right";
  ctx.fillStyle = THEME.mutedForeground;
  ctx.fillText(
    `${Math.round(readMax / 1_000_000)} MB/s  ${Math.round(latencyMax)} ms`,
    width - 6,
    3,
  );
}

function compareCandidates(): string[] {
  return comparisonCandidates(
    files,
    index,
    selection.selected,
    bursts,
    (path) => photoTier(entries.get(path)?.focus) === "good",
  );
}

function closeCompareFrames(): void {
  for (const frame of compareFrames) frame.bitmap.close();
  compareFrames = [];
}

function stopComparing(): void {
  comparing = false;
  compareSeq += 1;
  closeCompareFrames();
  compareActivePath = null;
  renderEmpty();
}

function drawCompare(): void {
  const dpr = window.devicePixelRatio;
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.save();
  context.scale(dpr, dpr);
  const count = compareFrames.length;
  const columns = count <= 2 ? Math.max(count, 1) : 2;
  const rows = Math.max(Math.ceil(count / columns), 1);
  const gap = 4;
  const cellWidth = (width - gap * (columns - 1)) / columns;
  const cellHeight = (height - gap * (rows - 1)) / rows;
  compareFrames.forEach(({ path, bitmap, orientation }, at) => {
    const col = at % columns;
    const row = Math.floor(at / columns);
    const x = col * (cellWidth + gap);
    const y = row * (cellHeight + gap);
    const labelHeight = 28;
    const quarterTurn = orientation === 6 || orientation === 8;
    const uprightWidth = quarterTurn ? bitmap.height : bitmap.width;
    const uprightHeight = quarterTurn ? bitmap.width : bitmap.height;
    const scale = Math.min(cellWidth / uprightWidth, (cellHeight - labelHeight) / uprightHeight);
    const drawWidth = bitmap.width * scale;
    const drawHeight = bitmap.height * scale;
    context.save();
    context.beginPath();
    context.rect(x, y, cellWidth, cellHeight - labelHeight);
    context.clip();
    context.translate(x + cellWidth / 2, y + (cellHeight - labelHeight) / 2);
    if (orientation === 6) context.rotate(Math.PI / 2);
    else if (orientation === 8) context.rotate(-Math.PI / 2);
    else if (orientation === 3) context.rotate(Math.PI);
    context.drawImage(bitmap, -drawWidth / 2, -drawHeight / 2, drawWidth, drawHeight);
    context.restore();
    const isActive = path === compareActivePath;
    context.fillStyle = THEME.card;
    context.fillRect(x, y + cellHeight - labelHeight, cellWidth, labelHeight);
    context.fillStyle = THEME.foreground;
    context.font = "12px system-ui, sans-serif";
    context.textBaseline = "middle";
    const suffix = isActive ? "  ACTIVE" : "";
    context.fillText(`${baseName(path)}${suffix}`, x + 8, y + cellHeight - labelHeight / 2);
    context.strokeStyle = THEME.border;
    context.lineWidth = 1;
    context.strokeRect(x + 0.5, y + 0.5, cellWidth - 1, cellHeight - 1);
    if (isActive) {
      context.strokeStyle = THEME.primary;
      context.lineWidth = 2;
      context.strokeRect(x + 3.5, y + 3.5, cellWidth - 7, cellHeight - 7);
    }
  });
  context.restore();
}

async function loadCompare(): Promise<void> {
  const paths = compareCandidates();
  compareActivePath = reconcileActive(paths, compareActivePath, files[index]);
  if (paths.length < 2) {
    stopComparing();
    renderMeta();
    draw();
    return;
  }
  const request = ++compareSeq;
  closeCompareFrames();
  drawCompare();
  try {
    const frames = await loadComparisonFrames(
      paths,
      async (path) => {
        const payload = await window.__TAURI__.core.invoke<ArrayBuffer>("preview", { path });
        const header = new DataView(payload, 0, PREVIEW_HEADER_LEN);
        if (header.getUint16(0, true) !== PREVIEW_KIND_JPEG_V1) {
          throw new Error("unknown preview payload");
        }
        const orientation = header.getUint16(2, true);
        const bitmap = await createImageBitmap(
          new Blob([payload.slice(PREVIEW_HEADER_LEN)], { type: "image/jpeg" }),
        );
        return { path, bitmap, orientation };
      },
      (frame) => frame.bitmap.close(),
    );
    if (!comparing || request !== compareSeq) {
      for (const frame of frames) frame.bitmap.close();
      return;
    }
    compareFrames = frames;
    drawCompare();
  } catch (error) {
    if (comparing && request === compareSeq) setStatus(String(error));
  }
}

function toggleCompare(): void {
  if (comparing) {
    stopComparing();
    renderMeta();
    draw();
    return;
  }
  const candidates = compareCandidates();
  if (candidates.length < 2) {
    setStatus(COMPARE_NEEDS_FRAMES);
    return;
  }
  comparing = true;
  compareActivePath = files[index] ?? candidates[0];
  if (zoomed) toggleZoom();
  renderMeta();
  void loadCompare();
}

// Make the comparison pane under the pointer the active one. Returns whether
// a pane was hit.
function activateComparePane(clientX: number, clientY: number): boolean {
  if (!comparing || compareFrames.length === 0) return false;
  const rect = canvas.getBoundingClientRect();
  const at = comparePaneAt(
    clientX - rect.left,
    clientY - rect.top,
    rect.width,
    rect.height,
    compareFrames.length,
  );
  const frame = at === null ? undefined : compareFrames[at];
  if (frame === undefined) return false;
  compareActivePath = frame.path;
  drawCompare();
  renderMeta();
  return true;
}

canvas.addEventListener("click", (event) => {
  activateComparePane(event.clientX, event.clientY);
});

canvas.addEventListener("contextmenu", (event) => {
  event.preventDefault();
  if (files.length === 0) return;
  if (comparing && !activateComparePane(event.clientX, event.clientY)) return;
  openContextMenu(event.clientX, event.clientY);
});

// Record a judgment locally: the `ratings` and `flags` maps and the strip
// cell. `null` (or `0`) is unrated.
function applyRating(
  path: string,
  rating: number | null,
  flag: PickFlag,
  label: string | null,
): void {
  if (rating === null || rating === 0) {
    ratings.delete(path);
  } else {
    ratings.set(path, rating);
  }
  if (flag === "none") {
    flags.delete(path);
  } else {
    flags.set(path, flag);
  }
  if (label === null) {
    labels.delete(path);
  } else {
    labels.set(path, label);
  }
  const at = fileIndex.get(path);
  if (at !== undefined) {
    strip.setRating(at, ratings.get(path) ?? null, flag, label);
  }
}

function flagOf(path: string): PickFlag {
  return flags.get(path) ?? "none";
}

function passes(path: string): boolean {
  return filterPasses(
    filterState,
    {
      rating: ratings.get(path) ?? null,
      flag: flagOf(path),
      label: labels.get(path) ?? null,
    },
    entries.get(path)?.exif,
    entries.get(path)?.orientation,
    afEyeState(entries.get(path)?.focus),
    entries.get(path)?.focus?.eyes,
    fileGenres.get(path),
    outputOf(fileOutputs, outputsListing, path),
  );
}

// The sort in force: `recommended` reads as capture order while the ranking
// is off.
function sortInForce(): SortKey {
  const key = sortFor(viewOnly, sortKey);
  return key === "recommended" && !rankingOn ? "capture" : key;
}

function ordered(): string[] {
  return orderFiles(sortInForce(), allFiles, (path) => {
    const entry = entries.get(path);
    return {
      captureTime: entry?.capture_time ?? undefined,
      subsec: entry?.subsec ?? undefined,
      // A reject sorts after the unrated files whatever its stars.
      rating: flagOf(path) === "reject" ? -1 : ratings.get(path),
      burst: bursts.get(path)?.burst,
      recommended: fileRanks.get(path)?.score,
    };
  });
}

// Rebuild `files` from `allFiles` after a filter, sort or judgment change.
// The current file stays current if it still passes; otherwise the next
// passing file after it (in sort order) takes over, or the last one before it,
// or the empty view. A judgment that drops the current file out of the
// filter therefore hides it at once and moves on to the next passing file.
// Returns whether it rebuilt the strip with `strip.setFiles`.
//
// `force` skips the unchanged-list early return: the first `folder_entries`
// refresh after a folder opens with a pending resume target passes it, since
// that refresh's list is often identical to the pre-entries one (same name
// order, no judgment filter), yet `index` still needs to move onto `anchor`.
// It also forces `show()` even when `files[index]` lands on `anchor` itself,
// since `openDirectory` only ever showed `files[0]`, so a resumed file that
// is its own anchor still has not been shown yet.
//
// `previous` is `resync`'s list from before the re-listing, so an anchor
// deleted from outside the app gives way to its neighbour there.
function refilter(
  anchor: string | undefined = files[index],
  keepScroll = false,
  force = false,
  previous?: readonly string[],
): boolean {
  const order = ordered();
  const next = order.filter(passes);
  if (!filterListChanged(files, next, force)) {
    if (comparing) void loadCompare();
    return false;
  }
  files = next;
  fileIndex.clear();
  files.forEach((path, at) => {
    fileIndex.set(path, at);
  });
  strip.setFiles(files, keepScroll);
  files.forEach((path, at) => {
    strip.setRating(at, ratings.get(path) ?? null, flagOf(path), labels.get(path) ?? null);
  });
  applyBursts();
  applyTiers();
  applyRecommended();
  applyFailures();
  if (files.length === 0) {
    closeContextMenu();
    index = 0;
    selection = prune(selection, files, index);
    if (comparing) stopComparing();
    seq += 1;
    dwell.cancel();
    shown?.bitmap.close();
    shown = null;
    meta = null;
    zoomed = false;
    draw();
    renderMeta();
    return true;
  }
  const target = anchorAfterFilter(order, passes, anchor, previous);
  index = (target === undefined ? undefined : fileIndex.get(target)) ?? 0;
  selection = settle(selection, files, index, force);
  paintSelection();
  if (mustReshow(force, files[index], anchor)) {
    show();
  } else {
    strip.setCurrent(index, !keepScroll);
    renderMeta();
    if (comparing) void loadCompare();
  }
  return true;
}

// A judgment key over the selection's targets: the command's new value is
// decided once from the focused file and set on every target, each keeping
// the fields the command does not touch. Update the maps and redraw first,
// then tell the backend per file. The invokes are never awaited for anything
// visible; only a failure is, which reverts that file (if the folder is still
// the one it belongs to) and says so in the status line. Returns the number
// of targets it addressed, changed or not (a repeated value still advances),
// else 0 when there are no files or the folder is view-only.
function judge(command: Command, forceLabel = false): number {
  if (files.length === 0 || viewOnly) {
    return 0;
  }
  const current = (comparing ? compareActivePath : null) ?? files[index];
  const paths = comparing ? [current] : targets(selection, files, index);
  record(paths, current, command, forceLabel);
  return paths.length;
}

// The changes `command` makes to `paths`, its value decided from `focused`,
// pushed as one undo entry and committed with `anchor` kept current. Returns
// whether anything changed.
function record(
  paths: readonly string[],
  focused: string,
  command: Command,
  forceLabel = false,
  anchor = focused,
): boolean {
  if (viewOnly) {
    return false;
  }
  // Writes nothing and pushes no undo entry when the pressed value is already
  // the current one (the caller may still advance). A forced label still goes
  // out while the file's real label is unknown.
  const changes: Change[] = judgments(
    paths,
    focused,
    (path) => ({
      rating: ratings.get(path) ?? null,
      flag: flagOf(path),
      label: labels.get(path) ?? null,
    }),
    command,
    (path) => forceLabel && !entries.has(path),
  ).map(({ before, after }) => ({ before, ...after, forceLabel }));
  if (changes.length === 0) {
    return false;
  }
  const batch = changes.map(({ before }) => before);
  history.push(batch);
  redoable.clear();
  commit(changes, forgetOnFail(history, batch), () => anchor);
  return true;
}

// `rejectRest`: reject every other member of the current file's burst, over
// `allFiles` so a member the filter hides is rejected too, as one undo entry.
// Members already rejected are skipped, so the batch holds real changes only.
function rejectRest(): void {
  if (viewOnly || files.length === 0) {
    return;
  }
  const current = files[index];
  const burst = bursts.get(current);
  if (burst === undefined || burst.size < 2) {
    return;
  }
  const changes: Change[] = [];
  for (const path of allFiles) {
    if (path === current || bursts.get(path)?.burst !== burst.burst) {
      continue;
    }
    const before = {
      path,
      rating: ratings.get(path) ?? null,
      flag: flagOf(path),
      label: labels.get(path) ?? null,
    };
    if (before.flag === "reject") {
      continue;
    }
    changes.push({ before, rating: before.rating, flag: "reject", label: before.label });
  }
  if (changes.length === 0) {
    return;
  }
  const batch = changes.map(({ before }) => before);
  history.push(batch);
  redoable.clear();
  // The current file stays current unless the filter now hides it.
  commit(changes, forgetOnFail(history, batch), () => current);
}

// A failed write drops its file from `batch`, and the batch from `from` once
// every file of it has failed: the rest still changed and stay undoable.
function forgetOnFail(
  from: History<Entry<Judgment>>,
  batch: Judgment[],
): (failed: Judgment) => void {
  return (failed) => {
    const at = batch.indexOf(failed);
    if (at !== -1) {
      batch.splice(at, 1);
    }
    if (batch.length === 0) {
      from.remove(batch);
    }
  };
}

// `forceLabel` sends `labelKnown: true` even before `folder_entries` has told
// us the file's label, so the sidecar's label is cleared whatever it is.
type Change = {
  before: Judgment;
  rating: number | null;
  flag: PickFlag;
  label: string | null;
  forceLabel?: boolean;
};

// Apply every change locally and refilter once, then tell the backend per
// file; each `before` is its file's state to revert to when its invoke fails.
// `anchor` picks the file to keep current once the new state is applied (the
// first change's file by default).
function commit(
  changes: Change[],
  onFail?: (failed: Judgment) => void,
  anchor?: () => string | undefined,
): void {
  for (const { before, rating, flag, label } of changes) {
    touched.add(before.path);
    applyRating(before.path, rating, flag, label);
  }
  renderMeta();
  refilter(anchor === undefined ? changes[0].before.path : anchor());
  for (const change of changes) {
    send(change, onFail);
  }
}

function send(
  { before, rating, flag, label, forceLabel }: Change,
  onFail?: (failed: Judgment) => void,
): void {
  const { path } = before;
  const token = folderToken;
  // `label` only carries a meaningful value once `folder_entries` has told us
  // this path's label; before that, `labelKnown: false` tells the backend to
  // keep whatever it already has instead of clearing it, unless this key
  // set the label itself.
  void window.__TAURI__.core
    .invoke("set_rating", {
      path,
      rating: rating ?? 0,
      flag,
      label,
      labelKnown: forceLabel === true || entries.has(path) || label !== before.label,
    })
    .catch((err: unknown) => {
      if (token !== folderToken) {
        return;
      }
      onFail?.(before);
      touched.delete(path);
      applyRating(path, before.rating, before.flag, before.label);
      refilter();
      setStatus(String(err));
    });
}

// `Edit > Undo` and `Edit > Redo`: pop the most recent batch off `from`, push
// its files' current states onto `to` and restore the popped states. A single
// file becomes current unless the filter now hides it, and when the focus
// moves it also becomes the selection, unless compare mode is holding the
// selection as its comparison set; a batch leaves the current file where it
// is. A trash run is restored or moved to the Trash again instead; see
// `undoTrash` and `redoTrash`.
function step(from: History<Entry<Judgment>>, to: History<Entry<Judgment>>, verb: string): void {
  if (trashStep !== null) {
    return;
  }
  const top = from.peek();
  if (top !== undefined && !isJudgments(top)) {
    if (from === history) {
      undoTrash(top);
    } else {
      redoTrash(top);
    }
    return;
  }
  if (openDir === null) {
    return;
  }
  const popped = from.pop();
  const batch =
    popped !== undefined && isJudgments(popped)
      ? popped.filter((entry) => allFiles.includes(entry.path))
      : [];
  if (batch.length === 0) {
    return;
  }
  const changes = batch.map((entry) => ({
    before: {
      path: entry.path,
      rating: ratings.get(entry.path) ?? null,
      flag: flagOf(entry.path),
      label: labels.get(entry.path) ?? null,
    },
    rating: entry.rating,
    flag: entry.flag,
    label: entry.label,
  }));
  to.push(changes.map(({ before }) => before));
  // A file the filter now hides leaves the current file where it is.
  const shownPath = files[index];
  if (batch.length > 1) {
    commit(changes, undefined, () => shownPath);
    setStatus(`${verb} ${batch.length} files`);
    return;
  }
  const { path } = batch[0];
  commit(changes, undefined, () => (passes(path) ? path : shownPath));
  // In compare mode the comparison set is the multi-selection, not the
  // focused file, so leave the selection alone; collapsing it would change
  // what `loadCompare()` compares.
  if (!comparing) {
    selection = restore(selection, files, shownPath, path);
    paintSelection();
  }
  const at = fileIndex.get(path);
  const name = path.split(/[\\/]/).pop();
  if (at === undefined) {
    setStatus(`${verb} ${name} (hidden by the filter)`);
    return;
  }
  if (at !== index) {
    index = at;
    show();
  }
  setStatus(`${verb} ${name}`);
}

// Restore the trash run `entry` from the Trash. It waits for a running scan
// like the run does, and stays on the history until the backend has
// restored it, so a refusal (a scan that slipped in) leaves it undoable. An
// undone run goes onto `redoable` when anything came back. When the open folder was
// among its folders, the restored RAWs go into `allFiles` at once, so an
// undo pressed before the rescan lands still reaches their judgments, and
// the rescan brings their thumbnails and judgments back. The current file
// and the selection stay.
function undoTrash(entry: TrashEntry): void {
  const label = entry.what === "sidecars" ? "Delete Sidecars" : "Move to Trash";
  whenIdle(`Undo ${label}`, () => {
    if (trashStep !== null || history.peek() !== entry) {
      return;
    }
    trashStep = entry;
    settleIdle(
      window.__TAURI__.core
        .invoke<TrashRestored>("trash_rejected_undo", { runId: entry.runId })
        .then((result) => {
          trashStep = null;
          history.remove(entry);
          const redo = undoneTrash(entry, result);
          if (redo !== null) {
            redoable.push(redo);
          }
          for (const { path, message } of result.failed) {
            errors.add(path, `${baseName(path)}: could not restore from the Trash: ${message}`);
          }
          setStatus(restoredStatus(result, entry));
          if (openDir !== null && opensTarget(openDir, entry.dirs, folders.ignoreCase)) {
            const back = restoredInto(openDir, result.restored, allFiles, folders.ignoreCase);
            if (back.length > 0) {
              allFiles = [...allFiles, ...back];
              setViewOnly(isViewOnly(allFiles));
              refilter();
            }
            resync("restore");
          }
        })
        .catch((err: unknown) => {
          trashStep = null;
          if (!stillHeld(String(err))) {
            history.remove(entry);
          }
          setStatus(String(err));
        }),
    );
  });
}

// Move what the undo of `entry` brought back to the Trash again, with no
// dialog, as a new run that goes onto the history. It waits for a running
// scan like the run does, and stays on `redoable` until the backend has
// moved the files, so a refusal leaves it redoable.
function redoTrash(entry: TrashEntry): void {
  const label = entry.what === "sidecars" ? "Delete Sidecars" : "Move to Trash";
  whenIdle(`Redo ${label}`, () => {
    if (trashStep !== null || trashing !== null || redoable.peek() !== entry) {
      return;
    }
    trashStep = entry;
    trashing = trashProgressStatus("trash", 0, entry.count);
    renderMeta();
    settleIdle(
      window.__TAURI__.core
        .invoke<TrashSummary>("trash_rejected_redo", { runId: entry.runId })
        .then((summary) => {
          trashStep = null;
          trashing = null;
          redoable.remove(entry);
          trashed(summary, entry.dirs, entry.what);
        })
        .catch((err: unknown) => {
          trashStep = null;
          trashing = null;
          if (!stillHeld(String(err))) {
            redoable.remove(entry);
          }
          setStatus(String(err));
        }),
    );
  });
}

function undo(): void {
  step(history, redoable, "Undid");
}

function redo(): void {
  step(redoable, history, "Redid");
}

// Hand the strip each displayed file's place in its burst, so the bracket
// opens and closes where a filter or sort separates members.
function applyBursts(): void {
  burstMarks(files, bursts).forEach((value, at) => {
    strip.setBurst(at, value);
  });
}

// Hand the strip the focus mark state of each displayed file.
function applyTiers(): void {
  files.forEach((path, at) => {
    strip.setTier(at, stripState(entries.get(path)?.focus));
  });
}

// Hand the strip which displayed files the personal ranking puts first in
// their burst; none while AI analysis is off, since `fileRanks` is empty.
function applyRecommended(): void {
  files.forEach((path, at) => {
    strip.setRecommended(at, recommended(fileRanks.get(path)));
  });
}

// Hand the strip why the scan failed on each displayed file.
function applyFailures(): void {
  files.forEach((path, at) => {
    strip.setFailure(at, entries.get(path)?.error ?? null);
  });
}

function refreshEntries(): void {
  if (openDir === null) {
    return;
  }
  if (entriesInFlight) {
    entriesPending = true;
    return;
  }
  const dir = openDir;
  const token = folderToken;
  entriesInFlight = true;
  const start = performance.now();
  // A checked `Genre` item filters on the labels, so the refilter below that
  // consumes `pendingResume` needs the folder's labels already in hand.
  const genresNow =
    !filterGenre.hidden && shownGenres.size > 0
      ? window.__TAURI__.core
          .invoke<Record<string, string[]>>("genres_of", { dir })
          .catch(() => null)
      : Promise.resolve(null);
  void Promise.all([
    window.__TAURI__.core.invoke<IndexedFile[]>("folder_entries", { dir }),
    genresNow,
  ])
    .then(([rows, found]) => {
      const invoked = performance.now();
      entriesInFlight = false;
      if (entriesPending) {
        entriesPending = false;
        refreshEntries();
      }
      if (dir !== openDir || token !== folderToken) {
        return;
      }
      if (found !== null) {
        fileGenres = new Map(Object.entries(found));
      }
      entries.clear();
      sharpness.clear();
      faceCache.clear();
      eyesCache.clear();
      requestEyes();
      for (const row of rows) {
        entries.set(row.path, row);
        if (row.sharpness !== null) {
          sharpness.set(row.path, row.sharpness);
        }
        if (!touched.has(row.path)) {
          applyRating(row.path, row.rating, row.flag, row.label);
        }
      }
      const rebuilt = performance.now();
      bursts = groupBursts(allFiles, (path) => {
        const entry = entries.get(path);
        return {
          captureTime: entry?.capture_time ?? undefined,
          subsec: entry?.subsec ?? undefined,
        };
      });
      const grouped = performance.now();
      rebuildExifMenu();
      const exifed = performance.now();
      renderMeta();
      const metaed = performance.now();
      draw();
      const drawn = performance.now();
      applyBursts();
      const bracketed = performance.now();
      applyTiers();
      applyFailures();
      const marked = performance.now();
      const hadPendingResume = pendingResume !== undefined;
      const anchor = firstEntriesAnchor(pendingResume, files[index]);
      pendingResume = undefined;
      const setFiles = refilter(anchor, true, hadPendingResume);
      const end = performance.now();
      debugLog(
        refreshTimingLine({
          rows: rows.length,
          invoke: invoked - start,
          entries: rebuilt - invoked,
          bursts: grouped - rebuilt,
          exif: exifed - grouped,
          meta: metaed - exifed,
          draw: drawn - metaed,
          applyBursts: bracketed - drawn,
          candidates: marked - bracketed,
          refilter: end - marked,
          setFiles,
          total: end - start,
        }),
      );
      refreshGenres();
      refreshRanks();
    })
    .catch(() => {
      entriesInFlight = false;
      if (entriesPending) {
        entriesPending = false;
        refreshEntries();
      }
      // A folder with no index cache simply has no focus marks. Still
      // resolve a pending resume target against the already-listed
      // `files`, and clear it either way: `show()` skips its write while
      // `pendingResume` is set, and a rejected request otherwise leaves it
      // stuck, so nothing browsed in this folder gets remembered until a
      // later `folder_entries` happens to succeed.
      if (dir === openDir && token === folderToken && pendingResume !== undefined) {
        const anchor = pendingResume;
        pendingResume = undefined;
        refilter(anchor, true, true);
      }
    });
}

// Re-read the open folder's genre labels, at most one read at a time, and
// refilter when a `Genre` item is checked. Nothing is read while AI analysis
// is off.
function refreshGenres(): void {
  if (openDir === null || filterGenre.hidden) {
    return;
  }
  if (genresInFlight) {
    genresPending = true;
    return;
  }
  const dir = openDir;
  const token = folderToken;
  genresInFlight = true;
  const settle = () => {
    genresInFlight = false;
    if (genresPending) {
      genresPending = false;
      refreshGenres();
    }
  };
  void window.__TAURI__.core
    .invoke<Record<string, string[]>>("genres_of", { dir })
    .then((found) => {
      settle();
      if (dir !== openDir || token !== folderToken || filterGenre.hidden) {
        return;
      }
      fileGenres = new Map(Object.entries(found));
      if (shownGenres.size > 0) {
        refilter(files[index], true);
      }
    })
    .catch(settle);
}

// Re-read the open folder's ranking, at most one read at a time, reorder the
// strip under the `recommended` sort and redraw the meta pane. Nothing is
// read while AI analysis is off.
function refreshRanks(): void {
  if (openDir === null || !rankingOn) {
    return;
  }
  if (ranksInFlight) {
    ranksPending = true;
    return;
  }
  const dir = openDir;
  const token = folderToken;
  ranksInFlight = true;
  const settle = () => {
    ranksInFlight = false;
    if (ranksPending) {
      ranksPending = false;
      refreshRanks();
    }
  };
  void window.__TAURI__.core
    .invoke<{ model: RankModel; files: Record<string, Ranked> }>("ranking_of", { dir })
    .then(({ model, files: found }) => {
      settle();
      if (dir !== openDir || token !== folderToken || !rankingOn) {
        return;
      }
      fileRanks = new Map(Object.entries(found));
      rankModel = model;
      if (sortInForce() === "recommended") {
        refilter(files[index], true);
      }
      applyRecommended();
      renderMeta();
    })
    .catch(settle);
}

// FocusLocation is in unrotated sensor coordinates, so the point is scaled
// onto the unrotated preview and drawn inside the same transform the image
// got; drawing it after the rotation would put it on the wrong edge. Mirrors
// the arithmetic in `riffle-cli focusbox`.
//
// When Sony `FocusFrameSize` is valid, the AF frame the camera used is drawn
// around the point as well, in the same sensor coordinates; a body that
// records only the point gets the crosshair alone, and a manual-focus shot,
// whose recorded point is not trusted, gets no mark. The mark is bright green
// for a good photo (`photoTier`: the eyes of the face nearest the AF point are
// sharp and open and the face is toward the camera), gray when that face was
// judged and the frame is not good (its eyes not sharp included), and
// white when Riffle does not know: no face near the point, or the second scan
// pass has not reached the file yet.
function drawFocusMark(drawWidth: number, drawHeight: number): void {
  if (!showFocus || files.length === 0) {
    return;
  }
  const mark = focusMark(entries.get(files[index])?.focus, drawWidth, drawHeight);
  if (mark === null) {
    return;
  }
  const { x, y, rect, state } = mark;
  const arm = FOCUS_MARK_ARM;
  const gap = FOCUS_MARK_GAP;
  // A state color over a dark outline: the color carries the mark on most
  // photos, and the outline still draws its edge where the subject shares the
  // color. The same path is stroked twice, the outline first and wider, and
  // square caps give the arm ends the same 1px outline as their sides.
  context.save();
  context.lineCap = "square";
  context.beginPath();
  context.moveTo(x - gap - arm, y);
  context.lineTo(x - gap, y);
  context.moveTo(x + gap, y);
  context.lineTo(x + gap + arm, y);
  context.moveTo(x, y - gap - arm);
  context.lineTo(x, y - gap);
  context.moveTo(x, y + gap);
  context.lineTo(x, y + gap + arm);
  if (rect !== null) {
    context.rect(rect.x, rect.y, rect.width, rect.height);
  }
  context.strokeStyle = "rgba(0, 0, 0, 0.8)";
  context.lineWidth = 4;
  context.stroke();
  context.strokeStyle = FOCUS_MARK_COLORS[state];
  context.lineWidth = 2;
  context.stroke();
  context.restore();
}

// The faces Riffle detects on the current file's preview, in the same
// unrotated coordinates as the AF mark: a box per face and a dot on each of
// its five landmarks (the eyes, the nose tip, the mouth corners). They are
// detected when first drawn and appear once `faces_of`
// answers; the 1:1 view and Compare do not draw them.
function drawFaceMarks(drawWidth: number, drawHeight: number): void {
  if (!showFocus || viewOnly || files.length === 0) {
    return;
  }
  // `shown` still holds the previous file's bitmap between `show()` and the
  // new preview's decode. Skip drawing (and requesting) faces until `shown`
  // belongs to the current file, or a fast response would paint the new
  // file's boxes on the old bitmap, same as `drawZoom`'s `shown.seq === seq`
  // guard.
  if (shown === null || shown.seq !== seq) {
    return;
  }
  const path = files[index];
  const found = faceCache.get(path);
  if (found === undefined) {
    requestFaces(path);
    return;
  }
  if (found.faces.length === 0) {
    return;
  }
  const marks = faceMarks(found.faces, found.width, found.height, drawWidth, drawHeight);
  // The same outline-then-color passes as the AF mark.
  context.save();
  context.beginPath();
  for (const { rect } of marks) {
    context.rect(rect.x, rect.y, rect.width, rect.height);
  }
  context.strokeStyle = "rgba(0, 0, 0, 0.8)";
  context.lineWidth = 4;
  context.stroke();
  context.strokeStyle = FACE_MARK_COLOR;
  context.lineWidth = 2;
  context.stroke();
  context.beginPath();
  for (const { points } of marks) {
    for (const p of points) {
      context.moveTo(p.x + FACE_MARK_EYE_RADIUS, p.y);
      context.arc(p.x, p.y, FACE_MARK_EYE_RADIUS, 0, 2 * Math.PI);
    }
  }
  context.strokeStyle = "rgba(0, 0, 0, 0.8)";
  context.stroke();
  context.fillStyle = FACE_MARK_COLOR;
  context.fill();
  context.restore();
}

// Each face box's label (`faceLabel`), drawn after `draw()` undoes the
// image's rotation so the text reads upright on a turned frame, under the
// same gates as the boxes.
function drawFaceLabels(drawWidth: number, drawHeight: number, orientation: number): void {
  if (!showFocus || viewOnly || files.length === 0) {
    return;
  }
  if (shown === null || shown.seq !== seq) {
    return;
  }
  const found = faceCache.get(files[index]);
  if (found === undefined || found.faces.length === 0) {
    return;
  }
  const marks = faceMarks(
    found.faces,
    found.width,
    found.height,
    drawWidth,
    drawHeight,
    orientation,
  );
  const dpr = window.devicePixelRatio;
  context.save();
  context.translate(canvas.width / 2, canvas.height / 2);
  context.scale(dpr, dpr);
  context.font = "11px system-ui, sans-serif";
  context.textBaseline = "top";
  context.textAlign = "left";
  context.lineJoin = "round";
  context.lineWidth = 3;
  context.strokeStyle = "rgba(0, 0, 0, 0.8)";
  context.fillStyle = FACE_MARK_COLOR;
  for (const { rect, label } of marks) {
    const width = context.measureText(label).width;
    const at = faceLabelAt(rect, orientation, drawWidth, drawHeight, width);
    context.strokeText(label, at.x, at.y);
    context.fillText(label, at.x, at.y);
  }
  context.restore();
}

// The face parts the `Eyes` judgment looked at (face oval, eyes, brows, nose,
// lips, and the irises when the eyes are open), over the judged face only, in
// the same unrotated coordinates as the face boxes. It appears once `eyes_of`
// answers, under the same gates as the boxes.
function drawFaceMesh(drawWidth: number, drawHeight: number): void {
  if (!showFocus || viewOnly || files.length === 0) {
    return;
  }
  if (shown === null || shown.seq !== seq) {
    return;
  }
  const eyes = eyesCache.get(files[index]);
  if (eyes === undefined || eyes === null) {
    return;
  }
  const { mesh } = eyes;
  const points = meshPoints(mesh.points, mesh.width, mesh.height, drawWidth, drawHeight);
  const { edges, dots } = meshEdges(eyes.state);
  context.save();
  context.beginPath();
  for (let i = 0; i < edges.length; i += 2) {
    const [ax, ay] = points[edges[i]];
    const [bx, by] = points[edges[i + 1]];
    context.moveTo(ax, ay);
    context.lineTo(bx, by);
  }
  context.strokeStyle = "rgba(0, 0, 0, 0.8)";
  context.lineWidth = FACE_MESH_OUTLINE_WIDTH;
  context.stroke();
  context.strokeStyle = FACE_MESH_COLOR;
  context.lineWidth = FACE_MESH_LINE_WIDTH;
  context.stroke();
  if (dots.length > 0) {
    context.beginPath();
    for (const dot of dots) {
      const [x, y] = points[dot];
      context.moveTo(x + FACE_MARK_EYE_RADIUS, y);
      context.arc(x, y, FACE_MARK_EYE_RADIUS, 0, 2 * Math.PI);
    }
    context.strokeStyle = "rgba(0, 0, 0, 0.8)";
    context.stroke();
    context.fillStyle = FACE_MESH_COLOR;
    context.fill();
  }
  context.restore();
}

// A failed detection is logged and cached as no faces, so a bad file is not
// retried on every draw.
function requestFaces(path: string): void {
  const token = faceCache.request(path);
  if (token === null) {
    return;
  }
  void window.__TAURI__.core
    .invoke<Faces>("faces_of", { path })
    .catch((err: unknown) => {
      console.error(err);
      return NO_FACES;
    })
    .then((found) => {
      if (faceCache.settle(path, token, found) && files[index] === path) {
        draw();
      }
    });
}

// The 1:1 view: the same rotation `draw()` applies, with the focus point at
// the canvas center. Everything here is in device pixels, so the `scale(dpr,
// dpr)` of `draw()` is deliberately not applied. The crop is cut in unrotated
// JPEG coordinates, exactly like the focus mark, so the rotation carries it.
function drawZoom(): void {
  const dpr = window.devicePixelRatio;
  canvas.width = Math.round(canvas.clientWidth * dpr);
  canvas.height = Math.round(canvas.clientHeight * dpr);
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.save();
  context.translate(canvas.width / 2, canvas.height / 2);
  // The placeholder (the scaled preview) is only drawn while `shown` belongs
  // to the current file; otherwise it would position the previous file's
  // bitmap with the new file's focus point.
  const placeholderShown = shown !== null && shown.seq === seq ? shown : null;
  const orientation =
    crop !== null && crop.cropSeq === seq ? crop.orientation : (placeholderShown?.orientation ?? 1);
  if (orientation === 6) {
    context.rotate(Math.PI / 2);
  } else if (orientation === 8) {
    context.rotate(-Math.PI / 2);
  } else if (orientation === 3) {
    context.rotate(Math.PI);
  }
  const focus = files.length > 0 ? entries.get(files[index])?.focus?.point : undefined;
  if (placeholderShown !== null && focus !== undefined && focus !== null) {
    const full =
      crop !== null && crop.cropSeq === seq
        ? { width: crop.fullWidth, height: crop.fullHeight }
        : null;
    const { x, y, width, height } = placeholderRect(
      placeholderShown.bitmap.width,
      placeholderShown.bitmap.height,
      focus,
      full,
    );
    context.drawImage(placeholderShown.bitmap, -x, -y, width, height);
  }
  if (crop !== null && crop.cropSeq === seq) {
    context.drawImage(crop.bitmap, -crop.pointX, -crop.pointY);
  }
  context.restore();
}

// True when the held crop was cut for a viewport size that no longer
// matches the canvas (a resize while zoomed).
function cropViewportStale(): boolean {
  if (crop === null) {
    return false;
  }
  const dpr = window.devicePixelRatio;
  return (
    crop.requestedWidth !== Math.round(canvas.clientWidth * dpr) ||
    crop.requestedHeight !== Math.round(canvas.clientHeight * dpr)
  );
}

function requestCrop(): void {
  if (cropInFlight || files.length === 0) {
    zoomKeypressAt = null;
    return;
  }
  cropInFlight = true;
  const current = seq;
  // Every mark below is measured from this request, not from the last
  // keypress: a crop asked for by a resize or a page turn has nothing to do
  // with a zoom key pressed minutes ago.
  const requestStartedAt = performance.now();
  const keypressAt = zoomKeypressAt;
  zoomKeypressAt = null;
  const dpr = window.devicePixelRatio;
  const requestedWidth = Math.round(canvas.clientWidth * dpr);
  const requestedHeight = Math.round(canvas.clientHeight * dpr);
  window.__TAURI__.core
    .invoke<ArrayBuffer>("focus_crop", {
      path: files[index],
      width: requestedWidth,
      height: requestedHeight,
    })
    .then(async (payload) => {
      cropInFlight = false;
      const invokeAt = performance.now() - requestStartedAt;
      if (current !== seq) {
        if (zoomed) {
          requestCrop();
        }
        return;
      }
      const header = new DataView(payload, 0, CROP_HEADER_LEN);
      const kind = header.getUint16(0, true);
      if (kind !== CROP_KIND_RGBA_V3) {
        throw new Error(`unknown crop payload kind ${kind}`);
      }
      const orientation = header.getUint16(2, true);
      const width = header.getUint32(4, true);
      const height = header.getUint32(8, true);
      // `putImageData` ignores the rotation transform, so the pixels go
      // through a bitmap; no JPEG is involved, so no worker is needed.
      const pixels = new ImageData(new Uint8ClampedArray(payload, CROP_HEADER_LEN), width, height);
      const bitmap = await createImageBitmap(pixels);
      const bitmapAt = performance.now() - requestStartedAt;
      const readMs = header.getUint32(20, true) / 1000;
      const decodeMs = header.getUint32(24, true) / 1000;
      const sinceKeypress =
        keypressAt === null
          ? ""
          : ` keypressToPixels=${(performance.now() - keypressAt).toFixed(1)}ms`;
      debugLog(
        `zoom crop=${bitmapAt.toFixed(1)}ms` +
          ` read=${readMs.toFixed(1)}ms` +
          ` decode=${decodeMs.toFixed(1)}ms` +
          ` ipc=${(invokeAt - readMs - decodeMs).toFixed(1)}ms` +
          ` bitmap=${(bitmapAt - invokeAt).toFixed(1)}ms` +
          sinceKeypress,
      );
      if (current !== seq) {
        bitmap.close();
        if (zoomed) {
          requestCrop();
        }
        return;
      }
      crop?.bitmap.close();
      crop = {
        bitmap,
        pointX: header.getUint32(12, true),
        pointY: header.getUint32(16, true),
        cropSeq: current,
        orientation,
        fullWidth: header.getUint16(28, true),
        fullHeight: header.getUint16(30, true),
        requestedWidth,
        requestedHeight,
      };
      draw();
      // The viewport may have been resized while this request was in
      // flight; `requestCrop` silently no-ops on `cropInFlight` during that
      // window, so check here and re-fire if the settled crop is stale. The
      // re-fire goes through the debounce too, or a drag would simply chain
      // one crop into the next.
      if (zoomed && cropViewportStale()) {
        scheduleCropForResize();
      }
    })
    .catch((err: unknown) => {
      cropInFlight = false;
      if (current !== seq) {
        if (zoomed) {
          requestCrop();
        }
        return;
      }
      setStatus(String(err));
    });
}

// The zoom key (`z`) toggles the 1:1 view. The crop already held for this
// file is reused; otherwise one is requested and the scaled preview stands
// in.
function toggleZoom(): void {
  if (files.length === 0) {
    return;
  }
  zoomed = !zoomed;
  if (zoomed) {
    if (crop === null || crop.cropSeq !== seq || cropViewportStale()) {
      zoomKeypressAt = performance.now();
      debugLog("zoom keypress", zoomKeypressAt);
      requestCrop();
    }
  }
  renderMeta();
  draw();
}

// The current file's preview cannot be shown: drop whatever `shown` still
// holds (possibly an earlier file's) so it is not taken for this file's.
function failPreview(): void {
  previewFailedSeq = seq;
  shown?.bitmap.close();
  shown = null;
  draw();
}

// The current file's preview is on screen: start its dwell. A view-only
// folder and a provisional file shown before the resume target resolves do
// not count.
function startDwell(): void {
  const path = files[index];
  if (viewOnly || openDir === null || pendingResume !== undefined || path === undefined) {
    return;
  }
  dwell.shown(path);
}

function requestPreview(): void {
  if (inFlight || files.length === 0) {
    pageKeypressAt = null;
    return;
  }
  inFlight = true;
  const current = seq;
  const requestStartedAt = performance.now();
  const keypressAt = pageKeypressAt;
  pageKeypressAt = null;
  window.__TAURI__.core
    .invoke<ArrayBuffer>("preview", { path: files[index] })
    .then((payload) => {
      const invokeMs = performance.now() - requestStartedAt;
      inFlight = false;
      if (current !== seq) {
        requestPreview();
        return;
      }
      const header = new DataView(payload, 0, PREVIEW_HEADER_LEN);
      const kind = header.getUint16(0, true);
      if (kind !== PREVIEW_KIND_JPEG_V1) {
        throw new Error(`unknown preview payload kind ${kind}`);
      }
      return previewPixelLimit.then((maxPixels) => {
        if (current !== seq) {
          // `inFlight` was already cleared above, and the page turn that
          // invalidated this request already started its own
          // `requestPreview()` from `show()`. Calling it again here would
          // start a duplicate `preview` invoke for the same `seq`.
          return;
        }
        orientations.set(current, header.getUint16(2, true));
        const jpeg = payload.slice(PREVIEW_HEADER_LEN);
        pageTimings.set(current, {
          startedAt: requestStartedAt,
          invokeMs,
          postedAt: performance.now(),
          keypressAt,
        });
        worker.postMessage({ seq: current, jpeg, maxPixels }, [jpeg]);
      });
    })
    .catch((err: unknown) => {
      inFlight = false;
      if (current !== seq) {
        requestPreview();
        return;
      }
      failPreview();
      setStatus(String(err));
    });
}

function requestMetadata(): void {
  if (metaInFlight || files.length === 0) {
    return;
  }
  metaInFlight = true;
  const current = seq;
  window.__TAURI__.core
    .invoke<Metadata>("metadata", { path: files[index] })
    .then((found) => {
      metaInFlight = false;
      if (current !== seq) {
        requestMetadata();
        return;
      }
      meta = found;
      metaStale = false;
      renderMeta();
    })
    .catch(() => {
      metaInFlight = false;
      if (current !== seq) {
        requestMetadata();
        return;
      }
      // A file whose metadata cannot be read keeps its name and position;
      // the preview request reports the error itself.
      meta = null;
      metaStale = false;
      renderMeta();
    });
}

// Mirrors `EyesResponse` in `crates/app/src/commands.rs`.
interface EyesResponse {
  eyes: Eyes | null;
  superseded: boolean;
  read_us: number;
  decode_us: number;
  detect_us: number;
  model_us: number;
}

// Judge the current file's eyes unless they are cached or another judgment
// is running; once that one settles, the file current by then is asked for,
// so paging with a key held never queues a judgment per file passed. A
// failure is logged and cached as unknown, so a bad file is not retried on
// every page turn.
function requestEyes(): void {
  if (files.length === 0 || viewOnly) {
    return;
  }
  const path = files[index];
  const ticket = eyesCache.request(path);
  if (ticket === null) {
    return;
  }
  const startedAt = performance.now();
  void window.__TAURI__.core
    .invoke<EyesResponse>("eyes_of", { path, request: ticket.id })
    .then(
      (found) => {
        const totalMs = performance.now() - startedAt;
        const ms = (us: number) => `${(us / 1000).toFixed(1)}ms`;
        const backendUs = found.read_us + found.decode_us + found.detect_us + found.model_us;
        debugLog(
          `eyes total=${totalMs.toFixed(1)}ms` +
            ` read=${ms(found.read_us)}` +
            ` decode=${ms(found.decode_us)}` +
            ` detect=${ms(found.detect_us)}` +
            ` model=${ms(found.model_us)}` +
            ` ipc=${(totalMs - backendUs / 1000).toFixed(1)}ms` +
            (found.superseded ? " superseded" : ""),
        );
        return found.superseded ? undefined : found.eyes;
      },
      (err: unknown) => {
        console.error(err);
        return null;
      },
    )
    .then((eyes) => {
      if (eyesCache.settle(path, ticket, eyes) && files[index] === path) {
        renderMeta();
        if (showFocus) {
          draw();
        }
      }
      requestEyes();
    });
}

function show(): void {
  seq += 1;
  dwell.cancel();
  switchAt = performance.now();
  previewFailedSeq = null;
  metaStale = true;
  // While a resume target is still pending, `files[index]` is only a
  // provisional anchor picked before entries loaded; writing it here would
  // overwrite the remembered file with it. The first `refreshEntries` clears
  // `pendingResume` once it resolves the real anchor.
  if (openDir !== null && files[index] !== undefined && pendingResume === undefined) {
    rememberViewed({ dir: openDir, path: files[index] });
  }
  strip.setCurrent(index);
  sendScanFocus();
  setStatus();
  requestPreview();
  requestMetadata();
  requestEyes();
  if (zoomed) {
    requestCrop();
  } else {
    crop?.bitmap.close();
    crop = null;
  }
  if (comparing) void loadCompare();
}

worker.addEventListener("message", (event: MessageEvent<DecodeResponse>) => {
  const { seq: responseSeq, bitmap, error } = event.data;
  const orientation = orientations.get(responseSeq) ?? 1;
  orientations.delete(responseSeq);
  const timing = pageTimings.get(responseSeq);
  pageTimings.delete(responseSeq);
  if (responseSeq !== seq) {
    bitmap?.close();
    return;
  }
  if (bitmap === undefined) {
    failPreview();
    setStatus(error ?? "decode failed");
    return;
  }
  shown?.bitmap.close();
  shown = { bitmap, orientation, seq: responseSeq };
  setStatus();
  const decodedAt = performance.now();
  draw();
  startDwell();
  if (timing !== undefined) {
    const drawnAt = performance.now();
    const sinceKeypress =
      timing.keypressAt === null
        ? ""
        : ` keypressToPixels=${(drawnAt - timing.keypressAt).toFixed(1)}ms`;
    debugLog(
      `page invoke=${timing.invokeMs.toFixed(1)}ms` +
        ` decode=${(decodedAt - timing.postedAt).toFixed(1)}ms` +
        ` total=${(drawnAt - timing.startedAt).toFixed(1)}ms` +
        sinceKeypress,
    );
    if (perf.on) {
      perf.addLatency({ at: drawnAt, ms: drawnAt - latencyStart(timing.keypressAt, switchAt) });
      drawPerf();
    }
  }
});

function moveBurst(direction: -1 | 1): void {
  if (files.length === 0) {
    return;
  }
  const ids = files.map((path, at) => bursts.get(path)?.burst ?? -1 - at);
  const next = burstStep(ids, index, direction);
  if (next === index) {
    return;
  }
  index = next;
  selection = single(files[index]);
  paintSelection();
  pageKeypressAt = performance.now();
  show();
}

function moveBurstFrame(direction: -1 | 1): void {
  if (files.length === 0) {
    return;
  }
  const ids = files.map((path, at) => bursts.get(path)?.burst ?? -1 - at);
  const next = burstFrameStep(ids, index, direction);
  if (next === index) {
    return;
  }
  index = next;
  selection = single(files[index]);
  paintSelection();
  pageKeypressAt = performance.now();
  show();
}

function move(delta: number): void {
  if (files.length === 0) {
    return;
  }
  const next = Math.min(Math.max(index + delta, 0), files.length - 1);
  if (next === index) {
    return;
  }
  index = next;
  selection = single(files[index]);
  paintSelection();
  pageKeypressAt = performance.now();
  show();
}

// Moves the focus by one and grows or shrinks the range from the anchor.
function extendSelection(delta: -1 | 1): void {
  if (files.length === 0) {
    return;
  }
  const extended = extend(selection, files, index, delta);
  selection = extended.selection;
  paintSelection();
  if (extended.index === index) {
    renderMeta();
    if (comparing) void loadCompare();
    return;
  }
  index = extended.index;
  pageKeypressAt = performance.now();
  show();
}

// Selects every file the strip shows; the focus stays put and anchors it.
function selectAllFiles(): void {
  if (files.length === 0) {
    return;
  }
  selection = all(files, index);
  paintSelection();
  renderMeta();
  if (comparing) void loadCompare();
}

const contextMenu = document.getElementById("context-menu") as HTMLDivElement;

function closeContextMenu(): void {
  contextMenu.hidden = true;
}

function openContextMenu(x: number, y: number): void {
  const focused = (comparing ? compareActivePath : null) ?? files[index];
  const state = {
    rating: ratings.get(focused) ?? null,
    flag: flagOf(focused),
    label: labels.get(focused) ?? null,
  };
  const paths = comparing ? [focused] : targets(selection, files, index);
  void revealLabel.then((label) => {
    const groups = contextMenuGroups(
      keyBindings,
      state,
      label,
      viewOnly,
      paths.length,
      trashing !== null,
    );
    showMenu(groups, x, y, (action) => {
      if (action === "renameFile") {
        strip.startRename(index);
      } else if (action === "trashFiles") {
        confirmTrashFiles(paths, "trash");
      } else if (action === "deleteFiles") {
        confirmTrashFiles(paths, "delete");
      } else if (action === "revealFile") {
        window.__TAURI__.core.invoke("reveal_folder", { path: focused }).catch((err: unknown) => {
          setStatus(String(err));
        });
      } else if (action === "copyPath") {
        navigator.clipboard.writeText(paths.join("\n")).catch((err: unknown) => {
          setStatus(String(err));
        });
      } else if (action === "copyFileName") {
        navigator.clipboard.writeText(paths.map(baseName).join("\n")).catch((err: unknown) => {
          setStatus(String(err));
        });
      } else {
        runAction(action);
      }
    });
  });
}

// Give a menu item the check mark its leading column shows while it is
// `aria-checked`.
function addMenuCheck(item: HTMLElement): void {
  const mark = document.createElement("span");
  mark.className = "menu-check";
  mark.innerHTML = CHECK_SVG;
  item.prepend(mark);
}

function menuSeparator(): HTMLHRElement {
  const hr = document.createElement("hr");
  hr.className = "menu-separator";
  return hr;
}

for (const item of document.querySelectorAll<HTMLElement>('.menu-item[role^="menuitem"]')) {
  addMenuCheck(item);
}

// Fill `#context-menu` with `groups` at (`x`, `y`); a click on an item
// closes the menu and hands its action to `run`.
function showMenu(groups: MenuItem[][], x: number, y: number, run: (action: string) => void): void {
  contextMenu.replaceChildren(
    ...groups.flatMap((group, i) => {
      const items: HTMLElement[] = group.map(({ action, label, shortcut, checked, disabled }) => {
        const item = document.createElement("button");
        item.type = "button";
        item.className = "menu-item";
        item.disabled = disabled === true;
        if (checked === undefined) {
          item.setAttribute("role", "menuitem");
        } else {
          item.setAttribute("role", "menuitemradio");
          item.setAttribute("aria-checked", String(checked));
        }
        const name = document.createElement("span");
        name.textContent = label;
        const key = document.createElement("span");
        key.className = "shortcut";
        key.textContent = shortcut;
        item.append(name, key);
        addMenuCheck(item);
        item.addEventListener("click", () => {
          closeContextMenu();
          run(action);
        });
        return item;
      });
      return i === 0 ? items : [menuSeparator(), ...items];
    }),
  );
  contextMenu.hidden = false;
  const { left, top } = menuPosition(
    x,
    y,
    contextMenu.offsetWidth,
    contextMenu.offsetHeight,
    window.innerWidth,
    window.innerHeight,
  );
  contextMenu.style.left = `${left}px`;
  contextMenu.style.top = `${top}px`;
}

document.addEventListener("mousedown", (event) => {
  if (!contextMenu.hidden && !(event.target as Element).closest("#context-menu")) {
    closeContextMenu();
  }
});

// Push the selection to the strip as indices into `files`.
function paintSelection(): void {
  strip.setSelected(
    [...selection.selected].flatMap((path) => {
      const at = fileIndex.get(path);
      return at === undefined ? [] : [at];
    }),
  );
}

// A Cmd/Ctrl+click only changes the selection; a Shift+click also moves the
// focus to the clicked file, so the range always holds the focused file.
strip.init(
  (selected, modifiers) => {
    selection = click(selection, files, index, selected, modifiers);
    paintSelection();
    focusFile(files[modifiers.toggle ? index : selected]);
  },
  (selected, x, y) => {
    if (!selection.selected.has(files[selected])) {
      selection = single(files[selected]);
      paintSelection();
    }
    if (selected !== index) {
      index = selected;
      show();
    }
    openContextMenu(x, y);
  },
  renameFile,
  () => !viewOnly,
  sendScanFocus,
  cancelPendingRename,
  setStatus,
);

const revealLabel = window.__TAURI__.core.invoke<string>("reveal_label");

interface Renamed {
  path: string;
  warning: string | null;
}

// The folder `path` is now `name` at `newPath`. The tree follows the rename
// in place; when the open folder is the renamed one or under it, it is
// reopened under its new path, its judgments coming back from the rewritten
// index. The trash entries' folders follow the rename, as the backend's
// recorded runs do.
function folderRenamed(
  path: string,
  newPath: string,
  name: string,
  warning: string | null,
): Promise<void> | undefined {
  folders.renamed(path, newPath, name);
  const moved = (dir: string): string => rebase(dir, path, newPath, folders.ignoreCase) ?? dir;
  history.map(mapTrashDirs(moved));
  redoable.map(mapTrashDirs(moved));
  const warn = (): void => {
    if (warning !== null) {
      setStatus(warning);
    }
  };
  const reopen = openDir === null ? null : rebase(openDir, path, newPath, folders.ignoreCase);
  if (reopen === null) {
    warn();
    return;
  }
  return openDirectory(reopen, newFolderToken()).then(warn, (err: unknown) => {
    setStatus(String(err));
  });
}

// The tree's inline edit confirmed a new name; see `folderRenamed`.
function renameFolder(path: string, name: string): void {
  holdRename(path, name, folders, (clearPending) => {
    folders.renameStarted(path);
    settleIdle(
      window.__TAURI__.core.invoke<Renamed>("rename_folder", { dir: path, name }).then(
        ({ path: newPath, warning }) => {
          folders.renameSettled(path);
          clearPending();
          return folderRenamed(path, newPath, name, warning);
        },
        (err: unknown) => {
          folders.renameSettled(path);
          clearPending();
          setStatus(String(err));
        },
      ),
    );
  });
}

function moveKey<V>(map: Map<string, V>, from: string, to: string): void {
  const value = map.get(from);
  if (value !== undefined) {
    map.delete(from);
    map.set(to, value);
  }
}

// The strip's inline edit confirmed a new name. The session's judgments move
// to the new path (the index rows already have), so the cell keeps its marks
// and its place in the sort. `allFiles` / `files` / `fileIndex` are patched
// in place immediately (not just left to the trailing rescan), because a
// scan can start while the invoke is in flight (e.g. the window lost and
// regained focus, firing `resync()`); leaving `files` on the old, now
// nonexistent path until that scan's `faces-done` would let a judgment key
// pressed in between rate a gone file. With `files` already current, the
// rescan needs no special anchor: its default (`files[index]` at the time it
// actually runs) is already right, whether that is now or after a deferred
// scan drains.
function renameFile(path: string, name: string): void {
  holdRename(path, name, strip, (clearPending) => {
    if (openDir === null) {
      clearPending();
      return;
    }
    const dir = openDir;
    const token = folderToken;
    strip.renameStarted(path);
    settleIdle(
      window.__TAURI__.core.invoke<Renamed>("rename_file", { dir, path, name }).then(
        ({ path: newPath, warning }) => {
          strip.renameSettled(path);
          clearPending();
          if (dir !== openDir || token !== folderToken) {
            return;
          }
          // Read before the maps below are touched: the user may have moved on
          // to another file while the invoke was in flight.
          const focused = files[index] === path;
          moveKey(ratings, path, newPath);
          moveKey(flags, path, newPath);
          moveKey(labels, path, newPath);
          moveKey(sharpness, path, newPath);
          const entry = entries.get(path);
          if (entry !== undefined) {
            entries.delete(path);
            entries.set(newPath, { ...entry, path: newPath });
          }
          if (touched.delete(path)) {
            touched.add(newPath);
          }
          // Move the renamed file's judgments onto its new path so undo /
          // redo still targets the file instead of the old, now nonexistent
          // path (which would `set_rating` a gone file and mint an orphan
          // sidecar).
          const moved = (entry: Judgment) =>
            entry.path === path ? { ...entry, path: newPath } : entry;
          history.map(mapJudgments(moved));
          redoable.map(mapJudgments(moved));
          const allAt = allFiles.indexOf(path);
          if (allAt !== -1) {
            allFiles[allAt] = newPath;
          }
          const at = fileIndex.get(path);
          if (at !== undefined) {
            files[at] = newPath;
            fileIndex.delete(path);
            fileIndex.set(newPath, at);
          }
          // `strip.setFiles` may be skipped below (the rescan sees the same list
          // it already holds), so the strip's own `indexOf` and the live cell's
          // name need their own update, and so does the meta pane / title bar.
          strip.renamePath(path, newPath);
          if (focused) {
            if (meta !== null) {
              meta = { ...meta, name: baseName(newPath) };
            }
            renderMeta();
          }
          // `refilter`'s `prune` would normally repair `selection`, but its
          // rescan is skipped whenever the renamed file keeps its place in the
          // list, so the old path would otherwise linger as a selected member
          // or the anchor.
          if (selection.selected.has(path) || selection.anchor === path) {
            const selected = new Set(selection.selected);
            if (selected.delete(path)) {
              selected.add(newPath);
            }
            selection = {
              selected,
              anchor: selection.anchor === path ? newPath : selection.anchor,
            };
            paintSelection();
          }
          if (warning !== null) {
            setStatus(warning);
          }
          resync("rename");
        },
        (err: unknown) => {
          strip.renameSettled(path);
          clearPending();
          if (dir !== openDir || token !== folderToken) {
            return;
          }
          setStatus(String(err));
        },
      ),
    );
  });
}

// A folder clicked in the tree opens the way a drop does; a right-click
// offers to reveal it in the OS file manager, to copy its path or name, to
// rename it, to expand or collapse every subfolder under it, to rewrite its
// sidecars from the index or delete them, to sequence its JPEGs, or to import
// the photos under it.
folders.init(
  (path) => {
    if (!formatGate.isOpen) {
      return;
    }
    openDirectory(path, newFolderToken()).catch((err: unknown) => {
      setStatus(String(err));
    });
  },
  setStatus,
  (path, name, x, y, root) => {
    void revealLabel.then((label) => {
      showMenu(folderMenuGroups(label, root), x, y, (action) => {
        switch (action) {
          case "revealFolder":
            window.__TAURI__.core.invoke("reveal_folder", { path }).catch((err: unknown) => {
              setStatus(String(err));
            });
            break;
          case "copyPath":
            navigator.clipboard.writeText(path).catch((err: unknown) => {
              setStatus(String(err));
            });
            break;
          case "copyFolderName":
            navigator.clipboard.writeText(name).catch((err: unknown) => {
              setStatus(String(err));
            });
            break;
          case "refreshFolder":
            folders.refresh(path);
            if (openDir !== null && opensTarget(openDir, [path], folders.ignoreCase)) {
              resync("refresh");
            }
            break;
          case "renameFolder":
            folders.startRename(path);
            break;
          case "renameByDate":
            renameByDateIn(path);
            break;
          case "expandAll":
            void folders.expandAll(path);
            break;
          case "collapseAll":
            folders.collapseAll(path);
            break;
          case "rewriteSidecars":
            rewriteSidecarsIn(path);
            break;
          case "deleteSidecars":
            deleteSidecarsIn(path);
            break;
          case "sequenceTimestamps":
            sequenceTimestampsOf(path);
            break;
          case "importFolder":
            importFrom(path);
            break;
        }
      });
    });
  },
  renameFolder,
  cancelPendingRename,
);

// Reserve the right to be the folder the UI shows. The picker reserves its
// token before its dialog opens; a drop mints its token only after
// `dropped_folder` resolves, so an ignored drop cannot cancel an open picker
// dialog. That asymmetry leaves drops unordered among themselves, which
// `dropCounter` (below) orders separately. Either way, the async work of the
// side that lost the race is dropped instead of writing into the other's UI.
function newFolderToken(): number {
  clearTimeout(scanFocusTimer);
  folderToken += 1;
  return folderToken;
}

// Reopen the folder the last session had open, if it still exists.
function reopenLastFolder(): void {
  const token = newFolderToken();
  window.__TAURI__.core
    .invoke<string | null>("last_folder")
    .then((folder) => {
      if (folder === null || token !== folderToken) {
        return;
      }
      return openDirectory(folder, token);
    })
    .catch(() => {
      // The folder went away after the check; the opening hint stays up.
    });
}

// The diff-and-scan chain both folder paths share: `scan_folder` reconciles
// the index against the disk (deleting the rows of files that are gone or
// changed) and returns the ids of the work left, which `start_scan` runs.
// Shared by `openDirectory` (the reset path) and `resync` (the keep-state
// path).
function startScan(folder: string): Promise<void> {
  // Set before `scan_folder` resolves so a `resync()` during the prepare
  // phase (folder listing plus index and sidecar reconcile) is deferred too,
  // not just during `start_scan` — otherwise it starts a second
  // `scan_folder` that stampedes this one's `scanId`.
  setScanRunning(true);
  lastScanAt = Date.now();
  scanSeq += 1;
  const seq = scanSeq;
  currentScan = seq;
  return window.__TAURI__.core
    .invoke<{
      total: number;
      scan_id: number;
      sidecar_errors: { path: string; message: string }[];
      changed: number;
    }>("scan_folder", {
      dir: folder,
    })
    .then(({ scan_id, sidecar_errors, changed }) => {
      if (seq !== currentScan) {
        // A later `startScan` call (a different folder, or a deliberate
        // re-open of this same one) now owns `scanRunning` / `resyncPending`,
        // so leave them alone.
        return;
      }
      if (sidecar_errors.length > 0) {
        for (const { path, message } of sidecar_errors) {
          errors.add(path, `${baseName(path)}: ${message}`);
        }
        renderMeta();
      }
      scanId = scan_id;
      scanStarted = { scanId: scan_id, changed };
      return window.__TAURI__.core
        .invoke<void>("start_scan", {
          scanId: scan_id,
        })
        .then(() => {
          sendScanFocus();
          // A trash or delete held while `scan_folder` ran had no id to cancel yet.
          if (cancelsScan(idle.waiting)) {
            cancelScan();
          }
        });
    })
    .catch((err: unknown) => {
      if (seq !== currentScan) {
        return;
      }
      setScanRunning(false);
      setScanProgress(null);
      idle.drain();
      drainResync();
      setStatus(String(err));
    });
}

// Bring the open folder in line with the disk without losing anything the
// session holds: ratings, flags, labels, sharpness, `touched`, the undo
// history, the errors and the preview all stay, and the current file stays
// current (or, when it was deleted, gives way to the neighbor
// `anchorAfterFilter` picks). `entries` is left alone here; the
// `folder_entries` read on `scan-done` replaces the map wholesale, so rows of
// files that are gone drop out there.
//
// The same open, so no new folder token is minted: a listing that lands after
// another folder was opened is dropped by the guard below.
function resync(trigger: RescanTrigger, deferred = false): void {
  if (openDir === null) {
    return;
  }
  // A rescan while a scan runs would cancel and restart it (`scan_folder`
  // joins the running scan first), so it waits for `faces-done` instead. A
  // burst of triggers collapses into the one pending rescan. It also waits
  // out a deferred operation's invoke, whose confirm dialog closing refocuses
  // the window, so the backend does not see that rescan and refuse it.
  if (scanRunning || resyncInFlight || idle.inFlight) {
    resyncPending ??= trigger;
    debugLog(rescanLine(trigger, "defer"));
    return;
  }
  debugLog(rescanLine(trigger, deferred ? "drained" : "start"));
  const dir = openDir;
  const token = folderToken;
  const previous = files;
  const target = files[index];
  resyncInFlight = true;
  const listed = window.__TAURI__.core.invoke<string[]>("list_arw", { dir });
  Promise.all([listed, listed.then((found) => readOutputs(dir, found))])
    .then(([found, outputs]) => {
      resyncInFlight = false;
      if (dir !== openDir || token !== folderToken) {
        drainResync();
        return;
      }
      allFiles = found;
      setViewOnly(isViewOnly(found));
      applyOutputs(found, outputs);
      refilter(target, true, false, previous);
      renderMeta();
      return startScan(dir);
    })
    .catch((err: unknown) => {
      resyncInFlight = false;
      drainResync();
      setStatus(String(err));
    });
}

function drainResync(): void {
  if (resyncPending === null) {
    return;
  }
  const trigger = resyncPending;
  resyncPending = null;
  resync(trigger, true);
}

interface OutputsRead {
  found: Set<string> | null;
  failure: string | null;
}

// What `outputs_of` says of `listing`. A folder that could not be read is no
// answer, like one with no export folder, and never fails the listing itself.
function readOutputs(dir: string, listing: string[]): Promise<OutputsRead> {
  return window.__TAURI__.core.invoke<string[] | null>("outputs_of", { dir, files: listing }).then(
    (found) => ({ found: found === null ? null : new Set(found), failure: null }),
    (err: unknown) => ({ found: null, failure: `Output filter: ${String(err)}` }),
  );
}

function openDirectory(folder: string, token: number): Promise<void> {
  if (!formatGate.isOpen) {
    return Promise.resolve();
  }
  const listed = window.__TAURI__.core.invoke<string[]>("list_arw", { dir: folder });
  return Promise.all([
    listed,
    listed.then((found) => readOutputs(folder, found)),
    window.__TAURI__.core.invoke<string | null>("last_viewed", { dir: folder }),
    window.__TAURI__.core.invoke<string[]>("viewed_files", { dir: folder }),
  ]).then(([found, outputs, remembered, viewed]) => {
    if (token !== folderToken) {
      return;
    }
    closeContextMenu();
    stopComparing();
    for (const set of shownExif.values()) {
      set.clear();
    }
    allFiles = found;
    setViewOnly(isViewOnly(found));
    entries.clear();
    ratings.clear();
    flags.clear();
    labels.clear();
    fileGenres = new Map();
    fileRanks = new Map();
    applyOutputs(found, outputs);
    pendingResume = resumeTarget(remembered, allFiles);
    viewedFiles = new Set(viewed);
    const order = ordered();
    files = order.filter(passes);
    index = 0;
    selection = single(files[index]);
    if (switchesFolder(openDir, folder, folders.ignoreCase)) {
      void window.__TAURI__.core.invoke("refit_ranking_in_background");
    }
    openDir = folder;
    void folders.reveal(folder, () => token === folderToken);
    void window.__TAURI__.core.invoke("remember_folder", { dir: folder });
    rebuildExifMenu();
    sharpness.clear();
    faceCache.clear();
    eyesCache.clear();
    bursts = new Map();
    touched.clear();
    history.removeWhere(isJudgments);
    redoable.removeWhere(isJudgments);
    errors.clear();
    imported = null;
    fileIndex.clear();
    files.forEach((path, at) => {
      fileIndex.set(path, at);
    });
    refreshEntries();
    strip.setFiles(files);
    seq += 1;
    dwell.cancel();
    shown?.bitmap.close();
    shown = null;
    draw();
    scanning = null;
    scanId = null;
    scanDone = null;
    scanStarted = null;
    progressRefreshedFor = null;
    setScanRunning(false);
    // Open at 0% (only here: a `resync()` shows the bar once `scan-progress`
    // reports real pass-1 work).
    setScanProgress("0%");
    resyncPending = null;
    // A held rename dropped here reports itself, but the `setStatus()`
    // below would clear that note at once.
    const renameDropped = idle.waiting === "Rename…";
    idle.discard();
    void startScan(folder);
    if (files.length === 0) {
      meta = null;
      setStatus(renameDropped ? RENAME_CANCELED : undefined);
      return;
    }
    show();
    if (renameDropped) {
      setStatus(RENAME_CANCELED);
    }
  });
}

function openFolder(): void {
  if (!formatGate.isOpen) {
    return;
  }
  const token = newFolderToken();
  settleIdle(
    window.__TAURI__.core
      .invoke<string | null>("pick_folder")
      .then((folder) => {
        if (folder === null || token !== folderToken) {
          return;
        }
        return openDirectory(folder, token);
      })
      .catch((err: unknown) => {
        setStatus(String(err));
      }),
  );
}

// The menu accelerators of keymap actions (Open Folder, Undo, Redo) stay out
// of the way while the settings, the sequence, the trash, the sidecar or the import modal is open, or a folder
// or file name is being edited, as their keys do.
function modalOpen(): boolean {
  return (
    settings.isOpen ||
    sequenceFlow.isOpen ||
    trashFlow.busy ||
    sidecarFlow.isOpen ||
    dateFolderFlow.isOpen ||
    importFlow.isOpen ||
    folders.isEditing() ||
    strip.isEditing()
  );
}

void window.__TAURI__.event.listen("open-folder", () => {
  if (!modalOpen()) openFolder();
});
// `File > Reload Folder`, and the main window regaining focus: both rescan
// the open folder in place. The focus that follows launch finds no folder
// open yet, or a scan running, so it costs nothing. The focus listener is
// scoped to this window: a global `event.listen` also receives other
// windows' focus.
void window.__TAURI__.event.listen("reload-folder", () => {
  resync("reload");
});
void window.__TAURI__.window.getCurrentWindow().listen("tauri://focus", () => {
  // Skip while the settings modal is open: closing its Clear Cache confirm
  // dialog refocuses this window, and a resync here races clear_index for
  // the Scans lock (`a scan is running`). The folder watcher still catches
  // on-disk changes, and `index-cleared` reopens the folder after a clear.
  // Also skipped within `FOCUS_RESCAN_INTERVAL` of the last scan's start, so
  // an alt-tab back and forth (or the focus a picker dialog hands back right
  // after an open) is not a full rescan each time. The folder watcher catches
  // the on-disk changes too; this rescan is a belt-and-braces re-listing.
  // The folder picker's close is held off by `settleIdle` in `openFolder`
  // instead: `resync()` defers while it is in flight, the picked folder's
  // open discards that rescan and a canceled picker drains it.
  if (!settings.isOpen && focusRescanDue(lastScanAt, Date.now())) {
    lastScanAt = Date.now();
    resync("focus");
  }
});
// The folder watcher's trigger, debounced in Rust. The listener outlives every
// folder, so an event for a folder that is no longer open is dropped.
void window.__TAURI__.event.listen<{ dir: string }>("folder-changed", ({ payload }) => {
  if (payload.dir !== openDir) {
    return;
  }
  resync("watch");
});
void window.__TAURI__.event.listen("undo", () => {
  if (!modalOpen()) undo();
});
void window.__TAURI__.event.listen("redo", () => {
  if (!modalOpen()) redo();
});
// The `View` items: each runs its action as the key does, under the same gates.
void window.__TAURI__.event.listen<string>("menu-action", ({ payload }) => {
  if (
    !formatDialog.hidden ||
    modalOpen() ||
    (folders.hasFocus() && treeGate(payload) === "swallow")
  ) {
    return;
  }
  closeContextMenu();
  runAction(payload);
});
// `Edit > Select All` replaces the predefined item, so it selects a focused
// text input's text itself, and otherwise gates on focus as the keydown path
// does. If the accelerator also reaches the keydown handler, the second run
// is harmless: selecting all is idempotent.
void window.__TAURI__.event.listen("select-all", () => {
  const active = document.activeElement;
  if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
    active.select();
  } else if (!modalOpen() && !folders.hasFocus()) {
    selectAllFiles();
  }
});

// Make `path` the focused file after the selection changed, as a strip click
// does.
function focusFile(path: string): void {
  const at = fileIndex.get(path);
  if (at === undefined) return;
  if (at === index) {
    renderMeta();
    if (comparing) void loadCompare();
    return;
  }
  index = at;
  show();
}

// The MCP companion reads and drives the view through this; every getter is
// live.
const view: ViewApi = {
  get folder() {
    return openDir;
  },
  get files() {
    return files;
  },
  get index() {
    return index;
  },
  get selection() {
    return selection.selected;
  },
  get bursts() {
    return bursts;
  },
  sharpness,
  ratings,
  flags,
  labels,
  get zoomed() {
    return zoomed;
  },
  get comparing() {
    return comparing;
  },
  get compareActive() {
    return compareActivePath;
  },
  get sort() {
    return sortInForce();
  },
  get viewOnly() {
    return viewOnly;
  },
  get filtered() {
    return filterActive();
  },
  eyes: (path) => eyesCache.get(path),
  showPhoto(path) {
    selection = single(path);
    paintSelection();
    focusFile(path);
  },
  selectPhotos(paths) {
    selection = selectionOf(paths);
    paintSelection();
    focusFile(paths[0]);
  },
  setMode(mode) {
    if ((mode === "compare") !== comparing) toggleCompare();
    if (mode !== "compare" && (mode === "zoom") !== zoomed) toggleZoom();
  },
  judge(paths, command) {
    const current = (comparing ? compareActivePath : null) ?? files[index];
    record(paths, paths.includes(current) ? current : paths[0], command, false, current);
  },
};
void window.__TAURI__.event.listen<McpRequest>("mcp-request", ({ payload }) => {
  void respond(payload, view).then((reply) =>
    window.__TAURI__.core.invoke("mcp_reply", { ...reply }),
  );
});

// Tauri intercepts HTML5 drag-and-drop, so a DOM `drop` event never carries a
// usable path; the paths arrive only through these webview events.
function setDragging(dragging: boolean): void {
  document.body.classList.toggle("dragging", dragging);
}

for (const event of ["tauri://drag-enter", "tauri://drag-over"]) {
  void window.__TAURI__.event.listen(event, () => {
    setDragging(true);
  });
}

void window.__TAURI__.event.listen("tauri://drag-leave", () => {
  setDragging(false);
});

// Orders drops among themselves: a later drop's `dropped_folder` call may
// resolve before an earlier one's, so each handler checks it is still the
// most recent drop before minting a folder token (which would otherwise let
// a stale, slow-resolving drop overwrite a newer one).
let dropCounter = 0;

void window.__TAURI__.event.listen<{ paths: string[] }>("tauri://drag-drop", ({ payload }) => {
  setDragging(false);
  if (!formatGate.isOpen) {
    return;
  }
  const [path] = payload.paths;
  if (path === undefined || payload.paths.length > 1) {
    setStatus("Drop a single folder or RAW file.");
    return;
  }
  const drop = ++dropCounter;
  window.__TAURI__.core
    .invoke<string | null>("dropped_folder", { path })
    .then((folder) => {
      if (drop !== dropCounter) {
        return;
      }
      if (folder === null) {
        setStatus("Drop a single folder or RAW file.");
        return;
      }
      return openDirectory(folder, newFolderToken());
    })
    .catch((err: unknown) => {
      setStatus(String(err));
    });
});

void window.__TAURI__.event.listen<{
  dir: string;
  scan_id: number;
  done: number;
  total: number;
  ready: string[];
}>("scan-progress", ({ payload }) => {
  if (payload.scan_id !== scanId) {
    return;
  }
  scanning = `scanning ${payload.done} / ${payload.total}`;
  setScanProgress(progressWidth(payload.done, payload.total));
  renderMeta();
  strip.markReady(payload.ready);
  // Only when the row the focus mark needs is still missing, and then only on
  // the tick that commits it or once the current file changed, so a 10/s
  // progress stream does not re-read every row on every tick while it waits.
  const current = files.length > 0 ? files[index] : undefined;
  const hasRow = current !== undefined && entries.has(current);
  if (refreshOnProgress(current, hasRow, payload.ready, progressRefreshedFor)) {
    progressRefreshedFor = current ?? null;
    refreshEntries();
  }
});

void window.__TAURI__.event.listen<{
  dir: string;
  scan_id: number;
  total: number;
  errors: number;
}>("scan-done", ({ payload }) => {
  if (payload.scan_id !== scanId) {
    return;
  }
  scanErrors = payload.errors;
  scanDone = { scanId: payload.scan_id, total: payload.total };
  scanning = payload.errors === 0 ? null : `${payload.errors} failed`;
  setScanProgress(null);
  renderMeta();
  strip.refresh();
  if (refreshOnScanDone(scanStarted, payload.scan_id, payload.total)) {
    refreshEntries();
  }
});

// The second pass: the focus candidate state and the sharpness score of the
// files it has written, patched into `entries` and `sharpness` in place rather
// than re-read.
void window.__TAURI__.event.listen<{
  dir: string;
  scan_id: number;
  done: number;
  total: number;
  ready: FaceReady[];
}>("faces-progress", ({ payload }) => {
  if (payload.scan_id !== scanId) {
    return;
  }
  scanning = `analyzing ${payload.done} / ${payload.total}`;
  const current = applyFaceReady(entries, payload.ready, files[index]);
  applySharpnessReady(entries, sharpness, payload.ready);
  applyTiers();
  // The candidate, tier and eyes filters fill in as the pass runs; the strip keeps
  // its scroll offset, as on a resync.
  if (shownCandidates.size > 0 || shownEyes.size > 0) {
    refilter(files[index], true);
  }
  renderMeta();
  if (current) {
    draw();
  }
});

void window.__TAURI__.event.listen<{
  dir: string;
  scan_id: number;
  total: number;
  errors: number;
}>("faces-done", ({ payload }) => {
  if (payload.scan_id !== scanId) {
    return;
  }
  if (scanDone?.scanId !== payload.scan_id) {
    void window.__TAURI__.core
      .invoke("log_frontend", {
        kind: "invariant",
        line: `faces-done after scan-done: dir=${payload.dir} scan_id=${payload.scan_id}`,
      })
      .catch(() => {});
  }
  setScanRunning(false);
  idle.drain();
  const failed = scanErrors + payload.errors;
  scanning = failed === 0 ? null : `${failed} failed`;
  renderMeta();
  if (refreshOnFacesDone(scanDone, payload.scan_id, payload.total)) {
    refreshEntries();
  }
  drainResync();
});

// A sidecar the writer could not write: the writer retries it a few times,
// and if it still fails the judgment is still in the index and is retried
// on the next open of the folder, so this is a sticky error, not a revert.
void window.__TAURI__.event.listen<{ path: string; message: string }>(
  "sidecar-error",
  ({ payload }) => {
    if (!allFiles.includes(payload.path)) {
      return;
    }
    errors.add(payload.path, `${baseName(payload.path)}: ${payload.message}`);
    renderMeta();
  },
);

// The settings modal switched the format and the backend has reset the index:
// reopen the folder so the strip and the meta pane show the newly selected
// format's judgments. A fresh token drops any open still in flight.
void window.__TAURI__.event.listen<string>("sidecar-format", () => {
  if (openDir === null) {
    return;
  }
  openDirectory(openDir, newFolderToken()).catch((err: unknown) => {
    setStatus(String(err));
  });
});

// The settings modal cleared the index cache: the open folder's thumbnails
// and cached metadata are gone, so reopen it and let the scan fill them in
// again. A fresh token drops any open still in flight.
void window.__TAURI__.event.listen("index-cleared", () => {
  if (openDir === null) {
    return;
  }
  openDirectory(openDir, newFolderToken()).catch((err: unknown) => {
    setStatus(String(err));
  });
});

// AI analysis turned on with its model in place (the settings checkbox, or a
// model download finishing): rescan the open folder so its third pass
// embeds it. The initial read only sets the baseline.
let aiWasActive: boolean | null = null;
function applyAiState(state: AiState): void {
  setGenreSection(state.enabled);
  setRanking(state.enabled);
  const active = aiActive(state);
  if (aiWasActive === false && active) {
    resync("ai");
  }
  aiWasActive = active;
}
void window.__TAURI__.core.invoke<AiState>("ai_enabled").then((state) => {
  setGenreSection(state.enabled);
  setRanking(state.enabled);
  aiWasActive ??= aiActive(state);
});

// The personal ranking runs while AI analysis is on: the sort menu's
// `Recommended` item shows and the meta pane gets its rows.
function setRanking(enabled: boolean): void {
  if (rankingOn === enabled) {
    return;
  }
  rankingOn = enabled;
  sortRecommended.hidden = !enabled;
  syncSortChecks();
  if (enabled) {
    refreshRanks();
    return;
  }
  fileRanks = new Map();
  rankModel = null;
  if (sortFor(viewOnly, sortKey) === "recommended") {
    refilter();
  }
  applyRecommended();
  renderMeta();
}

// A refit (the settings' button, or the background refit at startup and on a
// folder switch) changes every score.
void window.__TAURI__.event.listen("ranking-updated", () => {
  refreshRanks();
});

// The filter menu's `Genre` section shows while AI analysis is on. Turning
// it off unchecks the section, so a hidden item filters nothing.
function setGenreSection(enabled: boolean): void {
  if (filterGenre.hidden !== enabled) {
    return;
  }
  filterGenre.hidden = !enabled;
  if (enabled) {
    refreshGenres();
    return;
  }
  fileGenres = new Map();
  if (shownGenres.size > 0) {
    shownGenres.clear();
    mirrorFilter();
    refilter();
  }
}

// The filter menu's `Output` section shows while the open folder has an export
// folder that could be read. Hiding it unchecks the section, so a hidden item
// filters nothing. Both callers filter right after, so no `refilter()` here:
// it would drop `resync`'s `previous` and run on `openDirectory`'s half-switched
// state.
function setOutputSection(found: Set<string> | null): void {
  fileOutputs = found;
  filterOutput.hidden = found === null;
  if (found === null && shownOutputs.size > 0) {
    shownOutputs.clear();
    mirrorFilter();
  }
}

function applyOutputs(listing: string[], { found, failure }: OutputsRead): void {
  outputsListing = new Set(listing);
  outputsFailure = failure;
  setOutputSection(found);
}

// The third pass stores embeddings as it runs: a checked `Genre` item fills
// in as it goes, and its end brings every label in.
void window.__TAURI__.event.listen<{ scan_id: number }>("embed-progress", ({ payload }) => {
  if (payload.scan_id === scanId && shownGenres.size > 0) {
    refreshGenres();
  }
});
void window.__TAURI__.event.listen<{ scan_id: number }>("embed-done", ({ payload }) => {
  if (payload.scan_id === scanId) {
    refreshGenres();
    refreshRanks();
  }
});
void window.__TAURI__.event.listen<AiState>("ai-state", ({ payload }) => {
  applyAiState(payload);
});

// The settings modal's "Timing logs" item toggles this; read the backend's
// `TimingLogs` state too, so a reloaded main window keeps it.
void window.__TAURI__.core.invoke<boolean>("timing_logs").then((enabled) => {
  debugLogging = enabled;
});

// The settings modal's Auto-advance checkbox sets this.
let autoAdvance = false;
void window.__TAURI__.core.invoke<boolean>("auto_advance").then((enabled) => {
  autoAdvance = enabled;
});

const filterToggle = document.getElementById("filter-toggle") as HTMLButtonElement;
const filterMenu = document.getElementById("filter-menu") as HTMLDivElement;
const filterItems = filterMenu.querySelectorAll<HTMLButtonElement>(
  "[data-flag], [data-stars], [data-label], [data-orientation], [data-candidate], [data-eyes], [data-output], [data-genre]",
);
const filterOutput = document.getElementById("filter-output") as HTMLDivElement;
const filterGenre = document.getElementById("filter-genre") as HTMLDivElement;
const filterExif = document.getElementById("filter-exif") as HTMLDivElement;
// The icons of the `AF eye` items, the strip's icon of the same state in the
// focus mark's color; `Unknown` keeps its slot empty.
for (const state of ["good", "not_candidate"] as const) {
  const face = filterMenu.querySelector<HTMLElement>(`[data-candidate="${state}"] .face`)!;
  face.innerHTML = FOCUS_MARK_ICONS[state];
  face.style.color = FOCUS_MARK_COLORS[state];
}

function exifSelected(): boolean {
  return [...shownExif.values()].some((set) => set.size > 0);
}

// Whether any flag, star, label, orientation, candidate, eyes, genre, output or
// EXIF filter is checked.
function filterActive(): boolean {
  return (
    shownFlags.size +
      shownStars.size +
      shownLabels.size +
      shownOrientations.size +
      shownCandidates.size +
      shownEyes.size +
      shownGenres.size +
      shownOutputs.size >
      0 || exifSelected()
  );
}

// Rebuild the EXIF sections from `entries`: one item per label present in the
// folder. A checked label no longer present is dropped, so a stale selection
// cannot hide everything.
function rebuildExifMenu(): void {
  filterExif.replaceChildren();
  for (const { group, heading } of exifGroups) {
    const found = new Map<string, number | string>();
    for (const row of entries.values()) {
      const key = exifKey(row.exif, group);
      if (key !== null) {
        found.set(key.label, key.order);
      }
    }
    const set = shownExif.get(group)!;
    for (const label of set) {
      if (!found.has(label)) {
        set.delete(label);
      }
    }
    if (found.size === 0) {
      continue;
    }
    const sorted = [...found].sort(([a, x], [b, y]) =>
      typeof x === "number" && typeof y === "number"
        ? x - y || a.localeCompare(b)
        : String(x).localeCompare(String(y)),
    );
    const title = document.createElement("div");
    title.className = "menu-label";
    title.textContent = heading;
    filterExif.append(menuSeparator(), title);
    for (const [label] of sorted) {
      const item = document.createElement("button");
      item.type = "button";
      item.className = "menu-item";
      item.setAttribute("role", "menuitemcheckbox");
      item.setAttribute("aria-checked", String(set.has(label)));
      item.dataset.group = group;
      item.dataset.value = label;
      item.textContent = label;
      addMenuCheck(item);
      filterExif.append(item);
    }
  }
  filterToggle.classList.toggle("active", filterActive());
}

function setFilterMenuOpen(open: boolean): void {
  filterMenu.hidden = !open;
  filterToggle.setAttribute("aria-expanded", String(open));
}

// Mirror the sets onto the menu's check marks and the button's lit state.
function mirrorFilter(): void {
  for (const item of filterItems) {
    const { flag, stars, label, orientation, candidate, eyes, genre, output } = item.dataset;
    const checked =
      flag !== undefined
        ? shownFlags.has(flag as Flag)
        : label !== undefined
          ? shownLabels.has(label)
          : orientation !== undefined
            ? shownOrientations.has(orientation as Orientation)
            : candidate !== undefined
              ? shownCandidates.has(candidate as AfEye)
              : eyes !== undefined
                ? shownEyes.has(eyes as EyeState)
                : genre !== undefined
                  ? shownGenres.has(genre)
                  : output !== undefined
                    ? shownOutputs.has(output as OutputState)
                    : shownStars.has(Number(stars));
    item.setAttribute("aria-checked", String(checked));
  }
  for (const item of filterExif.querySelectorAll<HTMLButtonElement>("[data-group]")) {
    const { group, value } = item.dataset;
    item.setAttribute("aria-checked", String(shownExif.get(group as ExifGroup)!.has(value!)));
  }
  filterToggle.classList.toggle("active", filterActive());
}

// Mirror the sets, rebuild the view and remember the filter.
function filterChanged(): void {
  mirrorFilter();
  refilter();
  void window.__TAURI__.core.invoke("set_filter", { filter: toStored(filterState) });
}

filterToggle.addEventListener("click", () => {
  // Drop the focus, or the zoom key (the 1:1 toggle) would press it again.
  filterToggle.blur();
  setFilterMenuOpen(!!filterMenu.hidden);
});

// The menu stays open while items are toggled, so several can be checked in
// one go; a click anywhere else closes it.
document.addEventListener("mousedown", (event) => {
  if (!filterMenu.hidden && !(event.target as Element).closest("#filter")) {
    setFilterMenuOpen(false);
  }
});

for (const item of filterItems) {
  item.addEventListener("click", () => {
    item.blur();
    const { flag, stars, label, orientation, candidate, eyes, genre, output } = item.dataset;
    const set: Set<string | number> =
      flag !== undefined
        ? shownFlags
        : label !== undefined
          ? shownLabels
          : orientation !== undefined
            ? shownOrientations
            : candidate !== undefined
              ? shownCandidates
              : eyes !== undefined
                ? shownEyes
                : genre !== undefined
                  ? shownGenres
                  : output !== undefined
                    ? shownOutputs
                    : shownStars;
    const value =
      flag ?? label ?? orientation ?? candidate ?? eyes ?? genre ?? output ?? Number(stars);
    if (set.has(value)) {
      set.delete(value);
    } else {
      set.add(value);
    }
    filterChanged();
  });
}

filterExif.addEventListener("click", (event) => {
  const item = (event.target as Element).closest<HTMLButtonElement>("[data-group]");
  if (item === null) {
    return;
  }
  item.blur();
  const set = shownExif.get(item.dataset.group as ExifGroup)!;
  const value = item.dataset.value!;
  if (set.has(value)) {
    set.delete(value);
  } else {
    set.add(value);
  }
  filterChanged();
});

(document.getElementById("filter-reset") as HTMLButtonElement).addEventListener(
  "click",
  (event) => {
    (event.currentTarget as HTMLButtonElement).blur();
    shownFlags.clear();
    shownStars.clear();
    shownLabels.clear();
    shownOrientations.clear();
    shownCandidates.clear();
    shownEyes.clear();
    shownGenres.clear();
    shownOutputs.clear();
    for (const set of shownExif.values()) {
      set.clear();
    }
    filterChanged();
  },
);

const sortToggle = document.getElementById("sort-toggle") as HTMLButtonElement;
const sortMenu = document.getElementById("sort-menu") as HTMLDivElement;
const sortItems = sortMenu.querySelectorAll<HTMLButtonElement>("[data-sort]");
const sortRecommended = document.getElementById("sort-recommended") as HTMLDivElement;

function setSortMenuOpen(open: boolean): void {
  sortMenu.hidden = !open;
  sortToggle.setAttribute("aria-expanded", String(open));
}

sortToggle.addEventListener("click", () => {
  sortToggle.blur();
  setSortMenuOpen(!!sortMenu.hidden);
});

document.addEventListener("mousedown", (event) => {
  if (!sortMenu.hidden && !(event.target as Element).closest("#sort")) {
    setSortMenuOpen(false);
  }
});

for (const item of sortItems) {
  item.addEventListener("click", () => {
    item.blur();
    setSortKey(item.dataset.sort as SortKey);
    void window.__TAURI__.core.invoke("set_sort_order", { order: sortKey });
    setSortMenuOpen(false);
    refilter();
  });
}

// The sort menu is off in a view-only folder, so the user's persisted key is
// neither shown as in force nor overwritten there. A rescan can turn a folder
// whose RAWs were all deleted view-only, so its judgments leave the undo
// history too; a trash run stays, to bring the RAWs back.
function setViewOnly(next: boolean): void {
  viewOnly = next;
  sortToggle.disabled = next;
  if (next) {
    setSortMenuOpen(false);
    history.removeWhere(isJudgments);
    redoable.removeWhere(isJudgments);
  }
}

function setSortKey(key: SortKey): void {
  sortKey = key;
  syncSortChecks();
}

// Checks the item of the sort in force, so `Capture time` shows checked while
// the hidden `Recommended` is the persisted key.
function syncSortChecks(): void {
  const inForce = sortKey === "recommended" && !rankingOn ? "capture" : sortKey;
  for (const item of sortItems) {
    item.setAttribute("aria-checked", String(item.dataset.sort === inForce));
  }
}

function showFormatDialog(): void {
  formatDialog.hidden = false;
  formatButtons[0].focus();
  void window.__TAURI__.core
    .invoke<{ names: LabelNames; presets: LabelPreset[] }>("label_names")
    .then(({ presets }) => {
      formatPresets = presets;
      formatLanguageSelect.replaceChildren(
        ...presets.map((preset) => new Option(preset.name, preset.code)),
      );
      formatLanguageSelect.value = defaultPreset(presets, navigator.language)?.code ?? "";
    })
    .catch((err: unknown) => {
      formatError.textContent = String(err);
      formatError.hidden = false;
    });
}

// Saves the label names first when the format writes XMP, then the format: the
// format is the one key that keeps the dialog from coming back, so it goes last
// and a failed names save leaves the dialog to reappear. The dialog closes only
// once both are saved.
function saveFormat(format: string, names: LabelNames | null, refocus: HTMLElement): void {
  const controls = [...formatButtons, formatLanguageSelect, formatContinue];
  for (const control of controls) {
    control.disabled = true;
  }
  formatError.hidden = true;
  Promise.resolve(
    names === null ? undefined : window.__TAURI__.core.invoke("set_label_names", { names }),
  )
    .then(() => window.__TAURI__.core.invoke("choose_sidecar_format", { format }))
    .then(
      () => {
        formatDialog.hidden = true;
        formatGate.open();
      },
      (err: unknown) => {
        formatError.textContent = String(err);
        formatError.hidden = false;
      },
    )
    .finally(() => {
      for (const control of controls) {
        control.disabled = false;
      }
      if (!formatDialog.hidden) {
        refocus.focus();
      }
    });
}

for (const button of formatButtons) {
  button.addEventListener("click", () => {
    const format = button.dataset.format ?? "";
    if (asksLanguage(format)) {
      chosenFormat = format;
      formatLanguage.hidden = false;
      formatLanguageSelect.focus();
      return;
    }
    saveFormat(format, null, button);
  });
}

formatContinue.addEventListener("click", () => {
  if (chosenFormat === null) {
    return;
  }
  const preset = formatPresets.find((p) => p.code === formatLanguageSelect.value);
  if (preset === undefined) {
    return;
  }
  saveFormat(chosenFormat, preset.names, formatContinue);
});

// Ask for the developing software while no sidecar format is saved. A failed
// check lets folders open, as the backend does for a store it cannot open.
void window.__TAURI__.core.invoke<boolean>("sidecar_format_saved").then(
  (saved) => {
    if (saved) {
      formatGate.open();
    } else {
      showFormatDialog();
    }
  },
  () => {
    formatGate.open();
  },
);
const side = document.getElementById("side") as HTMLElement;
const film = document.getElementById("film") as HTMLElement;
const info = document.getElementById("info") as HTMLElement;
let panels: Panels = { left: true, strip: true, right: true };

// Show and hide the panes, then fit the viewer to the space they leave. The
// strip rendered nothing while hidden (its width was 0), so it renders again
// around the current cell when it comes back.
function applyPanels(next: Panels): void {
  const stripShown = next.strip && !panels.strip;
  panels = next;
  side.hidden = !panels.left;
  film.hidden = !panels.strip;
  info.hidden = !panels.right;
  if (!panels.strip) {
    setFilterMenuOpen(false);
    setSortMenuOpen(false);
  }
  if (stripShown) {
    strip.setCurrent(index);
  }
  draw();
  if (zoomed) {
    scheduleCropForResize();
  }
}

function changePanels(next: Panels): void {
  applyPanels(next);
  // A hidden pane must not keep the keyboard, or the culling keys stay gated.
  if (!panels.left) {
    folders.blur();
  }
  void window.__TAURI__.core.invoke("set_panels", { panels });
}

void window.__TAURI__.core.invoke<Panels>("panels").then(applyPanels, () => {});

// Apply the remembered sort before the last folder opens, so it comes up in
// that order.
const sortLoaded = window.__TAURI__.core.invoke<SortKey>("sort_order").then(setSortKey, () => {});

// Restore the remembered filter the same way, so the folder's first list is
// already filtered. Only the menu is mirrored: no folder is open to refilter.
const filterLoaded = window.__TAURI__.core.invoke<StoredFilter>("filter").then(
  (stored) => {
    applyStored(filterState, stored);
    mirrorFilter();
  },
  () => {},
);

void Promise.allSettled([sortLoaded, filterLoaded]).then(() => {
  formatGate.whenOpen(reopenLastFolder);
});

// `Settings...` in the menu. The first-launch dialog is modal already, so the
// settings wait until it is answered.
void window.__TAURI__.event.listen("open-settings", () => {
  if (
    !formatDialog.hidden ||
    sequenceFlow.busy ||
    trashFlow.busy ||
    sidecarFlow.busy ||
    dateFolderFlow.busy ||
    importFlow.busy
  ) {
    return;
  }
  setFilterMenuOpen(false);
  setSortMenuOpen(false);
  closeContextMenu();
  settings.open();
});

// Key -> action, from the `shortcuts` command. Empty until it resolves.
let keymap = new Map<string, string>();

function applyKeymap(bindings: Binding[]): void {
  keymap = new Map(
    bindings.flatMap(({ action, keys }) => keys.map((key) => [key, action] as const)),
  );
  keyBindings = bindings;
  renderEmpty();
}

const keymapLoaded = window.__TAURI__.core.invoke<Binding[]>("shortcuts").then(applyKeymap);

// The tree's roots come after the keymap and the sort order, which the first
// frame needs more.
void Promise.allSettled([sortLoaded, keymapLoaded]).then(folders.loadRoots);

window.addEventListener("keydown", (event) => {
  folders.cancelSlowClick();
  strip.cancelSlowClick();
  // The dialog's controls take Enter, Space and the arrows natively, but Tab
  // would move focus past them to controls behind the overlay (there is no
  // `inert` on the `safari13` target), so trap it by cycling within the
  // dialog's visible controls (the language block shows only after XMP or Both).
  if (!formatDialog.hidden) {
    if (event.key === "Tab") {
      event.preventDefault();
      const controls = [
        ...formatDialog.querySelectorAll<HTMLButtonElement | HTMLSelectElement>("button, select"),
      ].filter((control) => control.closest("[hidden]") === null);
      const from = controls.indexOf(document.activeElement as HTMLButtonElement);
      controls[cycleFocus(controls.length, from, event.shiftKey ? -1 : 1)]?.focus();
    }
    return;
  }
  if (settings.isOpen) {
    settings.keydown(event);
    return;
  }
  if (sequenceFlow.isOpen) {
    sequenceKeydown(event);
    return;
  }
  if (trashFlow.busy) {
    trashKeydown(event);
    return;
  }
  if (sidecarFlow.isOpen) {
    sidecarKeydown(event);
    return;
  }
  if (dateFolderFlow.isOpen) {
    dateFolderKeydown(event);
    return;
  }
  if (importFlow.isOpen) {
    importKeydown(event);
    return;
  }
  // A live file rename takes every key: Enter and Escape end it, the rest
  // are the input's own (the `Enter` committing an IME conversion included).
  if (strip.isEditing()) {
    const decision = event.isComposing ? "native" : editKey(keyName(event));
    if (decision !== "native") {
      event.preventDefault();
      strip.finishRename(decision);
    }
    return;
  }
  const key = keyName(event);
  if (key === null) {
    return;
  }
  // Ahead of the tree, whose own `Escape` would hand the keys back instead.
  if (!contextMenu.hidden) {
    closeContextMenu();
    if (key === "escape") {
      event.preventDefault();
      return;
    }
  }
  if (folders.hasFocus()) {
    if (folders.keydown(event)) {
      return;
    }
    const action = keymap.get(key);
    if (treeGate(action) === "swallow") {
      if (action !== undefined) {
        event.preventDefault();
      }
      return;
    }
  }
  if (key === "escape" && !filterMenu.hidden) {
    setFilterMenuOpen(false);
    event.preventDefault();
    return;
  }
  if (key === "escape" && !sortMenu.hidden) {
    setSortMenuOpen(false);
    event.preventDefault();
    return;
  }
  if (key === "escape" && comparing) {
    toggleCompare();
    event.preventDefault();
    return;
  }
  const action = keymap.get(key);
  if (action === "grayscale") {
    setGrayscale(event.code);
    event.preventDefault();
    return;
  }
  if (action !== undefined && runAction(action)) {
    event.preventDefault();
  }
});

// Match the physical key: with a modified binding, whichever key goes up first ends the hold.
window.addEventListener("keyup", (event) => {
  if (grayscaleHeld !== null && (event.code === grayscaleHeld || isModifierCode(event.code))) {
    setGrayscale(null);
  }
});

window.addEventListener("blur", () => {
  setGrayscale(null);
});

function setGrayscale(code: string | null): void {
  grayscaleHeld = code;
  canvas.classList.toggle("grayscale", code !== null);
}

function runAction(action: string): boolean {
  const current = files[index];
  let judged = 0;
  switch (action) {
    case "previous":
      move(-1);
      break;
    case "next":
      move(1);
      break;
    case "burstPrevious":
      moveBurst(-1);
      break;
    case "burstNext":
      moveBurst(1);
      break;
    case "burstFramePrevious":
      moveBurstFrame(-1);
      break;
    case "burstFrameNext":
      moveBurstFrame(1);
      break;
    case "extendPrevious":
      extendSelection(-1);
      break;
    case "extendNext":
      extendSelection(1);
      break;
    case "selectAll":
      selectAllFiles();
      break;
    case "focus":
      showFocus = !showFocus;
      draw();
      break;
    case "zoom":
      toggleZoom();
      break;
    case "compare":
      toggleCompare();
      break;
    case "performance":
      togglePerf();
      break;
    case "open":
      openFolder();
      break;
    case "undo":
      undo();
      break;
    case "redo":
      redo();
      break;
    case "toggleLeft":
      changePanels(toggle(panels, "left"));
      break;
    case "toggleRight":
      changePanels(toggle(panels, "right"));
      break;
    case "toggleStrip":
      changePanels(toggle(panels, "strip"));
      break;
    case "toggleSides":
      changePanels(toggleSides(panels));
      break;
    case "rate1":
    case "rate2":
    case "rate3":
    case "rate4":
    case "rate5": {
      const stars = Number(action.slice(-1));
      judged = judge(() => (own) => ({ ...own, rating: stars }));
      break;
    }
    case "reject":
      // Sticky, not a toggle: reject twice is still a reject, and it replaces
      // a pick and keeps the stars. Unflag undoes it.
      judged = judge(() => (own) => ({ ...own, flag: "reject" }));
      break;
    case "rejectRest":
      rejectRest();
      break;
    case "pick":
      // Sticky like reject, replacing a reject and keeping the stars.
      judged = judge(() => (own) => ({ ...own, flag: "pick" }));
      break;
    case "unflag":
      // Clears a reject or a pick and keeps the stars; does nothing to a file
      // with neither.
      judged = judge(() => (own) => ({ ...own, flag: "none" }));
      break;
    case "clear":
      // Clears the stars and leaves the flag alone.
      judge(() => (own) => ({ ...own, rating: null }));
      break;
    case "red":
    case "orange":
    case "yellow":
    case "green":
    case "blue":
    case "pink":
    case "purple": {
      // Toggles: the same color again clears it; another color replaces it.
      const name = action.charAt(0).toUpperCase() + action.slice(1);
      judge((focused) => {
        const label = focused.label === name ? null : name;
        return (own) => ({ ...own, label });
      });
      break;
    }
    case "clearlabel":
      judge(() => (own) => ({ ...own, label: null }));
      break;
    case "clearall":
      // Clears the stars, the flag and the label in one undo entry.
      judge(() => () => ({ rating: null, flag: "none", label: null }), true);
      break;
    default:
      return false;
  }
  // A file that dropped out of the filter already moved the cursor on.
  if (
    shouldAdvance({
      action,
      targets: judged,
      autoAdvance,
      comparing,
      stillCurrent: files[index] === current,
    })
  ) {
    move(1);
  }
  return true;
}

function fitViewer(): void {
  draw();
  drawPerf();
  if (zoomed) {
    scheduleCropForResize();
  }
}

window.addEventListener("resize", fitViewer);
// The strip's horizontal scrollbar coming or going grows or shrinks `#film`,
// and so `#viewer`, without a window `resize`.
new ResizeObserver(fitViewer).observe(document.getElementById("viewer") as HTMLElement);

renderMeta();
draw();
