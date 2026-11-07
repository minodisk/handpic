import { describe, expect, test } from "vitest";
import {
  AF_EYE_HELP,
  AF_EYE_HELP_CLOSING,
  EYES_CLOSED_EAR,
  EYES_WIDE_OPEN_EAR,
  FOCUS_MARK_COLORS,
  FOCUS_MARK_ICONS,
  GOOD_EYE_EAR,
  GOOD_EYE_FOCUS,
  GOOD_MAX_PITCH,
  GOOD_MAX_YAW,
  MAX_EYE_OFFSET,
  MIN_EDGE_GAP,
  type FocusPoint,
  type MarkFocus,
  afEyeState,
  applyFaceReady,
  applySharpnessReady,
  FACE_LABEL_HEIGHT,
  eyesOpenness,
  faceLabel,
  faceLabelAt,
  faceMarks,
  focusMark,
  photoTier,
  stripState,
  uprightPoint,
} from "./focus.js";
import { SCAN_FACE_SVG, SCAN_SVG } from "./icons.js";

const noEyes = {
  eyes_ear: null,
  eyes: "unknown",
  eyes_closed: null,
  pose: null,
  eye_offset: null,
  edge_gap: null,
} as const;

const at: FocusPoint = {
  sensor_w: 7008,
  sensor_h: 4672,
  x: 1752,
  y: 1168,
  frame: null,
};

const point: MarkFocus = {
  point: at,
  manual_focus: false,
  candidate: "unknown",
  eye_focus: null,
  ...noEyes,
};

const good: MarkFocus = {
  ...point,
  candidate: "candidate",
  eye_focus: 1,
  eyes_ear: 0.35,
  eyes: "open",
  eyes_closed: 0.001,
  pose: { yaw: 10, pitch: -5, roll: 3 },
  eye_offset: 0.03,
  edge_gap: 2,
};

describe("focusMark", () => {
  test("draws nothing without a focus point", () => {
    expect(focusMark(null, 700, 467)).toBeNull();
    expect(focusMark(undefined, 700, 467)).toBeNull();
  });

  test("draws nothing for a manual-focus shot", () => {
    const frame = { width: 876, height: 584 };
    expect(
      focusMark({ ...point, point: { ...at, frame }, manual_focus: true }, 700, 467),
    ).toBeNull();
  });

  test("draws nothing for a focus with no recorded point, whatever its state", () => {
    expect(focusMark({ ...good, point: null }, 700, 467)).toBeNull();
    expect(focusMark({ ...point, point: null }, 700, 467)).toBeNull();
  });

  test("a point without a frame is a crosshair only", () => {
    expect(focusMark(point, 700, 468)).toEqual({
      x: -175,
      y: -117,
      rect: null,
      state: "neutral",
    });
  });

  test("a frame is scaled onto the preview and centered on the point", () => {
    const mark = focusMark(
      { ...point, point: { ...at, frame: { width: 876, height: 584 } } },
      700,
      468,
    );
    expect(mark).toEqual({
      x: -175,
      y: -117,
      rect: { x: -218.75, y: -146.25, width: 87.5, height: 58.5 },
      state: "neutral",
    });
  });

  test.each([
    ["not_candidate", "low"],
    ["unknown", "neutral"],
  ] as const)("maps the %s focus candidate to %s", (candidate, state) => {
    expect(focusMark({ ...point, candidate }, 700, 468)?.state).toBe(state);
  });

  test("a focus candidate carries its photoTier, else neutral", () => {
    expect(focusMark(good, 700, 468)?.state).toBe("good");
    expect(focusMark({ ...good, eyes_ear: GOOD_EYE_EAR }, 700, 468)?.state).toBe("good");
    expect(focusMark({ ...good, eyes_ear: 0.1 }, 700, 468)?.state).toBe("neutral");
    expect(focusMark({ ...point, candidate: "candidate" }, 700, 468)?.state).toBe("neutral");
  });

  test("a manual-focus or missing point stays null whatever the state", () => {
    expect(
      focusMark({ ...point, manual_focus: true, candidate: "candidate" }, 700, 468),
    ).toBeNull();
    expect(focusMark(null, 700, 468)).toBeNull();
  });
});

describe("FOCUS_MARK_COLORS", () => {
  test("bright green for good, gray for low, white for neutral", () => {
    expect(FOCUS_MARK_COLORS).toEqual({
      good: "#3f3",
      low: "#999",
      neutral: "#fff",
    });
  });
});

describe("FOCUS_MARK_ICONS", () => {
  test("scan-face for good, scan for low, none for neutral", () => {
    expect(FOCUS_MARK_ICONS).toEqual({
      good: SCAN_FACE_SVG,
      low: SCAN_SVG,
      neutral: null,
    });
  });
});

describe("stripState", () => {
  test("is neutral without a focus", () => {
    expect(stripState(null)).toBe("neutral");
    expect(stripState(undefined)).toBe("neutral");
  });

  test("is the mark's state, a manual-focus shot included", () => {
    expect(stripState(good)).toBe("good");
    expect(stripState({ ...good, eyes_ear: 0.1 })).toBe("neutral");
    expect(stripState({ ...point, candidate: "not_candidate" })).toBe("low");
    expect(stripState(point)).toBe("neutral");
    expect(stripState({ ...point, manual_focus: true, candidate: "not_candidate" })).toBe("low");
  });

  test("reads the cue of a focus with no recorded point", () => {
    expect(stripState({ ...good, point: null })).toBe("good");
    expect(stripState({ ...good, point: null, eyes_ear: 0.1 })).toBe("neutral");
    expect(stripState({ ...point, point: null, candidate: "not_candidate" })).toBe("low");
    expect(stripState({ ...point, point: null })).toBe("neutral");
  });
});

describe("afEyeState", () => {
  test("is neutral without a focus, as on a JPEG", () => {
    expect(afEyeState(null)).toBe("neutral");
    expect(afEyeState(undefined)).toBe("neutral");
  });

  test("is good for the photo tier", () => {
    expect(afEyeState(good)).toBe("good");
  });

  test("is good at GOOD_EYE_FOCUS with the other conditions met", () => {
    expect(afEyeState({ ...good, candidate: "candidate", eye_focus: GOOD_EYE_FOCUS })).toBe("good");
  });

  test("is neutral for a focus candidate that is not good", () => {
    expect(afEyeState({ ...good, eyes_ear: 0.1 })).toBe("neutral");
    expect(afEyeState({ ...good, eye_focus: GOOD_EYE_FOCUS - 0.01 })).toBe("neutral");
    expect(afEyeState({ ...point, candidate: "candidate", eye_focus: 0.772 })).toBe("neutral");
  });

  test("is low for a not_candidate focus, whatever else clears the rule", () => {
    expect(afEyeState({ ...point, candidate: "not_candidate", eye_focus: 0.7719 })).toBe("low");
    expect(afEyeState({ ...good, candidate: "not_candidate", eye_focus: 0.7719 })).toBe("low");
  });

  test("is neutral for an unknown focus: no face, or pass 2 not there yet", () => {
    expect(afEyeState({ ...point, candidate: "unknown", eye_focus: null })).toBe("neutral");
    expect(afEyeState({ ...good, candidate: "unknown", eye_focus: null })).toBe("neutral");
  });
});

describe("AF_EYE_HELP", () => {
  test("good names every cut with its figure", () => {
    for (const figure of ["90%", "openness 41", "35°", "45°"]) {
      expect(AF_EYE_HELP.good).toContain(figure);
    }
    expect(AF_EYE_HELP.good).not.toContain("77%");
  });

  test("neutral holds a candidate that misses Good and every file without a judged face", () => {
    expect(AF_EYE_HELP.neutral).toContain("77% or more");
    for (const holds of ["Good condition", "no face found", "a JPEG", "not reached"]) {
      expect(AF_EYE_HELP.neutral).toContain(holds);
    }
  });

  test("low is a cue on the judged face under the candidate threshold", () => {
    expect(AF_EYE_HELP.low).toContain("A face was judged");
    expect(AF_EYE_HELP.low).toContain("under 77%");
  });

  test("the closing line says nothing is automatic", () => {
    expect(AF_EYE_HELP_CLOSING).toContain("nothing is rejected for you");
  });
});

describe("eyesOpenness", () => {
  test("is 0 at the closed EAR and 100 at the wide open one", () => {
    expect(eyesOpenness(EYES_CLOSED_EAR)).toBe(0);
    expect(eyesOpenness(EYES_WIDE_OPEN_EAR)).toBe(100);
  });

  test("clamps below the closed EAR and above the wide open one", () => {
    expect(eyesOpenness(0.05)).toBe(0);
    expect(eyesOpenness(0)).toBe(0);
    expect(eyesOpenness(0.6)).toBe(100);
  });

  test("is linear in the EAR between", () => {
    expect(eyesOpenness((EYES_CLOSED_EAR + EYES_WIDE_OPEN_EAR) / 2)).toBeCloseTo(50, 10);
    expect(eyesOpenness(0.25)).toBeCloseTo(41.39, 2);
  });

  test("is null without an EAR", () => {
    expect(eyesOpenness(null)).toBeNull();
  });
});

describe("photoTier: good", () => {
  const goodPhoto = (focus: MarkFocus | null | undefined) => photoTier(focus) === "good";

  test("passes a candidate whose eyes and pose clear every cut", () => {
    expect(goodPhoto(good)).toBe(true);
  });

  test("passes a candidate with no recorded point on its values", () => {
    expect(photoTier({ ...good, point: null })).toBe("good");
    expect(photoTier({ ...good, point: null, manual_focus: true })).toBe("good");
    expect(photoTier({ ...good, point: null, eye_focus: GOOD_EYE_FOCUS - 0.01 })).toBeNull();
  });

  test("is false without a focus", () => {
    expect(goodPhoto(null)).toBe(false);
    expect(goodPhoto(undefined)).toBe(false);
  });

  test.each(["not_candidate", "unknown"] as const)(
    "is false for a %s frame even with good eyes and pose",
    (candidate) => {
      expect(goodPhoto({ ...good, candidate })).toBe(false);
    },
  );

  test("is false when any value is missing", () => {
    expect(goodPhoto({ ...good, eye_focus: null })).toBe(false);
    expect(goodPhoto({ ...good, eyes_ear: null })).toBe(false);
    expect(goodPhoto({ ...good, pose: null })).toBe(false);
    expect(goodPhoto({ ...good, eye_offset: null })).toBe(false);
    expect(goodPhoto({ ...good, edge_gap: null })).toBe(false);
  });

  test("the eye offset passes at and below its cut, not above", () => {
    expect(goodPhoto({ ...good, eye_offset: MAX_EYE_OFFSET })).toBe(true);
    expect(goodPhoto({ ...good, eye_offset: MAX_EYE_OFFSET - 0.001 })).toBe(true);
    expect(goodPhoto({ ...good, eye_offset: MAX_EYE_OFFSET + 0.001 })).toBe(false);
  });

  test("the edge gap passes at and above its cut, not below or outside", () => {
    expect(goodPhoto({ ...good, edge_gap: MIN_EDGE_GAP })).toBe(true);
    expect(goodPhoto({ ...good, edge_gap: MIN_EDGE_GAP + 0.001 })).toBe(true);
    expect(goodPhoto({ ...good, edge_gap: MIN_EDGE_GAP - 0.001 })).toBe(false);
    expect(goodPhoto({ ...good, edge_gap: -0.1 })).toBe(false);
  });

  test("the three frames the user named are not good", () => {
    // `_DSC2638`: the mesh off a rotated face.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.999,
        eyes_ear: 0.3601,
        pose: { yaw: 3, pitch: -11.8, roll: 11.8 },
        eye_offset: 0.1247,
        edge_gap: 0.2241,
      }),
    ).toBeNull();
    // `_DSC2827`: turned, the EAR inflated.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.9984,
        eyes_ear: 0.3032,
        pose: { yaw: -48.6, pitch: 5.2, roll: 9.4 },
        eye_offset: 0.0866,
        edge_gap: 1.6918,
      }),
    ).toBeNull();
    // `_DSC3345`: cut by the left edge, its mesh off too.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.9984,
        eyes_ear: 0.314,
        pose: { yaw: -15.3, pitch: 2.7, roll: -3.6 },
        eye_offset: 0.3521,
        edge_gap: 0.0076,
      }),
    ).toBeNull();
  });

  test("the 1-star frames a looser yaw or eye offset cut lets in are not good", () => {
    // `_DSC2050`: turned past the yaw cut.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.9966,
        eyes_ear: 0.3228,
        pose: { yaw: -36.6, pitch: 11.7, roll: 5.1 },
        eye_offset: 0.0241,
        edge_gap: 5.1189,
      }),
    ).toBeNull();
    // `_DSC6920`: the mesh off the eyes of a face YuNet sees in profile.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.9913,
        eyes_ear: 0.4054,
        pose: { yaw: 26.8, pitch: -17.4, roll: 3.6 },
        eye_offset: 0.1769,
        edge_gap: 3.3674,
      }),
    ).toBeNull();
  });

  test("a 5-star frame the Step 5b cuts left out is good", () => {
    // `_DSC3280`: the AF eyes at 0.92.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.9218,
        eyes_ear: 0.3468,
        pose: { yaw: -23.9, pitch: -16.1, roll: -4.7 },
        eye_offset: 0.0298,
        edge_gap: 7.2376,
      }),
    ).toBe("good");
    // `_DSC4912`: turned 34.5 degrees.
    expect(
      photoTier({
        ...good,
        eye_focus: 0.9986,
        eyes_ear: 0.4194,
        pose: { yaw: 34.5, pitch: 3.7, roll: 1.6 },
        eye_offset: 0.0456,
        edge_gap: 4.7066,
      }),
    ).toBe("good");
  });

  test("the eye_focus cut passes at and above, not below", () => {
    expect(goodPhoto({ ...good, eye_focus: GOOD_EYE_FOCUS })).toBe(true);
    expect(goodPhoto({ ...good, eye_focus: GOOD_EYE_FOCUS + 0.0005 })).toBe(true);
    expect(goodPhoto({ ...good, eye_focus: GOOD_EYE_FOCUS - 0.0005 })).toBe(false);
  });

  test("the EAR cut passes at and above, not below", () => {
    expect(goodPhoto({ ...good, eyes_ear: GOOD_EYE_EAR })).toBe(true);
    expect(goodPhoto({ ...good, eyes_ear: GOOD_EYE_EAR + 0.001 })).toBe(true);
    expect(goodPhoto({ ...good, eyes_ear: GOOD_EYE_EAR - 0.001 })).toBe(false);
  });

  test.each([1, -1])("the yaw cut passes at and within, not beyond, sign %d", (sign) => {
    const at = (yaw: number) =>
      goodPhoto({ ...good, pose: { yaw: sign * yaw, pitch: 0, roll: 0 } });
    expect(at(GOOD_MAX_YAW)).toBe(true);
    expect(at(GOOD_MAX_YAW - 0.1)).toBe(true);
    expect(at(GOOD_MAX_YAW + 0.1)).toBe(false);
  });

  test.each([1, -1])("the pitch cut passes at and within, not beyond, sign %d", (sign) => {
    const at = (pitch: number) =>
      goodPhoto({ ...good, pose: { yaw: 0, pitch: sign * pitch, roll: 0 } });
    expect(at(GOOD_MAX_PITCH)).toBe(true);
    expect(at(GOOD_MAX_PITCH - 0.1)).toBe(true);
    expect(at(GOOD_MAX_PITCH + 0.1)).toBe(false);
  });

  test("the roll is not a cut", () => {
    expect(goodPhoto({ ...good, pose: { yaw: 0, pitch: 0, roll: 90 } })).toBe(true);
  });
});

describe("photoTier: the folded tier", () => {
  test("is good or null, never another tier", () => {
    expect(photoTier(good)).toBe("good");
    expect(photoTier({ ...good, eyes_ear: 0.1 })).toBeNull();
  });

  test("the cuts are the Step 7 cuts", () => {
    expect(GOOD_EYE_FOCUS).toBe(0.9);
    expect(GOOD_EYE_EAR).toBe(0.25);
    expect(GOOD_MAX_YAW).toBe(35);
    expect(GOOD_MAX_PITCH).toBe(45);
    expect(MAX_EYE_OFFSET).toBe(0.1);
    expect(MIN_EDGE_GAP).toBe(0.02);
  });

  test("a frame below the Step 5 good cuts that cleared the fair ones is good", () => {
    expect(photoTier({ ...good, eye_focus: 0.995 })).toBe("good");
    expect(photoTier({ ...good, eyes_ear: 0.27 })).toBe("good");
    expect(photoTier({ ...good, eye_focus: 0.99, eyes_ear: 0.25 })).toBe("good");
  });

  test("the exclusions still apply at the folded cuts", () => {
    const edge = { ...good, eye_focus: GOOD_EYE_FOCUS, eyes_ear: GOOD_EYE_EAR };
    expect(photoTier(edge)).toBe("good");
    expect(photoTier({ ...edge, eye_offset: MAX_EYE_OFFSET + 0.001 })).toBeNull();
    expect(photoTier({ ...edge, edge_gap: MIN_EDGE_GAP - 0.001 })).toBeNull();
  });
});

describe("faceMarks", () => {
  const face = (x: number, y: number) => ({
    x,
    y,
    width: 100,
    height: 120,
    score: 0.934,
    left_eye: { x: x + 30, y: y + 40 },
    right_eye: { x: x + 70, y: y + 40 },
    nose: { x: x + 50, y: y + 70 },
    mouth_left: { x: x + 35, y: y + 95 },
    mouth_right: { x: x + 65, y: y + 95 },
  });
  const label = "0.93  100×120  eyes 40";

  test("a face at the preview's origin sits at the drawn image's corner", () => {
    expect(faceMarks([face(0, 0)], 1600, 1080, 1600, 1080)).toEqual([
      {
        rect: { x: -800, y: -540, width: 100, height: 120 },
        points: [
          { x: -770, y: -500 },
          { x: -730, y: -500 },
          { x: -750, y: -470 },
          { x: -765, y: -445 },
          { x: -735, y: -445 },
        ],
        label,
      },
    ]);
  });

  test("a face at the far corner ends at the drawn image's far corner", () => {
    expect(faceMarks([face(1500, 960)], 1600, 1080, 1600, 1080)).toEqual([
      {
        rect: { x: 700, y: 420, width: 100, height: 120 },
        points: [
          { x: 730, y: 460 },
          { x: 770, y: 460 },
          { x: 750, y: 490 },
          { x: 735, y: 515 },
          { x: 765, y: 515 },
        ],
        label,
      },
    ]);
  });

  test("a preview drawn smaller scales the faces by the preview size", () => {
    expect(faceMarks([face(400, 200)], 1600, 1080, 800, 540)).toEqual([
      {
        rect: { x: -200, y: -170, width: 50, height: 60 },
        points: [
          { x: -185, y: -150 },
          { x: -165, y: -150 },
          { x: -175, y: -135 },
          { x: -182.5, y: -122.5 },
          { x: -167.5, y: -122.5 },
        ],
        label,
      },
    ]);
  });
});

describe("faceLabel", () => {
  test("gives the score, the box size and the eye distance in preview pixels", () => {
    const face = {
      x: 0,
      y: 0,
      width: 211.6,
      height: 254.4,
      score: 0.934,
      left_eye: { x: 100, y: 100 },
      right_eye: { x: 160, y: 111 },
      nose: { x: 130, y: 140 },
      mouth_left: { x: 110, y: 180 },
      mouth_right: { x: 150, y: 180 },
    };
    expect(faceLabel(face)).toBe("0.93  212×254  eyes 61");
    expect(faceLabel({ ...face, score: 0.6 })).toBe("0.60  212×254  eyes 61");
  });

  test("swaps the box size on a quarter turn so it reads as drawn upright", () => {
    const face = {
      x: 0,
      y: 0,
      width: 211.6,
      height: 254.4,
      score: 0.934,
      left_eye: { x: 100, y: 100 },
      right_eye: { x: 160, y: 111 },
      nose: { x: 130, y: 140 },
      mouth_left: { x: 110, y: 180 },
      mouth_right: { x: 150, y: 180 },
    };
    expect(faceLabel(face, 6)).toBe("0.93  254×212  eyes 61");
    expect(faceLabel(face, 8)).toBe("0.93  254×212  eyes 61");
    expect(faceLabel(face, 3)).toBe("0.93  212×254  eyes 61");
  });
});

describe("uprightPoint", () => {
  test("turns a point as draw() rotates the image", () => {
    const p = { x: 10, y: 20 };
    expect(uprightPoint(p, 1)).toEqual({ x: 10, y: 20 });
    expect(uprightPoint(p, 6)).toEqual({ x: -20, y: 10 });
    expect(uprightPoint(p, 8)).toEqual({ x: 20, y: -10 });
    expect(uprightPoint(p, 3)).toEqual({ x: -10, y: -20 });
  });
});

describe("faceLabelAt", () => {
  // A 300x200 unrotated drawing: 200x300 upright on a quarter turn.
  const rect = { x: -50, y: -40, width: 40, height: 60 };

  test("sits above the upright box's top-left corner", () => {
    const at = (orientation: number) => faceLabelAt(rect, orientation, 300, 200, 80);
    expect(at(1)).toEqual({ x: -50, y: -40 - FACE_LABEL_HEIGHT });
    expect(at(6)).toEqual({ x: -20, y: -50 - FACE_LABEL_HEIGHT });
    expect(at(8)).toEqual({ x: -40, y: 10 - FACE_LABEL_HEIGHT });
    expect(at(3)).toEqual({ x: 10, y: -20 - FACE_LABEL_HEIGHT });
  });

  test("drops inside the box when the box touches the image's top", () => {
    const at = (r: typeof rect, orientation: number) => faceLabelAt(r, orientation, 300, 200, 80).y;
    expect(at({ ...rect, y: -100 }, 1)).toBe(-98);
    expect(at({ ...rect, x: -150 }, 6)).toBe(-148);
    expect(at({ ...rect, x: 110 }, 8)).toBe(-148);
    expect(at({ ...rect, y: 40 }, 3)).toBe(-98);
  });

  test("stays inside the image's right edge", () => {
    expect(faceLabelAt({ ...rect, x: 100 }, 1, 300, 200, 80).x).toBe(70);
    expect(faceLabelAt({ ...rect, y: -100 }, 6, 300, 200, 80).x).toBe(20);
  });
});

describe("applyFaceReady", () => {
  const rows = () =>
    new Map<string, { focus: MarkFocus | null }>([
      ["/d/a.ARW", { focus: { ...point } }],
      ["/d/b.ARW", { focus: { ...point } }],
      ["/d/c.ARW", { focus: null }],
      ["/d/d.DNG", { focus: { ...point, point: null } }],
    ]);

  test("patches the ready files and reports whether the current one was among them", () => {
    const entries = rows();
    const touched = applyFaceReady(
      entries,
      [
        { path: "/d/a.ARW", eye_focus: 0.9, candidate: "candidate", sharpness: null, ...noEyes },
        {
          path: "/d/b.ARW",
          eye_focus: 0.3,
          candidate: "not_candidate",
          sharpness: null,
          ...noEyes,
        },
      ],
      "/d/b.ARW",
    );
    expect(touched).toBe(true);
    expect(entries.get("/d/a.ARW")?.focus).toMatchObject({
      eye_focus: 0.9,
      candidate: "candidate",
    });
    expect(entries.get("/d/b.ARW")?.focus).toMatchObject({
      eye_focus: 0.3,
      candidate: "not_candidate",
    });
  });

  test("patches the stored eye state and head pose", () => {
    const entries = rows();
    const pose = { yaw: 12, pitch: -5, roll: 3 };
    applyFaceReady(
      entries,
      [
        {
          path: "/d/a.ARW",
          eye_focus: 0.9,
          candidate: "candidate",
          sharpness: null,
          eyes_ear: 0.08,
          eyes: "closed",
          eyes_closed: 0.9,
          pose,
          eye_offset: 0.05,
          edge_gap: -0.01,
        },
      ],
      undefined,
    );
    expect(entries.get("/d/a.ARW")?.focus).toMatchObject({
      eyes_ear: 0.08,
      eyes: "closed",
      eyes_closed: 0.9,
      pose,
      eye_offset: 0.05,
      edge_gap: -0.01,
    });
  });

  test("patches a focus with no recorded point", () => {
    const entries = rows();
    expect(
      applyFaceReady(
        entries,
        [
          {
            path: "/d/d.DNG",
            eye_focus: 0.95,
            candidate: "candidate",
            sharpness: null,
            ...noEyes,
          },
        ],
        "/d/d.DNG",
      ),
    ).toBe(true);
    expect(entries.get("/d/d.DNG")?.focus).toMatchObject({
      point: null,
      eye_focus: 0.95,
      candidate: "candidate",
    });
  });

  test("a current file outside the batch is not touched", () => {
    const entries = rows();
    expect(
      applyFaceReady(
        entries,
        [{ path: "/d/a.ARW", eye_focus: 0.9, candidate: "candidate", sharpness: null, ...noEyes }],
        "/d/b.ARW",
      ),
    ).toBe(false);
    expect(entries.get("/d/b.ARW")?.focus?.candidate).toBe("unknown");
  });

  test("a file with no row or no focus point is skipped", () => {
    const entries = rows();
    expect(
      applyFaceReady(
        entries,
        [
          { path: "/d/c.ARW", eye_focus: null, candidate: "unknown", sharpness: null, ...noEyes },
          {
            path: "/d/missing.ARW",
            eye_focus: 0.85,
            candidate: "candidate",
            sharpness: null,
            ...noEyes,
          },
        ],
        "/d/c.ARW",
      ),
    ).toBe(false);
    expect(entries.get("/d/c.ARW")?.focus).toBeNull();
    expect(entries.has("/d/missing.ARW")).toBe(false);
  });
});

describe("applySharpnessReady", () => {
  const ready = (path: string, sharpness: number | null) => ({
    path,
    eye_focus: null,
    candidate: "unknown" as const,
    sharpness,
    ...noEyes,
  });

  test("sets a new score in the map and the row", () => {
    const entries = new Map([["/d/a.ARW", { sharpness: null as number | null }]]);
    const scores = new Map<string, number>();
    applySharpnessReady(entries, scores, [ready("/d/a.ARW", 12.5)]);
    expect(scores.get("/d/a.ARW")).toBe(12.5);
    expect(entries.get("/d/a.ARW")?.sharpness).toBe(12.5);
  });

  test("a file the pass found no score for is deleted from the map and nulled in the row", () => {
    const entries = new Map([["/d/a.ARW", { sharpness: 3 as number | null }]]);
    const scores = new Map([["/d/a.ARW", 3]]);
    applySharpnessReady(entries, scores, [ready("/d/a.ARW", null)]);
    expect(scores.has("/d/a.ARW")).toBe(false);
    expect(entries.get("/d/a.ARW")?.sharpness).toBeNull();
  });

  test("a file with no row still gets its score in the map", () => {
    const entries = new Map<string, { sharpness: number | null }>();
    const scores = new Map<string, number>();
    applySharpnessReady(entries, scores, [ready("/d/missing.ARW", 7)]);
    expect(scores.get("/d/missing.ARW")).toBe(7);
    expect(entries.has("/d/missing.ARW")).toBe(false);
  });
});
