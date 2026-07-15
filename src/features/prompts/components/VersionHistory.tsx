import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronDownIcon,
  ChevronRightIcon,
  HistoryIcon,
  RotateCcwIcon,
  SaveIcon,
} from "lucide-react";
import type { Prompt, PromptVersion } from "../types";
import { diffPromptRevision } from "../versionDiff";

/** The 1,000-character note limit enforced by `version.create` (Req 7.8). */
const NOTE_MAX = 1000;

interface VersionHistoryProps {
  prompt: Prompt;
  versions: PromptVersion[];
  onCreateVersion: (note?: string) => void;
  onRollback: (version: number) => void;
}

/**
 * The version-history panel for the selected prompt (Req 7.1–7.4). Lists
 * versions newest-first, lets the user snapshot the current prompt with an
 * optional note (≤1000 chars, Req 7.2/7.8), roll back to a version (Req 7.3),
 * and delete a single version (Req 7.4).
 */
export function VersionHistory({
  prompt,
  versions,
  onCreateVersion,
  onRollback,
}: VersionHistoryProps) {
  const { t } = useTranslation();
  const [note, setNote] = useState("");
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Newest first for display; the backend returns ascending (Req 7.1).
  const ordered = [...versions].sort((a, b) => b.version - a.version);
  const noteTooLong = note.length > NOTE_MAX;

  const saveVersion = () => {
    if (noteTooLong) return;
    onCreateVersion(note.trim() === "" ? undefined : note.trim());
    setNote("");
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <HistoryIcon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
        <span className="text-sm font-semibold text-foreground">
          {t("promptsView.history.title")}
        </span>
      </div>

      <div className="flex flex-col gap-1.5 border-b border-border p-3">
        <textarea
          value={note}
          placeholder={t("promptsView.history.notePlaceholder")}
          onChange={(e) => setNote(e.target.value)}
          rows={2}
          className="w-full resize-y rounded-md border border-input bg-background px-2 py-1 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring"
        />
        {noteTooLong && (
          <span className="text-xs text-destructive">
            {t("promptsView.history.noteTooLong")}
          </span>
        )}
        <button
          type="button"
          onClick={saveVersion}
          disabled={noteTooLong}
          className="flex items-center justify-center gap-1.5 rounded-md border border-input px-3 py-1.5 text-xs font-medium text-foreground hover:bg-accent disabled:opacity-50"
        >
          <SaveIcon className="h-3.5 w-3.5" aria-hidden="true" />
          {t("promptsView.history.saveVersion")}
        </button>
      </div>

      <ul className="flex-1 overflow-y-auto p-2">
        {ordered.length === 0 ? (
          <li className="flex flex-col items-center gap-1 px-2 py-6 text-center">
            <p className="text-xs font-medium text-foreground">
              {t("promptsView.history.empty")}
            </p>
            <p className="text-xs text-muted-foreground">
              {t("promptsView.history.emptyHint")}
            </p>
          </li>
        ) : (
          ordered.map((version) => {
            const diff = diffPromptRevision(prompt, version);
            const expanded = expandedId === version.id;
            return (
            <li key={version.id} className="border-b border-border px-2 py-2 last:border-b-0">
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  title={t("promptsView.history.showDiff")}
                  aria-label={t("promptsView.history.showDiff")}
                  aria-expanded={expanded}
                  onClick={() => setExpandedId(expanded ? null : version.id)}
                  className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                >
                  {expanded ? (
                    <ChevronDownIcon className="h-3.5 w-3.5" aria-hidden="true" />
                  ) : (
                    <ChevronRightIcon className="h-3.5 w-3.5" aria-hidden="true" />
                  )}
                </button>
                <span className="rounded bg-muted px-1.5 py-0.5 text-xs font-medium text-foreground">
                  {t("promptsView.history.versionLabel", { version: version.version })}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t(`promptsView.history.sources.${version.sourceAction}`)}
                </span>
                <span className="ml-auto shrink-0">
                  <button
                    type="button"
                    title={t("promptsView.history.restore")}
                    aria-label={t("promptsView.history.restore")}
                    onClick={() => onRollback(version.version)}
                    className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                  >
                    <RotateCcwIcon className="h-3.5 w-3.5" aria-hidden="true" />
                  </button>
                </span>
              </div>
              {version.note && (
                <p className="line-clamp-2 text-xs text-muted-foreground">
                  {version.note}
                </p>
              )}
              {expanded && (
                <div className="mt-2 border-l border-border pl-2">
                  {diff.length === 0 ? (
                    <p className="text-xs text-muted-foreground">
                      {t("promptsView.history.noDiff")}
                    </p>
                  ) : (
                    <dl className="space-y-2">
                      {diff.map((entry) => (
                        <div key={entry.field}>
                          <dt className="text-xs font-medium text-foreground">
                            {entry.field === "messages"
                              ? t("evaluation.messages")
                              : t(`promptsView.history.fields.${entry.field}`)}
                          </dt>
                          <dd className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
                            <span className="break-words">{entry.revisionValue}</span>
                            <span className="break-words">{entry.currentValue}</span>
                          </dd>
                        </div>
                      ))}
                    </dl>
                  )}
                </div>
              )}
            </li>
            );
          })
        )}
      </ul>
    </div>
  );
}
