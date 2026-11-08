import type { EyeState, Eyes, Pose } from "./eyes.js";
import { eyesOpenness, GOOD_FULL_EYE_SHARP } from "./focus.js";

// Mirrors `Metadata` in `crates/app/src/commands.rs`: already formatted for
// display, so a field is either a string to show or null to leave out.
export interface Metadata {
  name: string;
  camera: string | null;
  lens: string | null;
  aperture: string | null;
  shutter: string | null;
  shutter_type: string | null;
  focus_mode: string | null;
  af_area: string | null;
  af_tracking: string | null;
  drive: string | null;
  stabilization: string | null;
  exposure_mode: string | null;
  metering: string | null;
  creative_style: string | null;
  dro: string | null;
  raw_type: string | null;
  iso: string | null;
  focal_length: string | null;
  exposure_bias: string | null;
  focus_distance: string | null;
  captured_at: string | null;
}

export interface MetaRow {
  label: string;
  value: string;
}

export interface MetaGroup {
  heading: string;
  rows: MetaRow[];
}

export const EXIF_HEADING = "EXIF";
export const MAKER_NOTE_HEADING = "Maker note";
export const ANALYSIS_HEADING = "Analysis";

export type FocusCandidate = "candidate" | "not_candidate" | "unknown";

// The second scan pass's result for one file, as `Focus` in `main.ts` carries
// it. `eyes_ear`, `eyes` and `pose` are what it stored for the judged face
// (nearest the AF point, else the largest on the whole preview), `null` /
// `unknown` when it ran no face mesh there. `full_eye_sharp` is the sharpness
// of its eyes on the full-size image, `null` when there is none.
export interface FocusCue {
  candidate: FocusCandidate;
  eye_focus: number | null;
  eyes_ear: number | null;
  eyes: EyeState;
  pose: Pose | null;
  full_eye_sharp: number | null;
}

// One file's place in its burst under the personal ranking, as `Ranked` in
// `crates/core/src/rank.rs` carries it: the score above the burst's mean in
// logits, the place among the burst's scored frames, what moved the
// score, `[reason, logits]`, largest first, and the genre the burst was
// ranked as, `null` when it has none.
export interface Ranked {
  score: number;
  place: number;
  of: number;
  reasons: [string, number][];
  genre: string | null;
}

// How much of the ranking is the user's own, as `ModelInfo` in
// `crates/app/src/rank.rs` carries it: the bursts and the folders it was fit
// on, `none` when it is the built-in prior alone.
export interface RankModel {
  bursts: number;
  folders: number;
  confidence: "none" | "low" | "medium" | "high";
}

export interface Ranking {
  ranked: Ranked;
  model: RankModel;
}

// A reason smaller than this, in logits, is not worth naming.
const MIN_REASON = 0.05;
const REASONS_SHOWN = 3;

function signed(x: number): string {
  return `${x < 0 ? "−" : "+"}${Math.abs(x).toFixed(2)}`;
}

function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

// The confidence with what the model was fit on, for the meta pane and the
// settings.
export function confidenceText(model: RankModel): string {
  if (model.confidence === "none") {
    return "Built-in only, nothing learned yet";
  }
  const level = model.confidence.charAt(0).toUpperCase() + model.confidence.slice(1);
  return `${level} · ${plural(model.bursts, "burst")} in ${plural(model.folders, "folder")}`;
}

// The ranking's rows: the place in the burst with the score and the genre
// it was ranked as, the largest reasons, and the confidence.
export function rankingRows(ranking: Ranking): [string, string | null][] {
  const { ranked, model } = ranking;
  const reasons = ranked.reasons
    .filter(([, v]) => Math.abs(v) >= MIN_REASON)
    .slice(0, REASONS_SHOWN)
    .map(([name, v]) => `${name} ${signed(v)}`);
  const genre = ranked.genre === null ? "" : ` · as ${ranked.genre}`;
  return [
    [
      "Recommended",
      `${ranked.place} of ${ranked.of} in its burst · ${signed(ranked.score)}${genre}`,
    ],
    ["Reason", reasons.length === 0 ? null : reasons.join(", ")],
    ["Confidence", confidenceText(model)],
  ];
}

function focusPercent(p: number | null): string | null {
  return p === null ? null : `${Math.round(p * 100)}%`;
}

// The open / closed judgment with the openness of the eyes, `Open · 82`.
function eyesValue(state: EyeState, ear: number | null): string | null {
  const openness = eyesOpenness(ear);
  if (openness === null || state === "unknown") {
    return null;
  }
  return `${state === "closed" ? "Closed" : "Open"} · ${Math.round(openness)}`;
}

// Whole signed degrees, a yaw past 90 shown as is (a far profile
// overshooting); a rounded `-0` prints as `0`.
function degrees(angle: number): string {
  return `${Math.round(angle)}°`;
}

// The full-size eye score with its verdict against the Good cut, `Soft · 0.143`.
function fullSharpValue(score: number | null): string | null {
  if (score === null) {
    return null;
  }
  return `${score >= GOOD_FULL_EYE_SHARP ? "Sharp" : "Soft"} · ${score.toFixed(3)}`;
}

function group(heading: string, rows: [string, string | null][]): MetaGroup {
  return {
    heading,
    rows: rows.flatMap(([label, value]) => (value === null ? [] : [{ label, value }])),
  };
}

// The meta pane's rows grouped by where each value comes from: standard
// EXIF/TIFF tags, the vendor MakerNote (itself an EXIF tag), and Riffle's own
// analysis. The analysis group needs no `meta`, so a file whose metadata could
// not be read still shows its analysis. The AF eye's in-focus probability gets a
// row, as a percentage, only when there is one, and so do the open / closed
// judgment with the openness of the eyes and the head pose of the same face: the
// values the scan stored for the judged face when it has them, else the `eyes`
// judgment taken when the file was shown. The full-size eye score gets a row,
// with its verdict against the Good cut, only when there is one. The personal
// ranking's rows follow while AI analysis is on and the file has a score.
export function metaGroups(
  meta: Metadata | null,
  focus?: FocusCue | null,
  eyes?: Eyes | null,
  ranking?: Ranking | null,
): MetaGroup[] {
  const groups: MetaGroup[] = [];
  const ear = focus?.eyes_ear ?? null;
  const stored = ear !== null;
  const pose = stored ? (focus?.pose ?? null) : (eyes?.pose ?? null);
  if (meta !== null) {
    groups.push(
      group(EXIF_HEADING, [
        ["Aperture", meta.aperture],
        ["Shutter", meta.shutter],
        ["ISO", meta.iso],
        ["Focal length", meta.focal_length],
        ["Exposure", meta.exposure_bias],
        ["Camera", meta.camera],
        ["Lens", meta.lens],
        ["Captured", meta.captured_at],
      ]),
      group(MAKER_NOTE_HEADING, [
        ["Shutter type", meta.shutter_type],
        ["Focus mode", meta.focus_mode],
        ["AF area", meta.af_area],
        ["AF tracking", meta.af_tracking],
        ["Drive", meta.drive],
        ["Stabilization", meta.stabilization],
        ["Exposure mode", meta.exposure_mode],
        ["Metering", meta.metering],
        ["Creative style", meta.creative_style],
        ["DRO", meta.dro],
        ["RAW type", meta.raw_type],
        ["Focus distance", meta.focus_distance],
      ]),
    );
  }
  groups.push(
    group(ANALYSIS_HEADING, [
      ["AF eye in focus", focusPercent(focus?.eye_focus ?? null)],
      ["Eyes at full size", fullSharpValue(focus?.full_eye_sharp ?? null)],
      [
        "Eyes",
        stored
          ? eyesValue(focus?.eyes ?? "unknown", ear)
          : eyesValue(eyes?.state ?? "unknown", eyes?.ear ?? null),
      ],
      ["Yaw", pose === null ? null : degrees(pose.yaw)],
      ["Pitch", pose === null ? null : degrees(pose.pitch)],
      ["Roll", pose === null ? null : degrees(pose.roll)],
      ...(ranking === null || ranking === undefined ? [] : rankingRows(ranking)),
    ]),
  );
  return groups.filter(({ rows }) => rows.length > 0);
}
