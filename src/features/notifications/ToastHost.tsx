import { useTranslation } from "react-i18next";
import { AlertCircleIcon, CheckCircle2Icon, InfoIcon, XIcon } from "lucide-react";
import { useToastStore } from "./toastStore";
import type { ToastTone } from "./toastStore";

import { IconButton } from "../../components/ui";
import { cn } from "../../components/ui/cn";

/**
 * Tone drives the accent bar, the glyph, and the glyph colour together, so a
 * toast cannot end up with mismatched signals.
 */
const TONES: Record<ToastTone, { bar: string; icon: string; Icon: typeof InfoIcon }> = {
  info: { bar: "bg-primary", icon: "text-primary", Icon: InfoIcon },
  success: { bar: "bg-success", icon: "text-success", Icon: CheckCircle2Icon },
  danger: { bar: "bg-destructive", icon: "text-destructive", Icon: AlertCircleIcon },
};

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
      {toasts.map((toast) => {
        const spec = TONES[toast.tone] ?? TONES.info;
        const Icon = spec.Icon;
        return (
          <div
            key={toast.id}
            className={cn(
              "toast-item pointer-events-auto relative flex items-start gap-2.5",
              "overflow-hidden rounded-md border border-border bg-card py-2 pl-3.5 pr-2",
              "text-body text-card-foreground shadow-md",
            )}
          >
            {/* 2px tone bar: the status read at a glance, before the text. */}
            <span
              aria-hidden="true"
              className={cn("absolute inset-y-0 left-0 w-0.5", spec.bar)}
            />
            <Icon
              className={cn("mt-0.5 h-4 w-4 shrink-0", spec.icon)}
              aria-hidden="true"
            />
            <p className="min-w-0 flex-1 break-words">{toast.message}</p>
            <IconButton
              label={t("promptsView.toast.dismiss")}
              icon={<XIcon className="h-3.5 w-3.5" aria-hidden="true" />}
              size="xs"
              onClick={() => dismiss(toast.id)}
            />
          </div>
        );
      })}
    </div>
  );
}
