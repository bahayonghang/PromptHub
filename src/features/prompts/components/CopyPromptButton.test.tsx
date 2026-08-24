// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { buildPromptCopyText, type PromptCopySource } from "../promptText";
import type { PromptCopyResult } from "../types";
import { CopyPromptButton } from "./CopyPromptButton";

const source: PromptCopySource = {
  systemPrompt: "Be terse.",
  userPrompt: "Summarize this.",
  messages: [],
  variables: [],
};

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("CopyPromptButton", () => {
  it("writes labeled text and shows a copied confirmation", async () => {
    vi.useFakeTimers();
    const writeText = vi.fn().mockResolvedValue(undefined);
    render(
      <CopyPromptButton source={source} name="Steelman" writeText={writeText} />,
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });

    expect(writeText).toHaveBeenCalledWith(
      "[System]\nBe terse.\n\n[User]\nSummarize this.",
    );
    expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy();

    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(screen.getByRole("button", { name: "Copy Steelman" })).toBeTruthy();
  });

  it("does not write when the prompt is locked", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    render(
      <CopyPromptButton
        source={source}
        name="Private"
        locked
        writeText={writeText}
      />,
    );

    const button = screen.getByRole("button", {
      name: "Unlock the prompt library to copy private content",
    });
    expect(button).toHaveProperty("disabled", true);
    fireEvent.click(button);
    expect(writeText).not.toHaveBeenCalled();
  });

  it("announces a failure on the same control", async () => {
    const writeText = vi.fn().mockRejectedValue(new Error("denied"));
    render(
      <CopyPromptButton source={source} name="Steelman" writeText={writeText} />,
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });

    expect(
      screen.getByRole("button", { name: "Could not copy prompt" }),
    ).toBeTruthy();
  });

  it("copies a prompt with no references byte for byte via prompt.copy", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const noRef: PromptCopySource = {
      systemPrompt: "Be terse.",
      userPrompt: "Hello {{name}}",
      messages: [],
      variables: [
        { name: "name", type: "text", required: false, defaultValue: "Ada" },
      ],
    };
    const expected = buildPromptCopyText(noRef);
    const copyPrompt = vi.fn(async (): Promise<PromptCopyResult> => ({
      systemPrompt: "Be terse.",
      userPrompt: "Hello Ada",
      messages: [],
      unexpanded: [],
    }));
    render(
      <CopyPromptButton
        source={noRef}
        promptId="p1"
        copyPrompt={copyPrompt}
        name="Steelman"
        writeText={writeText}
      />,
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });
    expect(copyPrompt).toHaveBeenCalledWith("p1", { name: "Ada" });
    expect(writeText).toHaveBeenCalledWith(expected);
    expect(expected).toBe("[System]\nBe terse.\n\n[User]\nHello Ada");
  });

  it("inlines @@Title from prompt.copy in text mode and chat mode", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const copyPrompt = vi.fn(async (): Promise<PromptCopyResult> => ({
      systemPrompt: null,
      userPrompt: "Hello inlined body",
      messages: [],
      unexpanded: [],
    }));
    render(
      <CopyPromptButton
        source={{
          systemPrompt: null,
          userPrompt: "Hello @@Title",
          messages: [],
          variables: [],
        }}
        promptId="p1"
        copyPrompt={copyPrompt}
        name="Source"
        writeText={writeText}
      />,
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Source" }));
    });
    expect(copyPrompt).toHaveBeenCalledWith("p1", {});
    expect(writeText).toHaveBeenCalledWith("Hello inlined body");
    cleanup();

    writeText.mockClear();
    copyPrompt.mockResolvedValueOnce({
      systemPrompt: null,
      userPrompt: "",
      messages: [{ role: "user" as const, content: "Hello inlined body" }],
      unexpanded: [],
    });
    render(
      <CopyPromptButton
        source={{
          systemPrompt: null,
          userPrompt: "",
          messages: [{ role: "user", content: "Hello @@Title" }],
          variables: [],
        }}
        promptId="p1"
        copyPrompt={copyPrompt}
        name="Chat"
        writeText={writeText}
      />,
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Chat" }));
    });
    expect(writeText).toHaveBeenCalledWith("[User]\nHello inlined body");
  });
});
