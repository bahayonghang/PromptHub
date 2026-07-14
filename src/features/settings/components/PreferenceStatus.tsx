import { AlertCircleIcon, CheckIcon, LoaderCircleIcon, RotateCcwIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PreferenceSaveStatus } from "../settingsStore";

interface PreferenceStatusProps {
  status?: PreferenceSaveStatus;
  errorKey?: string;
  onRetry: () => void;
}

export function PreferenceStatus({ status = "idle", errorKey, onRetry }: PreferenceStatusProps) {
  const { t } = useTranslation();
  if (status === "idle") return null;

  if (status === "saving") {
    return (
      <span role="status" className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
        <LoaderCircleIcon className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
        {t("settingsView.preferences.saving")}
      </span>
    );
  }

  if (status === "saved") {
    return (
      <span role="status" className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
        <CheckIcon className="h-3.5 w-3.5" aria-hidden="true" />
        {t("settingsView.preferences.saved")}
      </span>
    );
  }

  return (
    <span role="alert" className="inline-flex flex-wrap items-center gap-1.5 text-xs text-destructive">
      <AlertCircleIcon className="h-3.5 w-3.5" aria-hidden="true" />
      {t(errorKey ?? "settingsView.preferences.unsaved")}
      <button
        type="button"
        onClick={onRetry}
        className="inline-flex items-center gap-1 rounded px-1 py-0.5 font-medium underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <RotateCcwIcon className="h-3 w-3" aria-hidden="true" />
        {t("settingsView.preferences.retry")}
      </button>
    </span>
  );
}
