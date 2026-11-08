// Whether the eyes of the shown file's AF face are closed, judged on demand
// per file and kept for the session. Free of DOM and Tauri so it is tested
// without mocks; `main.ts` only makes the `eyes_of` call this decides on.

// Mirrors `EyesJudgment` in `crates/app/src/commands.rs`. `probability` is
// the probability that the eyes are closed, 0..1, and `ear` the EAR of the
// more closed eye it was read from. `mesh` is the face mesh the
// judgment was taken on, in the stored preview's pixel coordinates like
// `faces_of`'s faces. `pose` is the head pose of the same face, `null` when
// it could not be solved.
export interface Eyes {
  state: "open" | "closed";
  probability: number;
  ear: number;
  pose: Pose | null;
  mesh: { width: number; height: number; points: [number, number][] };
}

// Mirrors `EyesPose`: degrees, yaw positive toward the image's right, pitch
// positive up, roll positive clockwise on screen.
export interface Pose {
  yaw: number;
  pitch: number;
  roll: number;
}

// The eye state the scan's second pass stored for the judged face, as `Focus`
// and `faces-progress` carry it (`StoredEyes` in `crates/app/src/index.rs`):
// the state and the closed probability derived in Rust from the stored EAR,
// `unknown` / `null` when nothing was stored.
export type EyeState = "open" | "closed" | "unknown";

// `eye_offset` (how far the mesh's eyes sit from YuNet's eye landmarks) and
// `edge_gap` (how close the face and its mesh eye regions come to the
// preview's edge, negative outside) are in face box sides. `full_eye_sharp`
// is the sharpness of the eyes on the full-size embedded JPEG, `null` when it
// was not measured or the file has no such JPEG.
export interface StoredEyes {
  eyes_ear: number | null;
  eyes: EyeState;
  eyes_closed: number | null;
  pose: Pose | null;
  eye_offset: number | null;
  edge_gap: number | null;
  full_eye_sharp: number | null;
}

// What `request` hands out: the id the backend supersedes older requests by,
// and the generation the response is settled against.
export interface EyesTicket {
  id: number;
  generation: number;
}

export class EyesCache {
  // `null` is a file whose eyes are unknown (no face, a face too small, a
  // failure), cached so it is not judged again.
  private readonly judged = new Map<string, Eyes | null>();
  private inFlight: string | null = null;
  private generation = 0;
  private lastId = 0;

  get(path: string): Eyes | null | undefined {
    return this.judged.get(path);
  }

  // The ticket to judge `path` with, or `null` when it is cached or any
  // judgment is in flight: at most one runs at a time, so paging with a key
  // held never queues one per file passed. The caller asks again for the
  // file that is current once the running one settles.
  request(path: string): EyesTicket | null {
    if (this.judged.has(path) || this.inFlight !== null) {
      return null;
    }
    this.inFlight = path;
    this.lastId += 1;
    return { id: this.lastId, generation: this.generation };
  }

  // Whether the response was kept: one issued before the last `clear` is
  // dropped, since the file may have changed since, and leaves a judgment
  // started after the clear in flight. It is kept by path, so a response for
  // a file the user paged away from still saves a judgment when they come
  // back. `undefined` (a judgment the backend stopped as superseded) frees
  // the slot without caching anything.
  settle(path: string, ticket: EyesTicket, eyes: Eyes | null | undefined): boolean {
    if (ticket.generation !== this.generation) {
      return false;
    }
    this.inFlight = null;
    if (eyes === undefined) {
      return false;
    }
    this.judged.set(path, eyes);
    return true;
  }

  clear(): void {
    this.judged.clear();
    this.inFlight = null;
    this.generation += 1;
  }
}
