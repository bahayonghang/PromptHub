import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  buildPromptCopyText,
  deriveTextFieldsFromMessages,
  extractVariableNames,
  seedChatMessages,
  substituteVariables,
  syncVariables,
} from "./promptText";
import type { PromptMessage, Variable } from "./types";

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

describe("buildPromptCopyText", () => {
  const vars = (defaults: Record<string, string>): Variable[] =>
    Object.entries(defaults).map(([name, defaultValue]) => ({
      name,
      type: "text" as const,
      required: true,
      defaultValue,
    }));

  it("returns the user prompt when the system prompt is empty", () => {
    expect(
      buildPromptCopyText({
        systemPrompt: "",
        userPrompt: "Ask one question.",
        messages: [],
        variables: [],
      }),
    ).toBe("Ask one question.");
  });

  it("treats a whitespace-only system prompt as absent", () => {
    expect(
      buildPromptCopyText({
        systemPrompt: "  \n",
        userPrompt: "Ask one question.",
        messages: [],
        variables: [],
      }),
    ).toBe("Ask one question.");
  });

  it("joins system and user blocks when a system prompt exists", () => {
    expect(
      buildPromptCopyText({
        systemPrompt: "Be terse.",
        userPrompt: "Summarize this.",
        messages: [],
        variables: [],
      }),
    ).toBe("[System]\nBe terse.\n\n[User]\nSummarize this.");
  });

  it("substitutes declared defaults and leaves unmatched placeholders", () => {
    expect(
      buildPromptCopyText({
        systemPrompt: "Role: {{role}}",
        userPrompt: "Task: {{task}} in {{lang}}",
        messages: [],
        variables: vars({ role: "editor", lang: "zh" }),
      }),
    ).toBe("[System]\nRole: editor\n\n[User]\nTask: {{task}} in zh");
  });

  it("formats chat messages as labeled blocks", () => {
    const messages: PromptMessage[] = [
      { role: "system", content: "Stay curious." },
      { role: "user", content: "What is {{topic}}?" },
      { role: "assistant", content: "Ask a better question." },
    ];
    expect(
      buildPromptCopyText({
        systemPrompt: "",
        userPrompt: "",
        messages,
        variables: vars({ topic: "memory" }),
      }),
    ).toBe(
      "[System]\nStay curious.\n\n[User]\nWhat is memory?\n\n[Assistant]\nAsk a better question.",
    );
  });
});

describe("seedChatMessages", () => {
  it("always includes a user message and prepends a non-empty system prompt", () => {
    expect(seedChatMessages("Stay terse.", "Ask one question.")).toEqual([
      { role: "system", content: "Stay terse." },
      { role: "user", content: "Ask one question." },
    ]);
    expect(seedChatMessages("  ", "")).toEqual([{ role: "user", content: "" }]);
  });
});

describe("deriveTextFieldsFromMessages", () => {
  it("uses the first system message and the last user message", () => {
    expect(
      deriveTextFieldsFromMessages([
        { role: "system", content: "First system" },
        { role: "user", content: "First user" },
        { role: "assistant", content: "Reply" },
        { role: "user", content: "Last user" },
      ]),
    ).toEqual({
      systemPrompt: "First system",
      userPrompt: "Last user",
    });
  });

  it("returns empty strings when those roles are absent", () => {
    expect(deriveTextFieldsFromMessages([])).toEqual({
      systemPrompt: "",
      userPrompt: "",
    });
  });
});
