import { useTranslation } from "react-i18next";
import { RefreshCwIcon } from "lucide-react";
import { EmptyState, IconButton, Input, Select } from "../../../components/ui";
import { PanelHeading } from "./Field";
import { StatusDot } from "./StatusDot";
import type {
  EvaluationRun,
  EvaluationRunDetail,
  ExecutionProfileRevision,
  PromptRun,
} from "../types";
import type { PromptVersion } from "../../prompts/types";

export interface HistoryPanelProps {
  runs: PromptRun[];
  versions: PromptVersion[];
  profiles: ExecutionProfileRevision[];
  matrices: EvaluationRun[];
  selectedMatrix: EvaluationRunDetail | null;
  status: string;
  onStatusChange: (value: string) => void;
  profileId: string;
  onProfileChange: (value: string) => void;
  revisionId: string;
  onRevisionChange: (value: string) => void;
  date: string;
  onDateChange: (value: string) => void;
  onSelectMatrix: (id: string) => void;
  onRetryMatrix: (id: string) => void;
}

export function HistoryPanel({
  runs,
  versions,
  profiles,
  matrices,
  selectedMatrix,
  status,
  onStatusChange,
  profileId,
  onProfileChange,
  revisionId,
  onRevisionChange,
  date,
  onDateChange,
  onSelectMatrix,
  onRetryMatrix,
}: HistoryPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="grid shrink-0 grid-cols-1 gap-2 border-b border-border p-3 sm:grid-cols-2 lg:grid-cols-4">
        <Select
          value={status}
          onChange={(event) => onStatusChange(event.target.value)}
          aria-label={t("evaluation.status")}
          block
        >
          <option value="">{t("evaluation.allStatuses")}</option>
          {["success", "error", "cancelled"].map((item) => (
            <option key={item} value={item}>
              {item}
            </option>
          ))}
        </Select>
        <Select
          value={profileId}
          onChange={(event) => onProfileChange(event.target.value)}
          aria-label={t("evaluation.profile")}
          block
        >
          <option value="">{t("evaluation.allProfiles")}</option>
          {profiles.map((profile) => (
            <option key={profile.id} value={profile.id}>
              {profile.name}
            </option>
          ))}
        </Select>
        <Select
          value={revisionId}
          onChange={(event) => onRevisionChange(event.target.value)}
          aria-label={t("evaluation.revision")}
          block
        >
          <option value="">{t("evaluation.allRevisions")}</option>
          {versions.map((version) => (
            <option key={version.id} value={version.id}>
              v{version.version}
            </option>
          ))}
        </Select>
        <Input
          type="date"
          value={date}
          onChange={(event) => onDateChange(event.target.value)}
          aria-label={t("evaluation.date")}
        />
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-[minmax(0,1fr)_18rem] lg:overflow-hidden">
        <div className="min-w-0 overflow-auto">
          {runs.length === 0 ? (
            <EmptyState title={t("evaluation.noRuns")} />
          ) : (
            <table className="w-full border-separate border-spacing-0 text-label">
              <thead className="sticky top-0 z-10 bg-card">
                <tr className="text-left text-muted-foreground">
                  {[
                    t("evaluation.date"),
                    t("evaluation.revision"),
                    t("evaluation.profile"),
                    t("evaluation.status"),
                    t("evaluation.duration"),
                    t("evaluation.output"),
                  ].map((heading) => (
                    <th
                      key={heading}
                      scope="col"
                      className="border-b border-border p-2 font-medium"
                    >
                      {heading}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {runs.map((item) => (
                  <tr key={item.id} className="hover:bg-state-hover">
                    <td className="border-b border-border p-2 tabular-nums text-muted-foreground">
                      {new Date(item.startedAt).toLocaleString()}
                    </td>
                    <td className="border-b border-border p-2 font-mono tabular-nums">
                      v{versions.find((version) => version.id === item.promptRevisionId)?.version ?? "?"}
                    </td>
                    <td className="border-b border-border p-2">
                      {profiles.find((profile) => profile.id === item.profileRevisionId)?.name ?? "—"}
                    </td>
                    <td className="border-b border-border p-2">
                      <span className="flex items-center gap-1.5">
                        <StatusDot status={item.status} />
                        {item.status}
                      </span>
                    </td>
                    <td className="border-b border-border p-2 tabular-nums text-muted-foreground">
                      {item.durationMs != null ? `${item.durationMs} ms` : "—"}
                    </td>
                    <td className="max-w-xs truncate border-b border-border p-2 text-muted-foreground">
                      {item.output ?? item.error ?? "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <aside className="min-w-0 overflow-y-auto border-t border-border p-3 lg:border-l lg:border-t-0">
          <PanelHeading
            action={
              selectedMatrix && (
                <IconButton
                  label={t("evaluation.retry")}
                  icon={<RefreshCwIcon className="h-3.5 w-3.5" />}
                  size="sm"
                  onClick={() => onRetryMatrix(selectedMatrix.run.id)}
                />
              )
            }
          >
            {t("evaluation.matrixRuns")}
          </PanelHeading>
          <div className="mt-2 space-y-1">
            {matrices.map((matrix) => {
              const active = selectedMatrix?.run.id === matrix.id;
              return (
                <button
                  key={matrix.id}
                  type="button"
                  aria-current={active || undefined}
                  onClick={() => onSelectMatrix(matrix.id)}
                  className={`block w-full rounded-md border p-2 text-left text-label transition-colors duration-fast ease-out ${
                    active
                      ? "border-primary bg-state-selected"
                      : "border-input hover:border-border-strong hover:bg-state-hover"
                  }`}
                >
                  <span className="flex items-center gap-1.5 text-foreground">
                    <StatusDot status={matrix.status} />
                    {matrix.status}
                    <span className="ml-auto font-mono tabular-nums text-muted-foreground">
                      {matrix.completedCells}/{matrix.totalCells}
                    </span>
                  </span>
                  <span className="mt-0.5 block text-meta text-muted-foreground">
                    {new Date(matrix.startedAt).toLocaleString()}
                  </span>
                </button>
              );
            })}
          </div>
        </aside>
      </div>
    </div>
  );
}
