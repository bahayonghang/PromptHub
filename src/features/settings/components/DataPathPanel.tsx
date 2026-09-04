import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertTriangleIcon,
  FolderInputIcon,
  HardDriveIcon,
  RotateCcwIcon,
} from "lucide-react";
import type {
  DataPathAction,
  DataPathStatus,
  PreviewResult,
  RecoverySource,
} from "../types";

interface DataPathPanelProps {
  status: DataPathStatus | null;
  preview: PreviewResult | null;
  recoverySources: RecoverySource[];
  recoveryUnavailable: boolean;
  onPreview: (targetPath: string) => void;
  onClearPreview: () => void;
  onApply: (targetPath: string, action: DataPathAction) => void;
  onRecoveryScan: () => void;
  onRecoveryApply: (sourcePath: string) => void;
}

const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-body text-foreground outline-none";

/**
 * Data-path panel (Req 19.3-19.10). Shows the active data directory and, when a
 * configured change is pending, a restart indicator (Req 19.3). Lets the user
 * enter a target path, run a read-only preview (Req 19.4), then apply a change
 * with the action the preview recommends — migrate/switch/overwrite (Req 19.5).
 * Every apply reports restart-required, surfaced as a banner. A recovery section
 * scans for recoverable data (Req 19.6) and applies it (Req 19.8); it degrades
 * gracefully when the recovery capability is unavailable (Req 3.7).
 */
export function DataPathPanel({
  status,
  preview,
  recoverySources,
  recoveryUnavailable,
  onPreview,
  onClearPreview,
  onApply,
  onRecoveryScan,
  onRecoveryApply,
}: DataPathPanelProps) {
  const { t } = useTranslation();
  const [target, setTarget] = useState("");
  const [scanned, setScanned] = useState(false);

  const handlePreview = () => {
    if (target.trim() !== "") onPreview(target.trim());
  };

  const handleScan = () => {
    setScanned(true);
    onRecoveryScan();
  };

  // The preview's recommended action drives the primary apply button; overwrite
  // is offered as a secondary action only when the target already has data.
  const recommended = (preview?.recommendedAction ?? "migrate") as DataPathAction;

  return (
    <div className="flex flex-col gap-6">
      {/* Active path + restart indicator */}
      <section className="flex flex-col gap-3">
        <div className="flex items-start gap-3 rounded-md border border-border p-4">
          <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
            <HardDriveIcon className="h-5 w-5" aria-hidden="true" />
          </span>
          <div className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="text-label font-medium text-muted-foreground">
              {t("settingsView.dataPath.activeLabel")}
            </span>
            <span className="break-all text-body text-foreground">
              {status?.activePath ?? t("common.loading")}
            </span>
          </div>
        </div>

        {status?.restartRequired && (
          <div
            role="status"
            className="flex items-center gap-2 rounded-md border border-primary/40 bg-state-selected px-3 py-2 text-body text-foreground"
          >
            <AlertTriangleIcon className="h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
            <span>
              {t("settingsView.dataPath.restartPending")}
              {status.configuredPath && (
                <span className="break-all font-medium"> {status.configuredPath}</span>
              )}
            </span>
          </div>
        )}
      </section>

      {/* Change data path: target input + preview */}
      <section className="flex flex-col gap-3">
        <div className="flex flex-col gap-0.5">
          <h3 className="text-body font-medium text-foreground">
            {t("settingsView.dataPath.changeTitle")}
          </h3>
          <p className="text-label text-muted-foreground">
            {t("settingsView.dataPath.changeHint")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <input
            value={target}
            placeholder={t("settingsView.dataPath.targetPlaceholder")}
            aria-label={t("settingsView.dataPath.targetPlaceholder")}
            onChange={(e) => {
              setTarget(e.target.value);
              if (preview) onClearPreview();
            }}
            className={inputClass}
          />
          <button
            type="button"
            onClick={handlePreview}
            disabled={target.trim() === ""}
            className="flex shrink-0 items-center gap-1.5 rounded-md border border-input px-3 py-2 text-body text-foreground hover:bg-accent disabled:opacity-50"
          >
            {t("settingsView.dataPath.previewButton")}
          </button>
        </div>

        {preview && (
          <div className="flex flex-col gap-3 rounded-md border border-border p-4">
            <dl className="grid grid-cols-2 gap-x-4 gap-y-1.5 text-label">
              <dt className="text-muted-foreground">{t("settingsView.dataPath.previewExists")}</dt>
              <dd className="text-foreground">
                {preview.exists ? t("common.success") : t("settingsView.dataPath.previewNo")}
              </dd>
              <dt className="text-muted-foreground">{t("settingsView.dataPath.previewHasData")}</dt>
              <dd className="text-foreground">
                {preview.hasPromptHubData
                  ? t("settingsView.dataPath.previewYes")
                  : t("settingsView.dataPath.previewNo")}
              </dd>
              <dt className="text-muted-foreground">{t("settingsView.dataPath.previewIsCurrent")}</dt>
              <dd className="text-foreground">
                {preview.isCurrent
                  ? t("settingsView.dataPath.previewYes")
                  : t("settingsView.dataPath.previewNo")}
              </dd>
              <dt className="text-muted-foreground">{t("settingsView.dataPath.previewRecommended")}</dt>
              <dd className="font-medium text-foreground">
                {t(`settingsView.dataPath.action.${preview.recommendedAction}`)}
              </dd>
            </dl>

            {preview.markers.length > 0 && (
              <ul className="flex flex-wrap gap-1.5">
                {preview.markers.map((marker) => (
                  <li
                    key={marker.path}
                    className="rounded-full bg-muted px-2 py-0.5 text-meta text-foreground"
                  >
                    {marker.name}
                  </li>
                ))}
              </ul>
            )}

            {preview.isCurrent ? (
              <p className="text-label text-muted-foreground">
                {t("settingsView.dataPath.previewAlreadyCurrent")}
              </p>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  onClick={() => onApply(preview.targetPath, recommended)}
                  className="flex items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-body font-medium text-primary-foreground"
                >
                  <FolderInputIcon className="h-4 w-4" aria-hidden="true" />
                  {t(`settingsView.dataPath.applyAction.${recommended}`)}
                </button>
                {preview.hasPromptHubData && (
                  <button
                    type="button"
                    onClick={() => onApply(preview.targetPath, "overwrite")}
                    className="flex items-center gap-1.5 rounded-md border border-destructive/40 px-4 py-2 text-body text-destructive hover:bg-destructive/10"
                  >
                    {t("settingsView.dataPath.applyAction.overwrite")}
                  </button>
                )}
              </div>
            )}
          </div>
        )}
      </section>

      {/* Recovery */}
      <section className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-2">
          <div className="flex flex-col gap-0.5">
            <h3 className="text-body font-medium text-foreground">
              {t("settingsView.dataPath.recoveryTitle")}
            </h3>
            <p className="text-label text-muted-foreground">
              {t("settingsView.dataPath.recoveryHint")}
            </p>
          </div>
          <button
            type="button"
            onClick={handleScan}
            className="flex shrink-0 items-center gap-1.5 rounded-md border border-input px-3 py-2 text-body text-foreground hover:bg-accent"
          >
            <RotateCcwIcon className="h-4 w-4" aria-hidden="true" />
            {t("settingsView.dataPath.recoveryScan")}
          </button>
        </div>

        {recoveryUnavailable ? (
          <p className="text-label text-muted-foreground">
            {t("settingsView.dataPath.recoveryUnavailable")}
          </p>
        ) : recoverySources.length > 0 ? (
          <ul className="flex flex-col gap-1.5">
            {recoverySources.map((source) => (
              <li
                key={source.path}
                className="flex items-center gap-2 rounded-md border border-border px-3 py-2"
              >
                <span className="min-w-0 flex-1 break-all text-body text-foreground">
                  {source.path}
                </span>
                <button
                  type="button"
                  onClick={() => onRecoveryApply(source.path)}
                  className="shrink-0 rounded-md bg-primary px-2.5 py-1.5 text-label font-medium text-primary-foreground"
                >
                  {t("settingsView.dataPath.recoveryApply")}
                </button>
              </li>
            ))}
          </ul>
        ) : (
          scanned && (
            <p className="text-label text-muted-foreground">
              {t("settingsView.dataPath.recoveryEmpty")}
            </p>
          )
        )}
      </section>
    </div>
  );
}
