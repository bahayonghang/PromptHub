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
    // h-3/h-3.5/h-4 are icon glyph sizes, not control heights. h-10 is the
    // row itself (the 40px slice of the 112px chrome budget).
    const controlish = raw.filter((c: string) => {
      const n = Number(c.replace("h-", ""));
      return n >= 6 && n !== 10;
    });
    expect(controlish).toEqual([]);
    expect(toolbar).toContain("h-control-");
  });

  it("keeps the expanded sidebar at 236px", () => {
    const sidebar = read("src/components/layout/Sidebar.tsx");
    expect(sidebar).toContain("w-[236px]");
    expect(sidebar).not.toContain("w-[264px]");
  });

  it("keeps resting Prompts chrome at 112px", () => {
    // TitleBar 36 + LibraryHeader 36 + LibraryToolbar 40. Filter chips and the
    // import panel are opt-in and must not pad the resting stack.
    const titleBar = read("src/features/system/components/TitleBar.tsx");
    const header = read("src/features/prompts/components/LibraryHeader.tsx");
    const toolbar = read("src/features/prompts/components/LibraryToolbar.tsx");
    expect(titleBar).toMatch(/\bh-9\b/);
    expect(header).toMatch(/\bh-9\b/);
    expect(toolbar).toMatch(/\bh-10\b/);
    expect(header).not.toContain("useSystemStore");
    expect(header).not.toContain("subtitleWithPath");
  });
});
