/**
 * Settings view (Req 22.3). The real implementation — general preferences,
 * security (master password), sync (WebDAV/S3, export, backups), and the
 * data-path preview/apply UI — lives under `src/features/settings`. This module
 * re-exports it so the application shell's view registry (which imports from
 * `../views/SettingsView`) resolves to the full view (task 22.4).
 */
export { SettingsView } from "../../features/settings/SettingsView";
