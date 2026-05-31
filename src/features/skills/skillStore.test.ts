import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  filterSkills,
  selectFilteredSkills,
  selectSelectedSkill,
  useSkillStore,
} from "./skillStore";
import type { SkillApi } from "./api";
import type {
  Platform,
  PlatformInstallStatus,
  SafetyReport,
  Skill,
  SkillVersion,
} from "./types";

function makeSkill(partial: Partial<Skill> & { id: string }): Skill {
  return {
    name: partial.id,
    protocolType: "skill",
    tags: [],
    isFavorite: false,
    category: "general",
    isBuiltin: false,
    currentVersion: 0,
    versionTrackingEnabled: false,
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: "2024-01-01T00:00:00.000Z",
    ...partial,
  };
}

function makePlatform(id: string): Platform {
  return {
    id,
    name: id,
    isCustom: false,
    rootDir: `/home/.${id}`,
    skillsDir: `/home/.${id}/skills`,
  };
}

/** A controllable fake SkillApi. Each method is a vi mock with a default. */
function makeApi(overrides: Partial<SkillApi> = {}): SkillApi {
  return {
    listSkills: vi.fn(async () => []),
    getSkill: vi.fn(async () => makeSkill({ id: "s1" })),
    createSkill: vi.fn(async () => makeSkill({ id: "new" })),
    updateSkill: vi.fn(async () => makeSkill({ id: "s1" })),
    deleteSkill: vi.fn(async () => undefined),
    listVersions: vi.fn(async () => [] as SkillVersion[]),
    createVersion: vi.fn(async () => ({
      id: "v1",
      skillId: "s1",
      version: 1,
      createdAt: "2024-01-01T00:00:00.000Z",
    })),
    rollbackVersion: vi.fn(async () => makeSkill({ id: "s1" })),
    deleteVersion: vi.fn(async () => undefined),
    parseMd: vi.fn(async () => ({ frontmatter: {}, body: "" })),
    serializeMd: vi.fn(async () => ""),
    importSkill: vi.fn(async () => makeSkill({ id: "imp" })),
    localScan: vi.fn(async () => []),
    localTree: vi.fn(async () => []),
    localRead: vi.fn(async () => ""),
    localWrite: vi.fn(async () => undefined),
    localMkdir: vi.fn(async () => undefined),
    localRename: vi.fn(async () => undefined),
    localDelete: vi.fn(async () => undefined),
    localSync: vi.fn(async () => makeSkill({ id: "s1" })),
    listPlatforms: vi.fn(async () => [] as Platform[]),
    detectPlatforms: vi.fn(async () => [] as string[]),
    installSkill: vi.fn(async () => ({ platformId: "claude", installed: true })),
    uninstallSkill: vi.fn(async () => undefined),
    platformStatus: vi.fn(async () => [] as PlatformInstallStatus[]),
    safetyScan: vi.fn(async () => ({ level: "safe", findings: [] }) as SafetyReport),
    saveSafetyReport: vi.fn(async () => makeSkill({ id: "s1" })),
    fetchRemoteContent: vi.fn(async () => ""),
    scanRemoteRepo: vi.fn(async () => []),
    ...overrides,
  };
}

function resetStore(api: SkillApi) {
  useSkillStore.setState({
    api,
    skills: [],
    keyword: "",
    selectedSkillId: null,
    versions: [],
    platforms: [],
    detectedPlatforms: [],
    installStatus: [],
    safetyReport: null,
    loading: false,
    error: null,
    platformUnavailable: false,
  });
}

afterEach(() => vi.restoreAllMocks());

describe("filterSkills (Req 9.3)", () => {
  const skills = [
    makeSkill({ id: "a", name: "Code Reviewer", description: "reviews code", tags: ["dev"] }),
    makeSkill({ id: "b", name: "Translator", description: "translates text", tags: ["lang"] }),
  ];

  it("returns all skills for an empty keyword", () => {
    expect(filterSkills(skills, "")).toHaveLength(2);
    expect(filterSkills(skills, "   ")).toHaveLength(2);
  });

  it("matches case-insensitively across name, description, and tags", () => {
    expect(filterSkills(skills, "CODE").map((s) => s.id)).toEqual(["a"]);
    expect(filterSkills(skills, "translates").map((s) => s.id)).toEqual(["b"]);
    expect(filterSkills(skills, "lang").map((s) => s.id)).toEqual(["b"]);
  });

  it("returns an empty list when nothing matches", () => {
    expect(filterSkills(skills, "zzz")).toEqual([]);
  });
});

describe("skill store (Req 3.1, 9, 12, 13)", () => {
  beforeEach(() => resetStore(makeApi()));

  it("load() fetches skills and platforms (Req 9.3, 12.1, 12.2)", async () => {
    const api = makeApi({
      listSkills: vi.fn(async () => [makeSkill({ id: "s1" })]),
      listPlatforms: vi.fn(async () => [makePlatform("claude")]),
      detectPlatforms: vi.fn(async () => ["claude"]),
    });
    resetStore(api);

    await useSkillStore.getState().load();

    const state = useSkillStore.getState();
    expect(state.skills.map((s) => s.id)).toEqual(["s1"]);
    expect(state.platforms.map((p) => p.id)).toEqual(["claude"]);
    expect(state.detectedPlatforms).toEqual(["claude"]);
    expect(state.platformUnavailable).toBe(false);
  });

  it("load() degrades gracefully when platform integration is unavailable (Req 3.7)", async () => {
    const api = makeApi({
      listSkills: vi.fn(async () => [makeSkill({ id: "s1" })]),
      listPlatforms: vi.fn(async () => {
        throw { code: "CAPABILITY_UNAVAILABLE", message: "no platform" };
      }),
    });
    resetStore(api);

    await useSkillStore.getState().load();

    const state = useSkillStore.getState();
    expect(state.skills).toHaveLength(1);
    expect(state.platforms).toEqual([]);
    expect(state.platformUnavailable).toBe(true);
    // The capability gate must not surface as a blocking error.
    expect(state.error).toBeNull();
  });

  it("selectSkill() loads versions and install status (Req 9.6, 12.5)", async () => {
    const versions: SkillVersion[] = [
      { id: "v1", skillId: "s1", version: 1, createdAt: "2024-01-01T00:00:00.000Z" },
    ];
    const status: PlatformInstallStatus[] = [
      { platformId: "claude", installed: true, skillsDir: "/home/.claude/skills" },
    ];
    resetStore(
      makeApi({
        listVersions: vi.fn(async () => versions),
        platformStatus: vi.fn(async () => status),
      }),
    );
    useSkillStore.setState({ skills: [makeSkill({ id: "s1", name: "S1" })] });

    await useSkillStore.getState().selectSkill("s1");

    const state = useSkillStore.getState();
    expect(state.selectedSkillId).toBe("s1");
    expect(state.versions).toEqual(versions);
    expect(state.installStatus).toEqual(status);
  });

  it("createSkill() refreshes the list and selects the new skill (Req 9.1)", async () => {
    const created = makeSkill({ id: "new", name: "Fresh" });
    resetStore(
      makeApi({
        createSkill: vi.fn(async () => created),
        listSkills: vi.fn(async () => [created]),
      }),
    );

    const result = await useSkillStore.getState().createSkill({ name: "Fresh" });

    expect(result).toEqual(created);
    expect(useSkillStore.getState().selectedSkillId).toBe("new");
  });

  it("saveSkill() surfaces a BridgeError message on failure (Req 3.5)", async () => {
    resetStore(
      makeApi({
        updateSkill: vi.fn(async () => {
          throw { code: "NOT_FOUND", message: "Skill s9 not found" };
        }),
      }),
    );

    const result = await useSkillStore.getState().saveSkill("s9", { name: "x" });

    expect(result).toBeNull();
    expect(useSkillStore.getState().error).toBe("Skill s9 not found");
  });

  it("deleteSkill() clears the selection when the deleted skill was selected (Req 9.5)", async () => {
    resetStore(makeApi());
    useSkillStore.setState({
      skills: [makeSkill({ id: "s1" })],
      selectedSkillId: "s1",
    });

    await useSkillStore.getState().deleteSkill("s1");

    expect(useSkillStore.getState().selectedSkillId).toBeNull();
    expect(useSkillStore.getState().versions).toEqual([]);
  });

  it("rollbackVersion() reloads skills and history (Req 9.8)", async () => {
    const rollback = vi.fn(async () => makeSkill({ id: "s1" }));
    const listVersions = vi.fn(async () => [] as SkillVersion[]);
    resetStore(makeApi({ rollbackVersion: rollback, listVersions }));
    useSkillStore.setState({ selectedSkillId: "s1" });

    await useSkillStore.getState().rollbackVersion(2);

    expect(rollback).toHaveBeenCalledWith("s1", 2);
  });

  it("installToPlatform() installs the SKILL.md file set then refreshes status (Req 12.3)", async () => {
    const install = vi.fn(async () => ({ platformId: "claude", installed: true }));
    const status = vi.fn(async () => [
      { platformId: "claude", installed: true, skillsDir: "/home/.claude/skills" },
    ]);
    resetStore(makeApi({ installSkill: install, platformStatus: status }));
    useSkillStore.setState({
      skills: [makeSkill({ id: "s1", name: "my-skill", content: "body" })],
      selectedSkillId: "s1",
    });

    await useSkillStore.getState().installToPlatform("claude");

    expect(install).toHaveBeenCalledWith("claude", "my-skill", [
      { relativePath: "SKILL.md", content: "body" },
    ]);
    expect(useSkillStore.getState().installStatus).toHaveLength(1);
  });

  it("uninstallFromPlatform() uninstalls then refreshes status (Req 12.4)", async () => {
    const uninstall = vi.fn(async () => undefined);
    resetStore(makeApi({ uninstallSkill: uninstall }));
    useSkillStore.setState({
      skills: [makeSkill({ id: "s1", name: "my-skill" })],
      selectedSkillId: "s1",
    });

    await useSkillStore.getState().uninstallFromPlatform("claude");

    expect(uninstall).toHaveBeenCalledWith("claude", "my-skill");
  });

  it("scanSafety() stores the report and persists it (Req 13.1, 13.2)", async () => {
    const report: SafetyReport = {
      level: "warn",
      findings: [{ code: "x", severity: "warn", title: "t", detail: "d" }],
    };
    const scan = vi.fn(async () => report);
    const save = vi.fn(async () => makeSkill({ id: "s1" }));
    resetStore(makeApi({ safetyScan: scan, saveSafetyReport: save }));
    useSkillStore.setState({
      skills: [makeSkill({ id: "s1", content: "body" })],
      selectedSkillId: "s1",
    });

    await useSkillStore.getState().scanSafety();

    expect(scan).toHaveBeenCalledWith("body");
    expect(save).toHaveBeenCalledWith("s1", report);
    expect(useSkillStore.getState().safetyReport).toEqual(report);
  });

  it("selectSelectedSkill() resolves the open skill from the list", () => {
    resetStore(makeApi());
    const s1 = makeSkill({ id: "s1" });
    useSkillStore.setState({ skills: [s1], selectedSkillId: "s1" });
    expect(selectSelectedSkill(useSkillStore.getState())).toEqual(s1);
    useSkillStore.setState({ selectedSkillId: "missing" });
    expect(selectSelectedSkill(useSkillStore.getState())).toBeNull();
  });

  it("selectFilteredSkills() applies the keyword filter (Req 9.3)", () => {
    resetStore(makeApi());
    useSkillStore.setState({
      skills: [
        makeSkill({ id: "a", name: "Alpha" }),
        makeSkill({ id: "b", name: "Beta" }),
      ],
      keyword: "alph",
    });
    expect(selectFilteredSkills(useSkillStore.getState()).map((s) => s.id)).toEqual([
      "a",
    ]);
  });
});
