// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import type { Folder, Prompt, PromptTypeDefinition } from "../types";
import { PromptEditor } from "./PromptEditor";

function folder(id: string, name: string): Folder {
  return {
    id,
    name,
    parentId: null,
    sortOrder: 0,
    createdAt: "2026-07-15T00:00:00Z",
  };
}

function prompt(folderId: string | null): Prompt {
  return {
    id: "prompt-1",
    title: "Saved title",
    promptType: "text",
    userPrompt: "Saved body",
    messages: [],
    variables: [],
    tags: ["saved-tag"],
    folderId,
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

function promptTypeDefinition(
  id: string,
  name: string,
  baseKind: PromptTypeDefinition["baseKind"],
): PromptTypeDefinition {
  return {
    id,
    name,
    baseKind,
    createdAt: "2026-07-15T00:00:00Z",
  };
}

function renderEditor(
  onCreateFolder = vi.fn<() => Promise<Folder | null>>(async () => null),
  onCreatePromptType = vi.fn<() => Promise<PromptTypeDefinition | null>>(
    async () => null,
  ),
  promptTypeDefinitions: PromptTypeDefinition[] = [],
) {
  const onCreate = vi.fn();
  const onSave = vi.fn();
  const onCancelCreate = vi.fn();
  const folders = [folder("existing", "Existing")];

  const view = render(
    <PromptEditor
      prompt={null}
      creating
      folders={folders}
      promptTypeDefinitions={promptTypeDefinitions}
      knownTags={[]}
      onCreate={onCreate}
      onSave={onSave}
      onCancelCreate={onCancelCreate}
      onCreateFolder={onCreateFolder}
      onCreatePromptType={onCreatePromptType}
    />,
  );

  return {
    ...view,
    folders,
    onCreate,
    onCreateFolder,
    onCreatePromptType,
    rerenderEditor(
      nextFolders: Folder[],
      nextDefinitions = promptTypeDefinitions,
    ) {
      view.rerender(
        <PromptEditor
          prompt={null}
          creating
          folders={nextFolders}
          promptTypeDefinitions={nextDefinitions}
          knownTags={[]}
          onCreate={onCreate}
          onSave={onSave}
          onCancelCreate={onCancelCreate}
          onCreateFolder={onCreateFolder}
          onCreatePromptType={onCreatePromptType}
        />,
      );
    },
  };
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

afterEach(cleanup);

describe("PromptEditor inline folder creation", () => {
  it("creates a root folder with Enter, selects it, and preserves the draft", async () => {
    const created = folder("created", "Research");
    const onCreateFolder = vi.fn(async () => created);
    const view = renderEditor(onCreateFolder);

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Unsaved title" },
    });
    fireEvent.change(screen.getByLabelText("User Prompt"), {
      target: { value: "Unsaved body" },
    });
    fireEvent.change(screen.getByLabelText("Folder"), {
      target: { value: "existing" },
    });

    fireEvent.click(screen.getByRole("button", { name: "New Folder" }));
    const input = screen.getByRole("textbox", { name: "Folder name" });
    expect(document.activeElement).toBe(input);
    fireEvent.change(input, { target: { value: "  Research  " } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() =>
      expect(onCreateFolder).toHaveBeenCalledWith({
        name: "Research",
        parentId: null,
      }),
    );
    view.rerenderEditor([...view.folders, created]);

    await waitFor(() =>
      expect(
        (screen.getByRole("combobox", { name: "Folder" }) as HTMLSelectElement)
          .value,
      ).toBe("created"),
    );
    expect(document.activeElement).toBe(
      screen.getByRole("combobox", { name: "Folder" }),
    );
    expect((screen.getByLabelText("Title") as HTMLInputElement).value).toBe(
      "Unsaved title",
    );
    expect((screen.getByLabelText("User Prompt") as HTMLTextAreaElement).value).toBe(
      "Unsaved body",
    );
    expect(view.onCreate).not.toHaveBeenCalled();
  });

  it("cancels with Escape, restores trigger focus, and keeps the selection", () => {
    renderEditor();
    const folderSelect = screen.getByRole("combobox", { name: "Folder" });
    fireEvent.change(folderSelect, { target: { value: "existing" } });

    const trigger = screen.getByRole("button", { name: "New Folder" });
    fireEvent.click(trigger);
    const input = screen.getByRole("textbox", { name: "Folder name" });
    fireEvent.change(input, { target: { value: "Discard me" } });
    fireEvent.keyDown(input, { key: "Escape" });

    expect(screen.queryByRole("textbox", { name: "Folder name" })).toBeNull();
    expect((folderSelect as HTMLSelectElement).value).toBe("existing");
    expect(document.activeElement).toBe(trigger);
  });

  it("updates the folder in edit mode without changing the saved draft fields", async () => {
    const existing = folder("existing", "Existing");
    const created = folder("created", "Created");
    const onCreateFolder = vi.fn(async () => created);
    const onSave = vi.fn();
    const savedPrompt = prompt(existing.id);
    const view = render(
      <PromptEditor
        prompt={savedPrompt}
        creating={false}
        folders={[existing]}
        promptTypeDefinitions={[]}
        knownTags={["saved-tag"]}
        onCreate={vi.fn()}
        onSave={onSave}
        onCancelCreate={vi.fn()}
        onCreateFolder={onCreateFolder}
        onCreatePromptType={vi.fn(async () => null)}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "New Folder" }));
    const input = screen.getByRole("textbox", { name: "Folder name" });
    fireEvent.change(input, { target: { value: "Created" } });
    fireEvent.click(screen.getByRole("button", { name: "Create folder" }));

    await waitFor(() => expect(onCreateFolder).toHaveBeenCalledOnce());
    view.rerender(
      <PromptEditor
        prompt={savedPrompt}
        creating={false}
        folders={[existing, created]}
        promptTypeDefinitions={[]}
        knownTags={["saved-tag"]}
        onCreate={vi.fn()}
        onSave={onSave}
        onCancelCreate={vi.fn()}
        onCreateFolder={onCreateFolder}
        onCreatePromptType={vi.fn(async () => null)}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onSave).toHaveBeenCalledOnce();
    expect(onSave.mock.calls[0][0]).toBe("prompt-1");
    expect(onSave.mock.calls[0][1]).toMatchObject({
      title: "Saved title",
      userPrompt: "Saved body",
      tags: ["saved-tag"],
      folderId: "created",
    });
  });

  it("rejects empty and overlong names through the pointer and keyboard paths", () => {
    const { onCreateFolder } = renderEditor();
    fireEvent.click(screen.getByRole("button", { name: "New Folder" }));
    const input = screen.getByRole("textbox", { name: "Folder name" });

    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.click(screen.getByRole("button", { name: "Create folder" }));
    expect(screen.getByText("Folder name is required")).toBeTruthy();
    expect(input.getAttribute("aria-invalid")).toBe("true");

    fireEvent.change(input, { target: { value: "x".repeat(256) } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(screen.getByText("Folder name must be 255 characters or fewer")).toBeTruthy();
    expect(onCreateFolder).not.toHaveBeenCalled();
  });

  it("prevents duplicate creation and retains the name and selection on failure", async () => {
    let resolveCreate!: (value: Folder | null) => void;
    const onCreateFolder = vi.fn(
      () =>
        new Promise<Folder | null>((resolve) => {
          resolveCreate = resolve;
        }),
    );
    renderEditor(onCreateFolder);
    const folderSelect = screen.getByRole("combobox", { name: "Folder" });
    fireEvent.change(folderSelect, { target: { value: "existing" } });
    fireEvent.click(screen.getByRole("button", { name: "New Folder" }));
    const input = screen.getByRole("textbox", { name: "Folder name" });
    fireEvent.change(input, { target: { value: "Keep me" } });

    const createButton = screen.getByRole("button", { name: "Create folder" });
    fireEvent.click(createButton);
    fireEvent.click(createButton);
    expect(onCreateFolder).toHaveBeenCalledOnce();
    expect((createButton as HTMLButtonElement).disabled).toBe(true);
    expect(input.getAttribute("aria-busy")).toBe("true");

    await act(async () => resolveCreate(null));

    expect(
      (screen.getByRole("textbox", { name: "Folder name" }) as HTMLInputElement)
        .value,
    ).toBe("Keep me");
    expect((folderSelect as HTMLSelectElement).value).toBe("existing");
    expect((createButton as HTMLButtonElement).disabled).toBe(false);
  });
});

describe("PromptEditor inline prompt type creation", () => {
  it("creates, selects, and submits the authoritative custom type", async () => {
    const created = promptTypeDefinition("type-1", "Storyboard", "image");
    const onCreatePromptType = vi.fn(async () => created);
    const view = renderEditor(undefined, onCreatePromptType);
    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Unsaved title" },
    });
    fireEvent.change(screen.getByLabelText("User Prompt"), {
      target: { value: "Unsaved body" },
    });

    fireEvent.click(screen.getByRole("button", { name: "New custom type" }));
    const nameInput = screen.getByRole("textbox", { name: "Type name" });
    fireEvent.change(nameInput, { target: { value: "  Storyboard  " } });
    fireEvent.change(screen.getByRole("combobox", { name: "Base format" }), {
      target: { value: "image" },
    });
    fireEvent.keyDown(nameInput, { key: "Enter" });

    await waitFor(() =>
      expect(onCreatePromptType).toHaveBeenCalledWith({
        name: "Storyboard",
        baseKind: "image",
      }),
    );
    view.rerenderEditor(view.folders, [created]);
    expect(
      (screen.getByRole("combobox", { name: "Type" }) as HTMLSelectElement)
        .value,
    ).toBe("custom:type-1");
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(view.onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "Unsaved title",
        userPrompt: "Unsaved body",
        promptType: "image",
        typeDefinitionId: "type-1",
      }),
    );
  });

  it("validates locally and retains the draft after backend failure", async () => {
    let resolveCreate!: (value: PromptTypeDefinition | null) => void;
    const onCreatePromptType = vi.fn(
      () =>
        new Promise<PromptTypeDefinition | null>((resolve) => {
          resolveCreate = resolve;
        }),
    );
    renderEditor(undefined, onCreatePromptType);
    fireEvent.click(screen.getByRole("button", { name: "New custom type" }));
    const input = screen.getByRole("textbox", { name: "Type name" });

    fireEvent.change(input, { target: { value: " " } });
    fireEvent.click(screen.getByRole("button", { name: "Create type" }));
    expect(screen.getByText("Type name is required")).toBeTruthy();
    expect(onCreatePromptType).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: "Keep me" } });
    const createButton = screen.getByRole("button", { name: "Create type" });
    fireEvent.click(createButton);
    fireEvent.click(createButton);
    expect(onCreatePromptType).toHaveBeenCalledOnce();
    expect((createButton as HTMLButtonElement).disabled).toBe(true);
    await act(async () => resolveCreate(null));
    expect(
      (screen.getByRole("textbox", { name: "Type name" }) as HTMLInputElement)
        .value,
    ).toBe("Keep me");
    expect(
      (screen.getByRole("combobox", { name: "Type" }) as HTMLSelectElement)
        .value,
    ).toBe("base:text");
  });
});
