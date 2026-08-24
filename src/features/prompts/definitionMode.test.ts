import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  preferredChatMode,
  resetPreferredChatModeForTests,
  setPreferredChatMode,
} from "./definitionMode";

const memory = new Map<string, string>();

beforeEach(() => {
  memory.clear();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => memory.get(key) ?? null,
    setItem: (key: string, value: string) => {
      memory.set(key, value);
    },
    removeItem: (key: string) => {
      memory.delete(key);
    },
  });
  resetPreferredChatModeForTests();
});

afterEach(() => {
  resetPreferredChatModeForTests();
  vi.unstubAllGlobals();
});

describe("preferredChatMode", () => {
  it("defaults to chat", () => {
    expect(preferredChatMode()).toBe(true);
  });

  it("remembers an explicit text choice", () => {
    setPreferredChatMode(false);
    expect(preferredChatMode()).toBe(false);
    expect(memory.get("prompthub.editor.preferChatMode")).toBe("0");
  });

  it("rehydrates chat preference from storage", () => {
    memory.set("prompthub.editor.preferChatMode", "0");
    expect(preferredChatMode()).toBe(false);
  });
});
