/**
 * Frontend domain types for the prompt-editing view (Requirements 6, 7, 8).
 *
 * These mirror the Command_Layer DTOs the Tauri_Backend returns. Rust structs
 * derive `#[serde(rename_all = "camelCase")]`, so every field below is the
 * camelCase form of its `snake_case` Rust counterpart (design "Domain DTOs").
 */

/** A prompt's kind, constrained to exactly one value (Req 6.6). */
export type PromptType = "text" | "image" | "video";

/** All prompt types in display order, used for validation and the type picker. */
export const PROMPT_TYPES: readonly PromptType[] = ["text", "image", "video"];

export interface PromptTypeDefinition {
  id: string;
  name: string;
  baseKind: PromptType;
  createdAt: string;
}

export interface PromptTypeSnapshot {
  id: string;
  name: string;
  baseKind: PromptType;
}

export interface CreatePromptTypeInput {
  name: string;
  baseKind: PromptType;
}

/** The input control a variable renders as when a prompt is filled in. */
export type VariableType = "text" | "textarea" | "number" | "select";

/** All variable types in display order. */
export const VARIABLE_TYPES: readonly VariableType[] = [
  "text",
  "textarea",
  "number",
  "select",
];

/** A single `{{name}}` placeholder a prompt declares (Req 6.7). */
export interface Variable {
  name: string;
  type: VariableType;
  label?: string;
  defaultValue?: string;
  options?: string[];
  required: boolean;
}

export type PromptMessageRole = "system" | "user" | "assistant";

export interface PromptMessage {
  role: PromptMessageRole;
  content: string;
}

/** A stored prompt as returned by `prompt.get` / `prompt.list` (Req 6.2, 6.7). */
export interface Prompt {
  id: string;
  title: string;
  description?: string | null;
  promptType: PromptType;
  typeDefinitionId?: string | null;
  systemPrompt?: string | null;
  userPrompt: string;
  messages: PromptMessage[];
  variables: Variable[];
  tags: string[];
  folderId?: string | null;
  images: string[];
  videos: string[];
  isFavorite: boolean;
  isPinned: boolean;
  isPrivate: boolean;
  isLocked: boolean;
  currentVersion: number;
  usageCount: number;
  source?: string | null;
  notes?: string | null;
  lastAiResponse?: string | null;
  createdAt: string;
  updatedAt: string;
}

/** Provenance recorded for an immutable prompt revision. */
export type PromptRevisionSource =
  | "create"
  | "save"
  | "manual"
  | "rollback"
  | "import"
  | "replace";

/** A complete revision snapshot returned by `version.list`. */
export interface PromptVersion {
  id: string;
  promptId: string;
  version: number;
  systemPrompt?: string | null;
  userPrompt: string;
  messages: PromptMessage[];
  variables: Variable[];
  title: string;
  description?: string | null;
  promptType: PromptType;
  typeDefinitionId?: string | null;
  typeDefinition?: PromptTypeSnapshot | null;
  tags: string[];
  folderId?: string | null;
  images: string[];
  videos: string[];
  isFavorite: boolean;
  isPinned: boolean;
  isPrivate: boolean;
  source?: string | null;
  notes?: string | null;
  note?: string | null;
  aiResponse?: string | null;
  sourceAction: PromptRevisionSource;
  parentRevisionId?: string | null;
  createdAt: string;
}

/**
 * A folder as returned by `folder.list` (Req 8.2). `sortOrder` is the camelCase
 * form of the schema's `sort_order` column; siblings render in ascending order.
 */
export interface Folder {
  id: string;
  name: string;
  icon?: string | null;
  parentId?: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt?: string | null;
}

/** Arguments for `prompt.create` (Req 6.1). */
export interface CreatePromptInput {
  title: string;
  description?: string;
  promptType?: PromptType;
  typeDefinitionId?: string | null;
  systemPrompt?: string;
  userPrompt: string;
  messages?: PromptMessage[];
  variables?: Variable[];
  tags?: string[];
  folderId?: string | null;
  images?: string[];
  videos?: string[];
  source?: string;
  notes?: string;
  isPrivate?: boolean;
}

/** Partial patch for `prompt.update`; only supplied fields change (Req 6.4). */
export interface UpdatePromptInput {
  title?: string;
  description?: string;
  promptType?: PromptType;
  typeDefinitionId?: string | null;
  systemPrompt?: string;
  userPrompt?: string;
  messages?: PromptMessage[];
  variables?: Variable[];
  tags?: string[];
  folderId?: string | null;
  images?: string[];
  videos?: string[];
  isFavorite?: boolean;
  isPinned?: boolean;
  isPrivate?: boolean;
  source?: string;
  notes?: string;
}

/** Sortable fields a search query may order by (Req 5.5). */
export type SortField = "title" | "createdAt" | "updatedAt" | "usageCount";

/** Sort direction (Req 5.5). */
export type SortOrder = "asc" | "desc";

/** A prompt search query combining keyword and filters (Req 5.3–5.9). */
export interface SearchQuery {
  keyword?: string;
  tags?: string[];
  folderId?: string;
  isFavorite?: boolean;
  sortBy?: SortField;
  sortOrder?: SortOrder;
  limit?: number;
  offset?: number;
}

/** Counted deterministic page returned by `prompt.search`. */
export interface PromptPage {
  items: Prompt[];
  total: number;
  limit: number;
  offset: number;
  hasMore: boolean;
}

export interface UnexpandedReference {
  tokenTitle: string;
  reason: string;
}

export interface PromptCopyResult {
  systemPrompt?: string | null;
  userPrompt: string;
  messages: PromptMessage[];
  unexpanded: UnexpandedReference[];
}

export interface OutgoingReference {
  targetPromptId: string | null;
  targetTitle: string | null;
  tokenTitle: string;
  resolution: string;
}

export interface IncomingReference {
  sourcePromptId: string;
  sourceTitle: string;
  tokenTitle: string;
  resolution: string;
}

export interface ReferenceList {
  outgoing: OutgoingReference[];
  incoming: IncomingReference[];
}

export type ImportConflictPolicy = "skip" | "duplicate" | "replace";

export interface BundlePreview {
  formatVersion: number;
  prompts: number;
  revisions: number;
  folders: number;
  mediaFiles: number;
  additions: number;
  conflicts: number;
  privatePrompts: number;
  typeDefinitionAdditions: number;
  typeDefinitionConflicts: number;
}

export interface PortableExportResult {
  filePath: string;
  prompts: number;
  revisions: number;
  mediaFiles: number;
}

export interface PortableImportResult {
  added: number;
  skipped: number;
  replaced: number;
  backupId: string;
}

/** Arguments for `folder.create` (Req 8.1). */
export interface CreateFolderInput {
  name: string;
  icon?: string;
  parentId?: string | null;
}

/** Partial patch for `folder.update`; only supplied fields change (Req 8.3). */
export interface UpdateFolderInput {
  name?: string;
  icon?: string;
  parentId?: string | null;
}
