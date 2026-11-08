import { describe, expect, test } from "vitest";
import {
  LATENCY_FLOOR_MS,
  type LatencyStages,
  PerfModel,
  READ_FLOOR_BPS,
  STAGES,
  type StageMarks,
  WINDOW_MS,
  latencyStart,
  stagesOf,
} from "./perf.js";

function recording(): PerfModel {
  const model = new PerfModel();
  model.start();
  return model;
}

const EMPTY = {
  samples: [],
  latency: [],
  latencyMax: LATENCY_FLOOR_MS,
  readMax: READ_FLOOR_BPS,
};

const STAGED: LatencyStages = {
  wait: 1,
  read: 2,
  ipc: 3,
  post: 4,
  worker: 5,
  decode: 6,
  draw: 7,
};

function sample(at: number, read: number, disk: number | null = null) {
  return { at, process: 0, system: 0, read, disk };
}

describe("PerfModel", () => {
  test("records nothing until started", () => {
    const model = new PerfModel();
    model.addSample({ at: 0, process: 10, system: 20, read: 0, disk: null });
    model.addLatency({ at: 0, ms: 30 });
    expect(model.on).toBe(false);
    expect(model.view(0)).toEqual(EMPTY);
  });

  test("stop drops what was recorded and stops recording", () => {
    const model = recording();
    model.addSample({ at: 0, process: 10, system: 20, read: 0, disk: null });
    model.addLatency({ at: 0, ms: 30 });
    model.stop();
    model.addLatency({ at: 1, ms: 40 });
    expect(model.on).toBe(false);
    expect(model.view(1)).toEqual(EMPTY);
  });

  test("keeps points in insertion order", () => {
    const model = recording();
    model.addLatency({ at: 100, ms: 30 });
    model.addLatency({ at: 200, ms: 10 });
    model.addSample({ at: 150, process: 1, system: 2, read: 0, disk: null });
    model.addSample({ at: 650, process: 3, system: 4, read: 0, disk: null });
    const view = model.view(700);
    expect(view.latency.map((point) => point.at)).toEqual([100, 200]);
    expect(view.samples.map((sample) => sample.at)).toEqual([150, 650]);
  });

  test("prunes points older than the window on insertion", () => {
    const model = recording();
    model.addSample({ at: 0, process: 1, system: 1, read: 0, disk: null });
    model.addLatency({ at: 10, ms: 900 });
    model.addSample({ at: WINDOW_MS + 5, process: 2, system: 2, read: 0, disk: null });
    const view = model.view(WINDOW_MS + 5);
    expect(view.samples.map((sample) => sample.at)).toEqual([WINDOW_MS + 5]);
    expect(view.latency.map((point) => point.at)).toEqual([10]);
  });

  test("prunes on a tick with no insertion and keeps the window's edge", () => {
    const model = recording();
    model.addLatency({ at: 1_000, ms: 30 });
    model.addLatency({ at: 2_000, ms: 40 });
    expect(model.view(WINDOW_MS + 1_000).latency.map((point) => point.at)).toEqual([1_000, 2_000]);
    expect(model.view(WINDOW_MS + 1_001).latency.map((point) => point.at)).toEqual([2_000]);
    expect(model.view(WINDOW_MS + 2_001).latency).toEqual([]);
  });

  test("the latency axis maximum has a floor", () => {
    const model = recording();
    model.addLatency({ at: 0, ms: 20 });
    expect(model.view(0).latencyMax).toBe(LATENCY_FLOOR_MS);
  });

  test("the latency axis maximum follows a point above the floor until it leaves the window", () => {
    const model = recording();
    model.addLatency({ at: 0, ms: 1_200 });
    model.addLatency({ at: 10, ms: 30 });
    expect(model.view(10).latencyMax).toBe(1_200);
    expect(model.view(WINDOW_MS + 1).latencyMax).toBe(LATENCY_FLOOR_MS);
  });

  test("a point with stages keeps them", () => {
    const model = recording();
    model.addLatency({ at: 0, ms: 28, stages: STAGED });
    expect(model.view(0).latency).toEqual([{ at: 0, ms: 28, stages: STAGED }]);
  });

  test("the latency axis maximum uses the total, not a stage", () => {
    const model = recording();
    const stages = { ...STAGED, read: 2_000 };
    model.addLatency({ at: 0, ms: 900, stages });
    expect(model.view(0).latencyMax).toBe(900);
  });

  test("stop drops staged points and a staged point is not recorded while off", () => {
    const model = recording();
    model.addLatency({ at: 0, ms: 28, stages: STAGED });
    model.stop();
    model.addLatency({ at: 1, ms: 28, stages: STAGED });
    expect(model.view(1)).toEqual(EMPTY);
  });

  test("the read axis maximum has a floor", () => {
    const model = recording();
    model.addSample(sample(0, 200_000));
    expect(model.view(0).readMax).toBe(READ_FLOOR_BPS);
  });

  test("the read axis maximum follows a sample above the floor until it leaves the window", () => {
    const model = recording();
    model.addSample(sample(0, 80_000_000));
    model.addSample(sample(10, 1_000));
    expect(model.view(10).readMax).toBe(80_000_000);
    expect(model.view(WINDOW_MS + 1).readMax).toBe(READ_FLOOR_BPS);
  });

  test("keeps a sample without a disk busy reading", () => {
    const model = recording();
    model.addSample(sample(0, 0, null));
    model.addSample(sample(500, 0, 42));
    expect(model.view(500).samples.map((point) => point.disk)).toEqual([null, 42]);
  });
});

describe("stagesOf", () => {
  const marks: StageMarks = {
    startedAt: 100,
    requestedAt: 104,
    invokedAt: 124,
    readMs: 12,
    postedAt: 127,
    receivedAt: 157,
    decodeMs: 20,
    drawStartedAt: 157,
    drawnAt: 163,
  };

  test("turns the marks into the seven stages", () => {
    expect(stagesOf(marks)).toEqual({
      wait: 4,
      read: 12,
      ipc: 8,
      post: 3,
      worker: 10,
      decode: 20,
      draw: 6,
    });
  });

  test("sums to the span from the start to the drawn canvas", () => {
    const sum = Object.values(stagesOf(marks)).reduce((a, b) => a + b, 0);
    expect(sum).toBe(marks.drawnAt - marks.startedAt);
  });

  test("clamps ipc at zero when the read exceeds the invoke", () => {
    expect(stagesOf({ ...marks, readMs: 25 }).ipc).toBe(0);
  });

  test("clamps worker at zero when the decode exceeds the span", () => {
    expect(stagesOf({ ...marks, decodeMs: 40 }).worker).toBe(0);
  });

  test("names the stages in path order", () => {
    expect(STAGES).toEqual(["wait", "read", "ipc", "post", "worker", "decode", "draw"]);
  });
});

describe("latencyStart", () => {
  test("starts at the keypress when the switch came from a key", () => {
    expect(latencyStart(100, 105)).toBe(100);
  });

  test("starts at the selection change otherwise", () => {
    expect(latencyStart(null, 105)).toBe(105);
  });
});
