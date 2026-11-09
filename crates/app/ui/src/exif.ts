// Mirrors `Exif` in `crates/backend/src/exif.rs`: `value` sorts, `label` shows.
export interface Labeled {
  value: number;
  label: string;
}

export interface Exif {
  camera: string | null;
  lens: string | null;
  aperture: Labeled | null;
  shutter: Labeled | null;
  iso: Labeled | null;
  focal_length: Labeled | null;
}

export type ExifGroup = "camera" | "lens" | "aperture" | "shutter" | "iso" | "focal";

// Half-open `[lower, upper)`, so a frame falls in exactly one.
export const focalRanges: { upper: number; label: string }[] = [
  { upper: 24, label: "<24 mm" },
  { upper: 35, label: "24–35 mm" },
  { upper: 50, label: "35–50 mm" },
  { upper: 85, label: "50–85 mm" },
  { upper: 135, label: "85–135 mm" },
  { upper: 200, label: "135–200 mm" },
  { upper: Infinity, label: ">200 mm" },
];

export function focalRange(mm: number): number {
  return focalRanges.findIndex(({ upper }) => mm < upper);
}

// The label and sort key of `group` for one file, or null if it has none.
export function exifKey(
  exif: Exif | null | undefined,
  group: ExifGroup,
): { label: string; order: number | string } | null {
  if (!exif) {
    return null;
  }
  switch (group) {
    case "camera":
    case "lens": {
      const text = exif[group];
      return text === null ? null : { label: text, order: text };
    }
    case "focal": {
      if (exif.focal_length === null) {
        return null;
      }
      const at = focalRange(exif.focal_length.value);
      return { label: focalRanges[at].label, order: at };
    }
    default: {
      const labeled = exif[group];
      return labeled === null ? null : { label: labeled.label, order: labeled.value };
    }
  }
}
