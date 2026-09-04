// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { ToastHost } from "../../notifications/ToastHost";
import { useToastStore } from "../../notifications/toastStore";
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
  for (const toast of useToastStore.getState().toasts) {
    useToastStore.getState().dismiss(toast.id);
  }
  cleanup();
  vi.useRealTimers();
});

describe("CopyPromptButton", () => {
  it("writes labeled text and shows a copied confirmation", async () => {
    vi.useFakeTimers();
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn();
    render(
      <CopyPromptButton
        source={source}
        name="Steelman"
        writeText={writeText}
        incrementUsage={incrementUsage}
      />,
    );

    const copyButton = screen.getByRole("button", { name: "Copy Steelman" });
    expect(copyButton.className).toContain("h-9");
    expect(copyButton.className).toContain("w-9");
    expect(copyButton.querySelector("svg")?.classList.contains("h-5")).toBe(true);
    expect(copyButton.querySelector("svg")?.classList.contains("w-5")).toBe(true);

    await act(async () => {
      fireEvent.click(copyButton);
    });

    expect(writeText).toHaveBeenCalledWith(
      "[System]\nBe terse.\n\n[User]\nSummarize this.",
    );
    expect(incrementUsage).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy();
    expect(useToastStore.getState().toasts[0]?.message).toBe("Copied Steelman");
    expect(useToastStore.getState().toasts[0]?.tone).toBe("success");

    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(screen.getByRole("button", { name: "Copy Steelman" })).toBeTruthy();
  });

  it("does not write when the prompt is locked", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn();
    render(
      <CopyPromptButton
        source={source}
        name="Private"
        locked
        writeText={writeText}
        incrementUsage={incrementUsage}
      />,
    );

    const button = screen.getByRole("button", {
      name: "Unlock the prompt library to copy private content",
    });
    expect(button).toHaveProperty("disabled", true);
    fireEvent.click(button);
    expect(writeText).not.toHaveBeenCalled();
    expect(incrementUsage).not.toHaveBeenCalled();
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("announces a failure on the same control", async () => {
    const writeText = vi.fn().mockRejectedValue(new Error("denied"));
    const incrementUsage = vi.fn();
    render(
      <>
        <CopyPromptButton
          source={source}
          name="Steelman"
          writeText={writeText}
          incrementUsage={incrementUsage}
        />
        <ToastHost />
      </>,
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });

    expect(
      screen.getByRole("button", { name: "Could not copy prompt" }),
    ).toBeTruthy();
    expect(incrementUsage).not.toHaveBeenCalled();
    expect(screen.getByRole("status").textContent).toContain("Could not copy prompt");
  });

  it("increments usage after a successful persisted copy", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn().mockResolvedValue(1);
    const copyPrompt = vi.fn(async (): Promise<PromptCopyResult> => ({
      systemPrompt: "Be terse.",
      userPrompt: "Summarize this.",
      messages: [],
      unexpanded: [],
    }));
    render(
      <CopyPromptButton
        source={source}
        promptId="p1"
        copyPrompt={copyPrompt}
        name="Steelman"
        writeText={writeText}
        incrementUsage={incrementUsage}
      />,
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });

    expect(writeText).toHaveBeenCalled();
    expect(incrementUsage).toHaveBeenCalledWith("p1");
    expect(writeText.mock.invocationCallOrder[0]).toBeLessThan(
      incrementUsage.mock.invocationCallOrder[0],
    );
  });

  it("keeps copied confirmation when increment fails", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn().mockRejectedValue(new Error("NOT_FOUND"));
    const copyPrompt = vi.fn(async (): Promise<PromptCopyResult> => ({
      systemPrompt: "Be terse.",
      userPrompt: "Summarize this.",
      messages: [],
      unexpanded: [],
    }));
    render(
      <CopyPromptButton
        source={source}
        promptId="p1"
        copyPrompt={copyPrompt}
        name="Steelman"
        writeText={writeText}
        incrementUsage={incrementUsage}
      />,
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });

    expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy();
    expect(useToastStore.getState().toasts[0]?.tone).toBe("success");
  });

  it("copies a prompt with no references byte for byte via prompt.copy", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn();
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
        incrementUsage={incrementUsage}
      />,
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });
    expect(copyPrompt).toHaveBeenCalledWith("p1", { name: "Ada" });
    expect(writeText).toHaveBeenCalledWith(expected);
    expect(expected).toBe("[System]\nBe terse.\n\n[User]\nHello Ada");
    expect(incrementUsage).toHaveBeenCalledWith("p1");
  });

  it("inlines @@Title from prompt.copy in text mode and chat mode", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn();
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
        incrementUsage={incrementUsage}
      />,
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Source" }));
    });
    expect(copyPrompt).toHaveBeenCalledWith("p1", {});
    expect(writeText).toHaveBeenCalledWith("Hello inlined body");
    cleanup();

    writeText.mockClear();
    incrementUsage.mockClear();
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
        incrementUsage={incrementUsage}
      />,
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Chat" }));
    });
    expect(writeText).toHaveBeenCalledWith("Hello inlined body");
    expect(incrementUsage).toHaveBeenCalledWith("p1");
  });
});
