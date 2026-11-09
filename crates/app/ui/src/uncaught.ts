export type UncaughtKind = "error" | "unhandledrejection";

/** One `Handpic.log` line for an uncaught error or unhandled rejection. */
export function uncaughtLine(
  kind: UncaughtKind,
  message: string,
  source?: string,
  line?: number,
  col?: number,
  stack?: string,
): string {
  const parts = [`kind=${kind}`, `message=${oneLine(message, " ")}`];
  if (source) parts.push(`source=${source}`);
  if (line !== undefined) parts.push(`line=${line}`);
  if (col !== undefined) parts.push(`col=${col}`);
  if (stack) parts.push(`stack=${oneLine(stack, " | ")}`);
  return parts.join(" ");
}

function oneLine(text: string, separator: string): string {
  return text
    .split(/\r?\n/)
    .map((part) => part.trim())
    .filter((part) => part !== "")
    .join(separator);
}

export const UNCAUGHT_LIMIT = 20;
export const SUPPRESSED_LINE = "... suppressed further uncaught errors for this session";

/**
 * Lets the first `UNCAUGHT_LIMIT` distinct lines of a session through, then a
 * single suppression notice, so an error thrown every frame cannot rotate the
 * rest of the session out of the log.
 */
export class UncaughtGate {
  private readonly seen = new Set<string>();
  private suppressed = false;

  constructor(private readonly limit = UNCAUGHT_LIMIT) {}

  /** The line to forward for `line`, or `null` to drop it. */
  pass(line: string): string | null {
    if (this.seen.has(line) || this.suppressed) return null;
    if (this.seen.size >= this.limit) {
      this.suppressed = true;
      return SUPPRESSED_LINE;
    }
    this.seen.add(line);
    return line;
  }
}
