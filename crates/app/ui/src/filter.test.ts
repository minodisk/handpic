import { describe, expect, test } from "vitest";
import type { Exif, ExifGroup } from "./exif.js";
import type { EyeState } from "./eyes.js";
import {
  type AfEye,
  type FilterState,
  type Flag,
  type Judgment,
  type Orientation,
  type OutputState,
  anchorAfterFilter,
  applyStored,
  filterListChanged,
  orientationOf,
  outputOf,
  passes,
  toStored,
  UNKNOWN_GENRE,
} from "./filter.js";
import { type MarkFocus, afEyeState } from "./focus.js";

function state(
  flags: Flag[] = [],
  stars: number[] = [],
  exif: [ExifGroup, string[]][] = [],
  labels: string[] = [],
  orientations: Orientation[] = [],
  candidates: AfEye[] = [],
  eyes: EyeState[] = [],
  genres: string[] = [],
  outputs: OutputState[] = [],
): FilterState {
  return {
    flags: new Set(flags),
    stars: new Set(stars),
    labels: new Set(labels),
    orientations: new Set(orientations),
    candidates: new Set(candidates),
    eyes: new Set(eyes),
    genres: new Set(genres),
    outputs: new Set(outputs),
    exif: new Map(exif.map(([group, values]) => [group, new Set(values)])),
  };
}

const exif: Exif = {
  camera: "ILCE-7RM5",
  lens: null,
  aperture: null,
  shutter: null,
  iso: null,
  focal_length: null,
};

const unjudged: Judgment = { rating: null, flag: "none", label: null };
const rejected: Judgment = { rating: null, flag: "reject", label: null };
const threeStars: Judgment = { rating: 3, flag: "none", label: null };
const pickedTwo: Judgment = { rating: 2, flag: "pick", label: null };

describe("passes", () => {
  test("empty groups pass everything", () => {
    for (const judgment of [unjudged, rejected, threeStars, pickedTwo]) {
      expect(passes(state(), judgment, undefined, undefined)).toBe(true);
    }
  });

  test("ORs the checked flags", () => {
    const s = state(["picked", "rejected"]);
    expect(passes(s, pickedTwo, undefined, undefined)).toBe(true);
    expect(passes(s, rejected, undefined, undefined)).toBe(true);
    expect(passes(s, unjudged, undefined, undefined)).toBe(false);
  });

  test("ORs the checked stars", () => {
    const s = state([], [0, 3]);
    expect(passes(s, unjudged, undefined, undefined)).toBe(true);
    expect(passes(s, threeStars, undefined, undefined)).toBe(true);
    expect(passes(s, pickedTwo, undefined, undefined)).toBe(false);
  });

  test("ANDs the groups", () => {
    const s = state(["untagged"], [3]);
    expect(passes(s, threeStars, undefined, undefined)).toBe(true);
    expect(passes(s, unjudged, undefined, undefined)).toBe(false);
    expect(passes(s, pickedTwo, undefined, undefined)).toBe(false);
  });

  test("a reject counts as rejected and 0 stars", () => {
    expect(passes(state(["rejected"], [0]), rejected, undefined, undefined)).toBe(true);
    expect(passes(state(["untagged"]), rejected, undefined, undefined)).toBe(false);
  });

  test("a reject with stars counts as rejected and its stars", () => {
    const rejectedFour = { rating: 4, flag: "reject" as const, label: null };
    expect(passes(state(["rejected"], [4]), rejectedFour, undefined, undefined)).toBe(true);
    expect(passes(state([], [0]), rejectedFour, undefined, undefined)).toBe(false);
  });

  test("a pick with stars counts as picked and its stars", () => {
    expect(passes(state(["picked"], [2]), pickedTwo, undefined, undefined)).toBe(true);
    expect(passes(state(["untagged"]), pickedTwo, undefined, undefined)).toBe(false);
    expect(passes(state([], [0]), pickedTwo, undefined, undefined)).toBe(false);
  });

  test("an EXIF selection matches by label and fails a file without EXIF", () => {
    const s = state([], [], [["camera", ["ILCE-7RM5"]]]);
    expect(passes(s, unjudged, exif, undefined)).toBe(true);
    expect(passes(state([], [], [["camera", ["Other"]]]), unjudged, exif, undefined)).toBe(false);
    expect(passes(s, unjudged, undefined, undefined)).toBe(false);
    expect(passes(s, unjudged, null, undefined)).toBe(false);
  });
});

describe("passes: color label", () => {
  const red: Judgment = { rating: null, flag: "none", label: "Red" };
  const blue: Judgment = { rating: null, flag: "none", label: "Blue" };
  const foreign: Judgment = { rating: null, flag: "none", label: "Approved" };
  const colors = ["red", "orange", "yellow", "green", "blue", "pink", "purple"];

  test("a labeled file fails none", () => {
    expect(passes(state([], [], [], ["none"]), red, undefined, undefined)).toBe(false);
    expect(passes(state([], [], [], ["none"]), unjudged, undefined, undefined)).toBe(true);
  });

  test("a Red file passes red and fails blue", () => {
    expect(passes(state([], [], [], ["red"]), red, undefined, undefined)).toBe(true);
    expect(passes(state([], [], [], ["blue"]), red, undefined, undefined)).toBe(false);
  });

  test("ORs the checked colors", () => {
    const s = state([], [], [], ["red", "blue"]);
    expect(passes(s, red, undefined, undefined)).toBe(true);
    expect(passes(s, blue, undefined, undefined)).toBe(true);
    expect(passes(s, unjudged, undefined, undefined)).toBe(false);
  });

  test("matches case-insensitively", () => {
    expect(passes(state([], [], [], ["red"]), { ...red, label: "RED" }, undefined, undefined)).toBe(
      true,
    );
  });

  test("a foreign label fails every color and none", () => {
    for (const key of [...colors, "none"]) {
      expect(passes(state([], [], [], [key]), foreign, undefined, undefined)).toBe(false);
    }
    expect(passes(state(), foreign, undefined, undefined)).toBe(true);
  });

  test("untagged + 0 + none selects exactly the unjudged files", () => {
    const s = state(["untagged"], [0], [], ["none"]);
    expect(passes(s, unjudged, undefined, undefined)).toBe(true);
    expect(passes(s, red, undefined, undefined)).toBe(false);
    expect(passes(s, threeStars, undefined, undefined)).toBe(false);
    expect(passes(s, { rating: null, flag: "pick", label: null }, undefined, undefined)).toBe(
      false,
    );
    expect(passes(s, rejected, undefined, undefined)).toBe(false);
  });
});

describe("anchorAfterFilter", () => {
  const all = ["a", "b", "c", "d"];

  test("stays on the anchor if it passes", () => {
    expect(anchorAfterFilter(all, () => true, "b")).toBe("b");
  });

  test("moves to the next passing file after it", () => {
    expect(anchorAfterFilter(all, (path) => path !== "b", "b")).toBe("c");
    expect(anchorAfterFilter(all, (path) => path === "a" || path === "d", "b")).toBe("d");
  });

  test("falls back to the last passing file before it", () => {
    expect(anchorAfterFilter(all, (path) => path === "a" || path === "b", "c")).toBe("b");
  });

  test("returns undefined when nothing passes", () => {
    expect(anchorAfterFilter(all, () => false, "b")).toBeUndefined();
    expect(anchorAfterFilter(all, () => true, undefined)).toBeUndefined();
  });
});

describe("anchorAfterFilter with the anchor deleted from the listing", () => {
  const previous = ["a", "b", "c", "d"];

  test("moves to the next file in the previous list", () => {
    expect(anchorAfterFilter(["a", "c", "d"], () => true, "b", previous)).toBe("c");
  });

  test("moves to the previous file when the last one was deleted", () => {
    expect(anchorAfterFilter(["a", "b", "c"], () => true, "d", previous)).toBe("c");
  });

  test("skips a neighbour that does not pass the filter", () => {
    expect(anchorAfterFilter(["a", "c", "d"], (path) => path !== "c", "b", previous)).toBe("d");
  });

  test("skips a run of consecutive deletions", () => {
    expect(anchorAfterFilter(["a", "d"], () => true, "b", previous)).toBe("d");
    expect(anchorAfterFilter(["a"], () => true, "b", previous)).toBe("a");
  });

  test("returns undefined when nothing survives", () => {
    expect(anchorAfterFilter([], () => true, "b", previous)).toBeUndefined();
  });

  test("keeps the anchor when it is still listed", () => {
    expect(anchorAfterFilter(previous, () => true, "b", previous)).toBe("b");
  });
});

describe("filterListChanged", () => {
  const all = ["a", "b", "c"];

  test("an unchanged list without force does not rebuild", () => {
    expect(filterListChanged(all, ["a", "b", "c"], false)).toBe(false);
  });

  test("a changed list rebuilds even without force", () => {
    expect(filterListChanged(all, ["a", "c"], false)).toBe(true);
  });

  // The first `folder_entries` refresh after an open with a pending resume
  // target: the name-order list built at open is often identical to the
  // one after entries load, yet the resume target still must become
  // current, so the caller passes `force` to rebuild anyway.
  test("an unchanged list with force rebuilds anyway", () => {
    expect(filterListChanged(all, ["a", "b", "c"], true)).toBe(true);
  });
});

describe("a judgment under untagged + 0 + none", () => {
  const s = state(["untagged"], [0], [], ["none"]);
  const judgments: Record<string, Judgment> = {
    rating: { rating: 3, flag: "none", label: null },
    reject: { rating: null, flag: "reject", label: null },
    pick: { rating: null, flag: "pick", label: null },
    label: { rating: null, flag: "none", label: "Red" },
  };

  function after(all: string[], judged: string, judgment: Judgment) {
    const pass = (path: string) =>
      passes(s, path === judged ? judgment : unjudged, undefined, undefined);
    return anchorAfterFilter(all, pass, judged);
  }

  for (const [name, judgment] of Object.entries(judgments)) {
    test(`${name} moves to the next unjudged file after it`, () => {
      expect(after(["a", "b", "c"], "b", judgment)).toBe("c");
    });

    test(`${name} falls back to the last unjudged file before it`, () => {
      expect(after(["a", "b", "c"], "c", judgment)).toBe("b");
    });

    test(`${name} on the only file leaves the empty view`, () => {
      expect(after(["a"], "a", judgment)).toBeUndefined();
    });
  }
});

describe("orientationOf", () => {
  test("a quarter turn is portrait", () => {
    expect(orientationOf(6)).toBe("portrait");
    expect(orientationOf(8)).toBe("portrait");
  });

  test("everything else is landscape", () => {
    expect(orientationOf(1)).toBe("landscape");
    expect(orientationOf(3)).toBe("landscape");
    expect(orientationOf(99)).toBe("landscape");
  });
});

describe("passes: orientation", () => {
  test("an empty group passes both shapes", () => {
    expect(passes(state(), unjudged, undefined, 1)).toBe(true);
    expect(passes(state(), unjudged, undefined, 6)).toBe(true);
  });

  test("portrait passes a quarter turn only", () => {
    const s = state([], [], [], [], ["portrait"]);
    expect(passes(s, unjudged, undefined, 6)).toBe(true);
    expect(passes(s, unjudged, undefined, 8)).toBe(true);
    expect(passes(s, unjudged, undefined, 1)).toBe(false);
    expect(passes(s, unjudged, undefined, 3)).toBe(false);
  });

  test("landscape passes the unrotated tags only", () => {
    const s = state([], [], [], [], ["landscape"]);
    expect(passes(s, unjudged, undefined, 1)).toBe(true);
    expect(passes(s, unjudged, undefined, 3)).toBe(true);
    expect(passes(s, unjudged, undefined, 6)).toBe(false);
    expect(passes(s, unjudged, undefined, 8)).toBe(false);
  });

  test("both checked passes both shapes", () => {
    const s = state([], [], [], [], ["portrait", "landscape"]);
    expect(passes(s, unjudged, undefined, 1)).toBe(true);
    expect(passes(s, unjudged, undefined, 6)).toBe(true);
  });

  test("an unknown orientation fails a non-empty selection", () => {
    expect(passes(state([], [], [], [], ["landscape"]), unjudged, undefined, undefined)).toBe(
      false,
    );
  });

  test("ANDs with the other groups", () => {
    const s = state(["untagged"], [], [], [], ["portrait"]);
    expect(passes(s, unjudged, undefined, 6)).toBe(true);
    expect(passes(s, unjudged, undefined, 1)).toBe(false);
    expect(passes(s, pickedTwo, undefined, 6)).toBe(false);
  });
});

describe("passes: Face focus states", () => {
  const items = (...afEyes: AfEye[]) => state([], [], [], [], [], afEyes);
  const candidate: MarkFocus = {
    point: { sensor_w: 7008, sensor_h: 4672, x: 1752, y: 1168, frame: null },
    manual_focus: false,
    candidate: "candidate",
    eye_focus: 1,
    eyes_ear: 0.35,
    eyes: "open",
    eyes_closed: 0.001,
    pose: { yaw: 10, pitch: -5, roll: 3 },
    eye_offset: 0.03,
    edge_gap: 2,
    full_eye_sharp: null,
  };

  test("off passes every state", () => {
    for (const afEye of ["good", "neutral", "low", undefined] as const) {
      expect(passes(state(), unjudged, undefined, 1, afEye)).toBe(true);
    }
  });

  test("good passes a good frame only", () => {
    expect(passes(items("good"), unjudged, undefined, 1, "good")).toBe(true);
    expect(passes(items("good"), unjudged, undefined, 1, "neutral")).toBe(false);
    expect(passes(items("good"), unjudged, undefined, 1, "low")).toBe(false);
    expect(passes(items("good"), unjudged, undefined, 1, undefined)).toBe(false);
  });

  test("low passes a low frame only", () => {
    const low = items("low");
    expect(passes(low, unjudged, undefined, 1, "low")).toBe(true);
    expect(passes(low, unjudged, undefined, 1, "good")).toBe(false);
    expect(passes(low, unjudged, undefined, 1, "neutral")).toBe(false);
    expect(passes(low, unjudged, undefined, 1, undefined)).toBe(false);
  });

  test("a focus candidate that is not good passes Neutral, not Good or Low", () => {
    const notGood = afEyeState({ ...candidate, eyes_ear: 0.1 });
    expect(passes(items("neutral"), unjudged, undefined, 1, notGood)).toBe(true);
    expect(passes(items("good"), unjudged, undefined, 1, notGood)).toBe(false);
    expect(passes(items("low"), unjudged, undefined, 1, notGood)).toBe(false);
    const good = afEyeState(candidate);
    expect(passes(items("good"), unjudged, undefined, 1, good)).toBe(true);
    expect(passes(items("neutral"), unjudged, undefined, 1, good)).toBe(false);
  });

  test("neutral passes a neutral or not yet computed state", () => {
    const neutral = items("neutral");
    expect(passes(neutral, unjudged, undefined, 1, "neutral")).toBe(true);
    expect(passes(neutral, unjudged, undefined, 1, undefined)).toBe(true);
    expect(passes(neutral, unjudged, undefined, 1, "good")).toBe(false);
    expect(passes(neutral, unjudged, undefined, 1, "low")).toBe(false);
  });

  test("ORs the checked states", () => {
    const either = items("good", "neutral");
    expect(passes(either, unjudged, undefined, 1, "good")).toBe(true);
    expect(passes(either, unjudged, undefined, 1, "neutral")).toBe(true);
    expect(passes(either, unjudged, undefined, 1, undefined)).toBe(true);
    expect(passes(either, unjudged, undefined, 1, "low")).toBe(false);
  });

  test("Good and Low checked exclude neutral", () => {
    const ends = items("good", "low");
    expect(passes(ends, unjudged, undefined, 1, "good")).toBe(true);
    expect(passes(ends, unjudged, undefined, 1, "low")).toBe(true);
    expect(passes(ends, unjudged, undefined, 1, "neutral")).toBe(false);
    expect(passes(ends, unjudged, undefined, 1, undefined)).toBe(false);
  });

  test("the three items partition the frames", () => {
    const frames: [MarkFocus | undefined, AfEye][] = [
      [candidate, "good"],
      [{ ...candidate, eyes_ear: 0.1 }, "neutral"],
      [{ ...candidate, candidate: "not_candidate", eye_focus: 0.1 }, "low"],
      [{ ...candidate, candidate: "unknown", eye_focus: null }, "neutral"],
      [{ ...candidate, point: null }, "good"],
      [{ ...candidate, point: null, candidate: "not_candidate", eye_focus: 0.1 }, "low"],
      [{ ...candidate, point: null, candidate: "unknown", eye_focus: null }, "neutral"],
      [undefined, "neutral"],
    ];
    for (const item of ["good", "neutral", "low"] as const) {
      for (const [focus, owner] of frames) {
        expect(passes(items(item), unjudged, undefined, 1, afEyeState(focus))).toBe(item === owner);
      }
    }
  });

  test("ANDs with the other groups", () => {
    const flagged = state(["untagged"], [], [], [], [], ["good"]);
    expect(passes(flagged, unjudged, undefined, 1, "good")).toBe(true);
    expect(passes(flagged, pickedTwo, undefined, 1, "good")).toBe(false);
  });
});

describe("passes: eyes", () => {
  const eyes = (...states: EyeState[]) => state([], [], [], [], [], [], states);

  test("nothing checked passes every state", () => {
    for (const value of ["open", "closed", "unknown", undefined] as const) {
      expect(passes(state(), unjudged, undefined, 1, undefined, value)).toBe(true);
    }
  });

  test("open passes open eyes only", () => {
    expect(passes(eyes("open"), unjudged, undefined, 1, undefined, "open")).toBe(true);
    expect(passes(eyes("open"), unjudged, undefined, 1, undefined, "closed")).toBe(false);
    expect(passes(eyes("open"), unjudged, undefined, 1, undefined, "unknown")).toBe(false);
    expect(passes(eyes("open"), unjudged, undefined, 1, undefined, undefined)).toBe(false);
  });

  test("closed passes closed eyes only", () => {
    expect(passes(eyes("closed"), unjudged, undefined, 1, undefined, "closed")).toBe(true);
    expect(passes(eyes("closed"), unjudged, undefined, 1, undefined, "open")).toBe(false);
    expect(passes(eyes("closed"), unjudged, undefined, 1, undefined, "unknown")).toBe(false);
    expect(passes(eyes("closed"), unjudged, undefined, 1, undefined, undefined)).toBe(false);
  });

  test("unknown passes an unknown state or a file without a focus", () => {
    expect(passes(eyes("unknown"), unjudged, undefined, 1, undefined, "unknown")).toBe(true);
    expect(passes(eyes("unknown"), unjudged, undefined, 1, undefined, undefined)).toBe(true);
    expect(passes(eyes("unknown"), unjudged, undefined, 1, undefined, "open")).toBe(false);
    expect(passes(eyes("unknown"), unjudged, undefined, 1, undefined, "closed")).toBe(false);
  });

  test("ORs the checked states", () => {
    const either = eyes("open", "unknown");
    expect(passes(either, unjudged, undefined, 1, undefined, "open")).toBe(true);
    expect(passes(either, unjudged, undefined, 1, undefined, "unknown")).toBe(true);
    expect(passes(either, unjudged, undefined, 1, undefined, "closed")).toBe(false);
  });

  test("ANDs with the other groups", () => {
    const both = state(["untagged"], [], [], [], [], ["good"], ["closed"]);
    expect(passes(both, unjudged, undefined, 1, "good", "closed")).toBe(true);
    expect(passes(both, unjudged, undefined, 1, "low", "closed")).toBe(false);
    expect(passes(both, pickedTwo, undefined, 1, "good", "closed")).toBe(false);
    expect(passes(both, unjudged, undefined, 1, "good", "open")).toBe(false);
  });
});

describe("passes: genres", () => {
  const genres = (...labels: string[]) => state([], [], [], [], [], [], [], labels);
  const check = (s: FilterState, labels: string[] | undefined) =>
    passes(s, unjudged, undefined, 1, undefined, undefined, labels);

  test("nothing checked passes every file", () => {
    expect(check(state(), ["sports"])).toBe(true);
    expect(check(state(), undefined)).toBe(true);
  });

  test("a file passes when one of its labels is checked", () => {
    expect(check(genres("sports"), ["sports", "basketball"])).toBe(true);
    expect(check(genres("basketball"), ["sports", "basketball"])).toBe(true);
    expect(check(genres("portrait"), ["sports", "basketball"])).toBe(false);
  });

  test("ORs the checked labels", () => {
    const either = genres("portrait", "landscape");
    expect(check(either, ["landscape"])).toBe(true);
    expect(check(either, ["portrait", "snap"])).toBe(true);
    expect(check(either, ["snap"])).toBe(false);
  });

  test("Unknown passes a frame no genre clears or one without an embedding", () => {
    expect(check(genres(UNKNOWN_GENRE), [UNKNOWN_GENRE])).toBe(true);
    expect(check(genres(UNKNOWN_GENRE), undefined)).toBe(true);
    expect(check(genres(UNKNOWN_GENRE), ["snap"])).toBe(false);
    expect(check(genres("snap"), undefined)).toBe(false);
  });

  test("ANDs with the other groups", () => {
    const both = state(["untagged"], [], [], [], [], [], [], ["sports"]);
    expect(passes(both, unjudged, undefined, 1, undefined, undefined, ["sports"])).toBe(true);
    expect(passes(both, pickedTwo, undefined, 1, undefined, undefined, ["sports"])).toBe(false);
  });

  test("is not remembered", () => {
    expect(toStored(genres("sports"))).not.toHaveProperty("genres");
    const s = genres("sports");
    applyStored(s, toStored(state(["picked"])));
    expect([...s.genres]).toEqual(["sports"]);
  });
});

describe("passes: outputs", () => {
  const outputs = (...items: OutputState[]) => state([], [], [], [], [], [], [], [], items);
  const check = (s: FilterState, output: OutputState | undefined) =>
    passes(s, unjudged, undefined, 1, undefined, undefined, undefined, output);

  test("nothing checked lets every file through, known or not", () => {
    expect(check(outputs(), "yes")).toBe(true);
    expect(check(outputs(), "no")).toBe(true);
    expect(check(outputs(), undefined)).toBe(true);
  });

  test("Output passes the files with an export", () => {
    expect(check(outputs("yes"), "yes")).toBe(true);
    expect(check(outputs("yes"), "no")).toBe(false);
  });

  test("No output passes the files without an export", () => {
    expect(check(outputs("no"), "no")).toBe(true);
    expect(check(outputs("no"), "yes")).toBe(false);
  });

  test("both checked pass both states", () => {
    expect(check(outputs("yes", "no"), "yes")).toBe(true);
    expect(check(outputs("yes", "no"), "no")).toBe(true);
  });

  test("an unknown state passes no checked item", () => {
    expect(check(outputs("no"), undefined)).toBe(false);
    expect(check(outputs("yes"), undefined)).toBe(false);
    expect(check(outputs("yes", "no"), undefined)).toBe(false);
  });

  test("ANDs with the other groups", () => {
    const both = state(["untagged"], [], [], [], [], [], [], [], ["no"]);
    const pass = (judgment: Judgment) =>
      passes(both, judgment, undefined, 1, undefined, undefined, undefined, "no");
    expect(pass(unjudged)).toBe(true);
    expect(pass(pickedTwo)).toBe(false);
  });

  test("is not remembered", () => {
    expect(toStored(outputs("no"))).not.toHaveProperty("outputs");
    const s = outputs("no");
    applyStored(s, toStored(state(["picked"])));
    expect([...s.outputs]).toEqual(["no"]);
  });
});

describe("outputOf", () => {
  const listing = new Set(["A.ARW", "B.ARW"]);

  test("reads a listed file from the answer", () => {
    expect(outputOf(new Set(["A.ARW"]), listing, "A.ARW")).toBe("yes");
    expect(outputOf(new Set(["A.ARW"]), listing, "B.ARW")).toBe("no");
  });

  test("an empty answer is every listed file without an export", () => {
    expect(outputOf(new Set(), listing, "A.ARW")).toBe("no");
  });

  test("no answer leaves every file unknown", () => {
    expect(outputOf(null, listing, "A.ARW")).toBeUndefined();
  });

  test("a file the answer was not asked about is unknown", () => {
    expect(outputOf(new Set(["A.ARW"]), listing, "C.ARW")).toBeUndefined();
  });
});

describe("toStored / applyStored", () => {
  test("round-trips the remembered sections in set order", () => {
    const s = state(
      ["rejected", "picked"],
      [5, 0],
      [],
      ["red", "none"],
      ["portrait"],
      ["neutral", "low"],
      ["closed"],
    );
    const stored = toStored(s);
    expect(stored).toEqual({
      flags: ["rejected", "picked"],
      stars: [5, 0],
      labels: ["red", "none"],
      orientations: ["portrait"],
      candidates: ["neutral", "low"],
      eyes: ["closed"],
    });
    const restored = state(["untagged"], [3]);
    applyStored(restored, stored);
    expect(toStored(restored)).toEqual(stored);
  });

  test("stores an empty filter as empty sections and restores it as all-pass", () => {
    const empty = { flags: [], stars: [], labels: [], orientations: [], candidates: [], eyes: [] };
    expect(toStored(state())).toEqual(empty);
    const s = state(["picked"], [1], [], ["blue"], ["landscape"], ["good"], ["open"]);
    applyStored(s, empty);
    expect(toStored(s)).toEqual(empty);
    expect(passes(s, rejected, undefined, undefined)).toBe(true);
  });

  test("applyStored leaves the EXIF groups alone", () => {
    const s = state([], [], [["camera", ["ILCE-7RM5"]]]);
    applyStored(s, toStored(state(["picked"])));
    expect([...s.exif.get("camera")!]).toEqual(["ILCE-7RM5"]);
    expect(toStored(s)).not.toHaveProperty("exif");
  });
});
