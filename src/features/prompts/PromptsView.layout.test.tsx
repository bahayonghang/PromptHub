// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import i18n, { ensureBundle } from "../../runtime/i18n";
import { PromptsView } from "./PromptsView";
import { resetPreferredChatModeForTests } from "./definitionMode";
import {
  DEFAULT_FILTERS,
  usePromptStore,
} from "./promptStore";
import type { CreatePromptInput, Prompt } from "./types";

const initialStore = usePromptStore.getState();

const prompt: Prompt = {
  id: "prompt-1",
  title: "Release notes",
  description: "Summarize a release",
  promptType: "text",
  systemPrompt: "Be concise.",
  userPrompt: "Summarize {{changes}}",
  messages: [],
  variables: [
    {
      name: "changes",
      type: "textarea",
      required: true,
    },
  ],
  tags: ["writing"],
  folderId: null,
  images: [],
  videos: [],
  isFavorite: false,
  isPinned: false,
  isPrivate: false,
  isLocked: false,
  currentVersion: 1,
  usageCount: 0,
  source: null,
  notes: null,
  lastAiResponse: null,
  createdAt: "2026-07-15T00:00:00Z",
  updatedAt: "2026-07-15T00:00:00Z",
};

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
  resetPreferredChatModeForTests();
  usePromptStore.setState({
    folders: [],
    prompts: [prompt],
    total: 1,
    offset: 0,
    tags: ["writing"],
    filters: { ...DEFAULT_FILTERS },
    selectedPromptId: null,
    selectedPrompt: null,
    selectedPromptIds: [],
    versions: [],
    detailOpen: false,
    loading: false,
    error: null,
    load: async () => {},
    selectPrompt: async (id) => {
      const item = usePromptStore
        .getState()
        .prompts.find((row) => row.id === id);
      usePromptStore.setState({
        selectedPromptId: id,
        selectedPrompt:
          id == null ? null : { ...prompt, ...item, id },
        versions: [],
      });
    },
  });
});

afterEach(() => {
  cleanup();
  usePromptStore.setState(initialStore, true);
});

describe("PromptsView responsive workspace", () => {
  it("does not mount the folder tree or tag manager in the prompts view", () => {
    render(<PromptsView />);

    expect(screen.queryByRole("button", { name: "Folders" })).toBeNull();
    expect(screen.queryByRole("navigation", { name: "Prompt library" })).toBeNull();
    expect(screen.queryByText("Manage tags")).toBeNull();
    expect(screen.getByRole("searchbox", { name: "Search prompts..." })).toBeTruthy();
  });

  it("shows the empty state only when loading is false", () => {
    usePromptStore.setState({ loading: true, prompts: [], total: 0 });
    const { rerender } = render(<PromptsView />);
    expect(screen.getByText("Loading...")).toBeTruthy();
    expect(screen.queryByText("No prompts found")).toBeNull();

    usePromptStore.setState({ loading: false, prompts: [], total: 0 });
    rerender(<PromptsView />);
    expect(screen.getByText("No prompts found")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Clear all" })).toBeTruthy();
  });

  it("keeps selection and filters when switching grid and list", async () => {
    usePromptStore.setState({
      viewMode: "list",
      selectedPromptIds: [prompt.id],
      filters: { ...DEFAULT_FILTERS, keyword: "Release" },
      setViewMode: (next) => {
        usePromptStore.setState({ viewMode: next });
      },
    });
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "Grid view" }));
    expect(usePromptStore.getState().viewMode).toBe("grid");
    expect(usePromptStore.getState().selectedPromptIds).toEqual([prompt.id]);
    expect(usePromptStore.getState().filters.keyword).toBe("Release");
    fireEvent.click(screen.getByRole("button", { name: "List view" }));
    expect(usePromptStore.getState().viewMode).toBe("list");
    expect(usePromptStore.getState().selectedPromptIds).toEqual([prompt.id]);
  });

  it("opens the detail overlay and keeps the draft until close", async () => {
    render(<PromptsView />);

    fireEvent.click(screen.getByRole("button", { name: "Release notes" }));

    const title = await screen.findByRole("textbox", { name: "Title" });
    fireEvent.change(title, { target: { value: "Unpublished draft" } });

    expect(screen.getByRole("heading", { name: "Basics" })).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "Prompt content" }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Organization" })).toBeTruthy();
    expect(
      screen.getByRole("heading", { name: "Supporting details" }),
    ).toBeTruthy();
    expect(screen.getByRole("dialog")).toBeTruthy();

    fireEvent.click(screen.getAllByRole("button", { name: "Close" })[0]);
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Discard and close" }),
      ).toBeTruthy();
    });
    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(
      (screen.getByRole("textbox", { name: "Title" }) as HTMLInputElement)
        .value,
    ).toBe("Unpublished draft");
  });

  it("leaves create mode when a library prompt is selected", async () => {
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "New Prompt" }));
    await screen.findByRole("textbox", { name: "Title" });
    fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
    await waitFor(() => {
      expect(
        (screen.getByRole("textbox", { name: "Title" }) as HTMLInputElement)
          .value,
      ).toBe("Release notes");
    });
    expect(usePromptStore.getState().selectedPromptId).toBe(prompt.id);
  });

  it("keeps a dirty new-prompt overlay when keep-editing after a library select", async () => {
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "New Prompt" }));
    const title = await screen.findByRole("textbox", { name: "Title" });
    fireEvent.change(title, { target: { value: "Draft create" } });

    fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Keep editing" }),
      ).toBeTruthy();
    });
    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));

    expect(
      (screen.getByRole("textbox", { name: "Title" }) as HTMLInputElement)
        .value,
    ).toBe("Draft create");
    expect(usePromptStore.getState().selectedPromptId).toBeNull();
  });

  it("opens a library prompt after a new-prompt overlay is discarded", async () => {
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "New Prompt" }));
    const title = await screen.findByRole("textbox", { name: "Title" });
    fireEvent.change(title, { target: { value: "Draft create" } });

    fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Discard and close" }),
      ).toBeTruthy();
    });
    fireEvent.click(screen.getByRole("button", { name: "Discard and close" }));

    await waitFor(() => {
      expect(
        (screen.getByRole("textbox", { name: "Title" }) as HTMLInputElement)
          .value,
      ).toBe("Release notes");
    });
    expect(usePromptStore.getState().selectedPromptId).toBe(prompt.id);
  });

  it("clears library selection when the detail overlay is closed", async () => {
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
    await screen.findByRole("textbox", { name: "Title" });
    fireEvent.click(screen.getAllByRole("button", { name: "Close" })[0]);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
    expect(usePromptStore.getState().selectedPromptId).toBeNull();
    expect(usePromptStore.getState().detailOpen).toBe(false);
  });

  it("closes the overlay after Create and keeps the new prompt selected", async () => {
    stubCreatePrompt();
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "New Prompt" }));
    const title = await screen.findByRole("textbox", { name: "Title" });
    fireEvent.change(title, { target: { value: "Created prompt" } });
    fireEvent.change(screen.getByLabelText("Content for message 1"), {
      target: { value: "Created body" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
    expect(usePromptStore.getState().selectedPromptId).toBe("created-1");
    expect(usePromptStore.getState().detailOpen).toBe(false);
    expect(screen.queryByRole("heading", { name: "Created prompt" })).toBeNull();
    expect(screen.queryByText("Created prompt")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Created prompt" }));
    await waitFor(() => {
      expect(screen.getByRole("dialog")).toBeTruthy();
    });
    expect(
      (screen.getByRole("textbox", { name: "Title" }) as HTMLInputElement).value,
    ).toBe("Created prompt");
  });

  it("keeps the new prompt selected after create-mode Save and close", async () => {
    stubCreatePrompt();
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "New Prompt" }));
    const title = await screen.findByRole("textbox", { name: "Title" });
    fireEvent.change(title, { target: { value: "Created prompt" } });
    fireEvent.change(screen.getByLabelText("Content for message 1"), {
      target: { value: "Created body" },
    });
    fireEvent.click(screen.getAllByRole("button", { name: "Close" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Save and close" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
    expect(usePromptStore.getState().selectedPromptId).toBe("created-1");
    expect(usePromptStore.getState().detailOpen).toBe(false);
  });

  it("closes the overlay after create-mode registered save and keeps selection", async () => {
    stubCreatePrompt();
    render(<PromptsView />);
    fireEvent.click(screen.getByRole("button", { name: "New Prompt" }));
    const title = await screen.findByRole("textbox", { name: "Title" });
    fireEvent.change(title, { target: { value: "Created prompt" } });
    fireEvent.change(screen.getByLabelText("Content for message 1"), {
      target: { value: "Created body" },
    });
    await waitFor(() => {
      expect(usePromptStore.getState().detailActions).not.toBeNull();
    });
    await act(async () => {
      await usePromptStore.getState().detailActions?.save();
    });

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
    expect(usePromptStore.getState().selectedPromptId).toBe("created-1");
    expect(screen.queryByRole("heading", { name: "Created prompt" })).toBeNull();
  });
});

function stubCreatePrompt() {
  usePromptStore.setState({
    createPrompt: async (input: CreatePromptInput) => {
      const created: Prompt = {
        ...prompt,
        id: "created-1",
        title: input.title,
        userPrompt: input.userPrompt,
        messages: input.messages ?? [],
      };
      usePromptStore.setState({
        prompts: [...usePromptStore.getState().prompts, created],
        total: usePromptStore.getState().prompts.length + 1,
      });
      await usePromptStore.getState().selectPrompt(created.id);
      usePromptStore.getState().closeDetail();
      return created;
    },
  });
}
