import { describe, expect, test } from "vitest";
import { IMPORT_PATTERN_EXAMPLES, IMPORT_SPECIFIERS, rootNote } from "./importsettings.js";

describe("IMPORT_SPECIFIERS", () => {
  test("lists the date specifiers and the folder separator", () => {
    expect(IMPORT_SPECIFIERS.map(({ spec }) => spec)).toEqual([
      "%Y",
      "%y",
      "%m",
      "%d",
      "%b",
      "%B",
      "%a",
      "%A",
      "%j",
      "%F",
      "/",
    ]);
  });
});

describe("IMPORT_PATTERN_EXAMPLES", () => {
  test("starts with the default pattern", () => {
    expect(IMPORT_PATTERN_EXAMPLES).toEqual(["%Y/%F", "%F", "%Y/%m/%d", "%Y/%m"]);
  });
});

describe("rootNote", () => {
  test("asks for a root while none is set", () => {
    expect(rootNote("")).toBe("Choose the folder imports are copied into.");
  });

  test("is nothing once a root is set", () => {
    expect(rootNote("/Volumes/Photos")).toBeNull();
  });
});
