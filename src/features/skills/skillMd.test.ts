import { describe, expect, it } from "vitest";
import fc from "fast-check";
import { buildSkillMdPreview } from "./skillMd";
import type { Skill } from "./types";

function makeSkill(partial: Partial<Skill> & { id: string; name: string }): Skill {
  return {
    protocolType: "skill",
    tags: [],
    isFavorite: false,
    category: "general",
    isBuiltin: false,
    currentVersion: 0,
    versionTrackingEnabled: false,
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: "2024-01-01T00:00:00.000Z",
    ...partial,
  };
}

describe("buildSkillMdPreview (Req 10.2)", () => {
  it("emits a frontmatter block delimited by --- followed by the body", () => {
    const md = buildSkillMdPreview(
      makeSkill({
        id: "s1",
        name: "code-reviewer",
        description: "Reviews code",
        content: "# Instructions\nDo the thing.",
      }),
    );
    expect(md).toBe(
      "---\nname: code-reviewer\ndescription: Reviews code\n---\n\n# Instructions\nDo the thing.\n",
    );
  });

  it("omits empty/absent metadata fields", () => {
    const md = buildSkillMdPreview(makeSkill({ id: "s1", name: "n" }));
    expect(md).toBe("---\nname: n\n---\n");
    expect(md).not.toContain("description:");
    expect(md).not.toContain("author:");
    expect(md).not.toContain("tags:");
  });

  it("renders tags as a YAML list", () => {
    const md = buildSkillMdPreview(
      makeSkill({ id: "s1", name: "n", tags: ["a", "b"] }),
    );
    expect(md).toContain("tags:\n  - a\n  - b");
  });

  it("quotes values that would otherwise be ambiguous YAML", () => {
    const md = buildSkillMdPreview(
      makeSkill({ id: "s1", name: "n", description: "* leads a list" }),
    );
    expect(md).toContain('description: "* leads a list"');
  });

  it("always opens and closes the frontmatter block (property)", () => {
    const nonEmpty = fc.string({ minLength: 1 }).filter((s) => s.trim() !== "");
    fc.assert(
      fc.property(nonEmpty, fc.string(), (name, content) => {
        const md = buildSkillMdPreview(makeSkill({ id: "s1", name, content }));
        // Opens with a delimiter and contains a closing delimiter after the name.
        expect(md.startsWith("---\n")).toBe(true);
        expect(md.indexOf("\n---\n")).toBeGreaterThan(0);
      }),
    );
  });
});
