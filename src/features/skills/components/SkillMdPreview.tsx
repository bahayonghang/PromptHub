import { useTranslation } from "react-i18next";
import { FileTextIcon } from "lucide-react";
import { buildSkillMdPreview } from "../skillMd";
import type { Skill } from "../types";

interface SkillMdPreviewProps {
  skill: Skill;
}

/**
 * Read-only SKILL.md preview for the selected skill (Req 10.2, 22.3). Renders the
 * frontmatter-plus-body document built from the skill's metadata and content so
 * the user can see the on-disk SKILL.md shape without a backend round-trip.
 */
export function SkillMdPreview({ skill }: SkillMdPreviewProps) {
  const { t } = useTranslation();
  const markdown = buildSkillMdPreview(skill);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <FileTextIcon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
        <span className="text-sm font-semibold text-foreground">
          {t("skillsView.preview.title")}
        </span>
      </div>
      <pre className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap p-3 text-xs text-foreground">
        {markdown}
      </pre>
    </div>
  );
}
