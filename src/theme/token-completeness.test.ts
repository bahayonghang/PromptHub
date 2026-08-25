import { describe, expect, it } from "vitest";
import fc from "fast-check";
// @ts-expect-error - node types are not installed in this frontend project; the
// modules are available at runtime under Vitest's Node environment.
import { readFileSync } from "node:fs";
// @ts-expect-error - see above.
import { fileURLToPath } from "node:url";

/**
 * Property 45: Theme token completeness (Requirement 22.1).
 *
 * The design tokens in `src/styles/globals.css` are declared under a default
 * (light) `:root` scope and a `.dark` scope. A component that references a token
 * defined in only one scope would render an unresolved value in the other theme.
 * This test parses the stylesheet and asserts the two scopes declare exactly the
 * same set of custom properties.
 */

const cssPath = fileURLToPath(new URL("../styles/globals.css", import.meta.url));
const css: string = readFileSync(cssPath, "utf8");

/**
 * Extracts the declaration body of the first `selector { ... }` rule. The token
 * scopes in globals.css contain no nested braces, so matching up to the first
 * `}` is sufficient.
 */
function ruleBody(source: string, selector: string): string {
  const pattern = new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\{([^}]*)\\}`);
  const match = pattern.exec(source);
  if (!match) {
    throw new Error(`Could not locate the "${selector}" token scope in globals.css`);
  }
  return match[1];
}

/** Collects the names of CSS custom properties (`--token:`) declared in a body. */
function declaredTokens(body: string): Set<string> {
  const names = new Set<string>();
  const declaration = /(--[\w-]+)\s*:/g;
  let match: RegExpExecArray | null;
  while ((match = declaration.exec(body)) !== null) {
    names.add(match[1]);
  }
  return names;
}

const lightTokens = declaredTokens(ruleBody(css, ":root"));
const darkTokens = declaredTokens(ruleBody(css, ".dark"));
const allTokens = [...new Set([...lightTokens, ...darkTokens])];

describe("theme token completeness (Req 22.1)", () => {
  it("declares at least one token in each scope", () => {
    expect(lightTokens.size).toBeGreaterThan(0);
    expect(darkTokens.size).toBeGreaterThan(0);
  });

  it("declares the identical set of tokens in the light and dark scopes", () => {
    expect([...lightTokens].sort()).toEqual([...darkTokens].sort());
  });

  // Property 45: for any token referenced by a component (i.e. any declared
  // token), both themes define a value for it.
  // **Validates: Requirements 22.1**
  it("defines every referenced token in both the light and dark themes", () => {
    fc.assert(
      fc.property(fc.constantFrom(...allTokens), (token) => {
        expect(lightTokens.has(token)).toBe(true);
        expect(darkTokens.has(token)).toBe(true);
      }),
      { numRuns: 100 },
    );
  });
});

const COLOR_TOKENS = [
  "--background",
  "--foreground",
  "--card",
  "--card-foreground",
  "--popover",
  "--popover-foreground",
  "--surface-inset",
  "--primary",
  "--primary-foreground",
  "--secondary",
  "--secondary-foreground",
  "--muted",
  "--muted-foreground",
  "--muted-foreground-subtle",
  "--accent",
  "--accent-foreground",
  "--accent-alt",
  "--destructive",
  "--destructive-foreground",
  "--success",
  "--warning",
  "--diff-added",
  "--diff-removed",
  "--border",
  "--border-strong",
  "--input",
  "--ring",
  "--sidebar",
  "--sidebar-foreground",
  "--sidebar-accent",
  "--sidebar-border",
] as const;

function tokenMap(body: string): Map<string, string> {
  const values = new Map<string, string>();
  const declaration = /(--[\w-]+)\s*:\s*([^;]+);/g;
  let match: RegExpExecArray | null;
  while ((match = declaration.exec(body)) !== null) {
    values.set(match[1], match[2].trim());
  }
  return values;
}

const CHANNEL_HSL = /^[\d.]+\s+[\d.]+%\s+[\d.]+%$/;
const lightValues = tokenMap(ruleBody(css, ":root"));
const darkValues = tokenMap(ruleBody(css, ".dark"));
const tailwindPath = fileURLToPath(new URL("../../tailwind.config.js", import.meta.url));
const tailwind: string = readFileSync(tailwindPath, "utf8");

describe("PromptHub token format", () => {
  it("declares the extra surface, status, and mono tokens in both scopes", () => {
    for (const token of [
      "--surface-inset",
      "--muted-foreground-subtle",
      "--border-strong",
      "--success",
      "--warning",
      "--accent-alt",
      "--diff-added",
      "--diff-removed",
      "--font-mono",
    ]) {
      expect(lightTokens.has(token)).toBe(true);
      expect(darkTokens.has(token)).toBe(true);
    }
  });

  it("keeps color tokens as channel-only HSL with no hex wrapper", () => {
    for (const token of COLOR_TOKENS) {
      const light = lightValues.get(token);
      const dark = darkValues.get(token);
      expect(light, `light ${token}`).toMatch(CHANNEL_HSL);
      expect(dark, `dark ${token}`).toMatch(CHANNEL_HSL);
      expect(light).not.toMatch(/#|rgba?\(|hsl\(/i);
      expect(dark).not.toMatch(/#|rgba?\(|hsl\(/i);
    }
  });

  it("maps color utilities through hsl(var(--token)) so alpha modifiers resolve", () => {
    expect(tailwind).toContain('DEFAULT: "hsl(var(--primary))"');
    expect(tailwind).toContain('success: "hsl(var(--success))"');
    expect(tailwind).toContain('warning: "hsl(var(--warning))"');
    expect(tailwind).toContain('"accent-alt": "hsl(var(--accent-alt))"');
    expect(tailwind).toContain('"diff-added": "hsl(var(--diff-added))"');
    expect(tailwind).toContain('"diff-removed": "hsl(var(--diff-removed))"');
    expect(tailwind).toContain('"border-strong": "hsl(var(--border-strong))"');
    expect(tailwind).toContain('mono: ["var(--font-mono)"]');
  });

  it("points the mono stack at IBM Plex Mono", () => {
    expect(lightValues.get("--font-mono")).toContain("IBM Plex Mono");
    expect(darkValues.get("--font-mono")).toContain("IBM Plex Mono");
  });

  it("bundles IBM Plex Mono as woff2 and never fetches Google Fonts", () => {
    expect(css).toContain("ibm-plex-mono-latin-400-normal.woff2");
    expect(css).toContain("ibm-plex-mono-latin-500-normal.woff2");
    expect(css).not.toContain("fonts.googleapis.com");
    expect(css).not.toContain("fonts.gstatic.com");
    expect(css).not.toMatch(/ibm-plex-mono-latin-4(?:00|500)-normal\.woff"/);
  });
});
