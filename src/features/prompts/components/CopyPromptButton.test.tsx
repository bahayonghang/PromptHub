// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import type { PromptCopySource } from "../promptText";
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
});
