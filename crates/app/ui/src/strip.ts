// The thumbnail filmstrip along the bottom edge: a hand-rolled virtual list
// over the cached thumbnails the `thumbnail` command serves. A file is
// renamed in place: its cell's name becomes a text input until Enter, Escape
// or a click away ends the edit.

import { type BurstMark, burstBadge } from "./burst.js";
import { carriedIndices, carriedOffset } from "./carry.js";
import { FOCUS_MARK_COLORS, FOCUS_MARK_ICONS, type MarkState } from "./focus.js";
import { CLOCK_SVG, SPARKLES_SVG } from "./icons.js";
import {
  type Decision,
  type InlineRename,
  PENDING_TITLE,
  type Pending,
  PendingRename,
  RenamesInFlight,
  SLOW_CLICK_DELAY,
  SlowClick,
  commit,
  displayName,
  editOutcome,
  inlineRename,
  stemLength,
} from "./rename.js";
import type { Modifiers, PickFlag } from "./selection.js";

// Header layout of a `thumbnail` payload, see `crates/app/src/commands.rs`.
const THUMBNAIL_HEADER_LEN = 8;
const THUMBNAIL_KIND_JPEG_V1 = 2;

const strip = document.getElementById("strip") as HTMLDivElement;
const inner = document.getElementById("strip-inner") as HTMLDivElement;

// Cell geometry. A cell is a 144x96 image box plus the file name, and 96x144
// upright after a quarter turn. The pitch is read from the `--cell-width`
// custom property in `style.css` (the single source of truth for cell
// placement) rather than duplicated here. A fixed width keeps
// `scrollLeft -> index` arithmetic, which is what makes virtualization cheap.
const CELL_WIDTH = Number.parseFloat(getComputedStyle(inner).getPropertyValue("--cell-width"));
// Cells kept beyond the visible range, so a short scroll shows an image that
// is already decoded.
const RANGE_MARGIN = 4;
// Concurrent `thumbnail` invokes. The IPC hop, not the decode, is the cost.
const MAX_IN_FLIGHT = 4;

interface Cell {
  el: HTMLDivElement;
  img: HTMLImageElement;
  badge: HTMLSpanElement;
  flag: HTMLSpanElement;
  count: HTMLSpanElement;
  tier: HTMLSpanElement;
  recommended: HTMLSpanElement;
  reason: HTMLSpanElement;
  name: HTMLSpanElement;
  url: string | null;
  // Read by the listeners, since `setFiles` moves a cell whose file is still
  // listed to its new index.
  index: number;
}

let files: string[] = [];
// Path -> index in `files`, for `ready`. The strip's order is the UI's
// sorted and filtered order, so a scanned path has no positional
// correspondence with the scan's own list.
let indexOf = new Map<string, number>();
let current = 0;
// Bumped on every `setFiles`; a response tagged with an older generation
// belongs to a folder that is no longer open and is dropped.
let generation = 0;
const cells = new Map<number, Cell>();
// Indices with a request in flight or already answered, so a cell scrolling
// back into view does not ask again.
const requested = new Set<number>();
// Indices with an `invoke` currently in flight. Unlike `requested`, `render`
// never clears this, so a cell that scrolls out and back in while its
// request is still pending does not issue a second `invoke`.
const inFlightIndices = new Set<number>();
// Indices the index has no thumbnail for yet (the scan has not reached them,
// or the file errored). Cleared on `refresh`, and cleared per path by
// `ready`, so the cell is requested again.
const missing = new Set<number>();
// Indices the scan has reported committed. A request that was in flight when
// its path was reported comes back `Err` from a query that ran before the
// commit, and must not be left `missing` until `scan-done`.
const ready = new Set<number>();
// Indices whose request failed for a reason other than "not yet scanned".
// Kept separate from `missing` so a real failure is shown once instead of
// being retried forever like a not-yet-scanned file.
const failed = new Set<number>();
// The stars per index, mirroring the `ratings` map in `main.ts`: `1`-`5`
// stars, and a missing entry is unrated.
const ratings = new Map<number, number>();
// The pick / reject per index, mirroring the `flags` map in `main.ts`.
const flags = new Map<number, "pick" | "reject">();
// The color label per index, mirroring the `labels` map in `main.ts`.
const labels = new Map<number, string>();
// The burst band and badge per index, from `burstMarks` in `burst.ts`; a missing
// entry is not in a burst of two or more.
const bursts = new Map<number, BurstMark>();
// The focus mark state of the indices whose file has one other than `neutral`
// (`stripState` in `focus.ts`).
const tiers = new Map<number, MarkState>();
// The indices the personal ranking puts first in their burst (`recommended`
// in `recommend.ts`).
const recommendations = new Set<number>();
// Why the scan failed on a file, per index, from the index's error row.
const failures = new Map<number, string>();
// The selected indices besides `current`, mirroring the selection in
// `main.ts`.
const selected = new Set<number>();
const LABEL_COLORS = new Set(["red", "orange", "yellow", "green", "blue", "pink", "purple"]);
let inFlight = 0;
let select: (index: number, modifiers: Modifiers) => void = () => {};
let contextMenu: (index: number, x: number, y: number) => void = () => {};
let rename: (path: string, name: string) => void = () => {};
let canRename: () => boolean = () => false;
let cancelPending: (path: string) => void = () => {};
let reportError: (message: string) => void = () => {};
// The file rename held until the scan ends, keyed by path so it survives a
// cell being released and recreated, and `setFiles`.
const pending = new PendingRename();
// The files whose `rename_file` invoke is out: their cells still show paths
// about to move, so an edit on one is refused.
const renamesInFlight = new RenamesInFlight();
// The live inline rename and the index of its cell, which `render` keeps in
// the DOM even outside the virtual range. The cell's `name` span stays in
// `Cell` while the input stands in for it, so a badge repaint leaves the
// input alone.
let editing: { state: InlineRename; index: number; input: HTMLInputElement } | null = null;
const slow = new SlowClick();
let slowTimer: ReturnType<typeof setTimeout> | undefined;

// A rated cell carries its stars in the top-right corner. A picked or
// rejected cell carries a dot in the top-left corner, green or red (the two
// never coexist), and a rejected cell is also dimmed, its stars still shown.
function paintRating(index: number, cell: Cell): void {
  const rating = ratings.get(index);
  const rejected = flags.get(index) === "reject";
  const picked = flags.get(index) === "pick";
  cell.badge.textContent = rating === undefined ? "" : "\u2605".repeat(rating);
  cell.el.classList.toggle("rejected", rejected);
  cell.flag.textContent = picked || rejected ? "\u25CF" : "";
  cell.flag.classList.toggle("pick", picked);
  cell.flag.classList.toggle("reject", rejected);
  // The label tints the file-name strip along the bottom edge, as Lightroom's
  // cell does; a name outside the seven colors gets a neutral gray.
  const label = labels.get(index);
  const key = label?.toLowerCase();
  cell.name.classList.toggle("labeled", label !== undefined);
  cell.name.style.setProperty(
    "--label",
    label === undefined
      ? ""
      : LABEL_COLORS.has(key!)
        ? `var(--label-${key})`
        : "var(--label-other)",
  );
  cell.name.title = label ?? "";
}

// A band behind the cell, joined to the next and previous member of the same
// burst. Every member carries its `position/size` in the whole burst.
function paintBurst(index: number, cell: Cell): void {
  const value = bursts.get(index);
  cell.el.classList.toggle("burst", value !== undefined);
  cell.el.classList.toggle("burst-first", value?.first === true);
  cell.el.classList.toggle("burst-last", value?.last === true);
  cell.count.textContent = burstBadge(value ?? null);
}

// The focus mark state's icon at the image box's bottom-left, in the mark's
// color; none for a neutral one.
function paintTier(index: number, cell: Cell): void {
  const state = tiers.get(index) ?? "neutral";
  const icon = FOCUS_MARK_ICONS[state];
  cell.tier.hidden = icon === null;
  cell.tier.innerHTML = icon ?? "";
  cell.tier.style.color = FOCUS_MARK_COLORS[state];
}

// The recommended icon, right of the focus mark state's icon.
function paintRecommended(index: number, cell: Cell): void {
  const on = recommendations.has(index);
  cell.recommended.hidden = !on;
  cell.recommended.innerHTML = on ? SPARKLES_SVG : "";
}

// The scan's error text of a failed file, in the cell's tooltip and, on a
// `failed` cell, where the thumbnail would be.
function paintFailure(index: number, cell: Cell): void {
  const message = failures.get(index) ?? "";
  cell.el.title = message;
  cell.reason.textContent = message;
}

// The file's name, or its held rename's name in the pending style.
function paintName(cell: Cell): void {
  const path = files[cell.index];
  cell.name.textContent = displayName(pending.current, path, baseName(path));
  const held = pending.current !== null && pending.current.path === path;
  cell.name.classList.toggle("pending", held);
  if (held) {
    const icon = document.createElement("span");
    icon.className = "pending-icon";
    icon.title = PENDING_TITLE;
    icon.innerHTML = CLOCK_SVG;
    cell.name.prepend(icon);
  }
}

function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

function releaseCell(cell: Cell): void {
  if (cell.url !== null) {
    URL.revokeObjectURL(cell.url);
    cell.url = null;
  }
  cell.el.remove();
}

function createCell(index: number): Cell {
  const el = document.createElement("div");
  el.className = "cell";
  el.style.left = `${index * CELL_WIDTH}px`;
  const img = document.createElement("img");
  el.append(img);
  const name = document.createElement("span");
  name.className = "name";
  name.addEventListener("click", (event) => {
    if (
      cell.index === current &&
      !event.metaKey &&
      !event.ctrlKey &&
      !event.shiftKey &&
      canRename()
    ) {
      armSlowClick(cell.index);
    }
  });
  name.addEventListener("dblclick", cancelSlowClick);
  el.append(name);
  const badge = document.createElement("span");
  badge.className = "rating";
  el.append(badge);
  const flag = document.createElement("span");
  flag.className = "flag";
  el.append(flag);
  const count = document.createElement("span");
  count.className = "count";
  el.append(count);
  const marks = document.createElement("span");
  marks.className = "marks";
  const tier = document.createElement("span");
  tier.className = "tier";
  const recommended = document.createElement("span");
  recommended.className = "recommended";
  marks.append(tier, recommended);
  el.append(marks);
  const reason = document.createElement("span");
  reason.className = "reason";
  el.append(reason);
  el.addEventListener("click", (event) => {
    select(cell.index, { toggle: event.metaKey || event.ctrlKey, range: event.shiftKey });
  });
  el.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    contextMenu(cell.index, event.clientX, event.clientY);
  });
  inner.append(el);
  const cell: Cell = {
    el,
    img,
    badge,
    flag,
    count,
    tier,
    recommended,
    reason,
    name,
    url: null,
    index,
  };
  paintName(cell);
  paintRating(index, cell);
  paintBurst(index, cell);
  paintTier(index, cell);
  paintRecommended(index, cell);
  paintFailure(index, cell);
  return cell;
}

function highlight(): void {
  for (const [index, cell] of cells) {
    cell.el.classList.toggle("current", index === current);
    cell.el.classList.toggle("selected", index !== current && selected.has(index));
    cell.el.classList.toggle("failed", failed.has(index));
  }
}

// The cell nearest the middle of the viewport is wanted first, so scrolling
// fast fills what the user is looking at rather than what it flew past.
function pickNext(): number | null {
  const center = (strip.scrollLeft + strip.clientWidth / 2) / CELL_WIDTH;
  let best: number | null = null;
  let bestDistance = Infinity;
  for (const index of cells.keys()) {
    if (
      requested.has(index) ||
      inFlightIndices.has(index) ||
      missing.has(index) ||
      failed.has(index)
    ) {
      continue;
    }
    const distance = Math.abs(index - center);
    if (distance < bestDistance) {
      best = index;
      bestDistance = distance;
    }
  }
  return best;
}

function request(index: number): void {
  requested.add(index);
  inFlightIndices.add(index);
  inFlight += 1;
  const currentGeneration = generation;
  window.__TAURI__.core
    .invoke<ArrayBuffer>("thumbnail", { path: files[index] })
    .then((payload) => {
      const cell = cells.get(index);
      if (currentGeneration !== generation || cell === undefined) {
        return;
      }
      requested.add(index);
      const header = new DataView(payload, 0, THUMBNAIL_HEADER_LEN);
      const kind = header.getUint16(0, true);
      if (kind !== THUMBNAIL_KIND_JPEG_V1) {
        throw new Error(`unknown thumbnail payload kind ${kind}`);
      }
      const orientation = header.getUint16(2, true);
      const blob = new Blob([payload.slice(THUMBNAIL_HEADER_LEN)], {
        type: "image/jpeg",
      });
      if (cell.url !== null) {
        URL.revokeObjectURL(cell.url);
      }
      cell.url = URL.createObjectURL(blob);
      cell.img.src = cell.url;
      cell.img.className =
        orientation === 6 ? "cw" : orientation === 8 ? "ccw" : orientation === 3 ? "half" : "";
    })
    .catch((err: unknown) => {
      // A response for a folder that is no longer open: bookkeeping below
      // belongs to the new folder's `setFiles` reset (or to a re-request
      // that has already replaced this entry), so leave it alone.
      if (currentGeneration !== generation) {
        return;
      }
      requested.delete(index);
      // The index row exists but its `thumb` column is NULL only once the
      // file has actually been processed and failed (see
      // `Index::thumbnail` in `crates/app/src/index.rs`), so this message
      // means a permanent failure rather than "not yet scanned". Record it
      // separately so it is shown once instead of retried on every
      // `refresh`.
      if (String(err).includes("no cached thumbnail")) {
        failed.add(index);
      } else if (!ready.delete(index)) {
        missing.add(index);
      }
    })
    .finally(() => {
      inFlight -= 1;
      if (currentGeneration === generation) {
        inFlightIndices.delete(index);
        highlight();
      }
      pump();
    });
}

function pump(): void {
  while (inFlight < MAX_IN_FLIGHT) {
    const next = pickNext();
    if (next === null) {
      return;
    }
    request(next);
  }
}

// The indices the strip keeps cells for: the visible ones plus
// `RANGE_MARGIN` on each side. `last` is below `first` when there are no files.
export function visibleRange(): { first: number; last: number } {
  return {
    first: Math.max(0, Math.floor(strip.scrollLeft / CELL_WIDTH) - RANGE_MARGIN),
    last: Math.min(
      files.length - 1,
      Math.ceil((strip.scrollLeft + strip.clientWidth) / CELL_WIDTH) + RANGE_MARGIN,
    ),
  };
}

function render(): void {
  const { first, last } = visibleRange();
  for (const [index, cell] of cells) {
    if ((index < first || index > last) && index !== editing?.index) {
      releaseCell(cell);
      cells.delete(index);
      // The bytes are gone with the object URL, so a cell coming back has to
      // ask again.
      requested.delete(index);
    }
  }
  for (let index = first; index <= last; index += 1) {
    if (!cells.has(index)) {
      cells.set(index, createCell(index));
    }
  }
  highlight();
  pump();
}

// Record the judgment of one file, repainting its cell when it is on screen.
// `null` is unrated.
export function setRating(
  index: number,
  rating: number | null,
  flag: PickFlag,
  label: string | null,
): void {
  if (rating === null) {
    ratings.delete(index);
  } else {
    ratings.set(index, rating);
  }
  if (flag === "none") {
    flags.delete(index);
  } else {
    flags.set(index, flag);
  }
  if (label === null) {
    labels.delete(index);
  } else {
    labels.set(index, label);
  }
  const cell = cells.get(index);
  if (cell !== undefined) {
    paintRating(index, cell);
  }
}

// Record the burst band and badge of one file, repainting its cell when it is on
// screen. `null` is not in a burst of two or more.
export function setBurst(index: number, value: BurstMark | null): void {
  if (value === null) {
    bursts.delete(index);
  } else {
    bursts.set(index, value);
  }
  const cell = cells.get(index);
  if (cell !== undefined) {
    paintBurst(index, cell);
  }
}

// Record the focus mark state of one file, repainting its cell when it is on
// screen.
export function setTier(index: number, state: MarkState): void {
  if (state === "neutral") {
    tiers.delete(index);
  } else {
    tiers.set(index, state);
  }
  const cell = cells.get(index);
  if (cell !== undefined) {
    paintTier(index, cell);
  }
}

// Record whether one file is its burst's recommended frame, repainting its
// cell when it is on screen.
export function setRecommended(index: number, on: boolean): void {
  if (on) {
    recommendations.add(index);
  } else {
    recommendations.delete(index);
  }
  const cell = cells.get(index);
  if (cell !== undefined) {
    paintRecommended(index, cell);
  }
}

// Record why the scan failed on one file, repainting its cell when it is on
// screen. `null` is a file that did not fail.
export function setFailure(index: number, message: string | null): void {
  if (message === null) {
    failures.delete(index);
  } else {
    failures.set(index, message);
  }
  const cell = cells.get(index);
  if (cell !== undefined) {
    paintFailure(index, cell);
  }
}

// Show one cell per file, in `list_arw` order. A cell with a loaded
// thumbnail whose file is still listed moves to its new index; the rest are
// placeholders. `keepScroll` is for a rescan of the folder already shown: the
// view stays on the same files (see `carriedOffset`) instead of jumping back
// to the top.
export function setFiles(paths: string[], keepScroll = false): void {
  // The indices may no longer name the same files, but a rename still being
  // typed survives when its file is still in the new list: it is carried
  // over to its new index below instead of being silently dropped.
  const resume = editing !== null && paths.includes(editing.state.path) ? editing : null;
  const editedIndex = resume?.index;
  if (resume === null) {
    finishRename("cancel");
  } else {
    editing = null;
    resume.input.remove();
  }
  cancelSlowClick();
  const offset = strip.scrollLeft;
  const scrollLeft = carriedOffset(files, paths, offset, CELL_WIDTH, strip.clientWidth, current);
  const loaded = [...cells].filter(([index, cell]) => cell.url !== null && index !== editedIndex);
  const moves = carriedIndices(
    files,
    paths,
    loaded.map(([index]) => index),
  );
  const carried = loaded.filter(([index]) => moves.has(index));
  generation += 1;
  for (const [index, cell] of cells) {
    if (!moves.has(index)) {
      releaseCell(cell);
    }
  }
  cells.clear();
  requested.clear();
  inFlightIndices.clear();
  missing.clear();
  ready.clear();
  failed.clear();
  ratings.clear();
  flags.clear();
  labels.clear();
  bursts.clear();
  tiers.clear();
  recommendations.clear();
  failures.clear();
  selected.clear();
  files = paths;
  indexOf = new Map(paths.map((path, index) => [path, index]));
  current = 0;
  inner.style.width = `${files.length * CELL_WIDTH}px`;
  for (const [index, cell] of carried) {
    cell.index = moves.get(index) as number;
    cell.el.style.left = `${cell.index * CELL_WIDTH}px`;
    cells.set(cell.index, cell);
    requested.add(cell.index);
    paintName(cell);
    paintRating(cell.index, cell);
    paintBurst(cell.index, cell);
    paintTier(cell.index, cell);
    paintRecommended(cell.index, cell);
  }
  strip.scrollLeft = keepScroll ? scrollLeft : 0;
  if (resume !== null) {
    resume.index = indexOf.get(resume.state.path) as number;
    editing = resume;
  }
  render();
  if (resume !== null) {
    let cell = cells.get(resume.index);
    if (cell === undefined) {
      // Outside the range `render` just populated: the resumed edit still
      // needs a cell to live in.
      cell = createCell(resume.index);
      cells.set(resume.index, cell);
    }
    cell.name.replaceWith(resume.input);
    resume.input.focus();
  }
}

// Mark the selected indices; `highlight` paints them unless one is current.
export function setSelected(indices: Iterable<number>): void {
  selected.clear();
  for (const index of indices) {
    selected.add(index);
  }
  highlight();
}

// Highlight `index` and, unless `scroll` is false, scroll it into view, the
// `block: "nearest"` way.
export function setCurrent(index: number, scroll = true): void {
  current = index;
  const left = index * CELL_WIDTH;
  const right = left + CELL_WIDTH;
  if (scroll && left < strip.scrollLeft) {
    strip.scrollLeft = left;
  } else if (scroll && right > strip.scrollLeft + strip.clientWidth) {
    strip.scrollLeft = right - strip.clientWidth;
  }
  render();
}

// Ask again for the visible cells the index had nothing for, once the scan
// is done.
export function refresh(): void {
  missing.clear();
  pump();
}

// The scan has committed these paths, so the index can answer for them now.
// Paths that are not in the current list (filtered out, or belonging to
// another folder) are ignored.
export function markReady(paths: string[]): void {
  for (const path of paths) {
    const index = indexOf.get(path);
    if (index === undefined) {
      continue;
    }
    ready.add(index);
    missing.delete(index);
  }
  pump();
}

// A rename resolved: the file at `at` (still `oldPath` in `indexOf`, since
// `main.ts` patches `files` in place by reference before calling this, so
// `files[at]` is already `newPath`) keeps its place, but `indexOf` and the
// live cell's name still need to move to `newPath`.
export function renamePath(oldPath: string, newPath: string): void {
  const at = indexOf.get(oldPath);
  if (at === undefined) {
    return;
  }
  indexOf.delete(oldPath);
  indexOf.set(newPath, at);
  const cell = cells.get(at);
  if (cell !== undefined) {
    paintName(cell);
  }
}

// A rename of `path` to `name` is held until the scan ends.
export function markPending(path: string, name: string): Pending {
  const previous = pending.current?.path;
  const mark = pending.mark(path, name);
  repaintName(previous);
  repaintName(path);
  return mark;
}

// The held rename that set `mark` ran, was replaced or dropped: the cell
// shows its real name again, unless a later rename has marked it since.
export function clearPending(mark: Pending): void {
  if (pending.clear(mark)) {
    repaintName(mark.path);
  }
}

// `rename_file` was invoked on `path`; until `renameSettled(path)`, the
// strip refuses to start an edit on it.
export function renameStarted(path: string): void {
  renamesInFlight.start(path);
}

export function renameSettled(path: string): void {
  renamesInFlight.settle(path);
}

function repaintName(path: string | undefined): void {
  const at = path === undefined ? undefined : indexOf.get(path);
  const cell = at === undefined ? undefined : cells.get(at);
  if (cell !== undefined) {
    paintName(cell);
  }
}

// A click on the focused file's name arms a rename; the edit starts
// `SLOW_CLICK_DELAY` later unless something disarms it. The cell's own click
// still runs, which leaves the focused file as it is.
function armSlowClick(index: number): void {
  const path = files[index];
  slow.click(path, true, performance.now());
  clearTimeout(slowTimer);
  slowTimer = setTimeout(() => {
    if (slow.due(path, performance.now()) && files[index] === path && canRename()) {
      startRename(index);
    }
  }, SLOW_CLICK_DELAY);
}

export function cancelSlowClick(): void {
  slow.cancel();
  clearTimeout(slowTimer);
}

export function isEditing(): boolean {
  return editing !== null;
}

// Turns the file's name into a text input, its stem selected.
export function startRename(index: number): void {
  cancelSlowClick();
  if (files[index] !== undefined && renamesInFlight.blocks(files[index], false)) {
    reportError("This file is being renamed; try again in a moment.");
    return;
  }
  const cell = cells.get(index);
  if (cell === undefined) {
    return;
  }
  finishRename("confirm");
  const original = displayName(pending.current, files[index], baseName(files[index]));
  const state = inlineRename("file", files[index], original);
  const input = document.createElement("input");
  input.type = "text";
  input.className = "name";
  input.value = original;
  input.spellcheck = false;
  input.addEventListener("input", () => {
    state.value = input.value;
  });
  // Switching to another app blurs the input too; the edit waits for the
  // window to come back instead of confirming.
  input.addEventListener("blur", () => {
    if (document.hasFocus()) {
      finishRename("confirm");
    }
  });
  for (const type of ["mousedown", "click", "dblclick", "contextmenu"]) {
    input.addEventListener(type, (event) => {
      event.stopPropagation();
    });
  }
  editing = { state, index, input };
  cell.name.replaceWith(input);
  input.focus();
  input.setSelectionRange(0, stemLength(original));
}

// Ends the edit, once: the blur that follows an Enter or an Escape is a
// no-op. The input gives way to the name span in place, so a click away that
// ended the edit still reaches the cell it landed on.
export function finishRename(decision: Decision): void {
  if (editing === null || commit(editing.state, decision) === null) {
    return;
  }
  const { state, index, input } = editing;
  editing = null;
  const cell = cells.get(index);
  if (cell === undefined) {
    input.remove();
  } else {
    input.replaceWith(cell.name);
  }
  render();
  if (decision !== "confirm") {
    return;
  }
  // On a cell whose rename is held, typing the real name back cancels it.
  const outcome = editOutcome(pending.current, state.path, baseName(state.path), state.value);
  if (outcome === "cancel") {
    cancelPending(state.path);
  } else if (outcome !== "keep") {
    rename(state.path, outcome.rename);
  }
}

export function init(
  onSelect: (index: number, modifiers: Modifiers) => void,
  onContextMenu: (index: number, x: number, y: number) => void,
  onRename: (path: string, name: string) => void,
  renameAllowed: () => boolean,
  onScroll: () => void,
  onCancelPending: (path: string) => void,
  onError: (message: string) => void,
): void {
  select = onSelect;
  contextMenu = onContextMenu;
  rename = onRename;
  canRename = renameAllowed;
  cancelPending = onCancelPending;
  reportError = onError;
  // Any click anywhere disarms a pending slow click; the arming click's own
  // `mousedown` comes before its `click`, so it arms after this.
  document.addEventListener("mousedown", cancelSlowClick);
  strip.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
  // A vertical wheel scrolls the strip sideways, as Lightroom's filmstrip
  // does; a trackpad's horizontal swipe already scrolls it natively.
  strip.addEventListener(
    "wheel",
    (event) => {
      if (event.deltaX === 0 && event.deltaY !== 0) {
        event.preventDefault();
        strip.scrollLeft += event.deltaY;
      }
    },
    { passive: false },
  );
  strip.addEventListener("scroll", () => {
    render();
    onScroll();
  });
  window.addEventListener("resize", render);
}
