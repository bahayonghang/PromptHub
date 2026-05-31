import { useState } from "react";
import { useTranslation } from "react-i18next";
import { HistoryIcon, RotateCcwIcon, SaveIcon, Trash2Icon } from "lucide-react";
import type { SkillVersion } from "../types";

interface SkillVersionHistoryProps {
  versions: SkillVersion[];
  onCreateVersion: (note?: string) => void;
  onRollback: (version: number) => void;
  onDeleteVersion: (versionId: string) => void;
}

/**
 * The version-history panel for the selected skill (Req 9.6–9.9). Lists versions
 * newest-first, lets the user snapshot the current skill with an optional note
 * (Req 9.7), roll back to a version (Req 9.8), and delete a single version
 * (Req 9.9).
 */
export function SkillVersionHistory({
  versions,
  onCreateVersion,
  onRollback,
  onDeleteVersion,
}: SkillVersionHistoryProps) {
  const { t } = useTranslation();
  const [note, setNote] = useState("");

  // Newest first for display; the backend returns ascending (Req 9.6).
  const ordered = [...versions].sort((a, b) => b.version - a.version);

  const saveVersion = () => {
    onCreateVersion(note.trim() === "" ? undefined : note.trim());
    setNote("");
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <HistoryIcon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
        <span className="text-sm font-semibold text-foreground">
          {t("skillsView.history.title")}
        </span>
      </div>

      <div className="flex flex-col gap-1.5 border-b border-border p-3">
        <textarea
          value={note}
          placeholder={t("skillsView.history.notePlaceholder")}
          onChange={(e) => setNote(e.target.value)}
          rows={2}
          className="w-full resize-y rounded-md border border-input bg-background px-2 py-1 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring"
        />
        <button
          type="button"
          onClick={saveVersion}
          className="flex items-center justify-center gap-1.5 rounded-md border border-input px-3 py-1.5 text-xs font-medium text-foreground hover:bg-accent"
        >
          <SaveIcon className="h-3.5 w-3.5" aria-hidden="true" />
          {t("skillsView.history.saveVersion")}
        </button>
      </div>

      <ul className="flex-1 overflow-y-auto p-2">
        {ordered.length === 0 ? (
          <li className="flex flex-col items-center gap-1 px-2 py-6 text-center">
            <p className="text-xs font-medium text-foreground">
              {t("skillsView.history.empty")}
            </p>
            <p className="text-xs text-muted-foreground">
              {t("skillsView.history.emptyHint")}
            </p>
          </li>
        ) : (
          ordered.map((version) => (
            <li
              key={version.id}
              className="group flex flex-col gap-1 rounded-md border border-transparent px-2 py-2 hover:border-border hover:bg-card"
            >
              <div className="flex items-center gap-2">
                <span className="rounded bg-muted px-1.5 py-0.5 text-xs font-medium text-foreground">
                  {t("skillsView.history.versionLabel", { version: version.version })}
                </span>
                <span className="ml-auto hidden shrink-0 items-center gap-0.5 group-hover:flex">
                  <button
                    type="button"
                    title={t("skillsView.history.restore")}
                    aria-label={t("skillsView.history.restore")}
                    onClick={() => onRollback(version.version)}
                    className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                  >
                    <RotateCcwIcon className="h-3.5 w-3.5" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    title={t("skillsView.history.delete")}
                    aria-label={t("skillsView.history.delete")}
                    onClick={() => onDeleteVersion(version.id)}
                    className="rounded p-1 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
                  >
                    <Trash2Icon className="h-3.5 w-3.5" aria-hidden="true" />
                  </button>
                </span>
              </div>
              {version.note && (
                <p className="line-clamp-2 text-xs text-muted-foreground">
                  {version.note}
                </p>
              )}
            </li>
          ))
        )}
      </ul>
    </div>
  );
}
