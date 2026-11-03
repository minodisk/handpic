const ADVANCING = new Set([
  "rate1",
  "rate2",
  "rate3",
  "rate4",
  "rate5",
  "reject",
  "pick",
  "unflag",
]);

// Whether a judgment action moves on to the next file when Auto-advance is
// on. Clearing the flag is a judgment too; color labels are toggles and the
// star and label clears are corrections, so they stay on the file.
export function advancesAfter(action: string): boolean {
  return ADVANCING.has(action);
}

export interface AdvanceInput {
  action: string;
  targets: number;
  autoAdvance: boolean;
  comparing: boolean;
  stillCurrent: boolean;
}

// Whether a judgment key moves on to the next file: one file addressed,
// changed or already holding the value, outside Compare, and still the
// current file (a file the judgment filtered out already moved the cursor).
export function shouldAdvance({
  action,
  targets,
  autoAdvance,
  comparing,
  stillCurrent,
}: AdvanceInput): boolean {
  return targets === 1 && autoAdvance && !comparing && advancesAfter(action) && stillCurrent;
}
