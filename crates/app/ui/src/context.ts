import { type Binding, displayKey } from "./keys.js";
import type { PickFlag } from "./selection.js";

// `checked` is undefined for a plain command, which has no checked state.
export type MenuItem = {
  action: string;
  label: string;
  shortcut: string;
  checked: boolean | undefined;
};

export type MenuState = { rating: number | null; flag: PickFlag; label: string | null };

type Entry = [action: string, label: string, checked?: (state: MenuState) => boolean];

const LABEL_ENTRIES: Entry[] = ["Red", "Orange", "Yellow", "Green", "Blue", "Pink", "Purple"].map(
  (name) => [name.toLowerCase(), name, (state) => state.label === name],
);

const JUDGMENT_SECTIONS: Entry[][] = [
  [
    ["pick", "Pick", (state) => state.flag === "pick"],
    ["reject", "Reject", (state) => state.flag === "reject"],
    ["unflag", "Unflag", (state) => state.flag === "none"],
  ],
  [
    ...[1, 2, 3, 4, 5].map((stars): Entry => [
      `rate${stars}`,
      stars === 1 ? "1 star" : `${stars} stars`,
      (state) => state.rating === stars,
    ]),
    ["clear", "No stars", (state) => !state.rating],
  ],
  [...LABEL_ENTRIES, ["clearlabel", "No label", (state) => state.label === null]],
  [["renameFile", "Rename…"]],
];

// `Select All`, then the reveal item, labeled per platform by the
// `reveal_label` command, and copying the targeted files' paths or names, then
// the judgment groups and `Rename…`, then `Move to Trash…`. A view-only (JPEG)
// folder takes no judgment and no rename, so those are left out. `Rename…`
// and the reveal item act on the focused file alone; the copy items and
// `Move to Trash…` on the `count` files they target, the latter naming them
// when there are several.
export function contextMenuGroups(
  bindings: Binding[],
  state: MenuState,
  revealLabel: string,
  viewOnly = false,
  count = 1,
): MenuItem[][] {
  const sections: Entry[][] = [
    [["selectAll", "Select All"]],
    [["revealFile", revealLabel]],
    [
      ["copyPath", "Copy Path"],
      ["copyFileName", "Copy File Name"],
    ],
    ...(viewOnly ? [] : JUDGMENT_SECTIONS),
    [["trashFiles", "Move to Trash…"]],
  ];
  return sections.map((section) =>
    section.map(([action, label, checked]) => {
      const key = bindings.find((binding) => binding.action === action)?.keys[0];
      return {
        action,
        label: action === "trashFiles" && count > 1 ? `Move ${count} Files to Trash…` : label,
        shortcut: key === undefined ? "" : displayKey(key),
        checked: checked?.(state),
      };
    }),
  );
}

// The folder tree's right-click menu: the reveal item, labeled per platform by
// the `reveal_label` command, then copying the folder's path or name, then re-listing it (Refresh), then
// renaming it (by hand or by its photos' capture date), then expanding or collapsing every subfolder under it, then
// rewriting its sidecars from the index or deleting them, then the sequencing of the folder's
// JPEGs, then importing the photos under it (offered on a root too, since a
// memory card is a volume root). The renames and expanding / collapsing all
// are not offered on a root (home or a volume).
export function folderMenuGroups(revealLabel: string, root: boolean): MenuItem[][] {
  return [
    [{ action: "revealFolder", label: revealLabel, shortcut: "", checked: undefined }],
    [
      { action: "copyPath", label: "Copy Path", shortcut: "", checked: undefined },
      { action: "copyFolderName", label: "Copy Folder Name", shortcut: "", checked: undefined },
    ],
    [{ action: "refreshFolder", label: "Refresh", shortcut: "", checked: undefined }],
    ...(root
      ? []
      : [
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
        ]),
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
  ];
}

export function menuPosition(
  x: number,
  y: number,
  width: number,
  height: number,
  viewportWidth: number,
  viewportHeight: number,
): { left: number; top: number } {
  return {
    left: Math.max(0, x + width > viewportWidth ? x - width : x),
    top: Math.max(0, y + height > viewportHeight ? y - height : y),
  };
}
