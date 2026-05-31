/**
 * Pure helpers for composing the local SKILL.md preview (Req 10.2, 22.3). Kept
 * React-free so the rendering rules can be unit-tested directly. The backend's
 * `skill.serializeMd` is the authoritative serializer; this builds an equivalent
 * preview from a stored {@link Skill} without a round-trip so the editor can show
 * a live SKILL.md while the user types.
 */
import type { Skill } from "./types";

/** A frontmatter field rendered into the preview, in display order. */
interface FrontmatterField {
  key: string;
  value: string | string[] | undefined | null;
}

/** Serializes a single scalar value as a YAML string, quoting when needed. */
function yamlScalar(value: string): string {
  // Quote when the value would otherwise be ambiguous YAML (empty, leading/
  // trailing space, or characters that start a non-string token).
  if (value === "" || value !== value.trim() || /^[!&*?{}[\],#|>@`"']/.test(value)) {
    return JSON.stringify(value);
  }
  return value;
}

/** Renders one frontmatter field as a YAML line (or block for a list). */
function renderField(field: FrontmatterField): string | null {
  const { key, value } = field;
  if (value == null) return null;
  if (Array.isArray(value)) {
    if (value.length === 0) return null;
    const items = value.map((item) => `  - ${yamlScalar(item)}`).join("\n");
    return `${key}:\n${items}`;
  }
  if (value === "") return null;
  return `${key}: ${yamlScalar(value)}`;
}

/**
 * Builds a SKILL.md preview string from a {@link Skill}: a YAML frontmatter block
 * (`--- … ---`) carrying the skill's metadata, followed by its content body
 * (Req 10.2). Empty/absent fields are omitted. The body is the skill's `content`
 * (the SKILL.md instructions), or an empty string when none is stored.
 */
export function buildSkillMdPreview(skill: Skill): string {
  const fields: FrontmatterField[] = [
    { key: "name", value: skill.name },
    { key: "description", value: skill.description ?? undefined },
    { key: "version", value: skill.version ?? undefined },
    { key: "author", value: skill.author ?? undefined },
    { key: "tags", value: skill.tags },
  ];

  const frontmatter = fields
    .map(renderField)
    .filter((line): line is string => line !== null)
    .join("\n");

  const body = (skill.content ?? "").trim();
  return `---\n${frontmatter}\n---\n${body === "" ? "" : `\n${body}\n`}`;
}
