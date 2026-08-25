/**
 * Property-based tests for the Appearance_Controller normalization layer
 * (Requirements 2, 3, 4, 5, 6, 11). Properties P1-P5 from the design's
 * Testing Strategy, each run with fast-check at a minimum of 100 iterations.
 */
import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  ACCENT_COLORS,
  DEFAULT_ACCENT,
  DEFAULT_DENSITY,
  DEFAULT_FONT,
  DEFAULT_FONT_SCALE,
  DENSITIES,
  FLAVOR_BASE,
  FLAVORS,
  FONT_CATALOG,
  FONT_SCALES,
  normalizeAccent,
  normalizeAppearance,
  normalizeDensity,
  normalizeFlavor,
  normalizeFont,
  normalizeFontScale,
  type AppearanceBase,
} from "./index";

/** A scalar/structured generator covering arbitrary unknown inputs. */
const anyValue = fc.oneof(
  fc.string(),
  fc.integer(),
  fc.double(),
  fc.boolean(),
  fc.constant(null),
  fc.constant(undefined),
  fc.object(),
  fc.array(fc.anything()),
);

/** Per-field normalizers paired with their catalog and default. */
const FIELDS = [
  { name: "accent", normalize: normalizeAccent, catalog: ACCENT_COLORS, default: DEFAULT_ACCENT },
  { name: "fontScale", normalize: normalizeFontScale, catalog: FONT_SCALES, default: DEFAULT_FONT_SCALE },
  { name: "density", normalize: normalizeDensity, catalog: DENSITIES, default: DEFAULT_DENSITY },
] as const;

/** Font fields: open-ended, accept any non-empty string. */
const FONT_FIELDS = [
  { name: "displayFont", normalize: normalizeFont, catalog: FONT_CATALOG, default: DEFAULT_FONT },
  { name: "bodyFont", normalize: normalizeFont, catalog: FONT_CATALOG, default: DEFAULT_FONT },
] as const;

describe("Appearance normalization properties (P1-P5)", () => {
  // Feature: settings-appearance-redesign, Property 1: Normalization returns valid values unchanged
  it("P1: returns valid catalog values unchanged", () => {
    for (const field of FIELDS) {
      fc.assert(
        fc.property(fc.constantFrom(...field.catalog), (value) => {
          expect(field.normalize(value)).toBe(value);
        }),
        { numRuns: 100 },
      );
    }
    // Font fields: catalog values pass through unchanged.
    for (const field of FONT_FIELDS) {
      fc.assert(
        fc.property(fc.constantFrom(...field.catalog), (value) => {
          expect(field.normalize(value)).toBe(value);
        }),
        { numRuns: 100 },
      );
    }
    // flavor: a valid persisted flavor wins regardless of the legacy theme.
    fc.assert(
      fc.property(
        fc.constantFrom(...FLAVORS),
        fc.constantFrom("light", "dark", "system", undefined),
        (flavor, legacy) => {
          expect(normalizeFlavor(flavor, legacy)).toBe(flavor);
        },
      ),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 2: Normalization falls back to the documented default
  it("P2: falls back to the documented default for missing/invalid values", () => {
    const invalid = fc.oneof(
      fc.constant(null),
      fc.constant(undefined),
      fc.integer(),
      fc.object(),
      fc.string().filter((s) => !FLAVORS.includes(s as never)),
    );
    for (const field of FIELDS) {
      fc.assert(
        fc.property(
          invalid.filter((v) => !(field.catalog as readonly unknown[]).includes(v)),
          (value) => {
            expect(field.normalize(value)).toBe(field.default);
          },
        ),
        { numRuns: 100 },
      );
    }
    // flavor: invalid value -> Latte when legacy theme is light, else Mocha.
    fc.assert(
      fc.property(
        invalid.filter((v) => !FLAVORS.includes(v as never)),
        (value) => {
          expect(normalizeFlavor(value, "light")).toBe("Latte");
          expect(normalizeFlavor(value, "dark")).toBe("Mocha");
          expect(normalizeFlavor(value, undefined)).toBe("Mocha");
        },
      ),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 3: Normalization is idempotent
  it("P3: normalize(normalize(x)) === normalize(x)", () => {
    for (const field of FIELDS) {
      fc.assert(
        fc.property(anyValue, (value) => {
          const once = field.normalize(value);
          expect(field.normalize(once)).toBe(once);
        }),
        { numRuns: 100 },
      );
    }
    fc.assert(
      fc.property(anyValue, fc.constantFrom("light", "dark", undefined), (value, legacy) => {
        const once = normalizeFlavor(value, legacy);
        expect(normalizeFlavor(once, legacy)).toBe(once);
      }),
      { numRuns: 100 },
    );
    // The whole-appearance normalizer is also idempotent.
    fc.assert(
      fc.property(fc.object(), fc.constantFrom("light", "dark", undefined), (raw, legacy) => {
        const once = normalizeAppearance(raw as Record<string, unknown>, legacy);
        expect(normalizeAppearance(once, legacy)).toEqual(once);
      }),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 4: Normalization is total and closed over valid values
  it("P4: total over arbitrary unknown input and closed over the catalog", () => {
    for (const field of FIELDS) {
      fc.assert(
        fc.property(anyValue, (value) => {
          const result = field.normalize(value);
          expect((field.catalog as readonly unknown[]).includes(result)).toBe(true);
        }),
        { numRuns: 100 },
      );
    }
    fc.assert(
      fc.property(anyValue, anyValue, (value, legacy) => {
        expect(FLAVORS.includes(normalizeFlavor(value, legacy))).toBe(true);
      }),
      { numRuns: 100 },
    );
    // normalizeAppearance always yields a fully valid Appearance.
    fc.assert(
      fc.property(fc.object(), anyValue, (raw, legacy) => {
        const a = normalizeAppearance(raw as Record<string, unknown>, legacy);
        expect(FLAVORS.includes(a.flavor)).toBe(true);
        expect(ACCENT_COLORS.includes(a.accentColor)).toBe(true);
        expect(typeof a.displayFont === "string" && a.displayFont.length > 0).toBe(true);
        expect(typeof a.bodyFont === "string" && a.bodyFont.length > 0).toBe(true);
        expect(FONT_SCALES.includes(a.fontScale)).toBe(true);
        expect(DENSITIES.includes(a.density)).toBe(true);
      }),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 5: Every flavor maps to exactly one base
  it("P5: FLAVOR_BASE is total and yields the expected base for every flavor", () => {
    const EXPECTED: Record<string, AppearanceBase> = {
      Latte: "light",
      Frappé: "dark",
      Macchiato: "dark",
      Mocha: "dark",
      "Claude Light": "light",
      "Claude Dark": "dark",
      "PromptHub Light": "light",
      "PromptHub Dark": "dark",
    };
    fc.assert(
      fc.property(fc.constantFrom(...FLAVORS), (flavor) => {
        const base = FLAVOR_BASE[flavor];
        expect(base === "light" || base === "dark").toBe(true);
        expect(base).toBe(EXPECTED[flavor]);
      }),
      { numRuns: 100 },
    );
  });
});
