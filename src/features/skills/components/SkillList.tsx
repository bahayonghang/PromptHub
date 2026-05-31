import { useTranslation } from "react-i18next";
import { ShieldCheckIcon, StarIcon } from "lucide-react";
import type { SafetyLevel, Skill } from "../types";

interface SkillListProps {
  skills: Skill[];
  selectedSkillId: string | null;
  loading: boolean;
  onSelect: (id: string) => void;
}

/** Maps a safety level to the token-driven badge color classes. */
function safetyBadgeClass(level: SafetyLevel): string {
  switch (level) {
    case "safe":
      return "bg-primary/15 text-primary";
    case "warn":
      return "bg-muted text-muted-foreground";
    case "high-risk":
    case "blocked":
      return "bg-destructive/15 text-destructive";
  }
}

/**
 * The scrollable skill list (Req 9.3). Each row shows the skill name, a one-line
 * description, tags, and favorite/safety indicators, and selects the skill for
 * editing on click. Renders an empty state when no skill matches the search.
 */
export function SkillList({
  skills,
  selectedSkillId,
  loading,
  onSelect,
}: SkillListProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
        {t("skillsView.loading")}
      </div>
    );
  }

  if (skills.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-1 p-6 text-center">
        <p className="text-sm font-medium text-foreground">
          {t("skillsView.noSkills")}
        </p>
        <p className="max-w-xs text-xs text-muted-foreground">
          {t("skillsView.noSkillsHint")}
        </p>
      </div>
    );
  }

  return (
    <ul className="flex flex-col gap-1 p-2">
      {skills.map((skill) => {
        const selected = skill.id === selectedSkillId;
        const name = skill.name.trim() || t("skillsView.untitled");
        return (
          <li key={skill.id}>
            <button
              type="button"
              onClick={() => onSelect(skill.id)}
              aria-current={selected ? "true" : undefined}
              className={`flex w-full flex-col gap-1 rounded-lg border px-3 py-2 text-left transition-colors ${
                selected
                  ? "border-primary bg-primary/10"
                  : "border-transparent hover:bg-accent"
              }`}
            >
              <span className="flex items-center gap-1.5">
                {skill.iconEmoji && (
                  <span className="shrink-0 text-sm" aria-hidden="true">
                    {skill.iconEmoji}
                  </span>
                )}
                <span className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                  {name}
                </span>
                {skill.safetyLevel && (
                  <span
                    className={`flex shrink-0 items-center gap-0.5 rounded-full px-1.5 py-0.5 text-[10px] font-medium ${safetyBadgeClass(
                      skill.safetyLevel,
                    )}`}
                    title={t(`skillsView.safety.level.${skill.safetyLevel}`)}
                  >
                    <ShieldCheckIcon className="h-3 w-3" aria-hidden="true" />
                    {t(`skillsView.safety.level.${skill.safetyLevel}`)}
                  </span>
                )}
                {skill.isFavorite && (
                  <StarIcon
                    className="h-3.5 w-3.5 shrink-0 fill-current text-primary"
                    aria-label={t("skillsView.favorite")}
                  />
                )}
              </span>
              {skill.description && (
                <span className="line-clamp-2 text-xs text-muted-foreground">
                  {skill.description}
                </span>
              )}
              {skill.tags.length > 0 && (
                <span className="flex flex-wrap gap-1">
                  {skill.tags.slice(0, 4).map((tag) => (
                    <span
                      key={tag}
                      className="rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground"
                    >
                      {tag}
                    </span>
                  ))}
                </span>
              )}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
