// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../../../../src/runtime/i18n";
import { usePromptStore } from "../../../../../../src/features/prompts/promptStore";
import { PromptDetailModal } from "../../../../../../src/features/prompts/components/detail/PromptDetailModal";

const initialStore = usePromptStore.getState();

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

afterEach(() => {
  cleanup();
  usePromptStore.setState(initialStore, true);
});

describe("new Prompt title focus reproduction", () => {
  it("keeps title focus after the first character changes the draft", async () => {
    render(
      <PromptDetailModal
        open
        creating
        prompt={null}
        prompts={[]}
        versions={[]}
        folders={[]}
        promptTypeDefinitions={[]}
        knownTags={[]}
        onClose={vi.fn()}
        onCreate={vi.fn()}
        onSave={vi.fn()}
        onCreateFolder={vi.fn(async () => null)}
        onCreatePromptType={vi.fn(async () => null)}
        onToggleFavorite={vi.fn()}
        onTogglePin={vi.fn()}
        onDuplicate={vi.fn()}
        onDelete={vi.fn()}
        onCreateVersion={vi.fn()}
        onRollback={vi.fn()}
      />,
    );

    const pencil = screen.getByRole("button", { name: "Read only" });
    await waitFor(() => expect(document.activeElement).toBe(pencil));

    const title = screen.getByLabelText("Title") as HTMLInputElement;
    title.focus();
    expect(document.activeElement).toBe(title);

    fireEvent.change(title, { target: { value: "u" } });
    await act(async () => {
      await new Promise<void>((resolve) => setTimeout(resolve, 20));
    });

    expect(title.value).toBe("u");
    expect(screen.getByLabelText("Title")).toBe(title);
    expect(pencil.getAttribute("aria-pressed")).toBe("true");
    expect(document.activeElement).toBe(title);
  });
});
