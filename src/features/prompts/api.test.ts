import { describe, expect, it, vi } from "vitest";
import { createPromptApi } from "./api";
import type { RuntimeBridge } from "../../runtime";

/** A bridge whose invoke records the command + args and echoes a value. */
function makeBridge(returnValue: unknown = null) {
  const invoke = vi.fn(async () => returnValue);
  const bridge: RuntimeBridge = {
    capabilities: () => ({
      appUpdate: true,
      dataRecovery: true,
      desktopWindowControls: true,
    }),
    invoke: invoke as RuntimeBridge["invoke"],
    on: vi.fn(() => () => {}),
  };
  return { bridge, invoke };
}

describe("createPromptApi command contract (Req 3.1)", () => {
  it("routes prompt commands through the bridge with domain.action names", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createPromptApi(bridge);

    await api.listPrompts();
    await api.getPrompt("p1");
    await api.searchPrompts({ keyword: "hi" });
    await api.countPrompts();
    await api.createPrompt({ title: "T", userPrompt: "U" });
    await api.updatePrompt("p1", { title: "T2" });
    await api.deletePrompt("p1");
    await api.duplicatePrompt("p1");
    await api.batchMove(["p1", "p2"], "f1");
    await api.batchTag(["p1"], ["shared"]);
    await api.batchDelete(["p2"]);
    await api.copyPrompt("p1", { a: "b" });
    await api.incrementUsage("p1");
    await api.listReferences("p1");

    expect(invoke).toHaveBeenCalledWith("prompt.list");
    expect(invoke).toHaveBeenCalledWith("prompt.get", { id: "p1" });
    expect(invoke).toHaveBeenCalledWith("prompt.search", {
      query: { keyword: "hi" },
    });
    expect(invoke).toHaveBeenCalledWith("prompt.counts");
    expect(invoke).toHaveBeenCalledWith("prompt.create", {
      input: { title: "T", userPrompt: "U" },
    });
    expect(invoke).toHaveBeenCalledWith("prompt.update", {
      id: "p1",
      patch: { title: "T2" },
    });
    expect(invoke).toHaveBeenCalledWith("prompt.delete", { id: "p1" });
    expect(invoke).toHaveBeenCalledWith("prompt.duplicate", { id: "p1" });
    expect(invoke).toHaveBeenCalledWith("prompt.batchMove", {
      ids: ["p1", "p2"],
      folderId: "f1",
    });
    expect(invoke).toHaveBeenCalledWith("prompt.batchTag", {
      ids: ["p1"],
      tags: ["shared"],
    });
    expect(invoke).toHaveBeenCalledWith("prompt.batchDelete", { ids: ["p2"] });
    expect(invoke).toHaveBeenCalledWith("prompt.copy", {
      id: "p1",
      values: { a: "b" },
    });
    expect(invoke).toHaveBeenCalledWith("prompt.incrementUsage", { id: "p1" });
    expect(invoke).toHaveBeenCalledWith("reference.list", { promptId: "p1" });
  });

  it("routes folder commands through the bridge (Req 8)", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createPromptApi(bridge);

    await api.listFolders();
    await api.createFolder({ name: "F" });
    await api.updateFolder("f1", { name: "F2" });
    await api.deleteFolder("f1");
    await api.reorderFolders(["a", "b"]);

    expect(invoke).toHaveBeenCalledWith("folder.list");
    expect(invoke).toHaveBeenCalledWith("folder.create", {
      input: { name: "F" },
    });
    expect(invoke).toHaveBeenCalledWith("folder.update", {
      id: "f1",
      patch: { name: "F2" },
    });
    expect(invoke).toHaveBeenCalledWith("folder.delete", { id: "f1" });
    expect(invoke).toHaveBeenCalledWith("folder.reorder", {
      orderedIds: ["a", "b"],
    });
  });

  it("routes version commands through the bridge (Req 7)", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createPromptApi(bridge);

    await api.listVersions("p1");
    await api.createVersion("p1", "note");
    await api.rollbackVersion("p1", 3);

    expect(invoke).toHaveBeenCalledWith("version.list", { promptId: "p1" });
    expect(invoke).toHaveBeenCalledWith("version.create", {
      promptId: "p1",
      note: "note",
    });
    expect(invoke).toHaveBeenCalledWith("version.rollback", {
      promptId: "p1",
      version: 3,
    });
  });

  it("routes tag and portable bundle commands through the bridge", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createPromptApi(bridge);
    await api.listTags();
    await api.renameTag("old", "new");
    await api.deleteTag("stale");
    await api.exportBundle("D:/bundle.prompthub");
    await api.previewBundle("D:/bundle.prompthub");
    await api.importBundle("D:/bundle.prompthub", "replace");

    expect(invoke).toHaveBeenCalledWith("tag.list");
    expect(invoke).toHaveBeenCalledWith("tag.rename", {
      old: "old",
      new: "new",
    });
    expect(invoke).toHaveBeenCalledWith("tag.delete", { tag: "stale" });
    expect(invoke).toHaveBeenCalledWith("prompt.bundleExport", {
      destination: "D:/bundle.prompthub",
    });
    expect(invoke).toHaveBeenCalledWith("prompt.bundlePreview", {
      filePath: "D:/bundle.prompthub",
    });
    expect(invoke).toHaveBeenCalledWith("prompt.bundleImport", {
      filePath: "D:/bundle.prompthub",
      policy: "replace",
    });
  });

  it("routes prompt type commands through the bridge", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createPromptApi(bridge);

    await api.listPromptTypes();
    await api.createPromptType({ name: "Storyboard", baseKind: "image" });

    expect(invoke).toHaveBeenCalledWith("promptType.list");
    expect(invoke).toHaveBeenCalledWith("promptType.create", {
      input: { name: "Storyboard", baseKind: "image" },
    });
  });
});
