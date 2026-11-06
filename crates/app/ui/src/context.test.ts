import { describe, expect, test } from "vitest";
import { contextMenuGroups, folderMenuGroups, menuPosition } from "./context.js";

const defaults = [
  { action: "selectAll", keys: ["meta+a"] },
  { action: "pick", keys: ["p"] },
  { action: "reject", keys: ["x"] },
  { action: "unflag", keys: ["u"] },
  { action: "rate1", keys: ["1"] },
  { action: "rate2", keys: ["2"] },
  { action: "rate3", keys: ["3"] },
  { action: "rate4", keys: ["4"] },
  { action: "rate5", keys: ["5"] },
  { action: "clear", keys: ["0"] },
  { action: "red", keys: ["ctrl+alt+1"] },
  { action: "orange", keys: ["ctrl+alt+2"] },
  { action: "yellow", keys: ["ctrl+alt+3"] },
  { action: "green", keys: ["ctrl+alt+4"] },
  { action: "blue", keys: ["ctrl+alt+5"] },
  { action: "pink", keys: ["ctrl+alt+6"] },
  { action: "purple", keys: ["ctrl+alt+7"] },
  { action: "clearlabel", keys: ["ctrl+alt+0"] },
];

const unset = { rating: null, flag: "none" as const, label: null };
const reveal = "Reveal in File Explorer";

describe("contextMenuGroups", () => {
  test("greys out Move to Trash… alone while a move is in flight", () => {
    const disabled = (trashing: boolean): string[] =>
      contextMenuGroups(defaults, unset, reveal, false, 1, trashing)
        .flat()
        .filter((item) => item.disabled === true)
        .map((item) => item.action);
    expect(disabled(true)).toEqual(["trashFiles"]);
    expect(disabled(false)).toEqual([]);
  });

  test("lists Select All, the reveal and copy items, the flag, rating and label groups, Rename…, then Move to Trash…", () => {
    const groups = contextMenuGroups(defaults, unset, reveal);
    expect(groups.map((group) => group.map(({ action, label }) => [action, label]))).toEqual([
      [["selectAll", "Select All"]],
      [["revealFile", "Reveal in File Explorer"]],
      [
        ["copyPath", "Copy Path"],
        ["copyFileName", "Copy File Name"],
      ],
      [
        ["pick", "Pick"],
        ["reject", "Reject"],
        ["unflag", "Unflag"],
      ],
      [
        ["rate1", "1 star"],
        ["rate2", "2 stars"],
        ["rate3", "3 stars"],
        ["rate4", "4 stars"],
        ["rate5", "5 stars"],
        ["clear", "No stars"],
      ],
      [
        ["red", "Red"],
        ["orange", "Orange"],
        ["yellow", "Yellow"],
        ["green", "Green"],
        ["blue", "Blue"],
        ["pink", "Pink"],
        ["purple", "Purple"],
        ["clearlabel", "No label"],
      ],
      [["renameFile", "Rename…"]],
      [["trashFiles", "Move to Trash…"]],
    ]);
  });

  test("shows the default keys", () => {
    const shortcuts = contextMenuGroups(defaults, unset, reveal).map((group) =>
      group.map((item) => item.shortcut),
    );
    expect(shortcuts).toEqual([
      ["meta+a"],
      [""],
      ["", ""],
      ["p", "x", "u"],
      ["1", "2", "3", "4", "5", "0"],
      [
        "ctrl+alt+1",
        "ctrl+alt+2",
        "ctrl+alt+3",
        "ctrl+alt+4",
        "ctrl+alt+5",
        "ctrl+alt+6",
        "ctrl+alt+7",
        "ctrl+alt+0",
      ],
      [""],
      [""],
    ]);
  });

  test("shows an overridden key", () => {
    const bindings = defaults.map((binding) =>
      binding.action === "rate3" ? { action: "rate3", keys: ["space"] } : binding,
    );
    expect(contextMenuGroups(bindings, unset, reveal)[4]?.[2]?.shortcut).toBe("Space");
  });

  test("shows an overridden Select All key", () => {
    const bindings = defaults.map((binding) =>
      binding.action === "selectAll" ? { action: "selectAll", keys: ["ctrl+alt+a"] } : binding,
    );
    expect(contextMenuGroups(bindings, unset, reveal)[0]?.[0]?.shortcut).toBe("ctrl+alt+a");
  });

  test("gives Select All no checked state", () => {
    expect(contextMenuGroups(defaults, unset, reveal)[0]?.[0]?.checked).toBeUndefined();
  });

  test("shows the first key when an action has several", () => {
    const bindings = [{ action: "reject", keys: ["d", "x"] }];
    expect(contextMenuGroups(bindings, unset, reveal)[3]?.[1]?.shortcut).toBe("d");
  });

  test("gives an empty shortcut for an unbound action", () => {
    const shortcuts = contextMenuGroups([], unset, reveal).flatMap((group) =>
      group.map((item) => item.shortcut),
    );
    expect(shortcuts).toHaveLength(23);
    expect(shortcuts.every((shortcut) => shortcut === "")).toBe(true);
  });

  test("marks the unset items when nothing is set", () => {
    const checked = contextMenuGroups(defaults, unset, reveal)
      .flat()
      .filter((item) => item.checked)
      .map((item) => item.action);
    expect(checked).toEqual(["unflag", "clear", "clearlabel"]);
  });

  test("marks the current flag, rating and label", () => {
    const checked = contextMenuGroups(
      defaults,
      { rating: 3, flag: "pick", label: "Orange" },
      reveal,
    )
      .flat()
      .filter((item) => item.checked)
      .map((item) => item.action);
    expect(checked).toEqual(["pick", "rate3", "orange"]);
  });

  test("keeps only Select All, the reveal and copy items and Move to Trash… in a view-only folder", () => {
    expect(contextMenuGroups(defaults, unset, reveal, true)).toEqual([
      [{ action: "selectAll", label: "Select All", shortcut: "meta+a", checked: undefined }],
      [{ action: "revealFile", label: reveal, shortcut: "", checked: undefined }],
      [
        { action: "copyPath", label: "Copy Path", shortcut: "", checked: undefined },
        { action: "copyFileName", label: "Copy File Name", shortcut: "", checked: undefined },
      ],
      [{ action: "trashFiles", label: "Move to Trash…", shortcut: "", checked: undefined }],
    ]);
  });

  test("names the count of a multi-selection in Move to Trash…", () => {
    expect(contextMenuGroups(defaults, unset, reveal, false, 3).at(-1)).toEqual([
      { action: "trashFiles", label: "Move 3 Files to Trash…", shortcut: "", checked: undefined },
    ]);
    expect(contextMenuGroups(defaults, unset, reveal, true, 2).at(-1)?.[0]?.label).toBe(
      "Move 2 Files to Trash…",
    );
  });

  test("gives the reveal and copy items no shortcut and no checked state", () => {
    expect(contextMenuGroups(defaults, unset, reveal).slice(1, 3)).toEqual([
      [{ action: "revealFile", label: reveal, shortcut: "", checked: undefined }],
      [
        { action: "copyPath", label: "Copy Path", shortcut: "", checked: undefined },
        { action: "copyFileName", label: "Copy File Name", shortcut: "", checked: undefined },
      ],
    ]);
  });

  test("keeps the reveal and copy labels for a multi-selection", () => {
    const groups = contextMenuGroups(defaults, unset, reveal, false, 3);
    expect(groups.slice(1, 3).map((group) => group.map((item) => item.label))).toEqual([
      [reveal],
      ["Copy Path", "Copy File Name"],
    ]);
    expect(groups.at(-1)?.[0]?.label).toBe("Move 3 Files to Trash…");
  });

  test("keeps Move to Trash… singular for one file", () => {
    expect(contextMenuGroups(defaults, unset, reveal, false, 1).at(-1)?.[0]?.label).toBe(
      "Move to Trash…",
    );
  });
});

describe("folderMenuGroups", () => {
  test("holds the reveal item, then the copy items, then the refresh item, then the rename items, then the expand / collapse items, then the rewrite sidecars item, then the sequence item, then the import item, without a shortcut or checked state", () => {
    expect(folderMenuGroups("Reveal in Finder", false)).toEqual([
      [{ action: "revealFolder", label: "Reveal in Finder", shortcut: "", checked: undefined }],
      [
        { action: "copyPath", label: "Copy Path", shortcut: "", checked: undefined },
        { action: "copyFolderName", label: "Copy Folder Name", shortcut: "", checked: undefined },
      ],
      [{ action: "refreshFolder", label: "Refresh", shortcut: "", checked: undefined }],
      [
        { action: "renameFolder", label: "Rename…", shortcut: "", checked: undefined },
        {
          action: "renameByDate",
          label: "Rename by Capture Date…",
          shortcut: "",
          checked: undefined,
        },
      ],
      [
        { action: "expandAll", label: "Expand All", shortcut: "", checked: undefined },
        { action: "collapseAll", label: "Collapse All", shortcut: "", checked: undefined },
      ],
      [
        {
          action: "rewriteSidecars",
          label: "Rewrite Sidecars from Index…",
          shortcut: "",
          checked: undefined,
        },
        { action: "deleteSidecars", label: "Delete Sidecars…", shortcut: "", checked: undefined },
      ],
      [
        {
          action: "sequenceTimestamps",
          label: "Sequence JPEG Timestamps…",
          shortcut: "",
          checked: undefined,
        },
      ],
      [{ action: "importFolder", label: "Import…", shortcut: "", checked: undefined }],
    ]);
  });

  test("leaves the rename items and the expand / collapse items out for a root, keeping Import…", () => {
    expect(
      folderMenuGroups("Reveal in Finder", true).map((group) => group.map(({ action }) => action)),
    ).toEqual([
      ["revealFolder"],
      ["copyPath", "copyFolderName"],
      ["refreshFolder"],
      ["rewriteSidecars", "deleteSidecars"],
      ["sequenceTimestamps"],
      ["importFolder"],
    ]);
  });
});

describe("menuPosition", () => {
  test("leaves an interior point alone", () => {
    expect(menuPosition(100, 100, 50, 40, 800, 600)).toEqual({ left: 100, top: 100 });
  });

  test("flips left near the right edge", () => {
    expect(menuPosition(780, 100, 50, 40, 800, 600)).toEqual({ left: 730, top: 100 });
  });

  test("flips up near the bottom edge", () => {
    expect(menuPosition(100, 590, 50, 40, 800, 600)).toEqual({ left: 100, top: 550 });
  });

  test("shifts to the edge when flipping would leave the viewport", () => {
    expect(menuPosition(30, 20, 50, 40, 60, 50)).toEqual({ left: 0, top: 0 });
  });
});
