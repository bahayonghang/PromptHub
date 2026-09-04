import { useTranslation } from "react-i18next";
import { CheckIcon, TagIcon, XIcon } from "lucide-react";
import { Button, EmptyHint } from "../../../components/ui";
import { cn } from "../../../components/ui/cn";
import { PanelHeading } from "./Field";
import { StatusDot } from "./StatusDot";
import type { EvaluationCell, PromptLabel, PromptLabelHistory } from "../types";
import type { Prompt } from "../../prompts/types";

export interface ComparePanelProps {
  cells: EvaluationCell[];
  runById: Map<string, { output?: string | null }>;
  prompt: Prompt;
  labels: PromptLabel[];
  labelHistory: PromptLabelHistory[];
  onManual: (cellId: string, evaluatorId: string, passed: boolean) => void;
  onLabel: (label: PromptLabel["label"], revisionId: string, rollback: boolean) => void;
}

/**
 * Side-by-side view of the cells the user ticked in the matrix, plus the
 * baseline/candidate label controls for the prompt.
 */
export function ComparePanel({
  cells,
  runById,
  prompt,
  labels,
  labelHistory,
  onManual,
  onLabel,
}: ComparePanelProps) {
  const { t } = useTranslation();

  return (
    <aside className="min-w-0 overflow-y-auto border-t border-border p-3 lg:border-l lg:border-t-0">
      <PanelHeading>{t("evaluation.compare")}</PanelHeading>
      {cells.length === 0 && (
        <EmptyHint className="mt-2">{t("evaluation.compareEmpty")}</EmptyHint>
      )}

      <div className={`mt-2 grid gap-3 ${cells.length === 2 ? "grid-cols-2" : "grid-cols-1"}`}>
        {cells.map((cell) => (
          <section key={cell.id} className="min-w-0 border-t border-border pt-2">
            <div className="flex items-center justify-between gap-2 text-label">
              <span className="flex items-center gap-1.5 font-medium">
                <StatusDot status={cell.status} />
                {cell.status}
              </span>
              {cell.cacheHit && (
                <span className="text-meta uppercase tracking-wide text-muted-foreground-subtle">
                  {t("evaluation.cached")}
                </span>
              )}
            </div>

            <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap break-words rounded-md bg-surface-inset p-2 font-sans text-label text-foreground">
              {cell.promptRunId
                ? runById.get(cell.promptRunId)?.output ?? cell.error ?? "—"
                : cell.error ?? "—"}
            </pre>

            <div className="mt-2 space-y-2">
              {cell.results.map((result) => (
                <div key={result.evaluatorId} className="border-t border-border pt-2 text-label">
                  <div className="flex items-center gap-1.5">
                    {result.passed === true ? (
                      <CheckIcon className="h-3.5 w-3.5 text-success" aria-hidden="true" />
                    ) : result.passed === false ? (
                      <XIcon className="h-3.5 w-3.5 text-destructive" aria-hidden="true" />
                    ) : null}
                    <span className="font-medium">{result.kind}</span>
                  </div>
                  <p className="mt-0.5 text-muted-foreground">{result.evidence}</p>
                  {result.kind === "manual" && result.skipped && (
                    <div className="mt-1.5 flex gap-1">
                      <Button
                        size="sm"
                        onClick={() => onManual(cell.id, result.evaluatorId, true)}
                      >
                        {t("evaluation.pass")}
                      </Button>
                      <Button
                        size="sm"
                        onClick={() => onManual(cell.id, result.evaluatorId, false)}
                      >
                        {t("evaluation.fail")}
                      </Button>
                    </div>
                  )}
                </div>
              ))}
            </div>

            {cell.status === "success" && (
              <div className="mt-2 flex flex-wrap gap-1">
                {(["baseline", "candidate"] as const).map((label) => (
                  <Button
                    key={label}
                    size="sm"
                    onClick={() => onLabel(label, cell.promptRevisionId, false)}
                  >
                    <TagIcon className="h-3 w-3" aria-hidden="true" />
                    {label}
                  </Button>
                ))}
              </div>
            )}
          </section>
        ))}
      </div>

      <div className="mt-4 border-t border-border pt-3">
        <h4 className="text-label font-semibold text-foreground">{t("evaluation.labels")}</h4>
        {labels.map((label) => (
          <div
            key={label.label}
            className="mt-1 flex items-center justify-between gap-2 text-label"
          >
            <span className="text-muted-foreground">{label.label}</span>
            <span className="font-mono tabular-nums">
              {label.promptRevisionId.slice(0, 8)}
            </span>
          </div>
        ))}
        {labels.length === 0 && (
          <EmptyHint className="mt-1">
            {prompt.title}: {t("evaluation.noLabels")}
          </EmptyHint>
        )}

        {labelHistory.slice(0, 5).map((item) => (
          <button
            key={item.id}
            type="button"
            disabled={!item.fromRevisionId}
            onClick={() =>
              item.fromRevisionId && onLabel(item.label, item.fromRevisionId, true)
            }
            className={cn(
              "mt-1 block w-full rounded-md px-1 py-0.5 text-left text-label",
              "text-muted-foreground transition-colors duration-fast ease-out",
              "hover:bg-state-hover hover:text-foreground",
              "disabled:opacity-50 disabled:hover:bg-transparent",
            )}
          >
            <span className="tabular-nums">{new Date(item.createdAt).toLocaleString()}</span>
            {" · "}
            {item.action ?? "move"}
            {" · "}
            {item.label}
          </button>
        ))}
      </div>
    </aside>
  );
}
