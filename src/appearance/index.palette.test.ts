/**
 * Property and enumeration tests for the appearance value maps (Requirement 3.2,
 * Task 4.2 / 4.3). Property P7 plus the token-completeness enumeration.
 */
import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  ACCENT_COLORS,
  ACCENT_PALETTE,
  ACCENT_TOKENS,
  FLAVOR_BASE,
  FLAVOR_OVERRIDES,
  FLAVOR_PALETTE_TOKENS,
  FLAVORS,
} from "./index";

const BASES = ["light", "dark"] as const;

describe("Appearance value maps (P7, token completeness)", () => {
  // Feature: settings-appearance-redesign, Property 7: An accent palette is defined for every accent under both bases
  it("P7: every accent x base resolves a complete, non-empty accent token set", () => {
    fc.assert(
      fc.property(fc.constantFrom(...ACCENT_COLORS), fc.constantFrom(...BASES), (accent, base) => {
        const set = ACCENT_PALETTE[base][accent];
        expect(set).toBeDefined();
        for (const token of ACCENT_TOKENS) {
          expect(typeof set[token]).toBe("string");
          expect(set[token].length).toBeGreaterThan(0);
        }
      }),
      { numRuns: 100 },
    );
  });

  // Task 4.3: every token the apply step writes has a value under both bases.
  it("4.3: each accent token is defined and non-empty under both bases", () => {
    for (const base of BASES) {
      for (const accent of ACCENT_COLORS) {
        for (const token of ACCENT_TOKENS) {
          expect(ACCENT_PALETTE[base][accent][token]).toBeTruthy();
        }
      }
    }
  });

  // Task 4.3: each flavor override set covers exactly the expected token subset.
  it("4.3: each flavor override set covers the expected palette token subset", () => {
    const expected = [...FLAVOR_PALETTE_TOKENS].sort();
    for (const flavor of FLAVORS) {
      const overrides = FLAVOR_OVERRIDES[flavor];
      expect(Object.keys(overrides).sort()).toEqual(expected);
      for (const token of FLAVOR_PALETTE_TOKENS) {
        expect(overrides[token]).toBeTruthy();
      }
      // The flavor's base is one of the two known bases (apply targets it).
      expect(BASES.includes(FLAVOR_BASE[flavor])).toBe(true);
    }
  });
});
