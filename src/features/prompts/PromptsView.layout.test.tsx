// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import i18n, { ensureBundle } from "../../runtime/i18n";
import { PromptsView } from "./PromptsView";
import {
  DEFAULT_FILTERS,
  usePromptStore,
} from "./promptStore";
import type { Prompt } from "./types";

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
    loading: false,
    error: null,
    load: async () => {},
    selectPrompt: async (id) => {
      usePromptStore.setState({
        selectedPromptId: id,
        selectedPrompt: id === prompt.id ? prompt : null,
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

  it("preserves the selected prompt and draft across compact pane switches", async () => {
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

    fireEvent.click(screen.getByRole("button", { name: "Prompts" }));
    expect(usePromptStore.getState().selectedPromptId).toBe(prompt.id);

    fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
    await waitFor(() => {
      expect(
        (screen.getByRole("textbox", { name: "Title" }) as HTMLInputElement)
          .value,
      ).toBe("Unpublished draft");
    });
  });
});
