// The ring of decoded neighbour previews: which files around the current one
// to hold, which to read next, and what to give up on a list change. No DOM
// access; a bitmap is only ever handed back for the caller to `close()`.
//
// A bitmap is owned by exactly one place at a time, so eviction cannot close
// one on screen: `take` removes it from the ring for the view, and the view
// hands it back through `keep` only once another bitmap has replaced it.

// The ring's total, excluding the bitmap on screen. At 4 bytes a pixel a
// 1616x1080 bitmap is 7 MB and a 4416x2944 RAF one 52 MB, so 32 MP (128 MB)
// holds both neighbours of the largest RAW previews, while the whole JPEGs of
// a view-only folder (24 MP and up) may not fit two at once; a preview too
// large to share the cap with its neighbour is not kept at all.
export const PREFETCH_PIXEL_CAP = 32_000_000;

export type Direction = -1 | 1;

interface Sized {
  width: number;
  height: number;
}

export interface Held<B extends Sized> {
  bitmap: B;
  orientation: number;
  jpegKB: number;
}

// The files to hold around `index`, the next one in the paging direction
// first.
export function neighbours(
  files: readonly string[],
  index: number,
  direction: Direction,
): string[] {
  return [index + direction, index - direction]
    .filter((at) => at >= 0 && at < files.length)
    .map((at) => files[at]);
}

export class PrefetchRing<B extends Sized> {
  private readonly held = new Map<string, Held<B>>();
  private wanted: string[] = [];
  // Files that failed to load, not asked for again until they are forgotten.
  private readonly refused = new Set<string>();
  // Files evicted for space, not asked for again while they stay wanted, so a
  // neighbour that cannot share the cap is not re-read on every page turn.
  // They are dropped by `plan` once the page has moved past them.
  private readonly crowded = new Set<string>();
  // The one prefetch in flight. It stays here until its reply settles, even
  // when it no longer matters, so stale reads never pile up.
  private pending: { path: string; stale: boolean } | null = null;

  // Points the ring at `index`, returning the bitmaps no longer wanted. The
  // caller takes the file it is about to show first, or its bitmap goes too.
  plan(files: readonly string[], index: number, direction: Direction): B[] {
    this.wanted = neighbours(files, index, direction);
    const wanted = new Set(this.wanted);
    for (const path of this.crowded) {
      if (!wanted.has(path)) this.crowded.delete(path);
    }
    return this.drop((path) => !wanted.has(path));
  }

  // The file to read next, marked in flight, or null when there is none or
  // one is already in flight.
  next(): string | null {
    if (this.pending !== null) {
      return null;
    }
    const path = this.wanted.find(
      (at) => !this.held.has(at) && !this.refused.has(at) && !this.crowded.has(at),
    );
    if (path === undefined) {
      return null;
    }
    this.pending = { path, stale: false };
    return path;
  }

  // Whether the prefetch in flight still has a place in the ring.
  stillWanted(): boolean {
    return this.pending !== null && !this.pending.stale && this.wanted.includes(this.pending.path);
  }

  // Ends the prefetch in flight with its decoded preview, or null when it
  // could not be loaded, returning the bitmaps to close.
  settle(held: Held<B> | null): B[] {
    const wanted = this.stillWanted();
    const path = this.pending?.path;
    this.pending = null;
    if (!wanted || path === undefined) {
      return held === null ? [] : [held.bitmap];
    }
    if (held === null) {
      this.refused.add(path);
      return [];
    }
    return this.insert(path, held);
  }

  // Removes `path`'s entry for the view, which now owns its bitmap.
  take(path: string): Held<B> | undefined {
    const held = this.held.get(path);
    this.held.delete(path);
    return held;
  }

  // Takes back the bitmap the view has stopped drawing, returning the bitmaps
  // to close (the given one when `path` is not wanted).
  keep(path: string, held: Held<B>): B[] {
    return this.wanted.includes(path) ? this.insert(path, held) : [held.bitmap];
  }

  // A folder switch, or a re-listing that may have changed any file's bytes.
  clear(): B[] {
    this.markStale(() => true);
    this.refused.clear();
    this.crowded.clear();
    return this.drop(() => true);
  }

  // A file list change: only the files that left the list go.
  retain(files: readonly string[]): B[] {
    const kept = new Set(files);
    const gone = (path: string) => !kept.has(path);
    this.markStale(gone);
    for (const path of this.refused) {
      if (gone(path)) this.refused.delete(path);
    }
    for (const path of this.crowded) {
      if (gone(path)) this.crowded.delete(path);
    }
    return this.drop(gone);
  }

  // A rename or a delete: `paths` no longer name what the ring read.
  forget(paths: readonly string[]): B[] {
    const set = new Set(paths);
    const gone = (path: string) => set.has(path);
    this.markStale(gone);
    for (const path of paths) {
      this.refused.delete(path);
      this.crowded.delete(path);
    }
    return this.drop(gone);
  }

  private markStale(gone: (path: string) => boolean): void {
    if (this.pending !== null && gone(this.pending.path)) {
      this.pending.stale = true;
    }
  }

  private drop(gone: (path: string) => boolean): B[] {
    const closing: B[] = [];
    for (const [path, entry] of this.held) {
      if (gone(path)) {
        closing.push(entry.bitmap);
        this.held.delete(path);
      }
    }
    return closing;
  }

  // Adds an entry, then evicts the lowest-priority entries (the last wanted
  // first) until the ring is under the cap.
  private insert(path: string, held: Held<B>): B[] {
    const replaced = this.held.get(path);
    this.held.set(path, held);
    const closing = replaced === undefined ? [] : [replaced.bitmap];
    for (let at = this.wanted.length - 1; at >= 0 && this.pixels() > PREFETCH_PIXEL_CAP; at -= 1) {
      const victim = this.wanted[at];
      const entry = this.held.get(victim);
      if (entry !== undefined) {
        closing.push(entry.bitmap);
        this.held.delete(victim);
        this.crowded.add(victim);
      }
    }
    return closing;
  }

  private pixels(): number {
    let total = 0;
    for (const { bitmap } of this.held.values()) {
      total += bitmap.width * bitmap.height;
    }
    return total;
  }
}
