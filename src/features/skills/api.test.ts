import { describe, expect, it, vi } from "vitest";
import { createSkillApi } from "./api";
import type { RuntimeBridge } from "../../runtime";
import type { ParsedSkillMd, SafetyReport, SkillFile } from "./types";

/** A bridge whose invoke records the command + args and echoes a value. */
function makeBridge(returnValue: unknown = null) {
  const invoke = vi.fn(async () => returnValue);
  const bridge: RuntimeBridge = {
    capabilities: () => ({
      appUpdate: true,
      dataRecovery: true,
      desktopWindowControls: true,
      skillDistribution: true,
      skillFileEditing: true,
      skillLocalScan: true,
      skillPlatformIntegration: true,
      skillStore: true,
    }),
    invoke: invoke as RuntimeBridge["invoke"],
    on: vi.fn(() => () => {}),
  };
  return { bridge, invoke };
}

describe("createSkillApi command contract (Req 3.1)", () => {
  it("routes skill CRUD commands through the bridge with domain.action names (Req 9)", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createSkillApi(bridge);

    await api.listSkills();
    await api.getSkill("s1");
    await api.createSkill({ name: "S" });
    await api.updateSkill("s1", { name: "S2" });
    await api.deleteSkill("s1");

    expect(invoke).toHaveBeenCalledWith("skill.list");
    expect(invoke).toHaveBeenCalledWith("skill.get", { id: "s1" });
    expect(invoke).toHaveBeenCalledWith("skill.create", { input: { name: "S" } });
    expect(invoke).toHaveBeenCalledWith("skill.update", {
      id: "s1",
      patch: { name: "S2" },
    });
    expect(invoke).toHaveBeenCalledWith("skill.delete", { id: "s1" });
  });

  it("routes skill version commands through the bridge (Req 9.6-9.9)", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createSkillApi(bridge);

    await api.listVersions("s1");
    await api.createVersion("s1", "note");
    await api.rollbackVersion("s1", 3);
    await api.deleteVersion("v1");

    expect(invoke).toHaveBeenCalledWith("skill.version.list", { skillId: "s1" });
    expect(invoke).toHaveBeenCalledWith("skill.version.create", {
      skillId: "s1",
      note: "note",
    });
    expect(invoke).toHaveBeenCalledWith("skill.version.rollback", {
      skillId: "s1",
      version: 3,
    });
    expect(invoke).toHaveBeenCalledWith("skill.version.delete", { id: "v1" });
  });

  it("routes SKILL.md parse/serialize/import commands (Req 10)", async () => {
    const { bridge, invoke } = makeBridge("");
    const api = createSkillApi(bridge);
    const parsed: ParsedSkillMd = { frontmatter: { name: "S" }, body: "B" };

    await api.parseMd("---\nname: S\n---\nB");
    await api.serializeMd(parsed);
    await api.importSkill('{"name":"S"}');

    expect(invoke).toHaveBeenCalledWith("skill.parseMd", {
      content: "---\nname: S\n---\nB",
    });
    expect(invoke).toHaveBeenCalledWith("skill.serializeMd", { parsed });
    expect(invoke).toHaveBeenCalledWith("skill.import", { json: '{"name":"S"}' });
  });

  it("routes local repository sync commands (Req 11)", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createSkillApi(bridge);

    await api.localScan(["/loc"]);
    await api.localTree("/repo");
    await api.localRead("/repo", "SKILL.md");
    await api.localWrite("/repo", "a.txt", "x");
    await api.localMkdir("/repo", "dir");
    await api.localRename("/repo", "a.txt", "b.txt");
    await api.localDelete("/repo", "b.txt");
    await api.localSync("s1", "/repo");

    expect(invoke).toHaveBeenCalledWith("skill.local.scan", { locations: ["/loc"] });
    expect(invoke).toHaveBeenCalledWith("skill.local.tree", { repoPath: "/repo" });
    expect(invoke).toHaveBeenCalledWith("skill.local.read", {
      repoPath: "/repo",
      relativePath: "SKILL.md",
    });
    expect(invoke).toHaveBeenCalledWith("skill.local.write", {
      repoPath: "/repo",
      relativePath: "a.txt",
      content: "x",
    });
    expect(invoke).toHaveBeenCalledWith("skill.local.mkdir", {
      repoPath: "/repo",
      relativePath: "dir",
    });
    expect(invoke).toHaveBeenCalledWith("skill.local.rename", {
      repoPath: "/repo",
      fromRelativePath: "a.txt",
      toRelativePath: "b.txt",
    });
    expect(invoke).toHaveBeenCalledWith("skill.local.delete", {
      repoPath: "/repo",
      relativePath: "b.txt",
    });
    expect(invoke).toHaveBeenCalledWith("skill.local.sync", {
      skillId: "s1",
      repoPath: "/repo",
    });
  });

  it("routes platform integration commands (Req 12)", async () => {
    const { bridge, invoke } = makeBridge([]);
    const api = createSkillApi(bridge);
    const files: SkillFile[] = [{ relativePath: "SKILL.md", content: "x" }];

    await api.listPlatforms();
    await api.detectPlatforms();
    await api.installSkill("claude", "my-skill", files);
    await api.uninstallSkill("claude", "my-skill");
    await api.platformStatus("my-skill");

    expect(invoke).toHaveBeenCalledWith("skill.platform.list");
    expect(invoke).toHaveBeenCalledWith("skill.platform.detect");
    expect(invoke).toHaveBeenCalledWith("skill.platform.install", {
      platformId: "claude",
      skillName: "my-skill",
      files,
    });
    expect(invoke).toHaveBeenCalledWith("skill.platform.uninstall", {
      platformId: "claude",
      skillName: "my-skill",
    });
    expect(invoke).toHaveBeenCalledWith("skill.platform.status", {
      skillName: "my-skill",
    });
  });

  it("routes safety + remote fetch commands (Req 13)", async () => {
    const { bridge, invoke } = makeBridge(null);
    const api = createSkillApi(bridge);
    const report: SafetyReport = { level: "safe", findings: [] };

    await api.safetyScan("content");
    await api.saveSafetyReport("s1", report);
    await api.fetchRemoteContent("https://example.com/SKILL.md");
    await api.scanRemoteRepo("https://api.github.com/repos/o/r/git/trees/main");

    expect(invoke).toHaveBeenCalledWith("skill.safety.scan", { content: "content" });
    expect(invoke).toHaveBeenCalledWith("skill.safety.save", {
      skillId: "s1",
      report,
    });
    expect(invoke).toHaveBeenCalledWith("skill.remote.fetchContent", {
      url: "https://example.com/SKILL.md",
    });
    expect(invoke).toHaveBeenCalledWith("skill.remote.scanRepo", {
      listingUrl: "https://api.github.com/repos/o/r/git/trees/main",
    });
  });
});
