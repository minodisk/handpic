// Where the focus mark goes on the unrotated preview, drawn `drawWidth` x
// `drawHeight` and centered on the origin. Free of DOM and Tauri so it is
// tested without mocks.

import type { StoredEyes } from "./eyes.js";
import { SCAN_FACE_SVG, SCAN_SVG } from "./icons.js";

// The AF point the camera recorded, in sensor coordinates.
export interface FocusPoint {
  sensor_w: number;
  sensor_h: number;
  x: number;
  y: number;
  frame: { width: number; height: number } | null;
}

// A RAW file's AF point, `null` when none was recorded, and the cue pass 2
// stored for its judged face: the one nearest a trusted AF point, else the
// largest confident face of the whole preview.
export interface MarkFocus extends StoredEyes {
  point: FocusPoint | null;
  manual_focus: boolean;
  candidate: "candidate" | "not_candidate" | "unknown";
  eye_focus: number | null;
}

// The tier `photoTier` puts a frame in: a good photo, one that is not a miss.
export type PhotoTier = "good";

// The state the mark's color, the strip's icon and the filter menu's
// `Face focus` item are read from: the frame's `photoTier`, `low` for a frame
// whose judged face (near the AF point, else the largest on the whole preview
// without a trusted one) is under the candidate threshold, which is Rust's
// `not_candidate` (the threshold lives in `candidate.rs`), else `neutral`: a
// focus candidate not in the tier, and both no face found and a file pass 2
// has not reached, which the frontend cannot tell apart.
export type MarkState = PhotoTier | "neutral" | "low";

export interface FocusMark {
  x: number;
  y: number;
  rect: { x: number; y: number; width: number; height: number } | null;
  state: MarkState;
}

// The mark's color per state: bright green for a good photo, gray when the
// judged face's eyes are likely not in focus, white for everything else.
export const FOCUS_MARK_COLORS = {
  good: "#3f3",
  low: "#999",
  neutral: "#fff",
} as const satisfies Record<MarkState, string>;

// The icon per state on the strip cell and the filter menu's `Face focus` item,
// in the mark's color: a face for a good photo, the bare frame for a low one, none
// for a neutral one.
export const FOCUS_MARK_ICONS = {
  good: SCAN_FACE_SVG,
  low: SCAN_SVG,
  neutral: null,
} as const satisfies Record<MarkState, string | null>;

// The good-photo cuts on the values pass 2 stores for the judged face, fitted
// on AF faces only (a face judged without an AF point gets the same cuts),
// read from the re-dump of six ARW folders (8915 faced AF frames) on
// 2026-10-09, tuned in Step 5, folded into one tier in Step 5b and re-tuned
// in Step 7 on the user's stars of 180 frames (`docs/plans/20261008-burst-keep-score/`
// `provisional.md`): 54 of the 58 frames these cuts mark pass (3 stars or
// more), none is a 1-star frame, so the tier means "not a miss".
// The in-focus probability of the AF eyes: the seven rated frames from 0.90 to
// 0.99 that clear the other cuts all pass; below 0.90 none clears them, so
// the cut stays there, above the candidate's 0.772.
export const GOOD_EYE_FOCUS = 0.9;
// The EAR of the more closed eye, at the 35th percentile (open probability
// 0.964, openness 41).
export const GOOD_EYE_EAR = 0.25;
// |yaw| in degrees, "both eyes visible": a turned face foreshortens the eye
// and inflates its EAR. 40 lets in a 1-star frame (`_DSC2050`, yaw -36.6).
export const GOOD_MAX_YAW = 35;
// |pitch| in degrees: 96% of the poses are within.
export const GOOD_MAX_PITCH = 45;

// Exclusions from the tier, on how well the mesh sits on the face.
// The mesh's eyes farther than this from YuNet's eye landmarks, in face box
// sides, mean a mesh fitted off the face (an in-plane rotated face): the
// sample's well-fitted tier frames were all under 0.09, the misfit
// `_DSC2638` at 0.125; 7.6% of the frames the Step 5 cuts alone tiered exceeded it.
// Loosening it, or skipping it where YuNet's eyes sit close together (a
// profile), lets 1-star frames in on the 180 rated frames.
export const MAX_EYE_OFFSET = 0.1;
// The face box or a mesh eye region closer to the preview's edge than this,
// in face box sides, means a face the frame cuts: YuNet's box ends at the
// edge of what is visible (`_DSC3345` at 0.008), so a small margin, not 0.
export const MIN_EDGE_GAP = 0.02;

// The candidate threshold as the `Face focus` help shows it, in percent:
// `candidate_probability()`, the sigmoid of `CANDIDATE_LOGIT` in
// `crates/core/src/candidate.rs`, rounded. Only the text reads it; the state
// comes from Rust's `not_candidate`. A test there fails if the figure moves.
export const CANDIDATE_PERCENT = 77;

// The EAR the eyes count as closed at and below, `EYES_CLOSED_EAR` in
// `crates/core/src/eyes.rs`.
export const EYES_CLOSED_EAR = 0.137;
// The EAR the openness reaches 100 at: the re-dump's 90th percentile (0.408)
// over the faced AF frames with an EAR, rounded.
export const EYES_WIDE_OPEN_EAR = 0.41;

// How open the eyes are, 0 at `EYES_CLOSED_EAR` and below to 100 at
// `EYES_WIDE_OPEN_EAR` and above, linear in the EAR between.
export function eyesOpenness(ear: number | null): number | null {
  if (ear === null) {
    return null;
  }
  const t = (ear - EYES_CLOSED_EAR) / (EYES_WIDE_OPEN_EAR - EYES_CLOSED_EAR);
  return Math.min(Math.max(t, 0), 1) * 100;
}

// What each `Face focus` item holds, from the cuts themselves so the text
// cannot drift from the rule.
export const AF_EYE_HELP: Record<MarkState, string> = {
  good: `A face was judged and all of these hold: its eyes are likely in focus (${Math.round(GOOD_EYE_FOCUS * 100)}% or more), open (openness ${Math.round(eyesOpenness(GOOD_EYE_EAR)!)} or more) and toward the camera (turned at most ${GOOD_MAX_YAW}°, tilted at most ${GOOD_MAX_PITCH}°), the face mesh sits on the face, and the frame's edge does not cut it.`,
  neutral: `Everything else: a judged face whose eyes are likely in focus (${CANDIDATE_PERCENT}% or more) but misses a Good condition, no judged face (a landscape, a snap without a face, faces away from the AF point), a JPEG, or a file the analysis has not reached yet.`,
  low: `A face was judged and its eyes are likely not in focus: the in-focus probability is under ${CANDIDATE_PERCENT}%.`,
};

// The line under the definitions.
export const AF_EYE_HELP_CLOSING =
  "A cue for your review, not a verdict; nothing is rejected for you.";

function clears({ eye_focus, eyes_ear, pose, eye_offset, edge_gap }: MarkFocus): boolean {
  return (
    eye_focus !== null &&
    eye_focus >= GOOD_EYE_FOCUS &&
    eyes_ear !== null &&
    eyes_ear >= GOOD_EYE_EAR &&
    pose !== null &&
    Math.abs(pose.yaw) <= GOOD_MAX_YAW &&
    Math.abs(pose.pitch) <= GOOD_MAX_PITCH &&
    eye_offset !== null &&
    eye_offset <= MAX_EYE_OFFSET &&
    edge_gap !== null &&
    edge_gap >= MIN_EDGE_GAP
  );
}

// A focus candidate whose AF eyes are in focus, whose eyes are open and whose
// face is toward the camera, all at once, is `good`. A mesh off the face or a
// face the frame's edge cuts is not, nor is any missing value (no face judged,
// a face too small for the mesh).
export function photoTier(focus: MarkFocus | null | undefined): PhotoTier | null {
  if (focus === null || focus === undefined || focus.candidate !== "candidate") {
    return null;
  }
  return clears(focus) ? "good" : null;
}

// The state of a file, `neutral` without a focus: the one place the strip's
// icon, the mark and the filter menu read it from.
export function afEyeState(focus: MarkFocus | null | undefined): MarkState {
  if (focus === null || focus === undefined) {
    return "neutral";
  }
  if (focus.candidate === "not_candidate") {
    return "low";
  }
  return photoTier(focus) ?? "neutral";
}

// The mark state of a file the strip's icon shows.
export function stripState(focus: MarkFocus | null | undefined): MarkState {
  return afEyeState(focus);
}

// `null` without a recorded point or for a manual-focus shot, whose recorded
// point is not trusted. The mark state rides along so the mark's color is read
// from the mark.
export function focusMark(
  focus: MarkFocus | null | undefined,
  drawWidth: number,
  drawHeight: number,
): FocusMark | null {
  if (focus === null || focus === undefined || focus.point === null || focus.manual_focus) {
    return null;
  }
  const { point } = focus;
  const x = -drawWidth / 2 + (point.x * drawWidth) / point.sensor_w;
  const y = -drawHeight / 2 + (point.y * drawHeight) / point.sensor_h;
  const state = afEyeState(focus);
  if (point.frame === null) {
    return { x, y, rect: null, state };
  }
  const width = (point.frame.width * drawWidth) / point.sensor_w;
  const height = (point.frame.height * drawHeight) / point.sensor_h;
  return {
    x,
    y,
    rect: { x: x - width / 2, y: y - height / 2, width, height },
    state,
  };
}

// One file the second scan pass has written, as `faces-progress` carries it.
export interface FaceReady extends StoredEyes {
  path: string;
  eye_focus: number | null;
  candidate: MarkFocus["candidate"];
  sharpness: number | null;
}

// Patch the focus of each ready file in `entries` in place, so the marks
// update without re-reading the whole folder. A file with no row or no focus
// (a JPEG file) is left alone; a RAW file with no AF point is patched. True
// when `current` was among the patched files.
export function applyFaceReady<T extends { focus: MarkFocus | null }>(
  entries: Map<string, T>,
  ready: FaceReady[],
  current: string | undefined,
): boolean {
  let touched = false;
  for (const item of ready) {
    const focus = entries.get(item.path)?.focus;
    if (focus === null || focus === undefined) {
      continue;
    }
    focus.eye_focus = item.eye_focus;
    focus.candidate = item.candidate;
    focus.eyes_ear = item.eyes_ear;
    focus.eyes = item.eyes;
    focus.eyes_closed = item.eyes_closed;
    focus.pose = item.pose;
    focus.eye_offset = item.eye_offset;
    focus.edge_gap = item.edge_gap;
    if (item.path === current) {
      touched = true;
    }
  }
  return touched;
}

// Patch the sharpness score of each ready file into `scores` (set, or deleted
// when the pass found none) and into its row in `entries`, if it has one.
export function applySharpnessReady<T extends { sharpness: number | null }>(
  entries: Map<string, T>,
  scores: Map<string, number>,
  ready: FaceReady[],
): void {
  for (const item of ready) {
    if (item.sharpness === null) {
      scores.delete(item.path);
    } else {
      scores.set(item.path, item.sharpness);
    }
    const entry = entries.get(item.path);
    if (entry !== undefined) {
      entry.sharpness = item.sharpness;
    }
  }
}

// What the `faces_of` command returns: the faces found on one file's preview,
// in the preview's stored (unrotated) pixel coordinates, each with YuNet's
// score and its five landmarks (each pair left of the image first).
export interface Faces {
  width: number;
  height: number;
  faces: {
    x: number;
    y: number;
    width: number;
    height: number;
    score: number;
    left_eye: { x: number; y: number };
    right_eye: { x: number; y: number };
    nose: { x: number; y: number };
    mouth_left: { x: number; y: number };
    mouth_right: { x: number; y: number };
  }[];
}

export interface FaceMark {
  rect: { x: number; y: number; width: number; height: number };
  // The eyes, the nose tip and the mouth corners.
  points: { x: number; y: number }[];
  label: string;
}

// The faces scaled onto the unrotated preview drawn `drawWidth` x
// `drawHeight` and centered on the origin, like `focusMark`, so the rotation
// the image got carries them too. The drawn bitmap may be a downscaled copy of
// the preview, so the scale comes from the preview size, not the bitmap's.
export function faceMarks(
  faces: Faces["faces"],
  previewWidth: number,
  previewHeight: number,
  drawWidth: number,
  drawHeight: number,
  orientation = 1,
): FaceMark[] {
  const sx = drawWidth / previewWidth;
  const sy = drawHeight / previewHeight;
  return faces.map((face) => ({
    rect: {
      x: face.x * sx - drawWidth / 2,
      y: face.y * sy - drawHeight / 2,
      width: face.width * sx,
      height: face.height * sy,
    },
    points: [face.left_eye, face.right_eye, face.nose, face.mouth_left, face.mouth_right].map(
      (p) => ({ x: p.x * sx - drawWidth / 2, y: p.y * sy - drawHeight / 2 }),
    ),
    label: faceLabel(face, orientation),
  }));
}

// The text beside a face box: YuNet's score, the box size and the distance
// between the eyes. The sizes are in the stored preview's pixels, not the
// drawn ones, because the scan's thresholds (`FACE_CONFIDENCE`, the face and
// eye region floors) are measured there, so the numbers compare to them. The
// label is drawn upright, so on a quarter turn (orientation 6 / 8) the stored
// width and height swap to read as the box looks on screen.
export function faceLabel(face: Faces["faces"][number], orientation = 1): string {
  const eyes = Math.hypot(face.right_eye.x - face.left_eye.x, face.right_eye.y - face.left_eye.y);
  const quarter = orientation === 6 || orientation === 8;
  const [width, height] = quarter ? [face.height, face.width] : [face.width, face.height];
  return `${face.score.toFixed(2)}  ${Math.round(width)}×${Math.round(height)}  eyes ${Math.round(eyes)}`;
}

// A point of the unrotated drawing centered on the origin, turned the way
// `draw()` rotates the image for `orientation`.
export function uprightPoint(
  { x, y }: { x: number; y: number },
  orientation: number,
): { x: number; y: number } {
  if (orientation === 6) {
    return { x: -y, y: x };
  }
  if (orientation === 8) {
    return { x: y, y: -x };
  }
  if (orientation === 3) {
    return { x: -x, y: -y };
  }
  return { x, y };
}

// The height a face label takes, its 11 px text plus the halo, in CSS pixels.
export const FACE_LABEL_HEIGHT = 14;

// The top-left of a face label `labelWidth` wide, drawn without rotation in
// the coordinates centered on the image: above the upright box's top-left
// corner, or just inside the box when the box touches the drawn image's top,
// and shifted left when it would run past the image's right edge.
export function faceLabelAt(
  rect: FaceMark["rect"],
  orientation: number,
  drawWidth: number,
  drawHeight: number,
  labelWidth: number,
): { x: number; y: number } {
  const a = uprightPoint(rect, orientation);
  const b = uprightPoint({ x: rect.x + rect.width, y: rect.y + rect.height }, orientation);
  const left = Math.min(a.x, b.x);
  const top = Math.min(a.y, b.y);
  const quarterTurn = orientation === 6 || orientation === 8;
  const imageWidth = quarterTurn ? drawHeight : drawWidth;
  const imageHeight = quarterTurn ? drawWidth : drawHeight;
  const x = Math.max(-imageWidth / 2, Math.min(left, imageWidth / 2 - labelWidth));
  const above = top - FACE_LABEL_HEIGHT;
  return { x, y: above < -imageHeight / 2 ? top + 2 : above };
}
