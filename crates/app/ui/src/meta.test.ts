import { describe, expect, test } from "vitest";
import { GOOD_FULL_EYE_SHARP } from "./focus.js";
import {
  EXIF_HEADING,
  type FocusCue,
  MAKER_NOTE_HEADING,
  type Metadata,
  ANALYSIS_HEADING,
  metaGroups,
  type Ranking,
} from "./meta.js";
import type { Eyes } from "./eyes.js";

const mesh: Eyes["mesh"] = { width: 1600, height: 1080, points: [[800, 540]] };

const empty: Metadata = {
  name: "DSC00001.ARW",
  camera: null,
  lens: null,
  aperture: null,
  shutter: null,
  shutter_type: null,
  focus_mode: null,
  af_area: null,
  af_tracking: null,
  drive: null,
  stabilization: null,
  exposure_mode: null,
  metering: null,
  creative_style: null,
  dro: null,
  raw_type: null,
  iso: null,
  focal_length: null,
  exposure_bias: null,
  focus_distance: null,
  captured_at: null,
};

const sony: Metadata = {
  ...empty,
  camera: "SONY ILCE-1",
  lens: "FE 50mm F1.2 GM",
  aperture: "f/2.8",
  shutter: "1/500",
  shutter_type: "Electronic front curtain",
  focus_mode: "AF-C",
  af_area: "Custom AF Area",
  af_tracking: "Face tracking",
  drive: "Continuous, frame 2",
  stabilization: "On",
  exposure_mode: "Manual",
  metering: "Multi-segment",
  creative_style: "Standard",
  dro: "Auto",
  raw_type: "Lossless Compressed RAW",
  iso: "ISO 100",
  focal_length: "50 mm",
  exposure_bias: "+0.3 EV",
  captured_at: "2026-09-24 12:00:00",
};

const leica: Metadata = {
  ...empty,
  name: "L1000001.DNG",
  camera: "Leica M11-P",
  aperture: "f/2.0 (est.)",
  focus_distance: "1.5 m",
};

const noEyes = { eyes_ear: null, eyes: "unknown", pose: null, full_eye_sharp: null } as const;

describe("metaGroups", () => {
  test("splits Sony-like input into EXIF, Maker note and Analysis", () => {
    expect(metaGroups(sony, { candidate: "candidate", eye_focus: 0.87, ...noEyes })).toEqual([
      {
        heading: EXIF_HEADING,
        rows: [
          { label: "Aperture", value: "f/2.8" },
          { label: "Shutter", value: "1/500" },
          { label: "ISO", value: "ISO 100" },
          { label: "Focal length", value: "50 mm" },
          { label: "Exposure", value: "+0.3 EV" },
          { label: "Camera", value: "SONY ILCE-1" },
          { label: "Lens", value: "FE 50mm F1.2 GM" },
          { label: "Captured", value: "2026-09-24 12:00:00" },
        ],
      },
      {
        heading: MAKER_NOTE_HEADING,
        rows: [
          { label: "Shutter type", value: "Electronic front curtain" },
          { label: "Focus mode", value: "AF-C" },
          { label: "AF area", value: "Custom AF Area" },
          { label: "AF tracking", value: "Face tracking" },
          { label: "Drive", value: "Continuous, frame 2" },
          { label: "Stabilization", value: "On" },
          { label: "Exposure mode", value: "Manual" },
          { label: "Metering", value: "Multi-segment" },
          { label: "Creative style", value: "Standard" },
          { label: "DRO", value: "Auto" },
          { label: "RAW type", value: "Lossless Compressed RAW" },
        ],
      },
      {
        heading: ANALYSIS_HEADING,
        rows: [{ label: "AF eye in focus", value: "87%" }],
      },
    ]);
  });

  test("puts the Leica focus distance in the Maker note group and drops null rows", () => {
    expect(metaGroups(leica)).toEqual([
      {
        heading: EXIF_HEADING,
        rows: [
          { label: "Aperture", value: "f/2.0 (est.)" },
          { label: "Camera", value: "Leica M11-P" },
        ],
      },
      { heading: MAKER_NOTE_HEADING, rows: [{ label: "Focus distance", value: "1.5 m" }] },
    ]);
  });

  test("leaves out the Maker note group when no maker note field is set", () => {
    const groups = metaGroups(
      {
        ...sony,
        shutter_type: null,
        focus_mode: null,
        af_area: null,
        af_tracking: null,
        drive: null,
        stabilization: null,
        exposure_mode: null,
        metering: null,
        creative_style: null,
        dro: null,
        raw_type: null,
      },
      { candidate: "candidate", eye_focus: 0.87, ...noEyes },
    );
    expect(groups.map(({ heading }) => heading)).toEqual([EXIF_HEADING, ANALYSIS_HEADING]);
  });

  test("leaves out the EXIF and Maker note groups when they have no rows", () => {
    expect(
      metaGroups(empty, { candidate: "candidate", eye_focus: 0.87, ...noEyes }).map(
        ({ heading }) => heading,
      ),
    ).toEqual([ANALYSIS_HEADING]);
  });

  test("shows only the analysis group when the metadata could not be read", () => {
    expect(metaGroups(null, { candidate: "candidate", eye_focus: 0.87, ...noEyes })).toEqual([
      {
        heading: ANALYSIS_HEADING,
        rows: [{ label: "AF eye in focus", value: "87%" }],
      },
    ]);
  });

  test("leaves out the analysis group without a value", () => {
    expect(metaGroups(sony).map(({ heading }) => heading)).toEqual([
      EXIF_HEADING,
      MAKER_NOTE_HEADING,
    ]);
    expect(metaGroups(null)).toEqual([]);
  });

  const judgment = (state: Eyes["state"], ear: number, pose: Eyes["pose"] = null): Eyes => ({
    state,
    probability: 0.5,
    ear,
    pose,
    mesh,
  });

  test("shows the AF eye in-focus probability as a percentage", () => {
    const riffle = (focus: FocusCue) => metaGroups(null, focus)[0]?.rows;
    expect(riffle({ candidate: "candidate", eye_focus: 0.8701, ...noEyes })).toEqual([
      { label: "AF eye in focus", value: "87%" },
    ]);
    expect(riffle({ candidate: "not_candidate", eye_focus: 0, ...noEyes })).toEqual([
      { label: "AF eye in focus", value: "0%" },
    ]);
  });

  test("leaves out the row when there is no value", () => {
    expect(metaGroups(null, { candidate: "unknown", eye_focus: null, ...noEyes })).toEqual([]);
  });

  test("shows the judgment and the openness after the AF eye row", () => {
    const focus: FocusCue = { candidate: "candidate", eye_focus: 0.87, ...noEyes };
    expect(metaGroups(null, focus, judgment("open", 0.25))[0]?.rows).toEqual([
      { label: "AF eye in focus", value: "87%" },
      { label: "Eyes", value: "Open · 41" },
    ]);
    expect(metaGroups(null, null, judgment("closed", 0.09))).toEqual([
      { heading: ANALYSIS_HEADING, rows: [{ label: "Eyes", value: "Closed · 0" }] },
    ]);
  });

  test("shows the openness of the judgment's EAR, not its closed probability", () => {
    const eyesRow = (eyes: Eyes) => metaGroups(null, null, eyes)[0]?.rows;
    expect(eyesRow(judgment("open", 0.45))).toEqual([{ label: "Eyes", value: "Open · 100" }]);
    expect(eyesRow({ ...judgment("open", 0.3), probability: 0.009 })).toEqual([
      { label: "Eyes", value: "Open · 60" },
    ]);
  });

  test("leaves out the eyes row when it is unknown or not yet judged", () => {
    const focus: FocusCue = { candidate: "candidate", eye_focus: 0.87, ...noEyes };
    const rows = [{ label: "AF eye in focus", value: "87%" }];
    expect(metaGroups(null, focus, null)[0]?.rows).toEqual(rows);
    expect(metaGroups(null, focus, undefined)[0]?.rows).toEqual(rows);
  });

  test("shows the head pose after the eyes row", () => {
    const pose = { yaw: 12.4, pitch: -5.2, roll: 3.5 };
    expect(metaGroups(null, null, judgment("open", 0.45, pose))).toEqual([
      {
        heading: ANALYSIS_HEADING,
        rows: [
          { label: "Eyes", value: "Open · 100" },
          { label: "Yaw", value: "12°" },
          { label: "Pitch", value: "-5°" },
          { label: "Roll", value: "4°" },
        ],
      },
    ]);
    const poseRows = (judged: Eyes["pose"]) =>
      metaGroups(null, null, judgment("open", 0.45, judged))[0]?.rows.slice(1);
    expect(poseRows({ yaw: -110.2, pitch: 0, roll: -1.6 })).toEqual([
      { label: "Yaw", value: "-110°" },
      { label: "Pitch", value: "0°" },
      { label: "Roll", value: "-2°" },
    ]);
  });

  test("rounds a small negative angle to 0°", () => {
    const judged = { yaw: -0.4, pitch: -0.2, roll: -0.49 };
    expect(metaGroups(null, null, judgment("open", 0.45, judged))[0]?.rows.slice(1)).toEqual([
      { label: "Yaw", value: "0°" },
      { label: "Pitch", value: "0°" },
      { label: "Roll", value: "0°" },
    ]);
  });

  test("leaves out the head pose when it is unknown or the eyes are", () => {
    const focus: FocusCue = { candidate: "candidate", eye_focus: 0.87, ...noEyes };
    const rows = [{ label: "AF eye in focus", value: "87%" }];
    expect(metaGroups(null, focus, judgment("open", 0.45))[0]?.rows).toEqual([
      ...rows,
      { label: "Eyes", value: "Open · 100" },
    ]);
    expect(metaGroups(null, focus, null)[0]?.rows).toEqual(rows);
    expect(metaGroups(null, focus, undefined)[0]?.rows).toEqual(rows);
  });

  test("shows the stored eye state, openness and head pose without a judgment", () => {
    const focus: FocusCue = {
      candidate: "candidate",
      eye_focus: 0.87,
      eyes_ear: 0.09,
      eyes: "closed",
      pose: { yaw: 12.4, pitch: -5.2, roll: 3.5 },
      full_eye_sharp: null,
    };
    expect(metaGroups(null, focus, undefined)[0]?.rows).toEqual([
      { label: "AF eye in focus", value: "87%" },
      { label: "Eyes", value: "Closed · 0" },
      { label: "Yaw", value: "12°" },
      { label: "Pitch", value: "-5°" },
      { label: "Roll", value: "4°" },
    ]);
  });

  test("shows the full-size eye score against the Good cut only when there is one", () => {
    const rows = (full_eye_sharp: number | null) =>
      metaGroups(null, { candidate: "candidate", eye_focus: 0.87, ...noEyes, full_eye_sharp })[0]
        ?.rows;
    expect(rows(0.143)).toEqual([
      { label: "AF eye in focus", value: "87%" },
      { label: "Eyes at full size", value: "Soft · 0.143" },
    ]);
    expect(rows(GOOD_FULL_EYE_SHARP)?.[1]).toEqual({
      label: "Eyes at full size",
      value: "Sharp · 0.162",
    });
    expect(rows(null)).toEqual([{ label: "AF eye in focus", value: "87%" }]);
  });

  test("falls back to the judgment without a stored eye state", () => {
    const focus: FocusCue = { candidate: "unknown", eye_focus: null, ...noEyes };
    const pose = { yaw: -20, pitch: 0, roll: 0 };
    expect(metaGroups(null, focus, judgment("open", 0.25, pose))[0]?.rows).toEqual([
      { label: "Eyes", value: "Open · 41" },
      { label: "Yaw", value: "-20°" },
      { label: "Pitch", value: "0°" },
      { label: "Roll", value: "0°" },
    ]);
  });

  test("prefers the stored eye state and head pose over the judgment", () => {
    const focus: FocusCue = {
      candidate: "candidate",
      eye_focus: null,
      eyes_ear: 0.3,
      eyes: "open",
      pose: { yaw: 1, pitch: 2, roll: 3 },
      full_eye_sharp: null,
    };
    const judged = { yaw: 40, pitch: 0, roll: 0 };
    expect(metaGroups(null, focus, judgment("closed", 0.09, judged))[0]?.rows).toEqual([
      { label: "Eyes", value: "Open · 60" },
      { label: "Yaw", value: "1°" },
      { label: "Pitch", value: "2°" },
      { label: "Roll", value: "3°" },
    ]);
  });

  test("leaves out the eye rows with neither a stored state nor a judgment", () => {
    const focus: FocusCue = { candidate: "not_candidate", eye_focus: 0.07, ...noEyes };
    expect(metaGroups(null, focus, null)[0]?.rows).toEqual([
      { label: "AF eye in focus", value: "7%" },
    ]);
  });

  test("shows the analysis group for the in-focus probability alone", () => {
    expect(metaGroups(null, { candidate: "not_candidate", eye_focus: 0.07, ...noEyes })).toEqual([
      {
        heading: ANALYSIS_HEADING,
        rows: [{ label: "AF eye in focus", value: "7%" }],
      },
    ]);
  });
});

describe("metaGroups with the personal ranking", () => {
  const ranking: Ranking = {
    ranked: {
      score: 0.734,
      place: 1,
      of: 6,
      reasons: [
        ["Look", 0.61],
        ["Burst position", -0.3],
        ["Sharpness", 0.04],
        ["Eyes open", 0.02],
      ],
      genre: "sports",
    },
    model: { bursts: 120, folders: 3, confidence: "low" },
  };

  test("adds the place with the genre, the largest reasons and the confidence to the analysis group", () => {
    expect(metaGroups(null, null, null, ranking)).toEqual([
      {
        heading: ANALYSIS_HEADING,
        rows: [
          { label: "Recommended", value: "1 of 6 in its burst · +0.73 · as sports" },
          { label: "Reason", value: "Look +0.61, Burst position −0.30" },
          { label: "Confidence", value: "Low · 120 bursts in 3 folders" },
        ],
      },
    ]);
  });

  test("says when the model is the built-in one alone, names no genre for an unknown one and leaves out a reason too small to name", () => {
    const prior: Ranking = {
      ranked: { score: -0.01, place: 2, of: 2, reasons: [["Look", -0.01]], genre: null },
      model: { bursts: 0, folders: 0, confidence: "none" },
    };
    expect(metaGroups(null, null, null, prior)[0]?.rows).toEqual([
      { label: "Recommended", value: "2 of 2 in its burst · −0.01" },
      { label: "Confidence", value: "Built-in only, nothing learned yet" },
    ]);
  });

  test("counts one burst and one folder in the singular", () => {
    const one: Ranking = { ...ranking, model: { bursts: 1, folders: 1, confidence: "low" } };
    expect(metaGroups(null, null, null, one)[0]?.rows.at(-1)).toEqual({
      label: "Confidence",
      value: "Low · 1 burst in 1 folder",
    });
  });
});
