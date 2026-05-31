import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  MAX_PASSWORD_LEN,
  MIN_PASSWORD_LEN,
  validateNewPassword,
  validatePasswordLength,
  validateS3Config,
  validateWebDavConfig,
} from "./validation";
import type { S3Config, WebDavConfig } from "./types";

describe("validatePasswordLength (Req 15.3)", () => {
  it("accepts the inclusive boundaries 8 and 128", () => {
    expect(validatePasswordLength("a".repeat(MIN_PASSWORD_LEN))).toBeNull();
    expect(validatePasswordLength("a".repeat(MAX_PASSWORD_LEN))).toBeNull();
  });

  it("rejects just outside the boundaries (7 and 129)", () => {
    expect(validatePasswordLength("a".repeat(MIN_PASSWORD_LEN - 1))).not.toBeNull();
    expect(validatePasswordLength("a".repeat(MAX_PASSWORD_LEN + 1))).not.toBeNull();
  });

  it("matches the 8-128 inclusive rule across random lengths (Req 15.3)", () => {
    fc.assert(
      fc.property(fc.integer({ min: 0, max: 200 }), (len) => {
        const inRange = len >= MIN_PASSWORD_LEN && len <= MAX_PASSWORD_LEN;
        const result = validatePasswordLength("a".repeat(len));
        expect(result === null).toBe(inRange);
      }),
    );
  });
});

describe("validateNewPassword (Req 15.3)", () => {
  it("returns null when both are valid and equal", () => {
    expect(validateNewPassword("password1", "password1")).toBeNull();
  });

  it("flags a length violation before a mismatch", () => {
    // Both too short and mismatched: length error wins.
    expect(validateNewPassword("short", "different")).toBe(
      "settingsView.security.passwordLength",
    );
  });

  it("flags a mismatch when both are long enough but differ", () => {
    expect(validateNewPassword("password1", "password2")).toBe(
      "settingsView.security.passwordMismatch",
    );
  });
});

describe("validateWebDavConfig (Req 17.13)", () => {
  const base: WebDavConfig = { url: "https://dav.example.com", username: "", password: "" };

  it("accepts an http(s) URL", () => {
    expect(validateWebDavConfig(base)).toBeNull();
    expect(validateWebDavConfig({ ...base, url: "http://dav.local" })).toBeNull();
  });

  it("rejects an empty or non-http(s) URL without issuing a request", () => {
    expect(validateWebDavConfig({ ...base, url: "" })).toBe(
      "settingsView.sync.webdav.urlRequired",
    );
    expect(validateWebDavConfig({ ...base, url: "ftp://dav" })).toBe(
      "settingsView.sync.webdav.urlScheme",
    );
  });
});

describe("validateS3Config (Req 17.13)", () => {
  const valid: S3Config = {
    endpoint: "https://s3.amazonaws.com",
    region: "us-east-1",
    bucket: "my-bucket",
    accessKeyId: "AKIA",
    secretAccessKey: "secret",
  };

  it("accepts a fully-populated config", () => {
    expect(validateS3Config(valid)).toBeNull();
  });

  it("rejects each missing field with its own key", () => {
    expect(validateS3Config({ ...valid, endpoint: "" })).toBe(
      "settingsView.sync.s3.endpointRequired",
    );
    expect(validateS3Config({ ...valid, endpoint: "ftp://x" })).toBe(
      "settingsView.sync.s3.endpointScheme",
    );
    expect(validateS3Config({ ...valid, region: "  " })).toBe(
      "settingsView.sync.s3.regionRequired",
    );
    expect(validateS3Config({ ...valid, bucket: "" })).toBe(
      "settingsView.sync.s3.bucketRequired",
    );
    expect(validateS3Config({ ...valid, accessKeyId: "" })).toBe(
      "settingsView.sync.s3.accessKeyRequired",
    );
    expect(validateS3Config({ ...valid, secretAccessKey: "" })).toBe(
      "settingsView.sync.s3.secretKeyRequired",
    );
  });
});
