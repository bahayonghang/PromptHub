/**
 * Frontend domain types for the skill-management view (Requirements 9–13).
 *
 * These mirror the Command_Layer DTOs the Tauri_Backend returns. Rust structs
 * derive `#[serde(rename_all = "camelCase")]`, so every field below is the
 * camelCase form of its `snake_case` Rust counterpart (`src-tauri/src/models`
 * and the `skill_*` services).
 */

/** Overall safety classification of a skill scan (Req 13.1). */
export type SafetyLevel = "safe" | "warn" | "high-risk" | "blocked";

/** All safety levels in ascending-risk order, used for the safety badge. */
export const SAFETY_LEVELS: readonly SafetyLevel[] = [
  "safe",
  "warn",
  "high-risk",
  "blocked",
];

/** Severity of an individual safety finding (Req 13.1). */
export type Severity = "info" | "warn" | "high";

/** A stored skill as returned by `skill.get` / `skill.list` (Req 9.2, 9.3). */
export interface Skill {
  id: string;
  name: string;
  description?: string | null;
  content?: string | null;
  protocolType: string;
  version?: string | null;
  author?: string | null;
  tags: string[];
  isFavorite: boolean;
  sourceUrl?: string | null;
  sourceId?: string | null;
  sourceLabel?: string | null;
  sourceBranch?: string | null;
  sourceDirectory?: string | null;
  canonicalSkillPath?: string | null;
  localRepoPath?: string | null;
  directoryFingerprint?: string | null;
  iconUrl?: string | null;
  iconEmoji?: string | null;
  iconBackground?: string | null;
  category: string;
  isBuiltin: boolean;
  registrySlug?: string | null;
  contentUrl?: string | null;
  safetyLevel?: SafetyLevel | null;
  safetyScore?: number | null;
  safetyReport?: SafetyReport | null;
  safetyScannedAt?: string | null;
  currentVersion: number;
  versionTrackingEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/** A single file captured in a multi-file skill version snapshot (Req 9.7). */
export interface SkillFileSnapshot {
  relativePath: string;
  content: string;
}

/** A skill version snapshot returned by `skill.version.list` (Req 9.6, 9.7). */
export interface SkillVersion {
  id: string;
  skillId: string;
  version: number;
  content?: string | null;
  filesSnapshot?: SkillFileSnapshot[] | null;
  note?: string | null;
  createdAt: string;
}

/** A single safety finding within a {@link SafetyReport} (Req 13.1). */
export interface SafetyFinding {
  code: string;
  severity: Severity;
  title: string;
  detail: string;
  evidence?: string | null;
}

/** A skill safety report returned by `skill.safety.scan` (Req 13.1). */
export interface SafetyReport {
  level: SafetyLevel;
  findings: SafetyFinding[];
  score?: number | null;
  summary?: string | null;
}

/**
 * A supported platform with its resolved filesystem locations, as returned by
 * `skill.platform.list` (Req 12.1).
 */
export interface Platform {
  id: string;
  name: string;
  isCustom: boolean;
  rootDir: string;
  skillsDir: string;
}

/** Per-platform install state for a skill from `skill.platform.status` (Req 12.5). */
export interface PlatformInstallStatus {
  platformId: string;
  installed: boolean;
  skillsDir: string;
}

/** Result of a successful `skill.platform.install` (Req 12.3). */
export interface InstallResult {
  platformId: string;
  installed: boolean;
}

/** One file in a skill's complete file set sent to `skill.platform.install` (Req 12.3). */
export interface SkillFile {
  relativePath: string;
  content: string;
}

/** A parsed SKILL.md document returned by `skill.parseMd` (Req 10.1). */
export interface ParsedSkillMd {
  frontmatter: Record<string, unknown>;
  body: string;
}

/** A skill discovered by a local scan via `skill.local.scan` (Req 11.1). */
export interface ScanEntry {
  repoPath: string;
  skillMdRelativePath: string;
}

/** One node in a skill repository file tree from `skill.local.tree` (Req 11.3). */
export interface TreeEntry {
  relativePath: string;
  isDir: boolean;
}

/** A SKILL.md entry discovered by `skill.remote.scanRepo` (Req 13.4). */
export interface DiscoveredSkill {
  path: string;
  directory: string;
}

/** Arguments for `skill.create` (Req 9.1). `name` is the only required field. */
export interface CreateSkillInput {
  name: string;
  description?: string;
  content?: string;
  protocolType?: string;
  version?: string;
  author?: string;
  tags?: string[];
  isFavorite?: boolean;
  category?: string;
  sourceUrl?: string;
  versionTrackingEnabled?: boolean;
}

/** Partial patch for `skill.update`; only supplied fields change (Req 9.4). */
export interface UpdateSkillInput {
  name?: string;
  description?: string;
  content?: string;
  protocolType?: string;
  version?: string;
  author?: string;
  tags?: string[];
  isFavorite?: boolean;
  category?: string;
  sourceUrl?: string;
  versionTrackingEnabled?: boolean;
}
