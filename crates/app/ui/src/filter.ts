import { type Exif, type ExifGroup, exifKey } from "./exif.js";
import type { EyeState } from "./eyes.js";
import type { MarkState } from "./focus.js";
import type { PickFlag } from "./selection.js";

// The filter menu in the strip pane, after PhotoLab's: the checked items of
// one group are OR-ed, the groups AND-ed, and a group with nothing checked
// lets everything through. `0` stars is unrated; a reject keeps its stars,
// independent of its flag. A label is keyed lowercased, or `none`
// when there is none; a label outside the menu's colors matches no item.
// The menu's `Face focus` items put `good`, `neutral` or `low` in `candidates`,
// and they partition the files like the focus mark's states (`afEyeState` in
// `focus.ts`): `good` is a good photo, `low` a judged face whose eyes are likely
// not in focus, and `neutral` everything else, a file the pass has not reached
// included.
// Its `Eyes` items put the AF face's stored eye state, `open`, `closed` or
// `unknown`, in `eyes`; a file without one counts as `unknown` the same way.
// Its `Genre` items put genre labels (`genre.rs`'s, `Unknown` included) in
// `genres`; a file passes when one of its labels is checked, and a file
// without an embedding yet counts as `Unknown` the same way.
// Its `Output` items put `yes` or `no` in `outputs`. The files `No output` lets
// through go to the trash, so unlike the sections above a file whose export
// state is not known passes neither item.
export type Flag = "picked" | "untagged" | "rejected";

// One item of the `Face focus` section: a mark state.
export type AfEye = MarkState;

// The displayed shape, decided by the EXIF Orientation tag alone: every
// sensor Handpic reads is landscape, so a quarter turn (6 or 8) is portrait.
export type Orientation = "portrait" | "landscape";

// One item of the `Output` section: whether a file has an export.
export type OutputState = "yes" | "no";

export function orientationOf(tag: number): Orientation {
  return tag === 6 || tag === 8 ? "portrait" : "landscape";
}

export interface FilterState {
  flags: Set<Flag>;
  stars: Set<number>;
  labels: Set<string>;
  orientations: Set<Orientation>;
  candidates: Set<AfEye>;
  eyes: Set<EyeState>;
  genres: Set<string>;
  outputs: Set<OutputState>;
  exif: Map<ExifGroup, Set<string>>;
}

// The genre label of a frame no genre clears, as `genre.rs` names it.
export const UNKNOWN_GENRE = "Unknown";

// The remembered filter, the wire shape of the `filter` / `set_filter`
// commands: every section but the EXIF groups, the genres and the outputs,
// which stay per session.
export interface StoredFilter {
  flags: Flag[];
  stars: number[];
  labels: string[];
  orientations: Orientation[];
  candidates: AfEye[];
  eyes: EyeState[];
}

export function toStored(state: FilterState): StoredFilter {
  return {
    flags: [...state.flags],
    stars: [...state.stars],
    labels: [...state.labels],
    orientations: [...state.orientations],
    candidates: [...state.candidates],
    eyes: [...state.eyes],
  };
}

// Replace the remembered sections' checks with `stored`, leaving `exif`.
export function applyStored(state: FilterState, stored: StoredFilter): void {
  const sections = [
    [state.flags, stored.flags],
    [state.stars, stored.stars],
    [state.labels, stored.labels],
    [state.orientations, stored.orientations],
    [state.candidates, stored.candidates],
    [state.eyes, stored.eyes],
  ] as [Set<string | number>, (string | number)[]][];
  for (const [set, values] of sections) {
    set.clear();
    for (const value of values) {
      set.add(value);
    }
  }
}

export interface Judgment {
  rating: number | null;
  flag: PickFlag;
  label: string | null;
}

export function passes(
  state: FilterState,
  { rating, flag: pickFlag, label }: Judgment,
  exif: Exif | null | undefined,
  orientation: number | undefined,
  afEye?: AfEye,
  eyes?: EyeState,
  genres?: readonly string[],
  output?: OutputState,
): boolean {
  const flag: Flag =
    pickFlag === "pick" ? "picked" : pickFlag === "reject" ? "rejected" : "untagged";
  const stars = rating ?? 0;
  const labelKey = label === null ? "none" : label.toLowerCase();
  return (
    (state.flags.size === 0 || state.flags.has(flag)) &&
    (state.stars.size === 0 || state.stars.has(stars)) &&
    (state.labels.size === 0 || state.labels.has(labelKey)) &&
    (state.orientations.size === 0 ||
      (orientation !== undefined && state.orientations.has(orientationOf(orientation)))) &&
    (state.candidates.size === 0 || state.candidates.has(afEye ?? "neutral")) &&
    (state.eyes.size === 0 || state.eyes.has(eyes ?? "unknown")) &&
    (state.genres.size === 0 || (genres ?? [UNKNOWN_GENRE]).some((g) => state.genres.has(g))) &&
    (state.outputs.size === 0 || (output !== undefined && state.outputs.has(output))) &&
    [...state.exif].every(([group, set]) => {
      if (set.size === 0) {
        return true;
      }
      const key = exifKey(exif, group);
      return key !== null && set.has(key.label);
    })
  );
}

// The export state of `path`, from the `found` subset `outputs_of` gave of
// `listing`, or null when it gave no answer. A file outside `listing`, one
// restored from the Trash since the read, is unknown rather than `no`.
export function outputOf(
  found: ReadonlySet<string> | null,
  listing: ReadonlySet<string>,
  path: string,
): OutputState | undefined {
  if (found === null || !listing.has(path)) {
    return undefined;
  }
  return found.has(path) ? "yes" : "no";
}

// The path that becomes current after a refilter: `anchor` if it still
// passes; otherwise the next passing path after it (in `allFiles` order), or
// the last one before it.
//
// `previous` is the list the user was looking at before `allFiles` was
// re-listed. When given and `anchor` is gone from `allFiles` (deleted from
// outside the app), the anchor's next surviving passing neighbour in
// `previous` wins, else its previous one, whether or not the anchor itself
// still passes.
export function anchorAfterFilter(
  allFiles: readonly string[],
  pass: (path: string) => boolean,
  anchor: string | undefined,
  previous?: readonly string[],
): string | undefined {
  if (anchor === undefined) {
    return undefined;
  }
  if (previous !== undefined) {
    const present = new Set(allFiles);
    if (!present.has(anchor)) {
      const survives = (path: string) => present.has(path) && pass(path);
      const from = previous.indexOf(anchor);
      const after = previous.slice(from + 1).find(survives);
      const before = previous.slice(0, Math.max(from, 0)).reverse().find(survives);
      return after ?? before;
    }
  }
  if (pass(anchor)) {
    return anchor;
  }
  const from = allFiles.indexOf(anchor);
  const after = allFiles.slice(from + 1).find(pass);
  const before = allFiles.slice(0, from).reverse().find(pass);
  return after ?? before;
}

// Whether `refilter` must rebuild `files`/`index`: the freshly filtered
// `next` list differs from the current `files`, or `force` says to rebuild
// regardless. `force` covers the first `folder_entries` refresh after a
// folder opens with a pending resume target: that refresh's list is often
// identical to the pre-entries one (same name order, no judgment filter
// active), yet the resume target still needs to become current.
export function filterListChanged(
  files: readonly string[],
  next: readonly string[],
  force: boolean,
): boolean {
  return force || next.length !== files.length || next.some((path, at) => path !== files[at]);
}
