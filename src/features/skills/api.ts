/**
 * Thin command wrappers for the skill-management view (Req 9–13). Every call is
 * routed through the Runtime_Bridge (Req 3.1); none touches `@tauri-apps/api`
 * directly. Command names follow the design's `domain.action` convention and
 * argument/field names use the camelCase DTO shapes the backend returns.
 *
 * Several command families are capability-gated by the Runtime_Bridge (local
 * scan/file editing, platform integration, remote store, import). When the
 * capability is unavailable the bridge rejects with a `CAPABILITY_UNAVAILABLE`
 * {@link BridgeError} without calling the backend (Req 3.7); the store surfaces
 * that as a normal error so the UI can degrade gracefully.
 */
import { runtime, type RuntimeBridge } from "../../runtime";
import type {
  CreateSkillInput,
  DiscoveredSkill,
  InstallResult,
  ParsedSkillMd,
  Platform,
  PlatformInstallStatus,
  SafetyReport,
  ScanEntry,
  Skill,
  SkillFile,
  SkillVersion,
  TreeEntry,
  UpdateSkillInput,
} from "./types";

/** The backend command surface this view depends on, grouped for injection. */
export interface SkillApi {
  // CRUD + versioning (Req 9)
  listSkills(): Promise<Skill[]>;
  getSkill(id: string): Promise<Skill>;
  createSkill(input: CreateSkillInput): Promise<Skill>;
  updateSkill(id: string, patch: UpdateSkillInput): Promise<Skill>;
  deleteSkill(id: string): Promise<void>;

  listVersions(skillId: string): Promise<SkillVersion[]>;
  createVersion(skillId: string, note?: string): Promise<SkillVersion>;
  rollbackVersion(skillId: string, version: number): Promise<Skill>;
  deleteVersion(versionId: string): Promise<void>;

  // SKILL.md parsing / serialization / import (Req 10)
  parseMd(content: string): Promise<ParsedSkillMd>;
  serializeMd(parsed: ParsedSkillMd): Promise<string>;
  importSkill(json: string): Promise<Skill>;

  // Local repository sync (Req 11)
  localScan(locations?: string[]): Promise<ScanEntry[]>;
  localTree(repoPath: string): Promise<TreeEntry[]>;
  localRead(repoPath: string, relativePath: string): Promise<string>;
  localWrite(repoPath: string, relativePath: string, content: string): Promise<void>;
  localMkdir(repoPath: string, relativePath: string): Promise<void>;
  localRename(repoPath: string, fromRelativePath: string, toRelativePath: string): Promise<void>;
  localDelete(repoPath: string, relativePath: string): Promise<void>;
  localSync(skillId: string, repoPath: string): Promise<Skill>;

  // Platform integration (Req 12)
  listPlatforms(): Promise<Platform[]>;
  detectPlatforms(): Promise<string[]>;
  installSkill(platformId: string, skillName: string, files: SkillFile[]): Promise<InstallResult>;
  uninstallSkill(platformId: string, skillName: string): Promise<void>;
  platformStatus(skillName: string): Promise<PlatformInstallStatus[]>;

  // Safety scanning + remote fetch (Req 13)
  safetyScan(content: string): Promise<SafetyReport>;
  saveSafetyReport(skillId: string, report: SafetyReport): Promise<Skill>;
  fetchRemoteContent(url: string): Promise<string>;
  scanRemoteRepo(listingUrl: string): Promise<DiscoveredSkill[]>;
}

/**
 * Builds the {@link SkillApi} bound to a Runtime_Bridge (the live `runtime` by
 * default). Tests inject a fake bridge to drive the view without a backend.
 */
export function createSkillApi(bridge: RuntimeBridge = runtime): SkillApi {
  return {
    listSkills: () => bridge.invoke<Skill[]>("skill.list"),
    getSkill: (id) => bridge.invoke<Skill>("skill.get", { id }),
    createSkill: (input) => bridge.invoke<Skill>("skill.create", { input }),
    updateSkill: (id, patch) => bridge.invoke<Skill>("skill.update", { id, patch }),
    deleteSkill: (id) => bridge.invoke<void>("skill.delete", { id }),

    listVersions: (skillId) =>
      bridge.invoke<SkillVersion[]>("skill.version.list", { skillId }),
    createVersion: (skillId, note) =>
      bridge.invoke<SkillVersion>("skill.version.create", { skillId, note }),
    rollbackVersion: (skillId, version) =>
      bridge.invoke<Skill>("skill.version.rollback", { skillId, version }),
    deleteVersion: (versionId) =>
      bridge.invoke<void>("skill.version.delete", { id: versionId }),

    parseMd: (content) => bridge.invoke<ParsedSkillMd>("skill.parseMd", { content }),
    serializeMd: (parsed) => bridge.invoke<string>("skill.serializeMd", { parsed }),
    importSkill: (json) => bridge.invoke<Skill>("skill.import", { json }),

    localScan: (locations) =>
      bridge.invoke<ScanEntry[]>("skill.local.scan", { locations }),
    localTree: (repoPath) =>
      bridge.invoke<TreeEntry[]>("skill.local.tree", { repoPath }),
    localRead: (repoPath, relativePath) =>
      bridge.invoke<string>("skill.local.read", { repoPath, relativePath }),
    localWrite: (repoPath, relativePath, content) =>
      bridge.invoke<void>("skill.local.write", { repoPath, relativePath, content }),
    localMkdir: (repoPath, relativePath) =>
      bridge.invoke<void>("skill.local.mkdir", { repoPath, relativePath }),
    localRename: (repoPath, fromRelativePath, toRelativePath) =>
      bridge.invoke<void>("skill.local.rename", {
        repoPath,
        fromRelativePath,
        toRelativePath,
      }),
    localDelete: (repoPath, relativePath) =>
      bridge.invoke<void>("skill.local.delete", { repoPath, relativePath }),
    localSync: (skillId, repoPath) =>
      bridge.invoke<Skill>("skill.local.sync", { skillId, repoPath }),

    listPlatforms: () => bridge.invoke<Platform[]>("skill.platform.list"),
    detectPlatforms: () => bridge.invoke<string[]>("skill.platform.detect"),
    installSkill: (platformId, skillName, files) =>
      bridge.invoke<InstallResult>("skill.platform.install", {
        platformId,
        skillName,
        files,
      }),
    uninstallSkill: (platformId, skillName) =>
      bridge.invoke<void>("skill.platform.uninstall", { platformId, skillName }),
    platformStatus: (skillName) =>
      bridge.invoke<PlatformInstallStatus[]>("skill.platform.status", { skillName }),

    safetyScan: (content) =>
      bridge.invoke<SafetyReport>("skill.safety.scan", { content }),
    saveSafetyReport: (skillId, report) =>
      bridge.invoke<Skill>("skill.safety.save", { skillId, report }),
    fetchRemoteContent: (url) =>
      bridge.invoke<string>("skill.remote.fetchContent", { url }),
    scanRemoteRepo: (listingUrl) =>
      bridge.invoke<DiscoveredSkill[]>("skill.remote.scanRepo", { listingUrl }),
  };
}

/** The production skill API bound to the live Runtime_Bridge. */
export const skillApi: SkillApi = createSkillApi();
