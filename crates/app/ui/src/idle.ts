// Holds an operation pressed while `busy` (a scan runs) until `drain` is
// called, instead of refusing it. One slot: a later press replaces the held
// one, firing its `cancel` (as `discard` does; `drain` only runs it).
// `enterInFlight`/`leaveInFlight` are called by the caller from
// dispatch until the operation settles, so a rescan can queue behind it. A
// count rather than a flag, since two deferred operations can overlap (for
// example a strip rename confirmed while a folder rename's reopen chain is
// still pending): the first to settle must not clear the flag out from
// under the other.
export class IdleGate {
  private inFlightCount = 0;
  private readonly busy: () => boolean;
  private held: { label: string; run: () => void; cancel?: () => void } | null = null;

  constructor(busy: () => boolean) {
    this.busy = busy;
  }

  get inFlight(): boolean {
    return this.inFlightCount > 0;
  }

  enterInFlight(): void {
    this.inFlightCount += 1;
  }

  leaveInFlight(): void {
    this.inFlightCount -= 1;
  }

  // Whether `run` was held rather than run at once.
  request(label: string, run: () => void, cancel?: () => void): boolean {
    this.discard();
    if (this.busy()) {
      this.held = { label, run, cancel };
      return true;
    }
    run();
    return false;
  }

  drain(): void {
    const held = this.held;
    this.held = null;
    held?.run();
  }

  discard(): void {
    const held = this.held;
    this.held = null;
    held?.cancel?.();
  }

  get waiting(): string | null {
    return this.held?.label ?? null;
  }
}

// Whether the operation held under `label` stops the running scan rather than
// waiting it out: its files are about to go, so scanning them is wasted.
export function cancelsScan(label: string | null): boolean {
  return label === "Move to Trash" || label === "Delete Permanently";
}
