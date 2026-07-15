import { describe, expect, it } from "vitest";
import type { Prompt, PromptVersion } from "./types";
import { diffPromptRevision } from "./versionDiff";

const prompt: Prompt = {
  id: "p1",
  title: "Current",
  promptType: "text",
  userPrompt: "current body",
  variables: [],
  tags: ["current"],
  images: [],
  videos: [],
  isFavorite: false,
  isPinned: true,
  isPrivate: false,
  isLocked: false,
  currentVersion: 2,
  usageCount: 0,
  createdAt: "2024-01-01T00:00:00.000Z",
  updatedAt: "2024-01-02T00:00:00.000Z",
};

const revision: PromptVersion = {
  id: "v1",
  promptId: "p1",
  version: 1,
  title: "Original",
  promptType: "text",
  userPrompt: "original body",
  variables: [],
  tags: ["original"],
  images: [],
  videos: [],
  isFavorite: false,
  isPinned: false,
  isPrivate: false,
  sourceAction: "create",
  createdAt: "2024-01-01T00:00:00.000Z",
};

describe("diffPromptRevision", () => {
  it("reports field-aware differences and omits unchanged fields", () => {
    const diff = diffPromptRevision(prompt, revision);
    expect(diff.map((entry) => entry.field)).toEqual([
      "title",
      "userPrompt",
      "tags",
      "pinned",
    ]);
    expect(diff.find((entry) => entry.field === "tags")).toEqual({
      field: "tags",
      revisionValue: "original",
      currentValue: "current",
    });
  });
});
