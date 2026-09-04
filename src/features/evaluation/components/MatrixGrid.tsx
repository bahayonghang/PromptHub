import { useTranslation } from "react-i18next";
import { EmptyState } from "../../../components/ui";
import type { EvaluationCell, EvaluationRunDetail, ExecutionProfileRevision, TestSet } from "../types";
import type { PromptVersion } from "../../prompts/types";
import { StatusDot } from "./StatusDot";

export interface MatrixGridProps {
  matrix: EvaluationRunDetail | null;
  testSet: TestSet | undefined;
  versions: PromptVersion[];
  profiles: ExecutionProfileRevision[];
  compareCells: string[];
  onToggleCompare: (cellId: string) => void;
}

/**
 * The cell grid for a matrix run: one row per test case, one button per
 * prompt-revision x profile combination.
 */
export function MatrixGrid({
  matrix,
  testSet,
  versions,
  profiles,
  compareCells,
  onToggleCompare,
}: MatrixGridProps) {
  const { t } = useTranslation();

  const cellsByCase = new Map<string, EvaluationCell[]>();
  for (const cell of matrix?.cells ?? []) {
    const list = cellsByCase.get(cell.testCaseId);
    if (list) list.push(cell);
    else cellsByCase.set(cell.testCaseId, [cell]);
  }

  if (!matrix) {
    return <EmptyState title={t("evaluation.matrixEmpty")} />;
  }

  return (
    <div className="min-w-0 overflow-auto">
      <table className="w-full border-separate border-spacing-0 text-label">
        <thead className="sticky top-0 z-10 bg-card">
          <tr>
            <th
              scope="col"
              className="border-b border-border p-2 text-left font-medium text-muted-foreground"
            >
              {t("evaluation.caseName")}
            </th>
            <th
              scope="col"
              className="border-b border-border p-2 text-left font-medium text-muted-foreground"
            >
              {t("evaluation.cells")}
            </th>
          </tr>
        </thead>
        <tbody>
          {(testSet?.cases ?? []).map((testCase) => (
            <tr key={testCase.id}>
              <th
                scope="row"
                className="border-b border-border p-2 text-left align-top font-medium text-foreground"
              >
                {testCase.name}
              </th>
              <td className="border-b border-border p-2">
                <div className="grid gap-1 sm:grid-cols-2 xl:grid-cols-3">
                  {(cellsByCase.get(testCase.id) ?? []).map((cell) => {
                    const selected = compareCells.includes(cell.id);
                    const profileName =
                      profiles.find((profile) => profile.id === cell.profileRevisionId)?.name ??
                      cell.profileRevisionId.slice(0, 8);
                    const version =
                      versions.find((item) => item.id === cell.promptRevisionId)?.version ?? "?";
                    return (
                      <button
                        key={cell.id}
                        type="button"
                        aria-pressed={selected}
                        onClick={() => onToggleCompare(cell.id)}
                        className={`flex w-full items-center gap-2 rounded-md border px-2 py-1.5 text-left transition-colors duration-fast ease-out ${
                          selected
                            ? "border-primary bg-state-selected"
                            : "border-input hover:border-border-strong hover:bg-state-hover"
                        }`}
                      >
                        <StatusDot status={cell.status} />
                        <span className="sr-only">{cell.status}</span>
                        <span className="min-w-0 flex-1 truncate">
                          {profileName}
                          <span className="text-muted-foreground"> · </span>
                          <span className="font-mono tabular-nums">v{version}</span>
                        </span>
                        {cell.cacheHit && (
                          <span className="shrink-0 text-meta uppercase tracking-wide text-muted-foreground-subtle">
                            {t("evaluation.cached")}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
