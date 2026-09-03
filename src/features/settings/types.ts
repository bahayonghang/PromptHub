/**
 * Frontend domain types for the settings view (Requirements 15, 17, 19).
 *
 * These mirror the Command_Layer DTOs the Tauri_Backend returns. The Rust
 * structs derive `#[serde(rename_all = "camelCase")]`, so every field below is
 * the camelCase form of its `snake_case` Rust counterpart in
 * `src-tauri/src/models/settings.rs` and the `security`, `sync`, and `data_path`
 * services.
 */

// ===========================================================================
// Settings (Req 19.1, 19.2) — mirrors `models::Settings`
// ===========================================================================

/** Backup/sync transport configuration stored in {@link Settings} (Req 17). */
export interface SyncSettings {
  enabled: boolean;
  /** Provider kind: `manual` | `webdav` | `self-hosted` | `s3`. */
  provider: string;
  endpoint?: string | null;
  username?: string | null;
  password?: string | null;
  remotePath?: string | null;
  autoSync?: boolean | null;
  lastSyncAt?: string | null;
}

/** Master-password / lock state summary embedded in {@link Settings} (Req 15.1). */
export interface SecuritySettingsSummary {
  masterPasswordConfigured: boolean;
  unlocked: boolean;
}

/** Persisted application settings as returned by `settings.get` (Req 19.1). */
export interface Settings {
  /** Theme selection: `light` | `dark` | `system`. */
  theme: string;
  /** UI language: `en` | `zh` | `zh-TW` | `ja` | `fr` | `de` | `es`. */
  language: string;
  autoSave: boolean;
  tagFilterMode?: string | null;
  promptTagCatalog?: string[] | null;
  defaultFolderId?: string | null;
  backgroundImageFileName?: string | null;
  backgroundImageOpacity?: number | null;
  backgroundImageBlur?: number | null;
  lastManualBackupAt?: string | null;
  lastManualBackupVersion?: string | null;
  sync?: SyncSettings | null;
  /** Update channel: `stable` | `preview`. */
  updateChannel?: string | null;
  launchAtStartup?: boolean | null;
  minimizeOnLaunch?: boolean | null;
  /**
   * When true, AI and sync HTTP may target private, loopback, or link-local
   * addresses. Absent or false keeps the public-only pin.
   */
  allowPrivateNetwork?: boolean | null;
  /** Accepted on `settings.update` patches; `settings.get` never returns it. */
  githubToken?: string | null;
  /** Whether a GitHub token is stored. `settings.get` reports this instead of the secret. */
  hasGithubToken?: boolean | null;
  /** Whether `sync.password` is stored. `settings.get` reports this instead of the secret. */
  hasSyncPassword?: boolean | null;
  security?: SecuritySettingsSummary | null;
  /** Theme flavor: Latte | Frappé | Macchiato | Mocha | Claude Light | Claude Dark. */
  flavor?: string | null;
  /** Theme family, independent from the light/dark/system color mode. */
  themeFamily?: string | null;
  /** Catppuccin variant used whenever the effective color mode is dark. */
  catppuccinDarkVariant?: string | null;
  /** Named accent color (one of the 14 Accent_Color names). */
  accentColor?: string | null;
  /** Display font family from the Font_Catalog. */
  displayFont?: string | null;
  /** Body font family from the Font_Catalog. */
  bodyFont?: string | null;
  /** Ordered interface font family names; the runtime appends safe fallbacks. */
  interfaceFontStack?: string[] | null;
  /** Font scale preset: Small | Default | Large | Extra Large. */
  fontScale?: string | null;
  /** Density preset: Compact | Default | Comfortable. */
  density?: string | null;
}

/** A partial settings patch for `settings.update`; only supplied fields change (Req 19.2). */
export type SettingsPatch = Partial<Settings>;

// ===========================================================================
// Security (Req 15) — mirrors `services::security::SecurityStatus`
// ===========================================================================

/** Security status returned by `security.status` (Req 15.1). */
export interface SecurityStatus {
  hasMasterPassword: boolean;
  isLocked: boolean;
}

// ===========================================================================
// Data path (Req 19.3–19.10) — mirrors `services::data_path`
// ===========================================================================

/** Kind of a discovered PromptHub data marker (Req 19.4). */
export type MarkerKind = "file" | "directory" | "other";

/** A PromptHub data marker discovered at an inspected path (Req 19.4, 19.7). */
export interface DataMarker {
  name: string;
  path: string;
  kind: MarkerKind;
}

/** Active data-path status from `data.getStatus` (Req 19.3). */
export interface DataPathStatus {
  activePath: string;
  /** The configured path awaiting a restart, when one differs from the active dir. */
  configuredPath?: string | null;
  restartRequired: boolean;
}

/** The three data-path apply actions (Req 19.5, 19.10). */
export type DataPathAction = "migrate" | "switch" | "overwrite";

/** Read-only preview of a data-path change from `data.previewChange` (Req 19.4). */
export interface PreviewResult {
  targetPath: string;
  exists: boolean;
  hasPromptHubData: boolean;
  isCurrent: boolean;
  /** Recommended action: `migrate` (target empty) or `switch` (target has data). */
  recommendedAction: string;
  markers: DataMarker[];
  /** One-time token required by the matching `data.applyChange`. */
  confirmToken: string;
}

/** Result of `data.applyChange` / `data.recoveryApply` (Req 19.5, 19.8). */
export interface ApplyResult {
  restartRequired: boolean;
  configuredPath: string;
}

/** A recoverable data source from `data.recoveryScan` (Req 19.6). */
export interface RecoverySource {
  path: string;
  markers: DataMarker[];
}

/** Read-only recovery preview from `data.recoveryPreview` (Req 19.7). */
export interface RecoveryPreview {
  sourcePath: string;
  exists: boolean;
  hasPromptHubData: boolean;
  markers: DataMarker[];
  /** One-time token required by the matching `data.recoveryApply`. */
  confirmToken: string;
}

// ===========================================================================
// Sync / backup (Req 17) — mirrors `services::sync`
// ===========================================================================

/** WebDAV server configuration sent to `webdav.test` (Req 17.1, 17.13). */
export interface WebDavConfig {
  url: string;
  username: string;
  password: string;
}

/** S3 bucket configuration sent to `s3.test` (Req 17.3, 17.13). */
export interface S3Config {
  endpoint: string;
  region: string;
  bucket: string;
  accessKeyId: string;
  secretAccessKey: string;
}

/** Explicit pass/fail outcome of a connection test (Req 17.1, 17.3). */
export interface ConnectionTestResult {
  success: boolean;
  message: string;
}

/** The selectable export categories for `data.exportZip` (Req 17.5). */
export interface ExportScope {
  data: boolean;
  media: boolean;
  rule: boolean;
}

/** Outcome of `data.exportZip` (Req 17.5, 17.11). */
export interface ExportResult {
  canceled: boolean;
  filePath?: string | null;
}

/** A backup entry from `backup.list` / `backup.create` (Req 17.6, 17.8). */
export interface BackupEntry {
  id: string;
  createdAt: string;
}

/** Result of `backup.restore` (Req 17.7). */
export interface RestoreResult {
  id: string;
  restartRequired: boolean;
}
