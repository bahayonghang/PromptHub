/**
 * Property-based tests for the Appearance_Controller DOM application
 * (Requirements 2.4, 3.3, 4.2, 4.3, 5.2, 6.2). Properties P6, P8-P11 driven
 * through a fake injected root, so these stay in the Node environment.
 */
import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  ACCENT_COLORS,
  ACCENT_PALETTE,
  ACCENT_TOKENS,
  createAppearanceController,
  DENSITIES,
  DENSITY_RHYTHM,
  FLAVOR_BASE,
  FLAVOR_OVERRIDES,
  FLAVOR_PALETTE_TOKENS,
  FLAVORS,
  FONT_CATALOG,
  FONT_SCALE_PERCENT,
  FONT_SCALES,
  FONT_STACK,
  normalizeAppearance,
  type AppearanceDeps,
} from "./index";

/** A fake root recording class toggles, CSS variables, and attributes. */
function makeFakeRoot() {
  const classes = new Set<string>();
  const vars = new Map<string, string>();
  const setCalls: string[] = [];
  const attrs = new Map<string, string>();
  const root: AppearanceDeps["root"] = {
    classList: {
      add: (t) => void classes.add(t),
      remove: (t) => void classes.delete(t),
    },
    style: {
      setProperty: (name, value) => {
        vars.set(name, value);
        setCalls.push(name);
      },
    },
    setAttribute: (name, value) => void attrs.set(name, value),
  };
  return { root, classes, vars, setCalls, attrs };
}

describe("Appearance DOM application properties (P6, P8-P11)", () => {
  // Feature: settings-appearance-redesign, Property 6: Flavor application sets the correct base and palette
  it("P6: flavor toggles .dark iff its base is dark and writes the palette overrides", () => {
    fc.assert(
      fc.property(fc.constantFrom(...FLAVORS), (flavor) => {
        const fake = makeFakeRoot();
        const controller = createAppearanceController({ root: fake.root });
        controller.applyField("flavor", flavor);
        const dark = FLAVOR_BASE[flavor] === "dark";
        expect(fake.classes.has("dark")).toBe(dark);
        for (const token of FLAVOR_PALETTE_TOKENS) {
          expect(fake.vars.get(token)).toBe(FLAVOR_OVERRIDES[flavor][token]);
        }
      }),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 8: Accent application writes the selected accent's tokens
  it("P8: accent writes --primary/--ring/--accent/--accent-foreground for accent x active base", () => {
    fc.assert(
      fc.property(fc.constantFrom(...ACCENT_COLORS), fc.constantFrom(...FLAVORS), (accent, flavor) => {
        const fake = makeFakeRoot();
        const controller = createAppearanceController({ root: fake.root });
        controller.applyField("flavor", flavor); // establish the active base
        controller.applyField("accentColor", accent);
        const base = FLAVOR_BASE[flavor];
        for (const token of ACCENT_TOKENS) {
          expect(fake.vars.get(token)).toBe(ACCENT_PALETTE[base][accent][token]);
        }
      }),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 9: Font application is per-target and independent
  it("P9: display font sets --font-display leaving --font-body, and vice versa", () => {
    fc.assert(
      fc.property(fc.constantFrom(...FONT_CATALOG), fc.constantFrom(...FONT_CATALOG), (display, body) => {
        // Display font is independent of the body font variable.
        const fd = makeFakeRoot();
        const cd = createAppearanceController({ root: fd.root });
        fd.setCalls.length = 0;
        cd.applyField("displayFont", display);
        expect(fd.vars.get("--font-display")).toBe(FONT_STACK[display]);
        expect(fd.setCalls).not.toContain("--font-body");

        // Body font is independent of the display font variable.
        const fb = makeFakeRoot();
        const cb = createAppearanceController({ root: fb.root });
        fb.setCalls.length = 0;
        cb.applyField("bodyFont", body);
        expect(fb.vars.get("--font-body")).toBe(FONT_STACK[body]);
        expect(fb.setCalls).not.toContain("--font-display");
      }),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 10: Font scale application maps each preset to its multiplier
  it("P10: font scale writes the unitless multiplier (Small 0.9, Default 1, Large 1.1, Extra Large 1.25)", () => {
    fc.assert(
      fc.property(fc.constantFrom(...FONT_SCALES), (scale) => {
        const fake = makeFakeRoot();
        const controller = createAppearanceController({ root: fake.root });
        controller.applyField("fontScale", scale);
        expect(fake.vars.get("--font-scale")).toBe(String(FONT_SCALE_PERCENT[scale] / 100));
      }),
      { numRuns: 100 },
    );
  });

  // Feature: settings-appearance-redesign, Property 11: Density application writes the correct rhythm tokens
  it("P11: density writes --density-padding/--density-gap and the data-density attribute", () => {
    fc.assert(
      fc.property(fc.constantFrom(...DENSITIES), (density) => {
        const fake = makeFakeRoot();
        const controller = createAppearanceController({ root: fake.root });
        controller.applyField("density", density);
        expect(fake.vars.get("--density-padding")).toBe(DENSITY_RHYTHM[density]["--density-padding"]);
        expect(fake.vars.get("--density-gap")).toBe(DENSITY_RHYTHM[density]["--density-gap"]);
        expect(fake.attrs.get("data-density")).toBe(density);
      }),
      { numRuns: 100 },
    );
  });

  // A full apply writes every token group at once and tracks current().
  it("apply() writes the full token set and current() reflects it", () => {
    const fake = makeFakeRoot();
    const controller = createAppearanceController({ root: fake.root });
    const appearance = normalizeAppearance({
      flavor: "Latte",
      accentColor: "Green",
      displayFont: "Inter",
      bodyFont: "JetBrains Mono",
      fontScale: "Large",
      density: "Compact",
    });
    controller.apply(appearance);
    expect(controller.current()).toEqual(appearance);
    expect(fake.vars.get("--font-display")).toBe(FONT_STACK.Inter);
    expect(fake.vars.get("--font-body")).toBe(FONT_STACK["JetBrains Mono"]);
    expect(fake.vars.get("--font-scale")).toBe("1.1");
    expect(fake.attrs.get("data-density")).toBe("Compact");
    expect(fake.classes.has("dark")).toBe(false); // Latte is a light base
  });
});
