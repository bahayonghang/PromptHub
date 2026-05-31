/**
 * Pure validation helpers for the settings view (Req 15, 17, 19). These mirror
 * the backend's own validation so the UI can give immediate feedback and avoid
 * issuing a command that the backend would reject anyway. The backend remains
 * the source of truth; these checks never replace it.
 */
import type { S3Config, WebDavConfig } from "./types";

/** Inclusive master-password length bounds, matching the backend (Req 15.3). */
export const MIN_PASSWORD_LEN = 8;
export const MAX_PASSWORD_LEN = 128;

/**
 * Validates a master password's length is 8–128 characters inclusive (Req 15.3).
 * Returns `null` when valid, or an i18n key describing the violation.
 */
export function validatePasswordLength(password: string): string | null {
  const len = [...password].length;
  if (len < MIN_PASSWORD_LEN || len > MAX_PASSWORD_LEN) {
    return "settingsView.security.passwordLength";
  }
  return null;
}

/**
 * Validates a new-password pair (the new password and its confirmation). Returns
 * `null` when both are well-formed and equal, or an i18n key for the first
 * problem found (length first, then mismatch).
 */
export function validateNewPassword(
  password: string,
  confirm: string,
): string | null {
  const lengthError = validatePasswordLength(password);
  if (lengthError) return lengthError;
  if (password !== confirm) return "settingsView.security.passwordMismatch";
  return null;
}

/**
 * Validates a WebDAV configuration before a connection test (Req 17.13). The
 * backend rejects an empty or non-http(s) URL; this mirrors that so the UI never
 * issues a request that is certain to fail. Returns `null` when valid, else an
 * i18n key.
 */
export function validateWebDavConfig(config: WebDavConfig): string | null {
  const url = config.url.trim();
  if (url === "") return "settingsView.sync.webdav.urlRequired";
  if (!/^https?:\/\//i.test(url)) return "settingsView.sync.webdav.urlScheme";
  return null;
}

/**
 * Validates an S3 configuration before a connection test (Req 17.13). All five
 * fields are required and the endpoint must be http(s). Returns `null` when
 * valid, else an i18n key for the first missing/invalid field.
 */
export function validateS3Config(config: S3Config): string | null {
  if (config.endpoint.trim() === "") return "settingsView.sync.s3.endpointRequired";
  if (!/^https?:\/\//i.test(config.endpoint.trim()))
    return "settingsView.sync.s3.endpointScheme";
  if (config.region.trim() === "") return "settingsView.sync.s3.regionRequired";
  if (config.bucket.trim() === "") return "settingsView.sync.s3.bucketRequired";
  if (config.accessKeyId.trim() === "") return "settingsView.sync.s3.accessKeyRequired";
  if (config.secretAccessKey.trim() === "")
    return "settingsView.sync.s3.secretKeyRequired";
  return null;
}
