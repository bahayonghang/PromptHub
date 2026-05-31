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
