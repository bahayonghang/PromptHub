import { useTranslation } from "react-i18next";
import { ExternalLinkIcon, Trash2Icon } from "lucide-react";
import type { RuntimePathsReport } from "../types";
import { useSystemStore } from "../systemStore";

/** Formats a byte count as a compact human-readable string (e.g. "1.2 MB"). */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}

/** The runtime-path rows shown, in display order, with their i18n label keys. */
const PATH_ROWS: ReadonlyArray<{ key: keyof RuntimePathsReport; labelKey: string }> = [
  { key: "data", labelKey: "systemView.paths.data" },
  { key: "database", labelKey: "systemView.paths.database" },
  { key: "media", labelKey: "systemView.paths.media" },
  { key: "rule", labelKey: "systemView.paths.rule" },
  { key: "backup", labelKey: "systemView.paths.backup" },
  { key: "log", labelKey: "systemView.paths.log" },
];

/**
 * Runtime paths + cache (Req 20.8, 20.9, 20.10). Lists the resolved data,
 * database, media, rule, backup, and log paths with an open-in-shell
 * action, and shows the cache size with a clear-cache action. Everything routes
 * through the system store to the Window_Manager via the Runtime_Bridge
 * (Req 3.1). When window controls are unavailable in the runtime (Req 3.7) the
 * panel shows a degraded message. All text resolves through i18n (Req 21.3) and
 * icons are from Lucide (Req 22.4).
 */
export function RuntimeInfoPanel() {
  const { t } = useTranslation();

  const paths = useSystemStore((s) => s.runtimePaths);
  const cacheSize = useSystemStore((s) => s.cacheSize);
  const platform = useSystemStore((s) => s.platform);
  const unavailable = useSystemStore((s) => s.windowControlsUnavailable);
  const clearCache = useSystemStore((s) => s.clearCache);
  const openPath = useSystemStore((s) => s.openPath);

  const labelClass = "text-body font-medium text-foreground";
  const hintClass = "text-label text-muted-foreground";

  return (
    <section className="flex flex-col gap-4">
      <div className="flex flex-col gap-0.5">
        <h3 className={labelClass}>{t("systemView.paths.title")}</h3>
        <p className={hintClass}>
          {t("systemView.paths.platform")}: {platform ?? "—"}
        </p>
      </div>

      {unavailable ? (
        <p className={hintClass}>{t("systemView.paths.unavailable")}</p>
      ) : (
        <>
          {/* Runtime paths (Req 20.9, 20.10) */}
          <div className="flex flex-col divide-y divide-border rounded-md border border-border">
            {PATH_ROWS.map(({ key, labelKey }) => {
              const value = paths?.[key];
              return (
                <div key={key} className="flex items-center gap-3 px-3 py-2">
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="text-label font-medium text-foreground">{t(labelKey)}</span>
                    <span className="truncate text-label text-muted-foreground" title={value ?? ""}>
                      {value ?? "—"}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => value && void openPath(value)}
                    disabled={!value}
                    title={t("systemView.paths.open")}
                    aria-label={t("systemView.paths.open")}
                    className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    <ExternalLinkIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                </div>
              );
            })}
          </div>

          {/* Cache (Req 20.8) */}
          <div className="flex items-center justify-between gap-4">
            <div className="flex flex-col gap-0.5">
              <h4 className={labelClass}>{t("systemView.paths.cache")}</h4>
              <p className={hintClass}>
                {cacheSize != null ? formatBytes(cacheSize) : "—"}
              </p>
            </div>
            <button
              type="button"
              onClick={() => void clearCache()}
              className="inline-flex items-center gap-2 rounded-md border border-input px-3 py-2 text-body text-foreground transition-colors hover:bg-accent"
            >
              <Trash2Icon className="h-4 w-4" aria-hidden="true" />
              {t("systemView.paths.clearCache")}
            </button>
          </div>
        </>
      )}
    </section>
  );
}
