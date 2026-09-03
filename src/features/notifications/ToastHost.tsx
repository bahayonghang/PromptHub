import { useTranslation } from "react-i18next";
import { AlertCircleIcon, CheckIcon, XIcon } from "lucide-react";
import { useToastStore } from "./toastStore";

export function ToastHost() {
  const { t } = useTranslation();
  const toasts = useToastStore((state) => state.toasts);
  const dismiss = useToastStore((state) => state.dismiss);

  return (
    <div
      role="status"
      aria-live="polite"
      aria-relevant="additions text"
      className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-[min(22rem,calc(100%-2rem))] flex-col gap-2"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`toast-item pointer-events-auto flex items-start gap-2 rounded-md border border-border bg-card px-3 py-2 text-sm text-card-foreground shadow-md ${
            toast.tone === "success"
              ? "border-success/40"
              : toast.tone === "danger"
                ? "border-destructive/40"
                : ""
          }`}
        >
          {toast.tone === "success" ? (
            <CheckIcon className="mt-0.5 h-4 w-4 shrink-0 text-success" aria-hidden="true" />
          ) : toast.tone === "danger" ? (
            <AlertCircleIcon className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
          ) : null}
          <p className="min-w-0 flex-1">{toast.message}</p>
          <button
            type="button"
            aria-label={t("promptsView.toast.dismiss")}
            onClick={() => dismiss(toast.id)}
            className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            <XIcon className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        </div>
      ))}
    </div>
  );
}
