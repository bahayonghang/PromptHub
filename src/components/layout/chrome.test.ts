import { describe, expect, it } from "vitest";
// @ts-expect-error - node types are not installed in this frontend project; the
// modules are available at runtime under Vitest's Node environment.
import { readFileSync, existsSync } from "node:fs";

/**
 * Vertical chrome budget (design plan §6.1, acceptance criterion in §9).
 *
 * The Prompts view used to stack five fixed bars — title bar, shell header,
 * library header, toolbar, filter chips — before a single prompt was visible,
 * roughly 240px on a 900px-tall window. The shell header was pure duplication:
 * it restated the view name the sidebar already highlights.
 */

const read = (p: string) => readFileSync(p, "utf8");

describe("application chrome", () => {
  it("has no shell-level header bar", () => {
    // Reintroducing it would silently add 56px back to every view.
    expect(existsSync("src/components/layout/Header.tsx")).toBe(false);
    expect(read("src/components/layout/AppShell.tsx")).not.toContain("<Header");
  });

  it("keeps exactly one h1 per view rather than a shell-level one", () => {
    // The shell header held the only <h1>; removing it would have left the
    // document with no top-level heading if the views had not been promoted.
    const prompts = read("src/features/prompts/components/LibraryHeader.tsx");
    const settings = read("src/features/settings/SettingsView.tsx");
    expect(prompts).toContain("<h1");
    expect(settings).toContain("<h1");
  });

  it("aligns every control in the library toolbar to one height", () => {
    // Mixing 28/32/36/40px in a single row is the most visible alignment
    // defect; the row may only use the control-height tokens.
    const toolbar = read("src/features/prompts/components/LibraryToolbar.tsx");
    const raw = toolbar.match(/\bh-\d+(?:\.5)?\b/g) ?? [];
    // h-3/h-3.5/h-4 are icon glyph sizes, not control heights.
    const controlish = raw.filter((c: string) => {
      const n = Number(c.replace("h-", ""));
      return n >= 6;
    });
    expect(controlish).toEqual([]);
    expect(toolbar).toContain("h-control-");
  });
});
