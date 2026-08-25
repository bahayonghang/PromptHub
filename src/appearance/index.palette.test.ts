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

function hslToRgb(value: string): [number, number, number] {
  const match = /([\d.]+)\s+([\d.]+)%\s+([\d.]+)%/.exec(value);
  if (!match) throw new Error(`Invalid HSL token: ${value}`);
  const h = Number(match[1]) / 360;
  const s = Number(match[2]) / 100;
  const l = Number(match[3]) / 100;
  if (s === 0) return [l, l, l];
  const hue = (p: number, q: number, t: number) => {
    let channel = t;
    if (channel < 0) channel += 1;
    if (channel > 1) channel -= 1;
    if (channel < 1 / 6) return p + (q - p) * 6 * channel;
    if (channel < 1 / 2) return q;
    if (channel < 2 / 3) return p + (q - p) * (2 / 3 - channel) * 6;
    return p;
  };
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  return [hue(p, q, h + 1 / 3), hue(p, q, h), hue(p, q, h - 1 / 3)];
}

function luminance(value: string): number {
  return hslToRgb(value)
    .map((channel) => (channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4))
    .reduce((sum, channel, index) => sum + channel * [0.2126, 0.7152, 0.0722][index], 0);
}

function contrast(a: string, b: string): number {
  const [lighter, darker] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (lighter + 0.05) / (darker + 0.05);
}

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

  it("keeps semantic text pairs at WCAG AA contrast in every variant", () => {
    const pairs = [
      ["--background", "--foreground"],
      ["--card", "--card-foreground"],
      ["--popover", "--popover-foreground"],
      ["--muted", "--muted-foreground"],
      ["--secondary", "--secondary-foreground"],
      ["--sidebar", "--sidebar-foreground"],
    ] as const;
    const failures: string[] = [];
    for (const flavor of FLAVORS) {
      for (const [background, foreground] of pairs) {
        const ratio = contrast(
          FLAVOR_OVERRIDES[flavor][background],
          FLAVOR_OVERRIDES[flavor][foreground],
        );
        if (ratio < 4.5) failures.push(`${flavor} ${foreground} on ${background}: ${ratio}`);
      }
    }
    expect(failures).toEqual([]);
  });

  it("maps the Violet accent to the design HSL and a contrast-passing foreground", () => {
    expect(ACCENT_PALETTE.dark.Violet["--primary"]).toBe("248 89% 73%");
    expect(ACCENT_PALETTE.light.Violet["--primary"]).toBe("247 67% 59%");
    expect(
      contrast(ACCENT_PALETTE.dark.Violet["--primary"], ACCENT_PALETTE.dark.Violet["--primary-foreground"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrast(ACCENT_PALETTE.light.Violet["--primary"], ACCENT_PALETTE.light.Violet["--primary-foreground"]),
    ).toBeGreaterThanOrEqual(4.5);
  });

  it("keeps every primary accent foreground at WCAG AA contrast", () => {
    const failures: string[] = [];
    for (const base of BASES) {
      for (const accent of ACCENT_COLORS) {
        const tokens = ACCENT_PALETTE[base][accent];
        const ratio = contrast(tokens["--primary"], tokens["--primary-foreground"]);
        if (ratio < 4.5) failures.push(`${base} ${accent}: ${ratio}`);
      }
    }
    expect(failures).toEqual([]);
  });

  it("keeps the focus ring at non-text contrast in every variant", () => {
    const failures: string[] = [];
    for (const flavor of FLAVORS) {
      const base = FLAVOR_BASE[flavor];
      for (const accent of ACCENT_COLORS) {
        const ratio = contrast(
          ACCENT_PALETTE[base][accent]["--ring"],
          FLAVOR_OVERRIDES[flavor]["--background"],
        );
        if (ratio < 3) failures.push(`${flavor} ${accent}: ${ratio}`);
      }
    }
    expect(failures).toEqual([]);
  });
});
