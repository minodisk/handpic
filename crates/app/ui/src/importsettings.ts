// The Import tab's text: the specifiers the folder pattern allows, the
// patterns offered as examples and the note while no root is set. The
// pattern itself is validated by the backend (`import_pattern_example`).

// The payload of the `import_settings` command.
export interface ImportSettings {
  root: string;
  pattern: string;
}

export interface Specifier {
  spec: string;
  meaning: string;
  example: string;
}

// Each example is for 24 October 2026.
export const IMPORT_SPECIFIERS: Specifier[] = [
  { spec: "%Y", meaning: "Year", example: "2026" },
  { spec: "%y", meaning: "Year, two digits", example: "26" },
  { spec: "%m", meaning: "Month, two digits", example: "10" },
  { spec: "%d", meaning: "Day, two digits", example: "24" },
  { spec: "%b", meaning: "Month, short name", example: "Oct" },
  { spec: "%B", meaning: "Month, full name", example: "October" },
  { spec: "%a", meaning: "Weekday, short name", example: "Sat" },
  { spec: "%A", meaning: "Weekday, full name", example: "Saturday" },
  { spec: "%j", meaning: "Day of the year", example: "297" },
  { spec: "%F", meaning: "Same as %Y-%m-%d", example: "2026-10-24" },
  { spec: "/", meaning: "Folder separator", example: "" },
];

export const IMPORT_PATTERN_EXAMPLES = ["%Y/%F", "%F", "%Y/%m/%d", "%Y/%m"];

// Shown under the root field while no destination root is set.
export function rootNote(root: string): string | null {
  return root === "" ? "Choose the folder imports are copied into." : null;
}
