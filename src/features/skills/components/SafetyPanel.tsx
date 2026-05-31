import { useTranslation } from "react-i18next";
import { ShieldAlertIcon, ShieldCheckIcon } from "lucide-react";
import type { SafetyLevel, SafetyReport, Severity } from "../types";

interface SafetyPanelProps {
  report: SafetyReport | null;
  scanning: boolean;
  onScan: () => void;
}

/** Maps a safety level to its token-driven text/background color classes. */
function levelClass(level: SafetyLevel): string {
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

/** Maps a finding severity to its token-driven text color class. */
function severityClass(severity: Severity): string {
  switch (severity) {
    case "info":
      return "text-muted-foreground";
    case "warn":
      return "text-foreground";
    case "high":
      return "text-destructive";
  }
}

/**
 * Safety-scan panel for the selected skill (Req 13.1, 13.2). Triggers an AI
 * safety scan and renders the resulting report: the overall level, an optional
 * score and summary, and the list of findings with their severity and detail.
 */
export function SafetyPanel({ report, scanning, onScan }: SafetyPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-2 border-t border-border p-4">
      <div className="flex items-center justify-between gap-2">
        <h3 className="flex items-center gap-1.5 text-sm font-semibold text-foreground">
          {report && (report.level === "high-risk" || report.level === "blocked") ? (
            <ShieldAlertIcon className="h-4 w-4 text-destructive" aria-hidden="true" />
          ) : (
            <ShieldCheckIcon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          )}
          {t("skillsView.safety.title")}
        </h3>
        <button
          type="button"
          onClick={onScan}
          disabled={scanning}
          className="flex shrink-0 items-center gap-1 rounded-md border border-input px-2.5 py-1.5 text-xs font-medium text-foreground hover:bg-accent disabled:opacity-50"
        >
          {scanning ? t("skillsView.safety.scanning") : t("skillsView.safety.scan")}
        </button>
      </div>

      {report == null ? (
        <p className="text-xs text-muted-foreground">
          {t("skillsView.safety.empty")}
        </p>
      ) : (
        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-medium ${levelClass(
                report.level,
              )}`}
            >
              {t(`skillsView.safety.level.${report.level}`)}
            </span>
            {report.score != null && (
              <span className="text-xs text-muted-foreground">
                {t("skillsView.safety.score", { score: report.score })}
              </span>
            )}
          </div>
          {report.summary && (
            <p className="text-xs text-muted-foreground">{report.summary}</p>
          )}
          {report.findings.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {t("skillsView.safety.noFindings")}
            </p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {report.findings.map((finding, index) => (
                <li
                  key={`${finding.code}-${index}`}
                  className="flex flex-col gap-0.5 rounded-md border border-border px-2.5 py-1.5"
                >
                  <span className="flex items-center gap-1.5">
                    <span
                      className={`text-[11px] font-semibold uppercase ${severityClass(
                        finding.severity,
                      )}`}
                    >
                      {t(`skillsView.safety.severity.${finding.severity}`)}
                    </span>
                    <span className="min-w-0 flex-1 truncate text-xs font-medium text-foreground">
                      {finding.title}
                    </span>
                  </span>
                  <span className="text-[11px] text-muted-foreground">
                    {finding.detail}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
