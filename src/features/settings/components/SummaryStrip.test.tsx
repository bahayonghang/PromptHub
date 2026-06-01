// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import fc from "fast-check";
import "../../../runtime/i18n";
import { deriveSummary, SUMMARY_CATEGORIES, SummaryStrip, type SummaryCategory } from "./SummaryStrip";
import {
  ACCENT_COLORS,
  DEFAULT_ACCENT,
  DEFAULT_APPEARANCE,
  DEFAULT_FONT_SCALE,
  FLAVORS,
  FONT_CATALOG,
  FONT_SCALES,
} from "../../../appearance";
import { DEFAULT_LOCALE, SUPPORTED } from "../../../runtime/i18n";

afterEach(cleanup);


/** The valid value catalog backing each summary category. */
const CATALOG: Record<SummaryCategory, readonly string[]> = {
  flavor: FLAVORS,
  language: SUPPORTED,
  accentColor: ACCENT_COLORS,
  fontScale: FONT_SCALES,
  displayFont: FONT_CATALOG,
  bodyFont: FONT_CATALOG,
};

interface BaseState {
  flavor: string;
  accentColor: string;
  displayFont: string;
  bodyFont: string;
  fontScale: string;
  locale: string;
}

const summaryOf = (b: BaseState) =>
  deriveSummary(
    {
      flavor: b.flavor,
      accentColor: b.accentColor,
      displayFont: b.displayFont,
      bodyFont: b.bodyFont,
      fontScale: b.fontScale,
    },
    b.locale,
  );

const baseArb = fc.record<BaseState>({
  flavor: fc.constantFrom(...FLAVORS),
  accentColor: fc.constantFrom(...ACCENT_COLORS),
  displayFont: fc.constantFrom(...FONT_CATALOG),
  bodyFont: fc.constantFrom(...FONT_CATALOG),
  fontScale: fc.constantFrom(...FONT_SCALES),
  locale: fc.constantFrom(...SUPPORTED),
});

describe("SummaryStrip derivation (Property 12)", () => {
  // Feature: settings-appearance-redesign, Property 12: Summary derivation isolates the changed category and defaults the unset ones
  it("Property 12: isolates the changed category and defaults the unset ones", () => {
    // Clause 1: changing one category updates only that category's value.
    fc.assert(
      fc.property(baseArb, fc.nat(), (base, pick) => {
        const cat = SUMMARY_CATEGORIES[pick % SUMMARY_CATEGORIES.length];
        const before = summaryOf(base);
        const catalog = CATALOG[cat];
        const alt = catalog[(catalog.indexOf(before[cat]) + 1) % catalog.length];
        const key = cat === "language" ? "locale" : cat;
        const after = summaryOf({ ...base, [key]: alt });

        expect(after[cat]).toBe(alt);
        expect(after[cat]).not.toBe(before[cat]);
        for (const other of SUMMARY_CATEGORIES) {
          if (other !== cat) expect(after[other]).toBe(before[other]);
        }
      }),
      { numRuns: 150 },
    );

    // Clause 2: a never-selected (missing/invalid) category resolves to its default.
    const invalid = fc.oneof(
      fc.constant(undefined),
      fc.constant(null),
      fc.integer(),
      fc.boolean(),
      fc.constant("not-a-valid-value"),
    );
    fc.assert(
      fc.property(invalid, invalid, invalid, invalid, invalid, invalid, (f, a, d, b, s, l) => {
        const summary = deriveSummary(
          { flavor: f, accentColor: a, displayFont: d, bodyFont: b, fontScale: s },
          l as string | null | undefined,
        );
        expect(summary.flavor).toBe("Mocha");
        expect(summary.language).toBe(DEFAULT_LOCALE);
        expect(summary.accentColor).toBe(DEFAULT_ACCENT);
        expect(summary.fontScale).toBe(DEFAULT_FONT_SCALE);
        // Font fields accept any non-empty string; only non-string/empty defaults.
        expect(typeof summary.displayFont === "string" && summary.displayFont.length > 0).toBe(true);
        expect(typeof summary.bodyFont === "string" && summary.bodyFont.length > 0).toBe(true);
        for (const c of SUMMARY_CATEGORIES) expect(summary[c].length).toBeGreaterThan(0);
      }),
      { numRuns: 100 },
    );
  });
});

describe("SummaryStrip composition (Req 9.1, 9.2)", () => {
  it("renders six labeled category values via i18n", () => {
    render(<SummaryStrip appearance={DEFAULT_APPEARANCE} locale="en" />);

    expect(screen.getAllByRole("listitem")).toHaveLength(6);
    // Category labels via i18n.
    expect(screen.getByText("Flavor")).toBeTruthy();
    expect(screen.getByText("Language")).toBeTruthy();
    expect(screen.getByText("Accent color")).toBeTruthy();
    // Effective values via i18n (defaults are never blank).
    expect(screen.getByText("Mocha")).toBeTruthy();
    expect(screen.getByText("English")).toBeTruthy();
    expect(screen.getByText("Blue")).toBeTruthy();
    expect(screen.getByText("Default (100%)")).toBeTruthy();
    expect(screen.getAllByText("System")).toHaveLength(2); // display + body font
  });
});
