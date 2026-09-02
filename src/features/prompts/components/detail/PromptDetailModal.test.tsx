// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../../runtime/i18n";
import type { Folder, Prompt, PromptTypeDefinition } from "../../types";
import { PromptDetailModal, type PromptDetailModalProps } from "./PromptDetailModal";
import { resetPreferredChatModeForTests } from "../../definitionMode";
import { usePromptStore } from "../../promptStore";

const initialStore = usePromptStore.getState();

function folder(id: string, name: string): Folder {
  return {
    id,
    name,
    parentId: null,
    sortOrder: 0,
    createdAt: "2026-07-15T00:00:00Z",
  };
}

function savedPrompt(): Prompt {
  return {
    id: "prompt-1",
    title: "Saved title",
    promptType: "text",
    userPrompt: "Saved body",
    messages: [],
    variables: [],
    tags: ["saved-tag"],
    folderId: null,
    images: [],
    videos: [],
    isFavorite: false,
    isPinned: false,
    isPrivate: false,
    isLocked: false,
    currentVersion: 1,
    usageCount: 0,
    createdAt: "2026-07-15T00:00:00Z",
    updatedAt: "2026-07-15T00:00:00Z",
  };
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
  resetPreferredChatModeForTests();
});

afterEach(() => {
  cleanup();
  usePromptStore.setState(initialStore, true);
});

describe("PromptDetailModal", () => {
  it("keeps focus on the new Prompt title while typing", async () => {
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
      await new Promise<void>((resolve) => window.setTimeout(resolve, 20));
    });

    expect(title.value).toBe("u");
    expect(screen.getByLabelText("Title")).toBe(title);
    expect(document.activeElement).toBe(title);
  });

  it("saves with the same update payload as the inline editor", async () => {
    const onSave = vi.fn(async () => savedPrompt());
    render(
      <PromptDetailModal
        open
        creating={false}
        prompt={savedPrompt()}
        prompts={[savedPrompt()]}
        versions={[]}
        folders={[folder("existing", "Existing")]}
        promptTypeDefinitions={[] as PromptTypeDefinition[]}
        knownTags={["saved-tag"]}
        onClose={vi.fn()}
        onCreate={vi.fn()}
        onSave={onSave}
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

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
    expect(onSave).toHaveBeenCalledWith(
      "prompt-1",
      expect.objectContaining({
        title: "Renamed",
        userPrompt: "Saved body",
        tags: ["saved-tag"],
      }),
    );
  });

  it("keeps the overlay open when save validation fails", async () => {
    const onSave = vi.fn();
    const onClose = vi.fn();
    render(
      <PromptDetailModal
        open
        creating={false}
        prompt={savedPrompt()}
        prompts={[savedPrompt()]}
        versions={[]}
        folders={[]}
        promptTypeDefinitions={[]}
        knownTags={[]}
        onClose={onClose}
        onCreate={vi.fn()}
        onSave={onSave}
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
    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "   " } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSave).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("offers three close answers when the draft is dirty", async () => {
    const onClose = vi.fn();
    render(
      <PromptDetailModal
        open
        creating={false}
        prompt={savedPrompt()}
        prompts={[savedPrompt()]}
        versions={[]}
        folders={[]}
        promptTypeDefinitions={[]}
        knownTags={[]}
        onClose={onClose}
        onCreate={vi.fn()}
        onSave={vi.fn(async () => savedPrompt())}
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
    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Dirty" },
    });
    fireEvent.click(screen.getAllByRole("button", { name: "Close" })[0]);
    expect(screen.getByRole("button", { name: "Save and close" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Discard and close" })).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    });
    expect(onClose).not.toHaveBeenCalled();
    expect(
      (screen.getByLabelText("Title") as HTMLInputElement).value,
    ).toBe("Dirty");
  });

  it("calls onCreate and not onClose when Create succeeds", async () => {
    const { onClose, onCreate } = renderCreateModal();
    fillCreateDraft();
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(onCreate).toHaveBeenCalledOnce());
    expect(onClose).not.toHaveBeenCalled();
  });

  it("saves through the registered detail action without closing", async () => {
    const { onClose, onCreate } = renderCreateModal();
    fillCreateDraft();
    await waitFor(() => {
      expect(usePromptStore.getState().detailActions).not.toBeNull();
    });
    await act(async () => {
      await usePromptStore.getState().detailActions?.save();
    });
    expect(onCreate).toHaveBeenCalledOnce();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("keeps the overlay open when create validation fails", () => {
    const { onClose, onCreate } = renderCreateModal();
    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "   " } });
    fireEvent.change(screen.getByLabelText("Content for message 1"), {
      target: { value: "Body" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(onCreate).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("keeps the overlay open when onCreate fails", async () => {
    const { onClose, onCreate } = renderCreateModal({
      onCreate: vi.fn(async () => null),
    });
    fillCreateDraft();
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(onCreate).toHaveBeenCalledOnce());
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("does not call onClose after create-mode Save and close", async () => {
    const { onClose, onCreate } = renderCreateModal();
    fillCreateDraft();
    fireEvent.click(screen.getAllByRole("button", { name: "Close" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Save and close" }));
    await waitFor(() => expect(onCreate).toHaveBeenCalledOnce());
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.queryByRole("heading", { name: "Unsaved changes" })).toBeNull();
    expect(screen.getByRole("dialog")).toBeTruthy();
  });
});

function renderCreateModal(
  overrides: {
    onClose?: PromptDetailModalProps["onClose"];
    onCreate?: PromptDetailModalProps["onCreate"];
  } = {},
) {
  const onClose = overrides.onClose ?? vi.fn();
  const onCreate = overrides.onCreate ?? vi.fn(async () => ({ id: "new" }));
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
      onClose={onClose}
      onCreate={onCreate}
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
  return { onClose, onCreate };
}

function fillCreateDraft(title = "New title", body = "New body") {
  fireEvent.change(screen.getByLabelText("Title"), { target: { value: title } });
  fireEvent.change(screen.getByLabelText("Content for message 1"), {
    target: { value: body },
  });
}
