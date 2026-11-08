// The performance graph's data: the CPU and disk samples (`perf-sample`) and
// the per-switch preview latencies of the last `WINDOW_MS`, all on the
// `performance.now()` clock. No DOM access, so the drawing takes `view()`'s
// output.

export const WINDOW_MS = 60_000;
// The latency axis never shrinks below this, so a quiet graph does not
// stretch 20 ms bars to full height.
export const LATENCY_FLOOR_MS = 500;
// The read axis never shrinks below this (10 MB/s), so the idle baseline of
// IPC and small reads stays near the bottom instead of filling the plot.
export const READ_FLOOR_BPS = 10_000_000;

// CPU and disk busy are 0-100; `read` is Riffle's reads in bytes per second;
// `disk` is null where the system's disk busy time cannot be read.
export interface PerfSample {
  at: number;
  process: number;
  system: number;
  read: number;
  disk: number | null;
}

// The stages of one preview switch, in path order.
export const STAGES = ["wait", "read", "ipc", "post", "worker", "decode", "draw"] as const;
export type LatencyStages = Record<(typeof STAGES)[number], number>;

export interface LatencyPoint {
  at: number;
  ms: number;
  stages?: LatencyStages;
}

// One switch's marks on the main thread's clock, and the two durations
// measured elsewhere (`readMs` by the backend, `decodeMs` by the worker).
export interface StageMarks {
  startedAt: number;
  requestedAt: number;
  invokedAt: number;
  readMs: number;
  postedAt: number;
  receivedAt: number;
  decodeMs: number;
  drawStartedAt: number;
  drawnAt: number;
}

export function stagesOf(marks: StageMarks): LatencyStages {
  return {
    wait: marks.requestedAt - marks.startedAt,
    read: marks.readMs,
    ipc: Math.max(0, marks.invokedAt - marks.requestedAt - marks.readMs),
    post: marks.postedAt - marks.invokedAt,
    worker: Math.max(0, marks.receivedAt - marks.postedAt - marks.decodeMs),
    decode: marks.decodeMs,
    draw: marks.drawnAt - marks.drawStartedAt,
  };
}

export interface PerfView {
  samples: readonly PerfSample[];
  latency: readonly LatencyPoint[];
  latencyMax: number;
  readMax: number;
}

// A switch's latency runs from the key that turned the page, else from the
// selection change, so a click or a request queued behind an in-flight one
// counts its wait too.
export function latencyStart(keypressAt: number | null, switchAt: number): number {
  return keypressAt ?? switchAt;
}

function prune<T extends { at: number }>(points: T[], now: number): void {
  const from = now - WINDOW_MS;
  let drop = 0;
  while (drop < points.length && points[drop].at < from) {
    drop += 1;
  }
  points.splice(0, drop);
}

// Records only between `start()` and `stop()`, so nothing accumulates while
// the overlay is off; `stop()` drops what was recorded.
export class PerfModel {
  private recording = false;
  private readonly samples: PerfSample[] = [];
  private readonly latency: LatencyPoint[] = [];

  get on(): boolean {
    return this.recording;
  }

  start(): void {
    this.recording = true;
  }

  stop(): void {
    this.recording = false;
    this.samples.length = 0;
    this.latency.length = 0;
  }

  addSample(sample: PerfSample): void {
    if (!this.recording) return;
    this.samples.push(sample);
    this.prune(sample.at);
  }

  addLatency(point: LatencyPoint): void {
    if (!this.recording) return;
    this.latency.push(point);
    this.prune(point.at);
  }

  view(now: number): PerfView {
    this.prune(now);
    const latencyMax = this.latency.reduce(
      (max, point) => Math.max(max, point.ms),
      LATENCY_FLOOR_MS,
    );
    const readMax = this.samples.reduce(
      (max, sample) => Math.max(max, sample.read),
      READ_FLOOR_BPS,
    );
    return { samples: this.samples, latency: this.latency, latencyMax, readMax };
  }

  private prune(now: number): void {
    prune(this.samples, now);
    prune(this.latency, now);
  }
}
