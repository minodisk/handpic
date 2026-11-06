import { describe, expect, test } from "vitest";
import { IdleGate, cancelsScan } from "./idle.js";

describe("IdleGate", () => {
  test("runs at once when idle", () => {
    const gate = new IdleGate(() => false);
    const ran: string[] = [];
    gate.request("a", () => ran.push("a"));
    expect(ran).toEqual(["a"]);
    expect(gate.waiting).toBeNull();
  });

  test("holds while busy and reports the label", () => {
    const gate = new IdleGate(() => true);
    const ran: string[] = [];
    gate.request("a", () => ran.push("a"));
    expect(ran).toEqual([]);
    expect(gate.waiting).toBe("a");
  });

  test("drain runs the held operation exactly once and clears it", () => {
    const gate = new IdleGate(() => true);
    const ran: string[] = [];
    gate.request("a", () => ran.push("a"));
    gate.drain();
    gate.drain();
    expect(ran).toEqual(["a"]);
    expect(gate.waiting).toBeNull();
  });

  test("a second request while busy replaces the first", () => {
    const gate = new IdleGate(() => true);
    const ran: string[] = [];
    gate.request("a", () => ran.push("a"));
    gate.request("b", () => ran.push("b"));
    expect(gate.waiting).toBe("b");
    gate.drain();
    expect(ran).toEqual(["b"]);
  });

  test("discard drops the held operation", () => {
    const gate = new IdleGate(() => true);
    const ran: string[] = [];
    gate.request("a", () => ran.push("a"));
    gate.discard();
    expect(gate.waiting).toBeNull();
    gate.drain();
    expect(ran).toEqual([]);
  });

  test("drain with nothing held is a no-op", () => {
    const gate = new IdleGate(() => false);
    gate.drain();
    expect(gate.waiting).toBeNull();
  });

  test("a held operation replaced by a later request fires its cancel once", () => {
    const gate = new IdleGate(() => true);
    const canceled: string[] = [];
    expect(
      gate.request(
        "a",
        () => {},
        () => canceled.push("a"),
      ),
    ).toBe(true);
    gate.request(
      "b",
      () => {},
      () => canceled.push("b"),
    );
    expect(canceled).toEqual(["a"]);
    gate.drain();
    expect(canceled).toEqual(["a"]);
  });

  test("discard fires the held operation's cancel", () => {
    const gate = new IdleGate(() => true);
    const canceled: string[] = [];
    gate.request(
      "a",
      () => {},
      () => canceled.push("a"),
    );
    gate.discard();
    gate.discard();
    expect(canceled).toEqual(["a"]);
  });

  test("drain runs without firing cancel", () => {
    const gate = new IdleGate(() => true);
    const events: string[] = [];
    gate.request(
      "a",
      () => events.push("run"),
      () => events.push("cancel"),
    );
    gate.drain();
    gate.discard();
    expect(events).toEqual(["run"]);
  });

  test("a request that runs at once neither holds nor fires its cancel", () => {
    const gate = new IdleGate(() => false);
    const events: string[] = [];
    const held = gate.request(
      "a",
      () => events.push("run"),
      () => events.push("cancel"),
    );
    gate.discard();
    expect(held).toBe(false);
    expect(gate.waiting).toBeNull();
    expect(events).toEqual(["run"]);
  });
});

describe("cancelsScan", () => {
  test("is true for Move to Trash", () => {
    expect(cancelsScan("Move to Trash")).toBe(true);
  });

  test("is false for the operations that wait out the scan, and for none", () => {
    expect(cancelsScan("Rename…")).toBe(false);
    expect(cancelsScan("Undo Move to Trash")).toBe(false);
    expect(cancelsScan("Delete Sidecars")).toBe(false);
    expect(cancelsScan(null)).toBe(false);
  });
});
