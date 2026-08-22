import { describe, expect, it } from "vitest";
import {
  DEFAULT_SCOPES,
  EXPIRY_PRESET_DAYS,
  EXPIRY_PRESETS,
  expiresAtForPreset,
  scopeError,
} from "../../src/pages/TokensPage";

describe("token scope validation", () => {
  it("accepts all-project mode regardless of selection", () => {
    expect(scopeError({ name: "t", scopeMode: "all", specifiedIds: [], scopes: [], expiresAt: "" })).toBeNull();
  });

  it("accepts specified mode with at least one project", () => {
    expect(scopeError({ name: "t", scopeMode: "specified", specifiedIds: [2], scopes: [], expiresAt: "" })).toBeNull();
  });

  it("rejects empty specified mode to avoid corrupting server-side token scope", () => {
    expect(scopeError({ name: "t", scopeMode: "specified", specifiedIds: [], scopes: [], expiresAt: "" })).toContain(
      "至少选择一个项目",
    );
  });
});

describe("token form expiry presets", () => {
  it("defaults new-token permissions to read plus artifacts write", () => {
    expect(DEFAULT_SCOPES).toEqual([
      "metadata:read",
      "artifacts:read",
      "artifacts:write",
    ]);
  });

  it("maps GitHub-style presets to the agreed day counts", () => {
    expect(EXPIRY_PRESETS).toEqual(["week", "month", "half-year", "year", "never"]);
    expect(EXPIRY_PRESET_DAYS.week).toBe(7);
    expect(EXPIRY_PRESET_DAYS.month).toBe(30);
    expect(EXPIRY_PRESET_DAYS["half-year"]).toBe(183);
    expect(EXPIRY_PRESET_DAYS.year).toBe(365);
    expect(EXPIRY_PRESET_DAYS.never).toBeNull();
  });

  it("computes deterministic UTC expiry timestamps and never", () => {
    const now = Date.UTC(2026, 0, 1, 0, 0, 0);
    expect(expiresAtForPreset("week", now)).toBe(new Date(now + 7 * 864e5).toISOString());
    expect(expiresAtForPreset("month", now)).toBe(new Date(now + 30 * 864e5).toISOString());
    expect(expiresAtForPreset("half-year", now)).toBe(new Date(now + 183 * 864e5).toISOString());
    expect(expiresAtForPreset("year", now)).toBe(new Date(now + 365 * 864e5).toISOString());
    expect(expiresAtForPreset("never", now)).toBeNull();
  });
});
