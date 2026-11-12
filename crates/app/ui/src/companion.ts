// The frontend half of the MCP companion: Rust relays a tool call as an
// `mcp-request` event, and the answer is computed here over the main
// window's view state. Free of DOM and Tauri so it is tested without mocks.

import type { BurstMember } from "./burst.js";
import { COMPARE_NEEDS_FRAMES } from "./compare.js";
import type { Eyes } from "./eyes.js";
import { type Command, type Judged, type PickFlag, targets } from "./selection.js";
import type { SortKey } from "./sort.js";
import { VIEW_ONLY_REFUSAL } from "./viewonly.js";

// The main window's view state, as `main.ts` exposes it.
export interface ViewApi {
  readonly folder: string | null;
  // The files that pass the filter, in strip order.
  readonly files: readonly string[];
  readonly index: number;
  readonly selection: ReadonlySet<string>;
  readonly bursts: ReadonlyMap<string, BurstMember>;
  readonly sharpness: ReadonlyMap<string, number>;
  readonly ratings: ReadonlyMap<string, number>;
  readonly flags: ReadonlyMap<string, "pick" | "reject">;
  readonly labels: ReadonlyMap<string, string>;
  readonly zoomed: boolean;
  readonly comparing: boolean;
  readonly compareActive: string | null;
  readonly sort: SortKey;
  readonly filtered: boolean;
  // A JPEG folder, where culling does not apply.
  readonly viewOnly: boolean;
  // The eyes judgment of `path` as `EyesCache.get` answers it: `undefined`
  // not judged yet, `null` judged with the eyes unknown.
  eyes(path: string): Eyes | null | undefined;
  // Make a visible `path` current and the only selected file.
  showPhoto(path: string): void;
  // Select the visible `paths` and make the first one current.
  selectPhotos(paths: readonly [string, ...string[]]): void;
  // Toggle compare and the 1:1 view until `mode` is reached; compare stays
  // off when there are fewer than two frames to compare.
  setMode(mode: ViewMode): void;
  // Judge the visible `paths` with `command` as a judgment key does: one
  // undo entry, the same sidecar writes, no auto-advance.
  judge(paths: readonly [string, ...string[]], command: Command): void;
}

export type ViewMode = "normal" | "zoom" | "compare";

export interface McpRequest {
  id: number;
  kind: string;
  args: unknown;
}

export interface McpReply {
  id: number;
  ok: boolean;
  value: unknown;
}

export interface BurstFrame {
  path: string;
  position: number;
  sharpness: number | null;
  rating: number | null;
  flag: PickFlag;
  label: string | null;
  // Whether the filter shows this frame; `false` means it is not among
  // `files`, so it cannot be passed to a tool that shows or selects photos.
  visible: boolean;
}

// `Eyes` without the face mesh, which is of no use to a client.
export type EyesSummary = Pick<Eyes, "state" | "probability" | "pose">;

export interface CurrentPhoto {
  path: string;
  position: number;
  // Absent until the file has been judged.
  eyes?: EyesSummary | null;
}

export interface ViewState {
  folder: string | null;
  count: number;
  current: CurrentPhoto | null;
  selected: string[];
  mode: ViewMode;
  compare_active: string | null;
  burst: BurstFrame[];
  sort: SortKey;
  filtered: boolean;
}

// The current file's burst in capture order, positions 1-based; empty when
// the file is a burst of its own. `bursts` groups over every file the scan
// found, not just the ones the filter shows, so a frame the filter hides can
// appear here with `visible: false`.
function burstOf(view: ViewApi, path: string): BurstFrame[] {
  const member = view.bursts.get(path);
  if (member === undefined || member.size < 2) return [];
  return [...view.bursts]
    .filter(([, other]) => other.burst === member.burst)
    .sort(([, a], [, b]) => a.position - b.position)
    .map(([other, { position }]) => ({
      path: other,
      position: position + 1,
      sharpness: view.sharpness.get(other) ?? null,
      rating: view.ratings.get(other) ?? null,
      flag: view.flags.get(other) ?? "none",
      label: view.labels.get(other) ?? null,
      visible: view.files.includes(other),
    }));
}

function currentOf(view: ViewApi, path: string): CurrentPhoto {
  const current: CurrentPhoto = { path, position: view.index + 1 };
  const eyes = view.eyes(path);
  if (eyes !== undefined) {
    current.eyes =
      eyes === null ? null : { state: eyes.state, probability: eyes.probability, pose: eyes.pose };
  }
  return current;
}

export function getView(view: ViewApi): ViewState {
  const path = view.files[view.index] as string | undefined;
  return {
    folder: view.folder,
    count: view.files.length,
    current: path === undefined ? null : currentOf(view, path),
    selected: view.files.filter((file) => view.selection.has(file)),
    mode: view.comparing ? "compare" : view.zoomed ? "zoom" : "normal",
    compare_active: view.comparing ? view.compareActive : null,
    burst: path === undefined ? [] : burstOf(view, path),
    sort: view.sort,
    filtered: view.filtered,
  };
}

function field(args: unknown, name: string): unknown {
  return typeof args === "object" && args !== null
    ? (args as Record<string, unknown>)[name]
    : undefined;
}

// `path` checked to be one of the files the strip shows.
function visiblePath(view: ViewApi, path: unknown): string {
  if (typeof path !== "string") throw new Error("path must be a string");
  if (!view.files.includes(path)) {
    throw new Error(
      `${path} is not shown in Handpic: not in the open folder, or hidden by the filter`,
    );
  }
  return path;
}

function showPhoto(view: ViewApi, args: unknown): ViewState {
  view.showPhoto(visiblePath(view, field(args, "path")));
  return getView(view);
}

function selectPhotos(view: ViewApi, args: unknown): ViewState {
  const paths = field(args, "paths");
  if (!Array.isArray(paths)) throw new Error("paths must be an array of strings");
  const [first, ...rest] = [...new Set(paths.map((path: unknown) => visiblePath(view, path)))];
  if (first === undefined) throw new Error("paths must name at least one photo");
  view.selectPhotos([first, ...rest]);
  return getView(view);
}

const MODES: readonly string[] = ["normal", "zoom", "compare"] satisfies ViewMode[];

function setView(view: ViewApi, args: unknown): ViewState {
  const mode = field(args, "mode");
  if (typeof mode !== "string" || !MODES.includes(mode)) {
    throw new Error(`mode must be one of ${MODES.join(", ")}`);
  }
  if (mode !== "normal" && view.files.length === 0) throw new Error("no photo is shown in Handpic");
  view.setMode(mode as ViewMode);
  if (mode === "compare" && !view.comparing) throw new Error(COMPARE_NEEDS_FRAMES);
  return getView(view);
}

const FLAGS: readonly string[] = ["none", "pick", "reject"] satisfies PickFlag[];

// The color labels the judgment keys set.
const LABELS: readonly string[] = ["Red", "Orange", "Yellow", "Green", "Blue", "Pink", "Purple"];

// The files a judgment applies to when no paths are given, as a key press
// picks them: the active pane in compare, else the selection.
function defaultTargets(view: ViewApi): string[] {
  const current = view.files[view.index] as string | undefined;
  if (current === undefined) throw new Error("no photo is shown in Handpic");
  if (view.comparing) return [view.compareActive ?? current];
  return targets({ selected: new Set(view.selection), anchor: undefined }, view.files, view.index);
}

function setJudgment(view: ViewApi, args: unknown): Judged[] {
  if (view.viewOnly) throw new Error(VIEW_ONLY_REFUSAL);
  const rating = field(args, "rating");
  const flag = field(args, "flag");
  const label = field(args, "label");
  if (
    rating !== undefined &&
    !(typeof rating === "number" && Number.isInteger(rating) && rating >= 0 && rating <= 5)
  ) {
    throw new Error("rating must be an integer from 0 to 5");
  }
  if (flag !== undefined && !(typeof flag === "string" && FLAGS.includes(flag))) {
    throw new Error(`flag must be one of ${FLAGS.join(", ")}`);
  }
  if (
    label !== undefined &&
    label !== null &&
    !(typeof label === "string" && LABELS.includes(label))
  ) {
    throw new Error(`label must be one of ${LABELS.join(", ")}, or null`);
  }
  if (rating === undefined && flag === undefined && label === undefined) {
    throw new Error("set at least one of rating, flag, label");
  }
  const given = field(args, "paths");
  let paths: string[];
  if (given === undefined || given === null) {
    paths = defaultTargets(view);
  } else {
    if (!Array.isArray(given)) throw new Error("paths must be an array of strings");
    paths = [...new Set(given.map((path: unknown) => visiblePath(view, path)))];
  }
  const [first, ...rest] = paths;
  if (first === undefined) throw new Error("paths must name at least one photo");
  view.judge([first, ...rest], () => (own) => ({
    rating: rating === undefined ? own.rating : rating === 0 ? null : (rating as number),
    flag: flag === undefined ? own.flag : (flag as PickFlag),
    label: label === undefined ? own.label : (label as string | null),
  }));
  return paths.map((path) => ({
    path,
    rating: view.ratings.get(path) ?? null,
    flag: view.flags.get(path) ?? "none",
    label: view.labels.get(path) ?? null,
  }));
}

export async function handleRequest(kind: string, args: unknown, view: ViewApi): Promise<unknown> {
  switch (kind) {
    case "get_view":
      return getView(view);
    case "show_photo":
      return showPhoto(view, args);
    case "select_photos":
      return selectPhotos(view, args);
    case "set_view":
      return setView(view, args);
    case "set_judgment":
      return setJudgment(view, args);
    default:
      throw new Error(`unknown request: ${kind}`);
  }
}

// The reply to `request`, never a rejection: a failure becomes `ok: false`
// with its message, so Rust does not wait for its timeout.
export async function respond(request: McpRequest, view: ViewApi): Promise<McpReply> {
  try {
    const value = await handleRequest(request.kind, request.args, view);
    return { id: request.id, ok: true, value: value ?? null };
  } catch (error) {
    return {
      id: request.id,
      ok: false,
      value: error instanceof Error ? error.message : String(error),
    };
  }
}
