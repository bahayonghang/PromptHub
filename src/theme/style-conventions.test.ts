import { describe, expect, it } from "vitest";
// @ts-expect-error - node types are not installed in this frontend project; the
// modules are available at runtime under Vitest's Node environment.
import { readFileSync, readdirSync, statSync } from "node:fs";
// @ts-expect-error - see above.
import { join } from "node:path";

/**
 * Executable form of the design plan's acceptance criteria (§9).
 *
 * The plan's whole premise is that the token layer was fine but nothing stopped
 * feature code from bypassing it. Fixing the existing violations by hand is
 * pointless if the next PR reintroduces them, so each cleared metric is pinned
 * here at zero.
 */

const SRC = "src";
/** The primitives are the one place allowed to spell these things out. */
const PRIMITIVES = join("src", "components", "ui");

function walk(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (full.endsWith(".tsx") || full.endsWith(".ts")) out.push(full);
  }
  return out;
}

const ALL = walk(SRC);
const TSX = ALL.filter(
  (f) => f.endsWith(".tsx") && !f.includes(".test."),
);
/** Feature code: everything that is not a primitive and not a test. */
const FEATURE = TSX.filter(
  (f) => !f.startsWith(PRIMITIVES) && !f.includes(".test."),
);

function read(file: string) {
  return readFileSync(file, "utf8");
}

/** Reports offenders as `path:line` so a failure points straight at the code. */
function offenders(files: string[], pattern: RegExp): string[] {
  const hits: string[] = [];
  for (const file of files) {
    read(file)
      .split("\n")
      .forEach((line: string, i: number) => {
        // Skip comment lines: the primitives document these patterns in prose.
        const trimmed = line.trim();
        if (trimmed.startsWith("*") || trimmed.startsWith("//")) return;
        if (new RegExp(pattern.source, pattern.flags).test(line)) {
          hits.push(`${file}:${i + 1}`);
        }
      });
  }
  return hits;
}

describe("style conventions", () => {
  it("routes every radius through the token scale", () => {
    // A bare `rounded` is Tailwind's hardcoded 4px and ignores `--radius`, so
    // it does not follow the user's theme.
    expect(offenders(TSX, /\brounded(?![-\w])/)).toEqual([]);
  });

  it("uses the type scale instead of arbitrary pixel sizes", () => {
    expect(offenders(TSX, /text-\[\d+px\]/)).toEqual([]);
  });

  it("keeps a single disabled opacity", () => {
    const values = new Set<string>();
    for (const file of TSX) {
      for (const m of read(file).matchAll(/disabled:opacity-(\d+)/g)) {
        values.add(m[1]);
      }
    }
    expect([...values]).toEqual(["50"]);
  });

  it("has no duplicated iconButtonClass constants", () => {
    expect(offenders(ALL, /const iconButtonClass\s*=/)).toEqual([]);
  });

  it("uses the themed confirm dialog rather than window.confirm", () => {
    // window.confirm is OS-drawn: it ignores the theme and blocks the thread.
    expect(offenders(ALL, /window\.confirm\s*\(/)).toEqual([]);
  });

  it("routes selects through the Select primitive", () => {
    expect(offenders(FEATURE, /<select\b/)).toEqual([]);
  });

  it("uses the semantic type scale rather than Tailwind's raw sizes", () => {
    // text-sm/text-xs carry no line-height or letter-spacing intent; the
    // scale steps (text-body, text-label, ...) do.
    const raw = /\btext-(?:xs|sm|base|xl|2xl|3xl)\b/;
    const allowed = new Set([
      // A font specimen must render at a fixed demonstrative size.
      "src/features/settings/components/SpecimenCard.tsx",
    ]);
    expect(
      offenders(TSX, raw).filter((hit) => !allowed.has(hit.split(":")[0])),
    ).toEqual([]);
  });

  it("declares the focus ring once, in the base layer", () => {
    // A per-component ring on top of the global outline paints both at once.
    expect(offenders(TSX, /focus(?:-visible)?:(?:ring|outline)-/)).toEqual([]);
    const css = read("src/styles/globals.css");
    expect(css).toContain(":focus-visible");
    expect(css).toContain("outline: 2px solid hsl(var(--ring))");
  });

  it("expresses selection through a single state token", () => {
    // bg-primary/10 vs /15 across views made "selected" read differently in
    // each list; --state-selected is the one source of truth.
    const bad: string[] = [];
    for (const file of TSX) {
      read(file)
        .split("\n")
        .forEach((line: string, i: number) => {
          for (const m of line.matchAll(/bg-primary\/(\d+)/g)) {
            // /70 is the usage-bar fill and /90 the primary-button hover;
            // neither denotes selection.
            if (m[1] !== "70" && m[1] !== "90") bad.push(`${file}:${i + 1}`);
          }
        });
    }
    expect(bad).toEqual([]);
  });

  it("keeps class strings short enough to read", () => {
    // A 200-char className is a component that should have been a primitive.
    const bad: string[] = [];
    for (const file of TSX) {
      read(file)
        .split("\n")
        .forEach((line: string, i: number) => {
          for (const m of line.matchAll(/className="([^"]{200,})"/g)) {
            void m;
            bad.push(`${file}:${i + 1}`);
          }
        });
    }
    expect(bad).toEqual([]);
  });

  it("binds every transition to the motion tokens", () => {
    // A bare `transition-colors` silently uses Tailwind's 150ms default,
    // which is unrelated to --dur-fast/--dur-base.
    const bad: string[] = [];
    for (const file of TSX) {
      read(file)
        .split("\n")
        .forEach((line: string, i: number) => {
          if (!/\btransition-(?:colors|opacity|transform|\[)/.test(line)) return;
          if (!/\bduration-(?:fast|base)\b/.test(line)) bad.push(`${file}:${i + 1}`);
        });
    }
    expect(bad).toEqual([]);
  });

  it("only uses alpha modifiers that exist on Tailwind's opacity scale", () => {
    // Off-scale values such as `/14` are silently dropped at build time,
    // leaving the element with no colour at all.
    const bad: string[] = [];
    for (const file of TSX) {
      read(file)
        .split("\n")
        .forEach((line: string, i: number) => {
          for (const m of line.matchAll(
            /\b(?:bg|text|border|ring|from|via|to|fill|stroke)-[a-z0-9-]+\/(\d+)\b/g,
          )) {
            if (Number(m[1]) % 5 !== 0) bad.push(`${file}:${i + 1} ${m[0]}`);
          }
        });
    }
    expect(bad).toEqual([]);
  });
});
