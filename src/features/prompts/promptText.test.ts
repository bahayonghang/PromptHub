import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  extractVariableNames,
  substituteVariables,
  syncVariables,
} from "./promptText";
import type { Variable } from "./types";

describe("extractVariableNames (Req 6.7)", () => {
  it("returns distinct names in first-seen order", () => {
    expect(
      extractVariableNames("Hello {{name}}, in {{lang}}. Again {{name}}."),
    ).toEqual(["name", "lang"]);
  });

  it("ignores the example segment after a colon", () => {
    expect(extractVariableNames("{{topic:Computer Science}}")).toEqual(["topic"]);
  });

  it("tolerates surrounding whitespace inside braces", () => {
    expect(extractVariableNames("{{  spaced  }}")).toEqual(["spaced"]);
  });

  it("returns an empty array when there are no placeholders", () => {
    expect(extractVariableNames("no variables here")).toEqual([]);
  });
});

describe("substituteVariables (Req 6.11)", () => {
  it("replaces matched names and leaves unmatched placeholders intact", () => {
    expect(
      substituteVariables("Hi {{name}}, learn {{lang}}", { name: "Sam" }),
    ).toBe("Hi Sam, learn {{lang}}");
  });

  it("substitutes a placeholder that carries an example segment", () => {
    expect(substituteVariables("{{topic:Math}}", { topic: "Physics" })).toBe(
      "Physics",
    );
  });

  it("leaves every matched placeholder substituted (property)", () => {
    const name = fc
      .array(fc.constantFrom(..."abcdefghijklmnop".split("")), {
        minLength: 1,
        maxLength: 6,
      })
      .map((c) => c.join(""));
    fc.assert(
      fc.property(name, fc.string(), (varName, value) => {
        const out = substituteVariables(`<{{${varName}}}>`, { [varName]: value });
        expect(out).toBe(`<${value}>`);
      }),
    );
  });
});

describe("syncVariables (Req 6.7)", () => {
  it("adds newly referenced names as required text variables", () => {
    const result = syncVariables([], "Use {{a}} and {{b}}");
    expect(result).toEqual<Variable[]>([
      { name: "a", type: "text", required: true },
      { name: "b", type: "text", required: true },
    ]);
  });

  it("preserves metadata of variables that remain referenced", () => {
    const existing: Variable[] = [
      { name: "a", type: "select", label: "A", required: false, options: ["x"] },
    ];
    const result = syncVariables(existing, "{{a}} {{b}}");
    expect(result[0]).toEqual(existing[0]);
    expect(result[1]).toEqual({ name: "b", type: "text", required: true });
  });

  it("drops variables no longer referenced in the text", () => {
    const existing: Variable[] = [
      { name: "gone", type: "text", required: true },
    ];
    expect(syncVariables(existing, "no placeholders")).toEqual([]);
  });
});
