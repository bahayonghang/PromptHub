/**
 * View-state store for the skill-management view (Req 9–13). Holds the loaded
 * skills, the current selection and its version history, the supported platforms
 * with detection and per-skill install status, and the latest safety report.
 * All backend access goes through an injectable {@link SkillApi} (default: the
 * live bridge-bound API) so the store can be driven in tests without a backend
 * (Req 3.1).
 *
 * Capability-gated families (local scan/editing, platform integration, remote
 * store, import) may reject with a `CAPABILITY_UNAVAILABLE` error (Req 3.7). The
 * store surfaces that message like any other error and clears the affected
 * collection so the UI degrades gracefully rather than crashing.
 */
import { create } from "zustand";
import { skillApi, type SkillApi } from "./api";
import type {
  Platform,
  PlatformInstallStatus,
  SafetyReport,
  Skill,
  SkillVersion,
} from "./types";

/** A `BridgeError`-shaped failure surfaced to the view (Req 3.5). */
function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/** The error code carried by a `BridgeError`, or `null` when not present. */
function errorCode(err: unknown): string | null {
  if (err && typeof err === "object" && "code" in err) {
    return String((err as { code: unknown }).code);
  }
  return null;
}

/** Filters a skill list by a case-insensitive keyword over name/description/tags. */
export function filterSkills(skills: Skill[], keyword: string): Skill[] {
  const term = keyword.trim().toLowerCase();
  if (term === "") return skills;
  return skills.filter((skill) => {
    const haystack = [
      skill.name,
      skill.description ?? "",
      skill.author ?? "",
      ...skill.tags,
    ]
      .join("\n")
      .toLowerCase();
    return haystack.includes(term);
  });
}

interface SkillStoreState {
  /** Backend command surface; injectable so tests can supply a fake. */
  api: SkillApi;

  skills: Skill[];
  /** Free-text filter applied to the list client-side (Req 9.3). */
  keyword: string;

  /** The id of the skill open in the editor, or `null` when none is selected. */
  selectedSkillId: string | null;
  /** Version history of the selected skill (Req 9.6), ascending by version. */
  versions: SkillVersion[];

  /** Supported platforms (Req 12.1); empty when integration is unavailable. */
  platforms: Platform[];
  /** Platform ids whose root directory exists on the host (Req 12.2). */
  detectedPlatforms: string[];
  /** Per-platform install status for the selected skill (Req 12.5). */
  installStatus: PlatformInstallStatus[];

  /** The latest safety report for the selected skill (Req 13.1), or `null`. */
  safetyReport: SafetyReport | null;

  loading: boolean;
  error: string | null;
  /** True when the last platform load failed because the capability is gated. */
  platformUnavailable: boolean;

  /** Loads the skill list and the supported platforms (Req 9.3, 12.1). */
  load: () => Promise<void>;
  /** Reloads just the skill list (Req 9.3). */
  refreshSkills: () => Promise<void>;
  /** Sets the client-side keyword filter (Req 9.3). */
  setKeyword: (keyword: string) => void;

  /** Selects a skill and loads its versions, install status, and report. */
  selectSkill: (id: string | null) => Promise<void>;

  /** Creates a skill, refreshes the list, and selects it (Req 9.1). */
  createSkill: (
    input: Parameters<SkillApi["createSkill"]>[0],
  ) => Promise<Skill | null>;
  /** Applies a partial update and refreshes the list (Req 9.4). */
  saveSkill: (
    id: string,
    patch: Parameters<SkillApi["updateSkill"]>[1],
  ) => Promise<Skill | null>;
  /** Deletes a skill and clears the selection if it was selected (Req 9.5). */
  deleteSkill: (id: string) => Promise<void>;

  /** Snapshots the selected skill as a new version (Req 9.7). */
  createVersion: (note?: string) => Promise<void>;
  /** Rolls the selected skill back to a version (Req 9.8). */
  rollbackVersion: (version: number) => Promise<void>;
  /** Deletes a single version from history (Req 9.9). */
  deleteVersion: (versionId: string) => Promise<void>;

  /** Reloads the per-platform install status for the selected skill (Req 12.5). */
  refreshInstallStatus: () => Promise<void>;
  /** Installs the selected skill onto a platform (Req 12.3). */
  installToPlatform: (platformId: string) => Promise<void>;
  /** Uninstalls the selected skill from a platform (Req 12.4). */
  uninstallFromPlatform: (platformId: string) => Promise<void>;

  /** Runs a safety scan on the selected skill and persists it (Req 13.1, 13.2). */
  scanSafety: () => Promise<void>;
}

export const useSkillStore = create<SkillStoreState>((set, get) => ({
  api: skillApi,

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

  load: async () => {
    const { api } = get();
    set({ loading: true, error: null });
    try {
      const skills = await api.listSkills();
      set({ skills, loading: false });
    } catch (err) {
      set({ error: errorMessage(err), loading: false });
    }
    // Platform integration is capability-gated (Req 3.7, 12.1); load it
    // best-effort so a runtime without it still shows the skill list.
    try {
      const [platforms, detectedPlatforms] = await Promise.all([
        api.listPlatforms(),
        api.detectPlatforms(),
      ]);
      set({ platforms, detectedPlatforms, platformUnavailable: false });
    } catch (err) {
      set({
        platforms: [],
        detectedPlatforms: [],
        platformUnavailable: errorCode(err) === "CAPABILITY_UNAVAILABLE",
      });
    }
  },

  refreshSkills: async () => {
    const { api } = get();
    try {
      set({ skills: await api.listSkills() });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  setKeyword: (keyword) => set({ keyword }),

  selectSkill: async (id) => {
    set({
      selectedSkillId: id,
      versions: [],
      installStatus: [],
      safetyReport: null,
    });
    if (id == null) return;
    const { api } = get();
    try {
      const versions = await api.listVersions(id);
      if (get().selectedSkillId === id) set({ versions });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
    // Seed the safety report from the loaded skill, if one is stored.
    const skill = get().skills.find((s) => s.id === id);
    if (skill?.safetyReport && get().selectedSkillId === id) {
      set({ safetyReport: skill.safetyReport });
    }
    await get().refreshInstallStatus();
  },

  createSkill: async (input) => {
    const { api } = get();
    set({ error: null });
    try {
      const skill = await api.createSkill(input);
      await get().refreshSkills();
      await get().selectSkill(skill.id);
      return skill;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  saveSkill: async (id, patch) => {
    const { api } = get();
    set({ error: null });
    try {
      const skill = await api.updateSkill(id, patch);
      await get().refreshSkills();
      return skill;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  deleteSkill: async (id) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.deleteSkill(id);
      if (get().selectedSkillId === id) {
        set({
          selectedSkillId: null,
          versions: [],
          installStatus: [],
          safetyReport: null,
        });
      }
      await get().refreshSkills();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  createVersion: async (note) => {
    const { api, selectedSkillId } = get();
    if (selectedSkillId == null) return;
    set({ error: null });
    try {
      await api.createVersion(selectedSkillId, note);
      set({ versions: await api.listVersions(selectedSkillId) });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  rollbackVersion: async (version) => {
    const { api, selectedSkillId } = get();
    if (selectedSkillId == null) return;
    set({ error: null });
    try {
      await api.rollbackVersion(selectedSkillId, version);
      await get().refreshSkills();
      set({ versions: await api.listVersions(selectedSkillId) });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  deleteVersion: async (versionId) => {
    const { api, selectedSkillId } = get();
    set({ error: null });
    try {
      await api.deleteVersion(versionId);
      if (selectedSkillId != null) {
        set({ versions: await api.listVersions(selectedSkillId) });
      }
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  refreshInstallStatus: async () => {
    const { api, selectedSkillId, skills, platformUnavailable } = get();
    if (selectedSkillId == null || platformUnavailable) {
      set({ installStatus: [] });
      return;
    }
    const skill = skills.find((s) => s.id === selectedSkillId);
    if (skill == null) return;
    try {
      const installStatus = await api.platformStatus(skill.name);
      if (get().selectedSkillId === selectedSkillId) set({ installStatus });
    } catch (err) {
      // A gated capability or a path-confinement rejection should not surface
      // as a blocking error; just leave the status empty (Req 3.7, 12.8).
      set({ installStatus: [] });
      if (errorCode(err) !== "CAPABILITY_UNAVAILABLE") {
        set({ error: errorMessage(err) });
      }
    }
  },

  installToPlatform: async (platformId) => {
    const { api, selectedSkillId, skills } = get();
    if (selectedSkillId == null) return;
    const skill = skills.find((s) => s.id === selectedSkillId);
    if (skill == null) return;
    set({ error: null });
    try {
      // The installed file set is the skill's SKILL.md content (Req 12.3).
      const files = [
        { relativePath: "SKILL.md", content: skill.content ?? "" },
      ];
      await api.installSkill(platformId, skill.name, files);
      await get().refreshInstallStatus();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  uninstallFromPlatform: async (platformId) => {
    const { api, selectedSkillId, skills } = get();
    if (selectedSkillId == null) return;
    const skill = skills.find((s) => s.id === selectedSkillId);
    if (skill == null) return;
    set({ error: null });
    try {
      await api.uninstallSkill(platformId, skill.name);
      await get().refreshInstallStatus();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  scanSafety: async () => {
    const { api, selectedSkillId, skills } = get();
    if (selectedSkillId == null) return;
    const skill = skills.find((s) => s.id === selectedSkillId);
    if (skill == null) return;
    set({ error: null });
    try {
      const report = await api.safetyScan(skill.content ?? "");
      set({ safetyReport: report });
      // Persist the report against the skill so it survives reload (Req 13.2).
      await api.saveSafetyReport(selectedSkillId, report);
      await get().refreshSkills();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },
}));

/** Selects the currently open skill from the loaded list, or `null`. */
export function selectSelectedSkill(state: SkillStoreState): Skill | null {
  if (state.selectedSkillId == null) return null;
  return state.skills.find((s) => s.id === state.selectedSkillId) ?? null;
}

/** Selects the keyword-filtered skill list (Req 9.3). */
export function selectFilteredSkills(state: SkillStoreState): Skill[] {
  return filterSkills(state.skills, state.keyword);
}
