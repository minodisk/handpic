import { describe, expect, test } from "vitest";
import { advancesAfter, shouldAdvance } from "./advance.js";

describe("advancesAfter", () => {
  test("advances after stars, reject, pick and unflag", () => {
    for (const action of [
      "rate1",
      "rate2",
      "rate3",
      "rate4",
      "rate5",
      "reject",
      "pick",
      "unflag",
    ]) {
      expect(advancesAfter(action)).toBe(true);
    }
  });

  test("stays after labels, corrections and navigation", () => {
    for (const action of ["red", "clearlabel", "clear", "clearall", "next", "previous", "zoom"]) {
      expect(advancesAfter(action)).toBe(false);
    }
  });
});

describe("shouldAdvance", () => {
  const base = {
    action: "rate3",
    targets: 1,
    autoAdvance: true,
    comparing: false,
    stillCurrent: true,
  };

  test("advances when the single file already holds the value", () => {
    for (const action of ["rate3", "reject", "pick", "unflag"]) {
      expect(shouldAdvance({ ...base, action })).toBe(true);
    }
  });

  test("stays when nothing was targeted or the folder is view-only", () => {
    expect(shouldAdvance({ ...base, targets: 0 })).toBe(false);
  });

  test("stays after a batch judgment", () => {
    expect(shouldAdvance({ ...base, targets: 2 })).toBe(false);
  });

  test("stays when off, comparing, filtered out or a label", () => {
    expect(shouldAdvance({ ...base, autoAdvance: false })).toBe(false);
    expect(shouldAdvance({ ...base, comparing: true })).toBe(false);
    expect(shouldAdvance({ ...base, stillCurrent: false })).toBe(false);
    expect(shouldAdvance({ ...base, action: "red" })).toBe(false);
  });
});
