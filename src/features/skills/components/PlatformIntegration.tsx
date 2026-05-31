import { useTranslation } from "react-i18next";
import { CheckCircle2Icon, DownloadIcon, Trash2Icon } from "lucide-react";
import type { Platform, PlatformInstallStatus } from "../types";

interface PlatformIntegrationProps {
  platforms: Platform[];
  detectedPlatforms: string[];
  installStatus: PlatformInstallStatus[];
  /** True when the platform-integration capability is unavailable (Req 3.7). */
  unavailable: boolean;
  onInstall: (platformId: string) => void;
  onUninstall: (platformId: string) => void;
}

/**
 * Platform install/uninstall UI for the selected skill (Req 12.1–12.5). Lists the
 * supported platforms, marks which are detected on the host (Req 12.2) and which
 * already have the skill installed (Req 12.5), and offers an install (Req 12.3)
 * or uninstall (Req 12.4) action per platform. When platform integration is not
 * available in the current runtime, it shows a graceful notice instead (Req 3.7).
 */
export function PlatformIntegration({
  platforms,
  detectedPlatforms,
  installStatus,
  unavailable,
  onInstall,
  onUninstall,
}: PlatformIntegrationProps) {
  const { t } = useTranslation();

  if (unavailable) {
    return (
      <div className="flex flex-col gap-2 border-t border-border p-4">
        <h3 className="text-sm font-semibold text-foreground">
          {t("skillsView.platform.title")}
        </h3>
        <p className="text-xs text-muted-foreground">
          {t("skillsView.platform.unavailable")}
        </p>
      </div>
    );
  }

  const installedById = new Map(
    installStatus.map((status) => [status.platformId, status.installed]),
  );
  const detected = new Set(detectedPlatforms);

  return (
    <div className="flex flex-col gap-2 border-t border-border p-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-foreground">
          {t("skillsView.platform.title")}
        </h3>
        <span className="text-xs text-muted-foreground">
          {t("skillsView.platform.detectedCount", { count: detectedPlatforms.length })}
        </span>
      </div>

      {platforms.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          {t("skillsView.platform.empty")}
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {platforms.map((platform) => {
            const installed = installedById.get(platform.id) ?? false;
            const isDetected = detected.has(platform.id);
            return (
              <li
                key={platform.id}
                className="flex items-center gap-2 rounded-md border border-border px-3 py-2"
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="flex items-center gap-1.5 text-sm font-medium text-foreground">
                    {platform.name}
                    {installed && (
                      <CheckCircle2Icon
                        className="h-3.5 w-3.5 text-primary"
                        aria-label={t("skillsView.platform.installed")}
                      />
                    )}
                  </span>
                  <span className="truncate text-[11px] text-muted-foreground">
                    {isDetected
                      ? t("skillsView.platform.detected")
                      : t("skillsView.platform.notDetected")}
                  </span>
                </span>
                {installed ? (
                  <button
                    type="button"
                    onClick={() => onUninstall(platform.id)}
                    className="flex shrink-0 items-center gap-1 rounded-md border border-input px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
                  >
                    <Trash2Icon className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("skillsView.platform.uninstall")}
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => onInstall(platform.id)}
                    className="flex shrink-0 items-center gap-1 rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground"
                  >
                    <DownloadIcon className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("skillsView.platform.install")}
                  </button>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
