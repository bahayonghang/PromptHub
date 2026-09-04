import { useTranslation } from "react-i18next";
import { DownloadIcon, RefreshCwIcon, RotateCwIcon } from "lucide-react";
import { downloadProgressPercent, useSystemStore } from "../systemStore";

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

/**
 * In-app updater panel (Req 24.2-24.7). Drives the check -> download -> install
 * lifecycle through the system store, which routes every call through the
 * Runtime_Bridge (Req 3.1) and tracks byte progress from `updater:status` events
 * (Req 24.3). The updater is capability-gated by `appUpdate` (Req 3.7); when
 * unavailable the panel shows a degraded message instead of the controls. All
 * text resolves through i18n (Req 21.3) and icons are from Lucide (Req 22.4).
 */
export function UpdaterPanel() {
  const { t } = useTranslation();

  const phase = useSystemStore((s) => s.updaterPhase);
  const check = useSystemStore((s) => s.updateCheck);
  const downloaded = useSystemStore((s) => s.downloaded);
  const total = useSystemStore((s) => s.total);
  const updaterError = useSystemStore((s) => s.updaterError);
  const unavailable = useSystemStore((s) => s.updaterUnavailable);
  const version = useSystemStore((s) => s.version);

  const checkUpdate = useSystemStore((s) => s.checkUpdate);
  const downloadUpdate = useSystemStore((s) => s.downloadUpdate);
  const installUpdate = useSystemStore((s) => s.installUpdate);

  const labelClass = "text-body font-medium text-foreground";
  const hintClass = "text-label text-muted-foreground";
  const percent = downloadProgressPercent(downloaded, total);

  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-col gap-0.5">
        <h3 className={labelClass}>{t("systemView.updater.title")}</h3>
        <p className={hintClass}>{t("systemView.updater.hint")}</p>
      </div>

      <p className={hintClass}>
        {t("systemView.updater.currentVersion")}: {version ?? "—"}
      </p>

      {unavailable ? (
        <p className={hintClass}>{t("systemView.updater.unavailable")}</p>
      ) : (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={() => void checkUpdate()}
              disabled={phase === "checking" || phase === "downloading" || phase === "installing"}
              className="inline-flex items-center gap-2 rounded-md border border-input px-3 py-2 text-body text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
            >
              <RefreshCwIcon className="h-4 w-4" aria-hidden="true" />
              {phase === "checking"
                ? t("systemView.updater.checking")
                : t("systemView.updater.check")}
            </button>

            {phase === "available" && (
              <button
                type="button"
                onClick={() => void downloadUpdate()}
                className="inline-flex items-center gap-2 rounded-md bg-primary px-3 py-2 text-body text-primary-foreground transition-colors hover:bg-primary/90"
              >
                <DownloadIcon className="h-4 w-4" aria-hidden="true" />
                {t("systemView.updater.download")}
              </button>
            )}

            {phase === "downloaded" && (
              <button
                type="button"
                onClick={() => void installUpdate()}
                className="inline-flex items-center gap-2 rounded-md bg-primary px-3 py-2 text-body text-primary-foreground transition-colors hover:bg-primary/90"
              >
                <RotateCwIcon className="h-4 w-4" aria-hidden="true" />
                {t("systemView.updater.install")}
              </button>
            )}
          </div>

          {phase === "upToDate" && (
            <p className={hintClass} role="status">
              {t("systemView.updater.upToDate")}
            </p>
          )}

          {phase === "available" && check?.version && (
            <p className="text-body text-foreground" role="status">
              {t("systemView.updater.available", { version: check.version })}
            </p>
          )}

          {phase === "downloading" && (
            <div className="flex flex-col gap-1">
              <div className="h-2 w-full overflow-hidden rounded-full bg-input">
                <div
                  className="h-full bg-primary transition-[width]"
                  style={{ width: `${percent ?? 0}%` }}
                />
              </div>
              <p className={hintClass} role="status">
                {percent != null
                  ? t("systemView.updater.downloadingPercent", { percent })
                  : downloaded != null
                    ? t("systemView.updater.downloadingBytes", {
                        downloaded: formatBytes(downloaded),
                      })
                    : t("systemView.updater.downloading")}
              </p>
            </div>
          )}

          {phase === "downloaded" && (
            <p className={hintClass} role="status">
              {t("systemView.updater.downloaded")}
            </p>
          )}

          {phase === "installing" && (
            <p className={hintClass} role="status">
              {t("systemView.updater.installing")}
            </p>
          )}

          {phase === "error" && updaterError && (
            <p className="text-body text-destructive" role="alert">
              {updaterError}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
